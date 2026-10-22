// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Experimental X11 advertisement of the existing live GTK4 menu. GTK exports application actions.
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Registration {
    connection: gio::DBusConnection,
    menu_id: Option<gio::MenuModelExportId>,
    surface: glib::WeakRef<gdk4_x11::X11Surface>,
}

impl Registration {
    fn new(window: &gtk::Window, menu: &gio::Menu) -> Option<Self> {
        let surface = window.surface()?.downcast::<gdk4_x11::X11Surface>().ok()?;
        let connection = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
        let path = format!("/com/tauri/muda/window_{}", window.as_ptr() as usize);
        let menu_id = connection.export_menu_model(&path, menu).ok()?;
        // Publish on the realized surface, including when the menu is attached
        // after window creation. The desktop selects the focused window's model.
        surface.set_utf8_property("_GTK_UNIQUE_BUS_NAME", connection.unique_name().as_deref());
        surface.set_utf8_property("_GTK_MENUBAR_OBJECT_PATH", Some(&path));
        Some(Self {
            connection,
            menu_id: Some(menu_id),
            surface: surface.downgrade(),
        })
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(surface) = self.surface.upgrade() {
            surface.set_utf8_property("_GTK_MENUBAR_OBJECT_PATH", None);
        }
        if let Some(id) = self.menu_id.take() {
            self.connection.unexport_menu_model(id);
        }
    }
}

struct Visibility {
    widget: glib::WeakRef<gtk::PopoverMenuBar>,
    registration: RefCell<Option<Registration>>,
    registrar_present: Cell<bool>,
    requested: Cell<bool>,
}

impl Visibility {
    fn update(&self) {
        if let Some(widget) = self.widget.upgrade() {
            let remote = self.registration.borrow().is_some() && self.registrar_present.get();
            widget.set_visible(self.requested.get() && !remote);
        }
    }
}

pub(super) struct Export {
    window: glib::WeakRef<gtk::Window>,
    handlers: Vec<glib::SignalHandlerId>,
    visibility: Rc<Visibility>,
    unwatch: Option<Box<dyn FnOnce()>>,
}

impl Export {
    pub(super) fn set_visible(&self, visible: bool) {
        self.visibility.requested.set(visible);
        self.visibility.update();
    }

    pub(super) fn new(
        window: &gtk::Window,
        menu: &gio::Menu,
        widget: &gtk::PopoverMenuBar,
    ) -> Self {
        let visibility = Rc::new(Visibility {
            widget: widget.downgrade(),
            registration: RefCell::new(Registration::new(window, menu)),
            registrar_present: Cell::new(false),
            requested: Cell::new(false),
        });
        // Like appmenu-gtk-module, watch the registrar without activating it.
        // A desktop setting may stay true after the last applet disappears.
        let watcher = gio::bus_watch_name(
            gio::BusType::Session,
            "com.canonical.AppMenu.Registrar",
            gio::BusNameWatcherFlags::NONE,
            {
                let visibility = Rc::downgrade(&visibility);
                move |_, _, _| {
                    if let Some(visibility) = visibility.upgrade() {
                        visibility.registrar_present.set(true);
                        visibility.update();
                    }
                }
            },
            {
                let visibility = Rc::downgrade(&visibility);
                move |_, _| {
                    if let Some(visibility) = visibility.upgrade() {
                        visibility.registrar_present.set(false);
                        visibility.update();
                    }
                }
            },
        );
        let realize = window.connect_realize({
            let menu = menu.clone();
            let visibility = visibility.clone();
            move |window| {
                visibility.registration.borrow_mut().take();
                *visibility.registration.borrow_mut() = Registration::new(window, &menu);
                visibility.update();
            }
        });
        let unrealize = window.connect_unrealize({
            let visibility = visibility.clone();
            move |_| {
                visibility.registration.borrow_mut().take();
                visibility.update();
            }
        });
        Self {
            window: window.downgrade(),
            handlers: vec![realize, unrealize],
            visibility,
            unwatch: Some(Box::new(move || gio::bus_unwatch_name(watcher))),
        }
    }
}

impl Drop for Export {
    fn drop(&mut self) {
        if let Some(window) = self.window.upgrade() {
            for handler in self.handlers.drain(..) {
                window.disconnect(handler);
            }
        }
        if let Some(unwatch) = self.unwatch.take() {
            unwatch();
        }
        self.visibility.registration.borrow_mut().take();
    }
}
