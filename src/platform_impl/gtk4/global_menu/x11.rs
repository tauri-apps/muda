// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

use gtk::{gio, glib, prelude::*};

pub(super) const REGISTRAR: &str = "com.canonical.AppMenu.Registrar";
const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";

pub(super) struct Advertisement {
    connection: gio::DBusConnection,
    xid: u32,
    path: glib::variant::ObjectPath,
    owner: Option<String>,
}

impl Advertisement {
    pub(super) fn new(
        connection: &gio::DBusConnection,
        surface: &gdk4_x11::X11Surface,
        path: &str,
    ) -> Option<Self> {
        Some(Self {
            connection: connection.clone(),
            xid: u32::try_from(surface.xid()).ok()?,
            path: glib::variant::ObjectPath::try_from(path).ok()?,
            owner: None,
        })
    }

    fn unregister(&self, owner: &str) {
        self.connection.call(
            Some(owner),
            REGISTRAR_PATH,
            REGISTRAR,
            "UnregisterWindow",
            Some(&(self.xid,).to_variant()),
            Some(glib::VariantTy::UNIT),
            gio::DBusCallFlags::NO_AUTO_START,
            1000,
            gio::Cancellable::NONE,
            |_| {},
        );
    }

    pub(super) fn set_owner(&mut self, owner: Option<&str>) {
        if self.owner.as_deref() == owner {
            return;
        }
        if let Some(previous) = self.owner.take() {
            self.unregister(&previous);
        }
        self.owner = owner.map(str::to_owned);
        let Some(owner) = owner else {
            return;
        };
        // Address the registrar's unique owner, so registration and cleanup
        // cannot accidentally target a replacement registrar.
        self.connection.call(
            Some(owner),
            REGISTRAR_PATH,
            REGISTRAR,
            "RegisterWindow",
            Some(&(self.xid, self.path.clone()).to_variant()),
            Some(glib::VariantTy::UNIT),
            gio::DBusCallFlags::NO_AUTO_START,
            3000,
            gio::Cancellable::NONE,
            |_| {},
        );
    }
}

impl Drop for Advertisement {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            self.unregister(&owner);
        }
    }
}
