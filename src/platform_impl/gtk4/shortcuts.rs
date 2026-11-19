// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use gtk::{gio, glib, prelude::*};

/// Keeps menu keyboard bindings scoped to their window or popover.
///
/// GtkApplication::set_accels_for_action() registers accelerators application-wide,
/// so a menu action can still be triggered from a window with no menu attached.
/// Instead, we attach a ShortcutController to each menu's window or popover.
/// The controller maps the menu rows' `accel` attributes to their `action` names;
/// the actions themselves remain registered on GtkApplication.
///
/// Changes to the menu or its submenus rebuild the controller's shortcut list.
/// Detaching or destroying the menu drops its bindings and removes the controller,
/// without affecting shortcuts belonging to menus in other windows.
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
        let bindings = Bindings::from_menu(menu);
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
    fn from_menu(menu: &impl IsA<gio::MenuModel>) -> Rc<Self> {
        let bindings = Rc::new(Self {
            menu: menu.as_ref().clone(),
            shortcuts: gio::ListStore::new::<gtk::Shortcut>(),
            handlers: RefCell::new(Vec::new()),
        });
        bindings.refresh();
        bindings
    }

    fn refresh(self: &Rc<Self>) {
        self.disconnect();

        let mut shortcuts = Vec::new();
        self.collect(&self.menu, &mut HashSet::new(), &mut shortcuts);

        let n_items = self.shortcuts.n_items();
        self.shortcuts.splice(0, n_items, &shortcuts);
    }

    fn collect(
        self: &Rc<Self>,
        menu: &gio::MenuModel,
        models: &mut HashSet<usize>,
        shortcuts: &mut Vec<gtk::Shortcut>,
    ) {
        // If we already saw and collected this model, return.
        if !models.insert(menu.as_ptr() as usize) {
            return;
        }

        // Watch this model for changes and rebuild shortcuts when it changes.
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
                // Get the index of any shortcut already collected for this action.
                let existing_index = shortcuts.iter().position(|shortcut| {
                    let named_action = shortcut.action().and_downcast::<gtk::NamedAction>();
                    named_action.is_some_and(|existing| existing.action_name() == action)
                });

                // If already collected, move the shortcut to the end to preserve registration order.
                if let Some(index) = existing_index {
                    let shortcut = shortcuts.remove(index);
                    shortcuts.push(shortcut);
                } else if let Some(trigger) = gtk::ShortcutTrigger::parse_string(&accelerator) {
                    // Otherwise, create a shortcut for the action if its accelerator is valid.
                    let action = gtk::NamedAction::new(&action);
                    let shortcut = gtk::Shortcut::new(Some(trigger), Some(action));
                    shortcuts.push(shortcut);
                }
            }

            // Collect bindings from the submenu recursively.
            if let Some(child) = menu.item_link(index, gio::MENU_LINK_SUBMENU) {
                self.collect(&child, models, shortcuts);
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
