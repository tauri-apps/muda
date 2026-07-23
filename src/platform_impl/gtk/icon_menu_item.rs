// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::cell::OnceCell;

use gtk4::{glib, prelude::*, subclass::prelude::*};

use crate::{accelerator::KeyAccelerator, Icon};

use super::mnemonic::to_gtk_mnemonic;

const MENU_ICON_SIZE: i32 = 16;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct IconMenuItem {
        pub(super) image: OnceCell<gtk4::Image>,
        pub(super) label: OnceCell<gtk4::Label>,
        pub(super) accelerator_label: OnceCell<gtk4::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IconMenuItem {
        const NAME: &'static str = "MudaIconMenuItem";
        type Type = super::IconMenuItem;
        type ParentType = gtk4::Button;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("modelbutton");
            klass.set_accessible_role(gtk4::AccessibleRole::MenuItem);
        }
    }

    impl ObjectImpl for IconMenuItem {}
    impl WidgetImpl for IconMenuItem {}
    impl ButtonImpl for IconMenuItem {}
}

glib::wrapper! {
    pub struct IconMenuItem(ObjectSubclass<imp::IconMenuItem>)
        @extends gtk4::Button, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Actionable;
}

impl IconMenuItem {
    pub fn new(
        text: &str,
        detailed_action: &str,
        icon: Option<&Icon>,
        key_accelerator: Option<&KeyAccelerator>,
    ) -> Self {
        let item: Self = glib::Object::new();
        item.set_has_frame(false);
        item.set_focus_on_click(false);
        item.add_css_class("flat");
        item.set_detailed_action_name(detailed_action);
        item.set_can_focus(true);
        item.set_focusable(true);
        item.set_halign(gtk4::Align::Fill);
        item.set_hexpand(true);

        let image = gtk4::Image::builder()
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .width_request(MENU_ICON_SIZE)
            .height_request(MENU_ICON_SIZE)
            .build();

        let label = gtk4::Label::builder().xalign(0.0).hexpand(true).build();

        let accelerator_label = gtk4::Label::builder()
            .css_name("accelerator")
            .margin_start(24)
            .build();

        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .hexpand(true)
            .build();
        content.append(&image);
        content.append(&label);
        content.append(&accelerator_label);

        let imp = item.imp();
        let _ = imp.image.set(image);
        let _ = imp.label.set(label.clone());
        let _ = imp.accelerator_label.set(accelerator_label);

        item.set_child(Some(&content));

        label.set_mnemonic_widget(Some(&item));

        item.add_menu_item_controllers();
        item.set_label(text);
        item.set_icon(icon);
        item.set_key_accelerator(key_accelerator);

        item
    }

    pub fn set_label(&self, text: &str) {
        let Some(label) = self.imp().label.get() else {
            return;
        };

        label.set_text_with_mnemonic(&to_gtk_mnemonic(text));
        label.set_mnemonic_widget(Some(self));
    }

    pub fn set_icon(&self, icon: Option<&Icon>) {
        let Some(image) = self.imp().image.get() else {
            return;
        };

        if let Some(icon) = icon {
            let texture = icon.inner.texture();
            image.set_paintable(Some(&texture));
        } else {
            image.set_paintable(gtk4::gdk::Paintable::NONE);
        }
    }

    pub fn set_key_accelerator(&self, key_accelerator: Option<&KeyAccelerator>) {
        let Some(accelerator_label) = self.imp().accelerator_label.get() else {
            return;
        };

        let accelerator = key_accelerator.and_then(|accelerator| {
            let (key, mods) = gtk4::accelerator_parse(accelerator.to_gtk())?;
            let label = gtk4::accelerator_get_label(key, mods);
            (!label.is_empty()).then_some(label)
        });

        if let Some(accelerator) = accelerator {
            accelerator_label.set_label(&accelerator);
            accelerator_label.set_visible(true);
        } else {
            accelerator_label.set_label("");
            accelerator_label.set_visible(false);
        }
    }

    fn add_menu_item_controllers(&self) {
        // Set PRELIGHT state when item is focused using keyboard navigation
        // and unset PRELIGHT state when item loses keyboard focus.
        //
        // Upon focus, we also clear the PRELIGHT and SELECTED state of
        // sibling menu items to ensure only one item is visually highlighted at a time.
        let focus = gtk4::EventControllerFocus::new();
        focus.connect_enter({
            move |controller| {
                if let Some(widget) = controller.widget() {
                    clear_sibling_selected_visual_state(&widget);
                    widget.set_state_flags(gtk4::StateFlags::PRELIGHT, false);
                }
            }
        });
        focus.connect_leave({
            move |controller| {
                if let Some(widget) = controller.widget() {
                    widget.unset_state_flags(gtk4::StateFlags::PRELIGHT);
                }
            }
        });
        self.add_controller(focus);

        // Close nearest popover menu (aka submenu hosting this item and its parents) when clicked
        self.connect_clicked(|button| {
            if let Some(popover) = button
                .ancestor(gtk4::Popover::static_type())
                .and_then(|widget| widget.downcast::<gtk4::Popover>().ok())
            {
                // popdown instead of hide to cascade close all submenus
                popover.popdown();
            }
        });

        // Close nearest sibling popover menu (aka submenu of the same parent as this item) when mouse enters this item
        let motion = gtk4::EventControllerMotion::new();
        motion.connect_enter({
            move |controller, _, _| {
                let Some(widget) = controller.widget() else {
                    return;
                };

                if let Some(popover) = find_sibling_popover_menu(&widget) {
                    // hide instead of popdown to avoid cascade closing all submenus
                    popover.set_visible(false);
                }
            }
        });

        self.add_controller(motion);
    }
}

/// Heirarchy of widgets for menu items in GTK4:
///
/// - GtkModelButton        -> Normal Menu Item
///   |_ GtkLabel
/// - GtkGizmo              -> Custom Menu Item
///   |_ MudaIconMenuItem   <- we start here
/// - GtkModelButton        -> Menu Item with Submenu
///   |_ GtkLabel
///   |_ GtkPopoverMenu
/// - GtkModelButton        -> Normal Menu Item
///   |_ GtkLabel
fn find_sibling_popover_menu(widget: &gtk4::Widget) -> Option<gtk4::PopoverMenu> {
    let parent = widget.parent()?.parent()?;

    let mut child = parent.first_child();
    while let Some(sibling) = child {
        if let Some(menu) = find_child_popover_menu(&sibling) {
            return Some(menu);
        }

        child = sibling.next_sibling();
    }

    None
}

fn find_child_popover_menu(widget: &gtk4::Widget) -> Option<gtk4::PopoverMenu> {
    let mut child = widget.first_child();
    while let Some(sibling) = child {
        if let Some(menu) = sibling.downcast_ref::<gtk4::PopoverMenu>() {
            return Some(menu.clone());
        }

        child = sibling.next_sibling();
    }

    None
}

fn clear_sibling_selected_visual_state(widget: &gtk4::Widget) {
    let Some(row) = widget.parent() else {
        return;
    };
    let Some(parent) = row.parent() else {
        return;
    };

    let mut child = parent.first_child();
    while let Some(sibling) = child {
        if sibling != row {
            sibling.unset_state_flags(gtk4::StateFlags::PRELIGHT | gtk4::StateFlags::SELECTED);
        }

        child = sibling.next_sibling();
    }
}
