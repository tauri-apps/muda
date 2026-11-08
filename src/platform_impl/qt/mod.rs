// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Qt Widgets backend (`qt` feature).
//!
//! Instead of mirroring every incremental change into each Qt widget instance, this backend
//! keeps the item tree that muda already owns as the source of truth: any change schedules a
//! single, coalesced rebuild of every attached `QMenuBar` on the Qt event loop. Context menus are
//! built on demand. Clicks reuse [`MenuItemAction`], so events, check toggling and predefined
//! items behave like the other backends.
//!
//! The host application must have created a `QApplication` (e.g. a Qt-based Tauri runtime) and
//! all calls must happen on its thread.

use std::{
    cell::{Cell, RefCell},
    ffi::{c_int, c_void, CString},
    rc::{Rc, Weak},
};

use dpi::Position;

use crate::{
    accelerator::MenuAccelerator,
    icon::RgbaIcon,
    items::{IconType, MenuItemAction, PredefinedMenuItemType},
    util::AddOp,
    MenuEvent, MenuItemKind, TextStyle,
};

pub(crate) type PlatformIcon = RgbaIcon;

mod ffi {
    use std::ffi::{c_char, c_int, c_void};

    pub type MqFn = unsafe extern "C" fn(*mut c_void);

    extern "C" {
        pub fn mq_post(f: MqFn, ud: *mut c_void);
        pub fn mq_track(widget: *mut c_void) -> *mut c_void;
        pub fn mq_tracked(tracker: *mut c_void) -> *mut c_void;
        pub fn mq_untrack(tracker: *mut c_void);
        pub fn mq_menubar_for(widget: *mut c_void, create: c_int) -> *mut c_void;
        pub fn mq_menubar_remove(widget: *mut c_void);
        pub fn mq_menubar_set_visible(widget: *mut c_void, visible: c_int);
        pub fn mq_menubar_is_visible(widget: *mut c_void) -> c_int;
        pub fn mq_container_clear(container: *mut c_void);
        pub fn mq_container_add_menu(
            container: *mut c_void,
            text: *const c_char,
            enabled: c_int,
            rgba: *const u8,
            w: c_int,
            h: c_int,
        ) -> *mut c_void;
        #[allow(clippy::too_many_arguments)]
        pub fn mq_container_add_action(
            container: *mut c_void,
            text: *const c_char,
            enabled: c_int,
            checkable: c_int,
            checked: c_int,
            shortcut: *const c_char,
            shortcut_hint: *const c_char,
            rgba: *const u8,
            w: c_int,
            h: c_int,
            on_click: Option<MqFn>,
            ud: *mut c_void,
            drop: Option<MqFn>,
        );
        pub fn mq_container_add_separator(container: *mut c_void);
        pub fn mq_popup_menu_new(widget: *mut c_void) -> *mut c_void;
        pub fn mq_popup_menu_show(
            menu: *mut c_void,
            widget: *mut c_void,
            has_pos: c_int,
            x: f64,
            y: f64,
            physical: c_int,
        );
        pub fn mq_run_predefined(kind: c_int);
        pub fn mq_show_about(
            title: *const c_char,
            html: *const c_char,
            rgba: *const u8,
            w: c_int,
            h: c_int,
        );
    }
}

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

// -------------------------------------------------------------------------------------------
// main-thread dispatch

/// Run `f` on the Qt thread from the event loop.
pub(crate) fn post<F: FnOnce() + 'static>(f: F) {
    unsafe extern "C" fn tramp(ud: *mut c_void) {
        let f = unsafe { Box::from_raw(ud as *mut Box<dyn FnOnce()>) };
        f();
    }
    let f: Box<dyn FnOnce()> = Box::new(f);
    unsafe { ffi::mq_post(tramp, Box::into_raw(Box::new(f)) as *mut c_void) }
}

#[cfg(feature = "snapshot")]
pub(crate) fn dispatch_on_main_thread<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    post(f);
}

// -------------------------------------------------------------------------------------------
// attached menubars and coalesced rebuilds

/// A `QPointer<QWidget>` owned by the [`PlatformMenu`] that attached a menubar to the widget.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Tracker(*mut c_void);

impl Tracker {
    fn new(widget: *mut c_void) -> Self {
        Self(unsafe { ffi::mq_track(widget) })
    }
    /// The widget, or null once it has been destroyed.
    fn widget(self) -> *mut c_void {
        unsafe { ffi::mq_tracked(self.0) }
    }
}

struct Attached {
    tracker: Tracker,
    children: Weak<RefCell<Vec<MenuItemKind>>>,
}

thread_local! {
    static ATTACHED: RefCell<Vec<Attached>> = const { RefCell::new(Vec::new()) };
    static REBUILD_PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Rebuild every attached menubar once the current call stack has unwound. Deferring also means
/// the rebuild reads the final item state, whatever order muda updated state and platform in.
fn schedule_rebuild() {
    if REBUILD_PENDING.with(|p| p.replace(true)) {
        return;
    }
    post(rebuild_all);
}

fn rebuild_all() {
    REBUILD_PENDING.with(|p| p.set(false));
    let attached: Vec<(*mut c_void, Rc<RefCell<Vec<MenuItemKind>>>)> = ATTACHED.with(|a| {
        a.borrow()
            .iter()
            .filter_map(|x| {
                let widget = x.tracker.widget();
                let children = x.children.upgrade()?;
                (!widget.is_null()).then_some((widget, children))
            })
            .collect()
    });
    for (widget, children) in attached {
        build_menubar(widget, &children.borrow());
    }
}

fn build_menubar(widget: *mut c_void, items: &[MenuItemKind]) {
    let bar = unsafe { ffi::mq_menubar_for(widget, 1) };
    if bar.is_null() {
        return;
    }
    unsafe { ffi::mq_container_clear(bar) };
    build_items(bar, items, true);
}

// -------------------------------------------------------------------------------------------
// building Qt menus from muda's item tree

fn icon_of(icon: &Option<IconType>) -> Option<(Vec<u8>, i32, i32)> {
    match icon {
        Some(IconType::Custom(icon)) => Some((
            icon.inner.rgba.clone(),
            icon.inner.width as i32,
            icon.inner.height as i32,
        )),
        _ => None,
    }
}

/// Predefined items the Qt backend can perform, as `mq_run_predefined` kinds.
fn predefined_kind(t: &PredefinedMenuItemType) -> Option<c_int> {
    use PredefinedMenuItemType::*;
    Some(match t {
        Copy => 1,
        Cut => 2,
        Paste => 3,
        SelectAll => 4,
        Undo => 5,
        Redo => 6,
        Delete => 7,
        Minimize => 8,
        Maximize => 9,
        Fullscreen => 10,
        Hide => 11,
        CloseWindow => 12,
        Quit => 13,
        _ => return None,
    })
}

/// Editing items whose shortcut the focused widget (e.g. a web view) already handles: show the
/// shortcut, but don't register it, or the menu would steal the key from the page.
fn is_editing(t: &PredefinedMenuItemType) -> bool {
    use PredefinedMenuItemType::*;
    matches!(t, Copy | Cut | Paste | SelectAll | Undo | Redo | Delete)
}

unsafe extern "C" fn on_click(ud: *mut c_void) {
    let action = unsafe { &*(ud as *const MenuItemAction) };
    activate(action);
}

unsafe extern "C" fn drop_action(ud: *mut c_void) {
    drop(unsafe { Box::from_raw(ud as *mut MenuItemAction) });
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
            run_predefined(&t);
        }
    }
}

fn run_predefined(t: &PredefinedMenuItemType) {
    if let PredefinedMenuItemType::About(Some(metadata)) = t {
        show_about_dialog(metadata);
    } else if let Some(kind) = predefined_kind(t) {
        unsafe { ffi::mq_run_predefined(kind) };
    }
}

#[allow(clippy::too_many_arguments)]
fn add_action(
    container: *mut c_void,
    text: &str,
    enabled: bool,
    checkable: bool,
    checked: bool,
    shortcut: Option<&str>,
    shortcut_hint: Option<&str>,
    icon: Option<(Vec<u8>, i32, i32)>,
    action: Option<MenuItemAction>,
) {
    let text = cstring(text);
    let shortcut = shortcut.map(cstring);
    let hint = shortcut_hint.map(cstring);
    let (rgba, w, h) = icon.unwrap_or_default();
    let ud = action.map(|a| Box::into_raw(Box::new(a)) as *mut c_void);
    unsafe {
        ffi::mq_container_add_action(
            container,
            text.as_ptr(),
            enabled as c_int,
            checkable as c_int,
            checked as c_int,
            shortcut.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            hint.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            if rgba.is_empty() {
                std::ptr::null()
            } else {
                rgba.as_ptr()
            },
            w,
            h,
            ud.map(|_| on_click as ffi::MqFn),
            ud.unwrap_or(std::ptr::null_mut()),
            ud.map(|_| drop_action as ffi::MqFn),
        )
    }
}

/// Fill a `QMenuBar`/`QMenu` with `items`. Shortcuts are registered only in menubars
/// (`register_shortcuts`); context menus just display them.
fn build_items(container: *mut c_void, items: &[MenuItemKind], register_shortcuts: bool) {
    for item in items {
        let args = item.platform_attach_args();
        let shortcut = args.accelerator.as_ref().and_then(MenuAccelerator::to_qt);
        match item {
            MenuItemKind::Submenu(_) => {
                let text = cstring(&args.text);
                let (rgba, w, h) = icon_of(&args.icon).unwrap_or_default();
                let menu = unsafe {
                    ffi::mq_container_add_menu(
                        container,
                        text.as_ptr(),
                        args.enabled as c_int,
                        if rgba.is_empty() {
                            std::ptr::null()
                        } else {
                            rgba.as_ptr()
                        },
                        w,
                        h,
                    )
                };
                build_items(menu, &item.items(), register_shortcuts);
            }
            MenuItemKind::Predefined(p) => {
                let t = p.state.borrow().predefined_item_type.clone();
                match t {
                    PredefinedMenuItemType::Separator => unsafe {
                        ffi::mq_container_add_separator(container)
                    },
                    PredefinedMenuItemType::About(_) => add_action(
                        container,
                        &args.text,
                        true,
                        false,
                        false,
                        None,
                        None,
                        None,
                        Some(item.click_action()),
                    ),
                    ref t if predefined_kind(t).is_some() => {
                        let display_only = is_editing(t) || !register_shortcuts;
                        add_action(
                            container,
                            &args.text,
                            true,
                            false,
                            false,
                            if display_only {
                                None
                            } else {
                                shortcut.as_deref()
                            },
                            if display_only {
                                shortcut.as_deref()
                            } else {
                                None
                            },
                            None,
                            Some(item.click_action()),
                        )
                    }
                    // macOS-only items (Services, Show All, ...) render disabled, like on GTK.
                    _ => add_action(
                        container, &args.text, false, false, false, None, None, None, None,
                    ),
                }
            }
            MenuItemKind::Check(_) => add_action(
                container,
                &args.text,
                args.enabled,
                true,
                args.checked,
                shortcut.as_deref().filter(|_| register_shortcuts),
                shortcut.as_deref().filter(|_| !register_shortcuts),
                None,
                Some(item.click_action()),
            ),
            MenuItemKind::MenuItem(_) | MenuItemKind::Icon(_) => add_action(
                container,
                &args.text,
                args.enabled,
                false,
                false,
                shortcut.as_deref().filter(|_| register_shortcuts),
                shortcut.as_deref().filter(|_| !register_shortcuts),
                icon_of(&args.icon),
                Some(item.click_action()),
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
    let title = cstring(&format!("About {name}"));
    let html = cstring(&html);
    let icon = metadata.icon.as_ref().map(|i| &i.inner);
    unsafe {
        ffi::mq_show_about(
            title.as_ptr(),
            html.as_ptr(),
            icon.map_or(std::ptr::null(), |i| i.rgba.as_ptr()),
            icon.map_or(0, |i| i.width as c_int),
            icon.map_or(0, |i| i.height as c_int),
        )
    }
}

// -------------------------------------------------------------------------------------------
// PlatformMenu

pub(crate) struct PlatformMenu {
    /// Mirror of the menu's children, kept in sync through `attach`/`remove_at`.
    children: Rc<RefCell<Vec<MenuItemKind>>>,
    /// Widgets this menu is installed on.
    widgets: Vec<Tracker>,
}

impl PlatformMenu {
    /// Forget widgets that have been destroyed.
    fn prune(&mut self) {
        let dead: Vec<Tracker> = self
            .widgets
            .iter()
            .copied()
            .filter(|t| t.widget().is_null())
            .collect();
        for t in dead {
            self.forget(t);
        }
    }

    fn forget(&mut self, tracker: Tracker) {
        self.widgets.retain(|t| *t != tracker);
        ATTACHED.with(|a| a.borrow_mut().retain(|x| x.tracker != tracker));
        unsafe { ffi::mq_untrack(tracker.0) };
    }

    fn tracker_for(&self, widget: *mut c_void) -> Option<Tracker> {
        self.widgets
            .iter()
            .copied()
            .find(|t| !widget.is_null() && t.widget() == widget)
    }
}

impl PlatformMenu {
    pub(crate) fn new() -> Self {
        Self {
            children: Rc::new(RefCell::new(Vec::new())),
            widgets: Vec::new(),
        }
    }

    pub(crate) fn attach(&mut self, child: &MenuItemKind, op: AddOp) -> crate::Result<()> {
        {
            let mut children = self.children.borrow_mut();
            match op {
                AddOp::Append => children.push(child.clone()),
                AddOp::Insert(i) => {
                    let i = i.min(children.len());
                    children.insert(i, child.clone())
                }
            }
        }
        if !self.widgets.is_empty() {
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
        if !self.widgets.is_empty() {
            schedule_rebuild();
        }
    }

    pub(crate) fn init_for_qt_window(
        &mut self,
        children: &[MenuItemKind],
        widget: *mut c_void,
    ) -> crate::Result<()> {
        if widget.is_null() {
            return Err(crate::Error::NotInitialized);
        }
        self.prune();
        if self.tracker_for(widget).is_some() {
            return Err(crate::Error::AlreadyInitialized);
        }
        *self.children.borrow_mut() = children.to_vec();
        let tracker = Tracker::new(widget);
        self.widgets.push(tracker);
        ATTACHED.with(|a| {
            a.borrow_mut().push(Attached {
                tracker,
                children: Rc::downgrade(&self.children),
            })
        });
        build_menubar(widget, children);
        Ok(())
    }

    pub(crate) fn remove_for_qt_window(&mut self, widget: *mut c_void) -> crate::Result<()> {
        let Some(tracker) = self.tracker_for(widget) else {
            return Err(crate::Error::NotInitialized);
        };
        self.forget(tracker);
        unsafe { ffi::mq_menubar_remove(widget) };
        Ok(())
    }

    pub(crate) fn hide_for_qt_window(&mut self, widget: *mut c_void) -> crate::Result<()> {
        if self.tracker_for(widget).is_none() {
            return Err(crate::Error::NotInitialized);
        }
        unsafe { ffi::mq_menubar_set_visible(widget, 0) };
        Ok(())
    }

    pub(crate) fn show_for_qt_window(&mut self, widget: *mut c_void) -> crate::Result<()> {
        if self.tracker_for(widget).is_none() {
            return Err(crate::Error::NotInitialized);
        }
        unsafe { ffi::mq_menubar_set_visible(widget, 1) };
        Ok(())
    }

    pub(crate) fn is_visible_on_qt_window(&self, widget: *mut c_void) -> bool {
        self.tracker_for(widget).is_some() && unsafe { ffi::mq_menubar_is_visible(widget) } != 0
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
        for tracker in self.widgets.clone() {
            let widget = tracker.widget();
            if !widget.is_null() {
                unsafe { ffi::mq_menubar_remove(widget) };
            }
            self.forget(tracker);
        }
    }
}

pub(crate) fn show_context_menu(
    children: &[MenuItemKind],
    widget: *mut c_void,
    position: Option<Position>,
) -> bool {
    if widget.is_null() {
        return false;
    }
    let menu = unsafe { ffi::mq_popup_menu_new(widget) };
    build_items(menu, children, false);
    let (has_pos, x, y, physical) = match position {
        Some(Position::Logical(p)) => (1, p.x, p.y, 0),
        Some(Position::Physical(p)) => (1, p.x as f64, p.y as f64, 1),
        None => (0, 0.0, 0.0, 0),
    };
    unsafe { ffi::mq_popup_menu_show(menu, widget, has_pos, x, y, physical) };
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
