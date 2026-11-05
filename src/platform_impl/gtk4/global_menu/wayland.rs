// Copyright 2026 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0 OR MIT

use gdk4_wayland::prelude::*;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_plasma::appmenu::client::{
    org_kde_kwin_appmenu::OrgKdeKwinAppmenu, org_kde_kwin_appmenu_manager::OrgKdeKwinAppmenuManager,
};

#[derive(Default)]
struct Globals {
    appmenu: Option<(u32, u32)>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for Globals {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == OrgKdeKwinAppmenuManager::interface().name {
                state.appmenu = Some((name, version));
            }
        }
    }
}

wayland_client::delegate_noop!(Globals: ignore OrgKdeKwinAppmenuManager);
wayland_client::delegate_noop!(Globals: ignore OrgKdeKwinAppmenu);

pub(super) struct Advertisement {
    appmenu: OrgKdeKwinAppmenu,
}

impl Advertisement {
    pub(super) fn new(
        surface: &gdk4_wayland::WaylandSurface,
        bus: &str,
        path: &str,
    ) -> Option<Self> {
        // Borrow GTK's connection: a surface cannot be referenced from a second
        // connection. The bindings do not read or dispatch GTK's event queue.
        let wl_surface = surface.wl_surface()?;
        let connection = Connection::from_backend(wl_surface.backend().upgrade()?);
        let mut queue = connection.new_event_queue::<Globals>();
        let qh = queue.handle();
        let registry = connection.display().get_registry(&qh, ());
        let mut globals = Globals::default();
        let discovered = queue.roundtrip(&mut globals).is_ok();
        let manager = if discovered {
            globals.appmenu.map(|(name, version)| {
                registry.bind::<OrgKdeKwinAppmenuManager, _, _>(name, version.min(2), &qh, ())
            })
        } else {
            None
        };
        // wl_registry has no protocol destructor. Destroy its local proxy so
        // later globals do not target the temporary discovery event queue.
        let _ = connection.backend().destroy_object(&registry.id());
        let manager = manager?;
        let appmenu = manager.create(&wl_surface, &qh, ());
        appmenu.set_address(bus.to_owned(), path.to_owned());
        if manager.version() >= 2 {
            manager.release();
        } else {
            let _ = connection.backend().destroy_object(&manager.id());
        }
        let advertisement = Self { appmenu };
        connection.flush().ok()?;
        Some(advertisement)
    }
}

impl Drop for Advertisement {
    fn drop(&mut self) {
        self.appmenu.release();
        if let Some(backend) = self.appmenu.backend().upgrade() {
            let _ = Connection::from_backend(backend).flush();
        }
    }
}
