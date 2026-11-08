// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Qt Widgets backend (`qt` feature), built on [`qtshim::menu`].
//!
//! Instead of mirroring every incremental change into each Qt widget instance, this backend
//! keeps the item tree that muda already owns as the source of truth: any change schedules a
//! single, coalesced rebuild of every attached `QMenuBar` on the Qt event loop. Context menus are
//! built on demand. Clicks reuse [`MenuItemAction`], so events, check toggling and predefined
//! items behave like the other backends.
//!
//! The host application must have created a `QApplication` through `qtshim` (e.g. a Qt-based
//! Tauri runtime). Window pointers passed in are validated with [`Widget::from_ptr`], and calls
//! made off the Qt thread (or without a running application) fail instead of touching Qt.

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    rc::{Rc, Weak},
};

use dpi::Position;
use qtshim::{
    app,
    menu::{
        self, ActionOptions, Container, Icon, MenuBar, PopupMenu, PopupPosition, StandardAction,
        Widget,
    },
};

use crate::{
    accelerator::MenuAccelerator,
    icon::RgbaIcon,
    items::{IconType, MenuItemAction, PredefinedMenuItemType},
    util::AddOp,
    MenuEvent, MenuItemKind, TextStyle,
};

pub(crate) type PlatformIcon = RgbaIcon;

#[cfg(feature = "snapshot")]
pub(crate) fn dispatch_on_main_thread<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    // Called from other threads (e.g. by tray-icon), so this must be `dispatch`, not `post`.
    app::dispatch(f);
}

// -------------------------------------------------------------------------------------------
// attached menubars and coalesced rebuilds

/// A menu's items, shared between the menu and the menubars showing it.
type Children = Rc<RefCell<Vec<MenuItemKind>>>;

/// A menubar installed on a window: the window (gone once destroyed) and the menu's items.
struct Attached {
    window: Weak<Widget>,
    children: Weak<RefCell<Vec<MenuItemKind>>>,
}

thread_local! {
    static ATTACHED: RefCell<Vec<Attached>> = const { RefCell::new(Vec::new()) };
    static REBUILD_PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Rebuild every attached menubar once the current call stack has unwound. Deferring also means
/// the rebuild reads the final item state, whatever order muda updated state and platform in.
fn schedule_rebuild() {
    // Items can change before the application exists or after it's gone; nothing is shown then.
    if !app::is_running() || REBUILD_PENDING.with(|p| p.replace(true)) {
        return;
    }
    app::post(rebuild_all);
}

fn rebuild_all() {
    REBUILD_PENDING.with(|p| p.set(false));
    let attached: Vec<(Rc<Widget>, Children)> = ATTACHED.with(|a| {
        let mut a = a.borrow_mut();
        a.retain(|x| x.window.strong_count() > 0 && x.children.strong_count() > 0);
        a.iter()
            .filter_map(|x| Some((x.window.upgrade()?, x.children.upgrade()?)))
            .collect()
    });
    for (window, children) in attached {
        build_menubar(&window, &children.borrow());
    }
}

fn build_menubar(window: &Widget, items: &[MenuItemKind]) {
    let Some(bar) = MenuBar::get_or_create(window) else {
        return;
    };
    bar.clear();
    build_items(&bar, items, true);
}

// -------------------------------------------------------------------------------------------
// building Qt menus from muda's item tree

fn icon_of(icon: &Option<IconType>) -> Option<Icon<'_>> {
    match icon {
        Some(IconType::Custom(icon)) => Some(Icon {
            rgba: &icon.inner.rgba,
            width: icon.inner.width,
            height: icon.inner.height,
        }),
        _ => None,
    }
}

/// The Qt action for a predefined item, if the backend can perform it.
fn standard_action(t: &PredefinedMenuItemType) -> Option<StandardAction> {
    use PredefinedMenuItemType as P;
    Some(match t {
        P::Copy => StandardAction::Copy,
        P::Cut => StandardAction::Cut,
        P::Paste => StandardAction::Paste,
        P::SelectAll => StandardAction::SelectAll,
        P::Undo => StandardAction::Undo,
        P::Redo => StandardAction::Redo,
        P::Delete => StandardAction::Delete,
        P::Minimize => StandardAction::Minimize,
        P::Maximize => StandardAction::Maximize,
        P::Fullscreen => StandardAction::Fullscreen,
        P::Hide => StandardAction::Hide,
        P::CloseWindow => StandardAction::CloseWindow,
        P::Quit => StandardAction::Quit,
        _ => return None,
    })
}

fn activate(action: &MenuItemAction) {
    match action {
        MenuItemAction::Emit(id) => MenuEvent::send(MenuEvent { id: id.clone() }),
        MenuItemAction::Toggle(id, state) => {
            if let Some(state) = state.upgrade() {
                let mut state = state.borrow_mut();
                state.checked = !state.checked;
            }
            MenuEvent::send(MenuEvent { id: id.clone() });
            schedule_rebuild();
        }
        MenuItemAction::Predefined(state) => {
            let Some(t) = state
                .upgrade()
                .map(|s| s.borrow().predefined_item_type.clone())
            else {
                return;
            };
            if let PredefinedMenuItemType::About(Some(metadata)) = &t {
                show_about_dialog(metadata);
            } else if let Some(action) = standard_action(&t) {
                action.trigger();
            }
        }
    }
}

fn on_click(action: MenuItemAction) -> Option<Box<dyn Fn()>> {
    Some(Box::new(move || activate(&action)))
}

/// Fill a menubar or menu with `items`. Shortcuts are registered only in menubars
/// (`register_shortcuts`); context menus just display them.
fn build_items(container: &impl Container, items: &[MenuItemKind], register_shortcuts: bool) {
    for item in items {
        let args = item.platform_attach_args();
        let shortcut = args.accelerator.as_ref().and_then(MenuAccelerator::to_qt);
        let (shortcut, shortcut_hint) = if register_shortcuts {
            (shortcut.as_deref(), None)
        } else {
            (None, shortcut.as_deref())
        };
        match item {
            MenuItemKind::Submenu(_) => {
                if let Some(menu) =
                    container.add_menu(&args.text, args.enabled, icon_of(&args.icon))
                {
                    build_items(&menu, &item.items(), register_shortcuts);
                }
            }
            MenuItemKind::Predefined(p) => {
                let t = p.state.borrow().predefined_item_type.clone();
                match &t {
                    PredefinedMenuItemType::Separator => container.add_separator(),
                    PredefinedMenuItemType::About(_) => container.add_action(
                        &ActionOptions {
                            text: &args.text,
                            ..ActionOptions::default()
                        },
                        on_click(item.click_action()),
                    ),
                    t => match standard_action(t) {
                        // Editing items only display their shortcut: the focused widget (e.g.
                        // a web view) handles the key itself.
                        Some(action) => container.add_action(
                            &ActionOptions {
                                text: &args.text,
                                shortcut: shortcut.filter(|_| !action.is_editing()),
                                shortcut_hint: shortcut_hint
                                    .or(shortcut.filter(|_| action.is_editing())),
                                ..ActionOptions::default()
                            },
                            on_click(item.click_action()),
                        ),
                        // macOS-only items (Services, Show All, ...) render disabled, like on GTK.
                        None => container.add_action(
                            &ActionOptions {
                                text: &args.text,
                                enabled: false,
                                ..ActionOptions::default()
                            },
                            None,
                        ),
                    },
                }
            }
            MenuItemKind::Check(_) => container.add_action(
                &ActionOptions {
                    text: &args.text,
                    enabled: args.enabled,
                    checkable: true,
                    checked: args.checked,
                    shortcut,
                    shortcut_hint,
                    icon: None,
                },
                on_click(item.click_action()),
            ),
            MenuItemKind::MenuItem(_) | MenuItemKind::Icon(_) => container.add_action(
                &ActionOptions {
                    text: &args.text,
                    enabled: args.enabled,
                    shortcut,
                    shortcut_hint,
                    icon: icon_of(&args.icon),
                    ..ActionOptions::default()
                },
                on_click(item.click_action()),
            ),
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn show_about_dialog(metadata: &crate::AboutMetadata) {
    let name = metadata.name.clone().unwrap_or_default();
    let mut html = format!("<h3>{}</h3>", html_escape(&name));
    if let Some(version) = metadata.full_version() {
        html.push_str(&format!("<p>Version {}</p>", html_escape(&version)));
    }
    if let Some(comments) = &metadata.comments {
        html.push_str(&format!("<p>{}</p>", html_escape(comments)));
    }
    if let Some(website) = &metadata.website {
        let label = metadata.website_label.as_deref().unwrap_or(website);
        html.push_str(&format!(
            "<p><a href=\"{}\">{}</a></p>",
            html_escape(website),
            html_escape(label)
        ));
    }
    if let Some(authors) = &metadata.authors {
        html.push_str(&format!(
            "<p>Authors: {}</p>",
            html_escape(&authors.join(", "))
        ));
    }
    if let Some(copyright) = &metadata.copyright {
        html.push_str(&format!("<p>{}</p>", html_escape(copyright)));
    }
    if let Some(license) = &metadata.license {
        html.push_str(&format!("<p>License: {}</p>", html_escape(license)));
    }
    let icon = metadata.icon.as_ref().map(|i| Icon {
        rgba: &i.inner.rgba,
        width: i.inner.width,
        height: i.inner.height,
    });
    menu::show_about(&format!("About {name}"), &html, icon);
}

// -------------------------------------------------------------------------------------------
// PlatformMenu

pub(crate) struct PlatformMenu {
    /// Mirror of the menu's children, kept in sync through `attach`/`remove_at`.
    children: Children,
    /// The windows this menu is installed on.
    windows: Vec<Rc<Widget>>,
}

/// Validate a window pointer handed to the public API. Fails off the Qt thread too.
fn window_from_ptr(widget: *mut c_void) -> crate::Result<Widget> {
    if !app::is_running() {
        return Err(crate::Error::NotInitialized);
    }
    Widget::from_ptr(widget).ok_or(crate::Error::NotInitialized)
}

impl PlatformMenu {
    pub(crate) fn new() -> Self {
        Self {
            children: Rc::new(RefCell::new(Vec::new())),
            windows: Vec::new(),
        }
    }

    pub(crate) fn attach(&mut self, child: &MenuItemKind, op: AddOp) -> crate::Result<()> {
        {
            let mut children = self.children.borrow_mut();
            match op {
                AddOp::Append => children.push(child.clone()),
                AddOp::Insert(i) => {
                    let i = i.min(children.len());
                    children.insert(i, child.clone());
                }
            }
        }
        if !self.windows.is_empty() {
            schedule_rebuild();
        }
        Ok(())
    }

    pub(crate) fn remove_at(&mut self, index: usize, _child: &MenuItemKind) {
        {
            let mut children = self.children.borrow_mut();
            if index < children.len() {
                children.remove(index);
            }
        }
        if !self.windows.is_empty() {
            schedule_rebuild();
        }
    }

    /// Forget windows that have been destroyed, then find `window` among the others.
    fn position_of(&mut self, window: &Widget) -> Option<usize> {
        self.windows.retain(|w| w.is_alive());
        self.windows.iter().position(|w| **w == *window)
    }

    pub(crate) fn init_for_qt_window(
        &mut self,
        children: &[MenuItemKind],
        widget: *mut c_void,
    ) -> crate::Result<()> {
        let window = window_from_ptr(widget)?;
        // A window has one muda menubar; another menu must be removed from it first.
        if self.position_of(&window).is_some() || MenuBar::get(&window).is_some() {
            return Err(crate::Error::AlreadyInitialized);
        }
        *self.children.borrow_mut() = children.to_vec();
        build_menubar(&window, children);
        let window = Rc::new(window);
        ATTACHED.with(|a| {
            a.borrow_mut().push(Attached {
                window: Rc::downgrade(&window),
                children: Rc::downgrade(&self.children),
            })
        });
        self.windows.push(window);
        Ok(())
    }

    pub(crate) fn remove_for_qt_window(&mut self, widget: *mut c_void) -> crate::Result<()> {
        let window = window_from_ptr(widget)?;
        let index = self
            .position_of(&window)
            .ok_or(crate::Error::NotInitialized)?;
        self.windows.remove(index);
        MenuBar::remove(&window);
        Ok(())
    }

    pub(crate) fn hide_for_qt_window(&mut self, widget: *mut c_void) -> crate::Result<()> {
        self.set_visible(widget, false)
    }

    pub(crate) fn show_for_qt_window(&mut self, widget: *mut c_void) -> crate::Result<()> {
        self.set_visible(widget, true)
    }

    fn set_visible(&mut self, widget: *mut c_void, visible: bool) -> crate::Result<()> {
        let window = window_from_ptr(widget)?;
        self.position_of(&window)
            .ok_or(crate::Error::NotInitialized)?;
        MenuBar::set_visible(&window, visible);
        Ok(())
    }

    pub(crate) fn is_visible_on_qt_window(&self, widget: *mut c_void) -> bool {
        window_from_ptr(widget).is_ok_and(|window| {
            self.windows.iter().any(|w| **w == window) && MenuBar::is_visible(&window)
        })
    }

    pub(crate) fn show_context_menu_for_qt_window(
        &mut self,
        children: &[MenuItemKind],
        widget: *mut c_void,
        position: Option<Position>,
    ) -> bool {
        show_context_menu(children, widget, position)
    }
}

impl Drop for PlatformMenu {
    fn drop(&mut self) {
        // After the application is gone its menubars are too.
        if !app::is_running() {
            return;
        }
        for window in self.windows.drain(..) {
            MenuBar::remove(&window);
        }
    }
}

/// Show `children` as a context menu in the window `widget`. Returns `false` if `widget` isn't a
/// live window or this isn't the Qt thread.
pub(crate) fn show_context_menu(
    children: &[MenuItemKind],
    widget: *mut c_void,
    position: Option<Position>,
) -> bool {
    let Some(menu) = window_from_ptr(widget)
        .ok()
        .and_then(|window| PopupMenu::new(&window))
    else {
        return false;
    };
    build_items(&menu, children, false);
    menu.show(position.map(|p| match p {
        Position::Logical(p) => PopupPosition::Logical { x: p.x, y: p.y },
        Position::Physical(p) => PopupPosition::Physical {
            x: f64::from(p.x),
            y: f64::from(p.y),
        },
    }));
    true
}

// -------------------------------------------------------------------------------------------
// PlatformMenuItem: state lives in muda's item state; any change just triggers a rebuild.

pub(crate) struct PlatformMenuItem;

impl PlatformMenuItem {
    pub(crate) fn new(_click: MenuItemAction) -> Self {
        Self
    }

    pub(crate) fn new_submenu(_click: MenuItemAction) -> Self {
        Self
    }

    pub(crate) fn text(&self) -> Option<String> {
        None
    }

    pub(crate) fn set_text(&mut self, _text: &str, _accelerator: Option<&MenuAccelerator>) {
        schedule_rebuild();
    }

    pub(crate) fn set_styled_text(
        &mut self,
        _text: &str,
        _parts: &[(String, TextStyle)],
        _accelerator: Option<&MenuAccelerator>,
    ) {
        schedule_rebuild();
    }

    pub(crate) fn is_enabled(&self) -> Option<bool> {
        None
    }

    pub(crate) fn set_enabled(&mut self, _enabled: bool) {
        schedule_rebuild();
    }

    pub(crate) fn set_accelerator(
        &mut self,
        _text: &str,
        _accelerator: Option<&MenuAccelerator>,
    ) -> crate::Result<()> {
        schedule_rebuild();
        Ok(())
    }

    pub(crate) fn is_checked(&self) -> Option<bool> {
        None
    }

    pub(crate) fn set_checked(&mut self, _checked: bool) {
        schedule_rebuild();
    }

    pub(crate) fn set_icon(&mut self, _icon: Option<&IconType>) {
        schedule_rebuild();
    }

    pub(crate) fn attach(&mut self, _child: &MenuItemKind, _op: AddOp) -> crate::Result<()> {
        schedule_rebuild();
        Ok(())
    }

    pub(crate) fn remove_at(&mut self, _index: usize, _child: &MenuItemKind) {
        schedule_rebuild();
    }
}
