// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Advertises live GTK menus using the protocols offered by the display server.
mod dbusmenu;
mod wayland;
mod x11;
use gtk::{gio, glib, prelude::*};
use std::{cell::RefCell, rc::Rc};

struct Registration {
    // Withdraw discovery before unexporting the shared DBusMenu object.
    advertisement: Advertisement,
    _menu: dbusmenu::Export,
}

enum Advertisement {
    X11(x11::Advertisement),
    Wayland {
        _advertisement: wayland::Advertisement,
    },
}

impl Registration {
    fn new(window: &gtk::Window, menu: &gio::Menu) -> Option<Self> {
        let app = window.application()?;
        let connection = app.dbus_connection()?;
        let surface = window.surface()?;
        let display = WidgetExt::display(window);
        if let Some(display) = display.downcast_ref::<gdk4_wayland::WaylandDisplay>() {
            if !display.query_registry("org_kde_kwin_appmenu_manager") {
                return None;
            }
        }
        let path = format!("/com/tauri/muda/menu_{}", super::COUNTER.next());
        let export = dbusmenu::Export::new(&connection, &path, menu, &app)?;
        let advertisement = if let Some(surface) = surface.downcast_ref::<gdk4_x11::X11Surface>() {
            Advertisement::X11(x11::Advertisement::new(&connection, surface, &path)?)
        } else {
            let surface = surface.downcast_ref::<gdk4_wayland::WaylandSurface>()?;
            Advertisement::Wayland {
                _advertisement: wayland::Advertisement::new(
                    surface,
                    connection.unique_name()?.as_str(),
                    &path,
                )?,
            }
        };
        Some(Self {
            advertisement,
            _menu: export,
        })
    }
}

struct State {
    registration: RefCell<Option<Registration>>,
    registrar_owner: RefCell<Option<String>>,
}

impl State {
    fn set_registration(&self, window: &gtk::Window, menu: &gio::Menu) {
        self.registration.borrow_mut().take();
        *self.registration.borrow_mut() = Registration::new(window, menu);
        self.sync_owner();
    }

    fn sync_owner(&self) {
        if let Some(Registration {
            advertisement: Advertisement::X11(registrar),
            ..
        }) = self.registration.borrow_mut().as_mut()
        {
            registrar.set_owner(self.registrar_owner.borrow().as_deref());
        }
    }

    fn set_owner(&self, owner: Option<&str>) {
        self.registrar_owner.replace(owner.map(str::to_owned));
        self.sync_owner();
    }
}

pub(super) struct Export {
    window: glib::WeakRef<gtk::Window>,
    handlers: Vec<glib::SignalHandlerId>,
    state: Rc<State>,
    unwatch: Option<Box<dyn FnOnce()>>,
}

impl Export {
    pub(super) fn new(window: &gtk::Window, menu: &gio::Menu) -> Self {
        // Discovery protocols do not acknowledge presentation. Exporting a
        // global menu must not hide the local bar when no applet can show it.
        let state = Rc::new(State {
            registration: RefCell::new(None),
            registrar_owner: RefCell::new(None),
        });
        state.set_registration(window, menu);
        // Native Wayland advertises on its surface and needs no X11 registrar.
        let unwatch = WidgetExt::display(window)
            .is::<gdk4_x11::X11Display>()
            .then(|| {
                let watcher = gio::bus_watch_name(
                    gio::BusType::Session,
                    x11::REGISTRAR,
                    gio::BusNameWatcherFlags::NONE,
                    {
                        let state = Rc::downgrade(&state);
                        move |_, _, owner| {
                            if let Some(state) = state.upgrade() {
                                state.set_owner(Some(owner));
                            }
                        }
                    },
                    {
                        let state = Rc::downgrade(&state);
                        move |_, _| {
                            if let Some(state) = state.upgrade() {
                                state.set_owner(None);
                            }
                        }
                    },
                );
                Box::new(move || gio::bus_unwatch_name(watcher)) as Box<dyn FnOnce()>
            });
        let realize = window.connect_realize({
            let menu = menu.clone();
            let state = state.clone();
            move |window| {
                state.set_registration(window, &menu);
            }
        });
        let unrealize = window.connect_unrealize({
            let state = state.clone();
            move |_| {
                state.registration.borrow_mut().take();
            }
        });
        Self {
            window: window.downgrade(),
            handlers: vec![realize, unrealize],
            state,
            unwatch,
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
        self.state.registration.borrow_mut().take();
    }
}
