// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! The iPadOS menu bar, built with the iPadOS 26 `UIMainMenuSystem`.
//!
//! UIKit menus are immutable and pulled by the system: it asks the app to fill a
//! `UIMenuBuilder` whenever the menu bar needs rebuilding. This backend therefore keeps no
//! native objects around. It reads the shared menu state when UIKit builds the menu bar, and
//! every change to an item just marks the menu bar as needing a rebuild.
//!
//! Context menus are shown as an edit menu, the one public UIKit menu that can be presented at
//! a point from code.

mod icon;

pub(crate) use icon::PlatformIcon;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::c_void,
    ptr::NonNull,
    rc::Rc,
};

use block2::RcBlock;
use dpi::Position;
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Imp, NSObject, NSObjectProtocol, ProtocolObject, Sel},
    sel, DefinedClass, MainThreadMarker, MainThreadOnly, Message,
};
use objc2_core_foundation::CGPoint;
use objc2_foundation::{NSArray, NSCopying, NSNumber, NSString};
use objc2_ui_kit::{
    UIAction, UICommand, UIEditMenuConfiguration, UIEditMenuInteraction,
    UIEditMenuInteractionAnimating, UIEditMenuInteractionDelegate, UIImage, UIKeyCommand,
    UIMainMenuSystem, UIMainMenuSystemConfiguration, UIMenu, UIMenuAbout, UIMenuApplication,
    UIMenuBringAllToFront, UIMenuBuilder, UIMenuClose, UIMenuElement, UIMenuElementAttributes,
    UIMenuElementState, UIMenuFullscreen, UIMenuHide, UIMenuIdentifier, UIMenuMinimizeAndZoom,
    UIMenuOptions, UIMenuQuit, UIMenuRoot, UIMenuServices, UIMenuSpeech, UIView,
};

use crate::{
    accelerator::MenuAccelerator,
    items::{IconType, MenuItemAction, PredefinedMenuItemType},
    util::{strip_mnemonic, AddOp},
    Menu, MenuEvent, MenuId, MenuItemKind,
};

/// The tallest a custom icon is drawn in a menu row, matching the macOS backend.
const MAX_ICON_HEIGHT: f64 = 18.0;

thread_local! {
    /// The menu shown in the menu bar, see [`Menu::init_for_uiapp`].
    static APP_MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
    /// Whether the build handler has been handed to the main menu system.
    static INSTALLED: Cell<bool> = const { Cell::new(false) };
    /// The items behind the commands of the latest menu bar build.
    static COMMANDS: RefCell<Commands> = RefCell::new(Commands::default());
    /// The context menu being shown, see [`show_context_menu_for_uiview`].
    static CONTEXT_MENU: RefCell<Option<ContextMenuPresentation>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct Commands {
    /// Bumped on every build, so a command from a previous build cannot reach the item now at
    /// its index.
    generation: u32,
    items: Vec<Rc<RefCell<PlatformMenuItem>>>,
}

impl Commands {
    fn get(&self, key: u64) -> Option<Rc<RefCell<PlatformMenuItem>>> {
        let (generation, index) = ((key >> 32) as u32, key as u32);
        (generation == self.generation)
            .then(|| self.items.get(index as usize).cloned())
            .flatten()
    }
}

#[cfg(feature = "snapshot")]
pub(crate) fn dispatch_on_main_thread<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    // TODO: Dispatch the callback to the iOS main thread instead of running it inline.
    f();
}

/// Runs the action of a predefined menu item activated from a menu snapshot. Must be called on
/// the main thread.
#[cfg(feature = "snapshot")]
pub(crate) fn run_predefined_action(_item_type: &crate::items::PredefinedMenuItemType) {
    // TODO: run predefined actions from a snapshot on this platform.
}

pub(crate) fn set_app_menu(menu: Option<Menu>) {
    let previous = APP_MENU.replace(menu);
    drop(previous);

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if !objc2::available!(ios = 26.0) {
        return;
    }

    if INSTALLED.replace(true) {
        UIMainMenuSystem::sharedSystem(mtm).setNeedsRebuild();
    } else {
        install(mtm);
    }
}

pub(crate) fn remove_app_menu(id: &MenuId) {
    let is_app_menu = APP_MENU.with_borrow(|menu| menu.as_ref().is_some_and(|m| m.id() == id));
    if is_app_menu {
        set_app_menu(None);
    }
}

/// Marks the menu bar as needing a rebuild, if it shows a muda menu.
///
/// UIKit coalesces these and rebuilds the menu bar once, on a later run loop pass, so the
/// shared state has caught up by the time [`build`] reads it.
fn request_rebuild() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if INSTALLED.get() && APP_MENU.with_borrow(Option::is_some) {
        UIMainMenuSystem::sharedSystem(mtm).setNeedsRebuild();
    }
}

fn install(mtm: MainThreadMarker) {
    // Commands are nil-targeted, so their action has to be implemented by a responder in the
    // chain. The application is always in it, whatever the first responder is.
    let types = c"v@:@";
    let imp: unsafe extern "C-unwind" fn(&AnyObject, Sel, Option<&AnyObject>) = perform_command;
    unsafe {
        objc2::ffi::class_addMethod(
            AnyClass::get(c"UIApplication").unwrap() as *const AnyClass as *mut AnyClass,
            sel!(mudaPerformCommand:),
            std::mem::transmute::<
                unsafe extern "C-unwind" fn(&AnyObject, Sel, Option<&AnyObject>),
                Imp,
            >(imp),
            types.as_ptr(),
        );
    }

    let build_handler = RcBlock::new(|builder: NonNull<ProtocolObject<dyn UIMenuBuilder>>| {
        // SAFETY: UIKit hands a valid builder to the build handler for the duration of the call.
        build(unsafe { builder.as_ref() })
    });
    let configuration = UIMainMenuSystemConfiguration::new(mtm);
    UIMainMenuSystem::sharedSystem(mtm)
        .setBuildConfiguration_buildHandler(&configuration, Some(&build_handler));
}

unsafe extern "C-unwind" fn perform_command(
    _this: &AnyObject,
    _cmd: Sel,
    sender: Option<&AnyObject>,
) {
    let key = sender
        .and_then(|sender| sender.downcast_ref::<UICommand>())
        .and_then(|command| command.propertyList())
        .and_then(|list| {
            list.downcast_ref::<NSNumber>()
                .map(|n| n.unsignedLongLongValue())
        });
    if let Some(item) = key.and_then(|key| COMMANDS.with_borrow(|commands| commands.get(key))) {
        activate(&item);
    }
}

/// Runs the action of an item picked from the menu bar or a context menu.
fn activate(item: &RefCell<PlatformMenuItem>) {
    let click = item.borrow().click.clone();
    match click {
        MenuItemAction::Emit(id) => MenuEvent::send(MenuEvent { id }),
        MenuItemAction::Toggle(id, state) => {
            if let Some(state) = state.upgrade() {
                let mut state = state.borrow_mut();
                state.checked = !state.checked;
            }
            request_rebuild();
            MenuEvent::send(MenuEvent { id });
        }
        MenuItemAction::Predefined(_) => {
            unreachable!("predefined menu items use their UIKit action")
        }
    }
}

/// Fills the menu bar with the app menu.
///
/// The first submenu fills the application menu, the one titled with the app name, as it does
/// on macOS. The other submenus replace the system's File, Edit, View, Window and Help menus.
fn build(builder: &ProtocolObject<dyn UIMenuBuilder>) {
    let Some(menu) = APP_MENU.with_borrow(Clone::clone) else {
        return;
    };
    let mtm = MainThreadMarker::new().expect("UIKit builds the menu bar on the main thread");

    let mut context = BuildContext {
        mtm,
        target: BuildTarget::MenuBar,
        system_commands: HashMap::new(),
        system_menus: HashMap::new(),
        used_sources: Vec::new(),
        commands: Commands {
            generation: COMMANDS.with_borrow(|commands| commands.generation.wrapping_add(1)),
            items: Vec::new(),
        },
    };

    let root_identifier = unsafe { UIMenuRoot };
    let application_identifier = unsafe { UIMenuApplication };

    let root = builder.menuForIdentifier(root_identifier);
    if let Some(root) = &root {
        context.collect_system_elements(root);
    }

    let mut submenus = menu
        .items()
        .into_iter()
        .filter(|item| item.as_submenu().is_some());

    let application_children = submenus.next().map(|submenu| {
        let enabled = submenu.platform_attach_args().enabled;
        context.elements(&submenu.items(), enabled)
    });
    let menus: Vec<_> = submenus
        .map(|submenu| context.menu(&submenu, true))
        .collect();

    // Only mutate the builder once every element is built: a command taken from a system menu
    // can't be inserted while that menu still holds it.
    for child in root.iter().flat_map(|root| root.children().to_vec()) {
        let Some(child) = child.downcast_ref::<UIMenu>() else {
            continue;
        };
        let identifier = child.identifier();
        if &*identifier != application_identifier {
            builder.removeMenuForIdentifier(&identifier);
        }
    }

    let mut anchor: Option<Retained<UIMenuIdentifier>> = None;
    if let Some(children) = application_children {
        let children = NSArray::from_retained_slice(&children);
        if let Some(application) = builder.menuForIdentifier(application_identifier) {
            builder.replaceMenuForIdentifier_withMenu(
                application_identifier,
                &application.menuByReplacingChildren(&children),
            );
            anchor = Some(application_identifier.copy());
        } else {
            let menu = UIMenu::menuWithTitle_image_identifier_options_children(
                &NSString::new(),
                None,
                None,
                UIMenuOptions::empty(),
                &children,
                mtm,
            );
            builder.insertChildMenu_atStartOfMenuForIdentifier(&menu, root_identifier);
            anchor = Some(menu.identifier());
        }
    }

    for menu in menus {
        match &anchor {
            Some(anchor) => builder.insertSiblingMenu_afterMenuForIdentifier(&menu, anchor),
            None => builder.insertChildMenu_atStartOfMenuForIdentifier(&menu, root_identifier),
        }
        anchor = Some(menu.identifier());
    }

    COMMANDS.set(context.commands);
}

/// What a [`BuildContext`] builds elements for.
#[derive(Clone, Copy, PartialEq)]
enum BuildTarget {
    /// The menu bar, where items are nil-targeted commands, so they can carry key commands.
    MenuBar,
    /// A context menu, where items are actions that hold on to their item.
    ContextMenu,
}

struct BuildContext {
    mtm: MainThreadMarker,
    target: BuildTarget,
    /// The commands of the system menus, by action, for the predefined items to reuse.
    system_commands: HashMap<Sel, Retained<UICommand>>,
    /// The system menus, by identifier, for the predefined items to reuse.
    system_menus: HashMap<String, Retained<UIMenu>>,
    /// The sources of the predefined items built so far. UIKit refuses a menu that holds the
    /// same command or menu twice.
    used_sources: Vec<PredefinedSource>,
    commands: Commands,
}

impl BuildContext {
    fn collect_system_elements(&mut self, menu: &UIMenu) {
        for child in menu.children().iter() {
            if let Some(menu) = child.downcast_ref::<UIMenu>() {
                self.system_menus
                    .entry(menu.identifier().to_string())
                    .or_insert_with(|| menu.retain());
                self.collect_system_elements(menu);
            } else if let Some(command) = child.downcast_ref::<UICommand>() {
                let action = unsafe { command.action() };
                self.system_commands
                    .entry(action)
                    .or_insert_with(|| command.retain());
            }
        }
    }

    /// Builds `items`, turning the runs between separators and section headers into inline
    /// sections, UIKit's equivalent of separators.
    fn elements(&mut self, items: &[MenuItemKind], enabled: bool) -> Vec<Retained<UIMenuElement>> {
        let mut sections: Vec<(Option<String>, Vec<Retained<UIMenuElement>>)> =
            vec![(None, vec![])];

        for item in items {
            let predefined_item_type = item
                .as_predefined_menuitem()
                .map(|item| item.state.borrow().predefined_item_type.clone());
            match predefined_item_type {
                Some(PredefinedMenuItemType::Separator) => sections.push((None, vec![])),
                Some(PredefinedMenuItemType::SectionHeader) => {
                    let title = strip_mnemonic(item.platform_attach_args().text);
                    sections.push((Some(title), vec![]));
                }
                _ => {
                    if let Some(element) = self.element(item, enabled) {
                        sections.last_mut().unwrap().1.push(element);
                    }
                }
            }
        }

        if let [(None, _)] = sections.as_slice() {
            return sections.pop().unwrap().1;
        }

        sections
            .into_iter()
            .filter(|(_, children)| !children.is_empty())
            .map(|(title, mut children)| {
                // A system menu reused for a predefined item is already its own section
                if title.is_none() && children.len() == 1 {
                    let is_section = children[0]
                        .downcast_ref::<UIMenu>()
                        .is_some_and(|menu| menu.options().contains(UIMenuOptions::DisplayInline));
                    if is_section {
                        return children.pop().unwrap();
                    }
                }

                let menu = UIMenu::menuWithTitle_image_identifier_options_children(
                    &NSString::from_str(title.as_deref().unwrap_or_default()),
                    None,
                    None,
                    UIMenuOptions::DisplayInline,
                    &NSArray::from_retained_slice(&children),
                    self.mtm,
                );
                Retained::into_super(menu)
            })
            .collect()
    }

    fn element(&mut self, item: &MenuItemKind, enabled: bool) -> Option<Retained<UIMenuElement>> {
        match item {
            MenuItemKind::Submenu(_) => Some(Retained::into_super(self.menu(item, enabled))),
            MenuItemKind::Predefined(_) => self.predefined_element(item, enabled),
            MenuItemKind::MenuItem(_) | MenuItemKind::Check(_) | MenuItemKind::Icon(_) => {
                Some(self.command(item, enabled))
            }
        }
    }

    fn menu(&mut self, item: &MenuItemKind, enabled: bool) -> Retained<UIMenu> {
        let args = item.platform_attach_args();
        let children = self.elements(&item.items(), enabled && args.enabled);
        UIMenu::menuWithTitle_image_identifier_options_children(
            &NSString::from_str(&strip_mnemonic(&args.text)),
            image(args.icon.as_ref()).as_deref(),
            None,
            UIMenuOptions::empty(),
            &NSArray::from_retained_slice(&children),
            self.mtm,
        )
    }

    fn command(&mut self, item: &MenuItemKind, enabled: bool) -> Retained<UIMenuElement> {
        let args = item.platform_attach_args();
        let title = NSString::from_str(&strip_mnemonic(&args.text));
        let image = image(args.icon.as_ref());
        let attributes = if enabled && args.enabled {
            UIMenuElementAttributes::empty()
        } else {
            UIMenuElementAttributes::Disabled
        };
        let state = if args.checked {
            UIMenuElementState::On
        } else {
            UIMenuElementState::Off
        };

        match self.target {
            BuildTarget::MenuBar => {
                let key =
                    (u64::from(self.commands.generation) << 32) | self.commands.items.len() as u64;
                self.commands.items.push(item.platform());
                let property_list = NSNumber::new_u64(key);

                let command = new_command(
                    &title,
                    image.as_deref(),
                    sel!(mudaPerformCommand:),
                    Some(&property_list),
                    args.accelerator.as_ref(),
                    self.mtm,
                );
                command.setAttributes(attributes);
                command.setState(state);
                Retained::into_super(command)
            }
            BuildTarget::ContextMenu => {
                let item = item.platform();
                let handler = RcBlock::new(move |_action| activate(&item));
                let action = unsafe {
                    UIAction::actionWithTitle_image_identifier_handler(
                        &title,
                        image.as_deref(),
                        None,
                        RcBlock::as_ptr(&handler),
                        self.mtm,
                    )
                };
                action.setAttributes(attributes);
                action.setState(state);
                Retained::into_super(action)
            }
        }
    }

    /// Builds a predefined item from the system's own element, so it keeps its localized title
    /// and the system's handling of it.
    ///
    /// The edit actions are implemented by the web view whether or not the system offers them,
    /// so they are built from scratch when it doesn't. The other kinds are skipped.
    fn predefined_element(
        &mut self,
        item: &MenuItemKind,
        enabled: bool,
    ) -> Option<Retained<UIMenuElement>> {
        let predefined = item.as_predefined_menuitem()?;
        let (text, predefined_item_type) = {
            let state = predefined.state.borrow();
            (state.text.clone(), state.predefined_item_type.clone())
        };
        let source = predefined_source(&predefined_item_type)?;
        if self.used_sources.contains(&source) {
            return None;
        }

        let element = match source {
            PredefinedSource::Action(action) => {
                let command = match self.system_commands.get(&action) {
                    Some(command) => command.copy(),
                    None if is_edit_action(&predefined_item_type) => new_command(
                        &NSString::from_str(&strip_mnemonic(&text)),
                        None,
                        action,
                        None,
                        predefined_item_type.accelerator().as_ref(),
                        self.mtm,
                    ),
                    None => return None,
                };

                if text != predefined_item_type.default_text(None) {
                    command.setTitle(&NSString::from_str(&strip_mnemonic(&text)));
                }
                if !(enabled && item.platform_attach_args().enabled) {
                    command.setAttributes(command.attributes() | UIMenuElementAttributes::Disabled);
                }

                Retained::into_super(command)
            }
            // The system fills these in when it shows the menu bar, wherever they are placed.
            PredefinedSource::Menu(identifier) => {
                let menu = self.system_menus.get(&identifier.to_string())?;
                Retained::into_super(menu.clone())
            }
        };
        self.used_sources.push(source);

        Some(element)
    }
}

fn new_command(
    title: &NSString,
    image: Option<&UIImage>,
    action: Sel,
    property_list: Option<&AnyObject>,
    accelerator: Option<&MenuAccelerator>,
    mtm: MainThreadMarker,
) -> Retained<UICommand> {
    let key = accelerator.and_then(|accelerator| {
        let input = accelerator.key_input().ok()?;
        Some((input, accelerator.modifier_flags()))
    });

    unsafe {
        match key {
            Some((input, modifier_flags)) => Retained::into_super(
                UIKeyCommand::commandWithTitle_image_action_input_modifierFlags_propertyList(
                    title,
                    image,
                    action,
                    &input,
                    modifier_flags,
                    property_list,
                    mtm,
                ),
            ),
            None => UICommand::commandWithTitle_image_action_propertyList(
                title,
                image,
                action,
                property_list,
                mtm,
            ),
        }
    }
}

fn image(icon: Option<&IconType>) -> Option<Retained<UIImage>> {
    match icon {
        Some(IconType::Custom(icon)) => icon.inner.to_uiimage(MAX_ICON_HEIGHT),
        // `NativeIcon` names AppKit images, which UIKit doesn't have.
        Some(IconType::Native(_)) | None => None,
    }
}

/// Where the UIKit element of a predefined item comes from.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PredefinedSource {
    /// A system command, found by its action.
    Action(Sel),
    /// A system menu, found by its identifier. On iPadOS, UIKit only fills these in when it
    /// shows the menu bar, so they are reused whole.
    Menu(&'static UIMenuIdentifier),
}

/// The UIKit element of a predefined item, or `None` if the system has no such element.
pub(crate) fn predefined_source(
    predefined_item_type: &PredefinedMenuItemType,
) -> Option<PredefinedSource> {
    use PredefinedMenuItemType::*;
    use PredefinedSource::{Action, Menu};

    let source = unsafe {
        match predefined_item_type {
            Copy => Action(sel!(copy:)),
            Cut => Action(sel!(cut:)),
            Paste => Action(sel!(paste:)),
            PasteAndMatchStyle => Action(sel!(pasteAndMatchStyle:)),
            Delete => Action(sel!(delete:)),
            SelectAll => Action(sel!(selectAll:)),
            Undo => Action(sel!(undo:)),
            Redo => Action(sel!(redo:)),
            About(_) => Menu(UIMenuAbout),
            Services => Menu(UIMenuServices),
            Hide | HideOthers | ShowAll => Menu(UIMenuHide),
            Quit => Menu(UIMenuQuit),
            CloseWindow => Menu(UIMenuClose),
            Fullscreen => Menu(UIMenuFullscreen),
            Minimize | Maximize => Menu(UIMenuMinimizeAndZoom),
            BringAllToFront => Menu(UIMenuBringAllToFront),
            StartSpeaking | StopSpeaking => Menu(UIMenuSpeech),
            _ => return None,
        }
    };
    Some(source)
}

fn is_edit_action(predefined_item_type: &PredefinedMenuItemType) -> bool {
    use PredefinedMenuItemType::*;

    matches!(
        predefined_item_type,
        Copy | Cut | Paste | PasteAndMatchStyle | Delete | SelectAll
    )
}

/// Shows `items` as a context menu over `view`, at `position` in the view or at its center.
///
/// # Safety
///
/// `view` must be a valid `UIView`.
pub(crate) unsafe fn show_context_menu_for_uiview(
    items: &[MenuItemKind],
    enabled: bool,
    view: *const c_void,
    position: Option<Position>,
) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    if !objc2::available!(ios = 16.0) {
        return false;
    }
    // SAFETY: Caller verifies that the view is valid.
    let view: &UIView = unsafe { &*view.cast() };

    let mut context = BuildContext {
        mtm,
        target: BuildTarget::ContextMenu,
        system_commands: HashMap::new(),
        system_menus: HashMap::new(),
        used_sources: Vec::new(),
        commands: Commands::default(),
    };
    let children = context.elements(items, enabled);
    let menu = UIMenu::menuWithTitle_image_identifier_options_children(
        &NSString::new(),
        None,
        None,
        UIMenuOptions::empty(),
        &NSArray::from_retained_slice(&children),
        mtm,
    );

    let source_point = match position {
        Some(position) => {
            let position = position.to_logical::<f64>(view.contentScaleFactor());
            CGPoint::new(position.x, position.y)
        }
        None => {
            let bounds = view.bounds();
            CGPoint::new(
                bounds.origin.x + bounds.size.width / 2.0,
                bounds.origin.y + bounds.size.height / 2.0,
            )
        }
    };

    remove_context_menu();

    let delegate = ContextMenuDelegate::new(mtm, menu);
    let interaction = UIEditMenuInteraction::initWithDelegate(
        mtm.alloc(),
        Some(ProtocolObject::from_ref(&*delegate)),
    );
    view.addInteraction(ProtocolObject::from_ref(&*interaction));
    let configuration = unsafe {
        UIEditMenuConfiguration::configurationWithIdentifier_sourcePoint(None, source_point, mtm)
    };
    interaction.presentEditMenuWithConfiguration(&configuration);

    CONTEXT_MENU.set(Some(ContextMenuPresentation {
        view: view.retain(),
        interaction,
        _delegate: delegate,
    }));

    // The menu is shown asynchronously, so nothing has been picked yet.
    false
}

struct ContextMenuPresentation {
    view: Retained<UIView>,
    interaction: Retained<UIEditMenuInteraction>,
    /// The interaction only holds a weak reference to its delegate.
    _delegate: Retained<ContextMenuDelegate>,
}

/// Dismisses the context menu being shown, if any, and removes its interaction from its view.
fn remove_context_menu() {
    if let Some(presentation) = CONTEXT_MENU.take() {
        presentation.interaction.dismissMenu();
        presentation
            .view
            .removeInteraction(ProtocolObject::from_ref(&*presentation.interaction));
    }
}

define_class!(
    /// Hands a context menu to the edit menu interaction showing it.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Retained<UIMenu>]
    struct ContextMenuDelegate;

    unsafe impl NSObjectProtocol for ContextMenuDelegate {}

    unsafe impl UIEditMenuInteractionDelegate for ContextMenuDelegate {
        #[unsafe(method_id(editMenuInteraction:menuForConfiguration:suggestedActions:))]
        fn menu_for_configuration(
            &self,
            _interaction: &UIEditMenuInteraction,
            _configuration: &UIEditMenuConfiguration,
            _suggested_actions: &NSArray<UIMenuElement>,
        ) -> Option<Retained<UIMenu>> {
            Some(self.ivars().clone())
        }

        #[unsafe(method(editMenuInteraction:willDismissMenuForConfiguration:animator:))]
        fn will_dismiss_menu(
            &self,
            interaction: &UIEditMenuInteraction,
            _configuration: &UIEditMenuConfiguration,
            animator: &ProtocolObject<dyn UIEditMenuInteractionAnimating>,
        ) {
            let interaction = interaction.retain();
            let completion = RcBlock::new(move || {
                let is_current = CONTEXT_MENU.with_borrow(|presentation| {
                    presentation
                        .as_ref()
                        .is_some_and(|p| std::ptr::eq(&*p.interaction, &*interaction))
                });
                if is_current {
                    remove_context_menu();
                }
            });
            animator.addCompletion(&completion);
        }
    }
);

impl ContextMenuDelegate {
    fn new(mtm: MainThreadMarker, menu: Retained<UIMenu>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(menu);
        unsafe { msg_send![super(this), init] }
    }
}

pub(crate) struct PlatformMenu;

impl PlatformMenu {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn attach(&mut self, child: &MenuItemKind, _op: AddOp) -> crate::Result<()> {
        validate_accelerator(child)?;
        request_rebuild();
        Ok(())
    }

    pub(crate) fn remove_at(&mut self, _index: usize, _child: &MenuItemKind) {
        request_rebuild();
    }
}

/// Rejects an accelerator UIKit has no key input for, as the other backends do when the item
/// is added.
fn validate_accelerator(item: &MenuItemKind) -> crate::Result<()> {
    if let Some(accelerator) = item.platform_attach_args().accelerator {
        if !matches!(item, MenuItemKind::Predefined(_)) {
            accelerator.key_input()?;
        }
    }
    Ok(())
}

pub(crate) struct PlatformMenuItem {
    click: MenuItemAction,
}

impl PlatformMenuItem {
    pub(crate) fn new(click: MenuItemAction) -> Self {
        Self { click }
    }

    pub(crate) fn new_submenu(click: MenuItemAction) -> Self {
        Self { click }
    }

    pub(crate) fn text(&self) -> Option<String> {
        None
    }

    pub(crate) fn set_text(&mut self, _text: &str, _accelerator: Option<&MenuAccelerator>) {
        request_rebuild();
    }

    pub(crate) fn is_enabled(&self) -> Option<bool> {
        None
    }

    pub(crate) fn set_enabled(&mut self, _enabled: bool) {
        request_rebuild();
    }

    pub(crate) fn set_accelerator(
        &mut self,
        _text: &str,
        accelerator: Option<&MenuAccelerator>,
    ) -> crate::Result<()> {
        if let Some(accelerator) = accelerator {
            accelerator.key_input()?;
        }
        request_rebuild();
        Ok(())
    }

    pub(crate) fn is_checked(&self) -> Option<bool> {
        None
    }

    pub(crate) fn set_checked(&mut self, _checked: bool) {
        request_rebuild();
    }

    pub(crate) fn set_icon(&mut self, _icon: Option<&IconType>) {
        request_rebuild();
    }

    pub(crate) fn attach(&mut self, child: &MenuItemKind, _op: AddOp) -> crate::Result<()> {
        validate_accelerator(child)?;
        request_rebuild();
        Ok(())
    }

    pub(crate) fn remove_at(&mut self, _index: usize, _child: &MenuItemKind) {
        request_rebuild();
    }
}
