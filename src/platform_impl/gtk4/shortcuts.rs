// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use gtk::{gio, glib, prelude::*};

/// Keeps a host's shortcuts in sync with its menu, including nested submenus.
/// Actions remain on GtkApplication; only the keyboard bindings belong to the host.
pub(super) struct Shortcuts {
    bindings: Rc<Bindings>,
    controller: gtk::ShortcutController,
}

struct Bindings {
    menu: gio::MenuModel,
    shortcuts: gio::ListStore,
    handlers: RefCell<Vec<(gio::MenuModel, glib::SignalHandlerId)>>,
}

impl Shortcuts {
    pub(super) fn new(host: &impl IsA<gtk::Widget>, menu: &impl IsA<gio::MenuModel>) -> Self {
        let bindings = Rc::new(Bindings {
            menu: menu.as_ref().clone(),
            shortcuts: gio::ListStore::new::<gtk::Shortcut>(),
            handlers: RefCell::new(Vec::new()),
        });
        bindings.refresh();
        let controller = gtk::ShortcutController::for_model(&bindings.shortcuts);
        // Run before the focused widget handles the key, like application accelerators.
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        // Windows handle their own events; popovers delegate to their current root.
        controller.set_scope(if host.as_ref().is::<gtk::Window>() {
            gtk::ShortcutScope::Local
        } else {
            gtk::ShortcutScope::Global
        });
        host.add_controller(controller.clone());
        Self {
            bindings,
            controller,
        }
    }
}

impl Bindings {
    fn refresh(self: &Rc<Self>) {
        self.disconnect();
        let mut shortcuts = Vec::new();
        self.collect(
            &self.menu,
            &mut HashSet::new(),
            &mut HashSet::new(),
            &mut shortcuts,
        );
        self.shortcuts
            .splice(0, self.shortcuts.n_items(), &shortcuts);
    }

    fn collect(
        self: &Rc<Self>,
        menu: &gio::MenuModel,
        models: &mut HashSet<usize>,
        actions: &mut HashSet<String>,
        shortcuts: &mut Vec<gtk::Shortcut>,
    ) {
        if !models.insert(menu.as_ptr() as usize) {
            return;
        }
        let weak = Rc::downgrade(self);
        let handler = menu.connect_items_changed(move |_, _, _, _| {
            if let Some(bindings) = weak.upgrade() {
                bindings.refresh();
            }
        });
        self.handlers.borrow_mut().push((menu.clone(), handler));
        for index in 0..menu.n_items() {
            let action = menu
                .item_attribute_value(index, "action", None)
                .and_then(|value| value.str().map(str::to_owned));
            let accelerator = menu
                .item_attribute_value(index, "accel", None)
                .and_then(|value| value.str().map(str::to_owned));
            if let (Some(action), Some(accelerator)) = (action, accelerator) {
                // Re-registering a shared item moves its binding to the end,
                // matching GtkApplication's accelerator registration order.
                if !actions.insert(action.clone()) {
                    shortcuts.retain(|shortcut| {
                        shortcut
                            .action()
                            .and_downcast::<gtk::NamedAction>()
                            .is_none_or(|existing| existing.action_name() != action)
                    });
                }
                if let Some(trigger) = gtk::ShortcutTrigger::parse_string(&accelerator) {
                    shortcuts.push(gtk::Shortcut::new(
                        Some(trigger),
                        Some(gtk::NamedAction::new(&action)),
                    ));
                }
            }
            for link in [gio::MENU_LINK_SUBMENU, gio::MENU_LINK_SECTION] {
                if let Some(child) = menu.item_link(index, link) {
                    self.collect(&child, models, actions, shortcuts);
                }
            }
        }
    }

    fn disconnect(&self) {
        for (menu, handler) in self.handlers.take() {
            menu.disconnect(handler);
        }
    }
}

impl Drop for Shortcuts {
    fn drop(&mut self) {
        self.bindings.disconnect();
        if let Some(host) = self.controller.widget() {
            host.remove_controller(&self.controller);
        }
    }
}
