// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::cell::OnceCell;

use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::{accelerator::MenuAccelerator, Icon};

use super::mnemonic::to_gtk_mnemonic;

const MENU_ICON_SIZE: i32 = 16;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct IconMenuItem {
        pub(super) image: OnceCell<gtk::Image>,
        pub(super) label: OnceCell<gtk::Label>,
        pub(super) accelerator_label: OnceCell<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IconMenuItem {
        const NAME: &'static str = "MudaIconMenuItem";
        type Type = super::IconMenuItem;
        type ParentType = gtk::Button;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("modelbutton");
            klass.set_accessible_role(gtk::AccessibleRole::MenuItem);
        }
    }

    impl ObjectImpl for IconMenuItem {}
    impl WidgetImpl for IconMenuItem {}
    impl ButtonImpl for IconMenuItem {}
}

glib::wrapper! {
    pub struct IconMenuItem(ObjectSubclass<imp::IconMenuItem>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl IconMenuItem {
    pub(super) fn new(
        text: &str,
        detailed_action: &str,
        icon: Option<&Icon>,
        native_icon: Option<&str>,
        accelerator: Option<&MenuAccelerator>,
    ) -> Self {
        let item: Self = glib::Object::new();
        item.set_has_frame(false);
        item.set_focus_on_click(false);
        item.add_css_class("flat");
        item.set_detailed_action_name(detailed_action);
        item.set_can_focus(true);
        item.set_focusable(true);
        item.set_halign(gtk::Align::Fill);
        item.set_hexpand(true);

        let image = gtk::Image::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .width_request(MENU_ICON_SIZE)
            .height_request(MENU_ICON_SIZE)
            .build();

        let label = gtk::Label::builder().xalign(0.0).hexpand(true).build();

        let accelerator_label = gtk::Label::builder()
            .css_name("accelerator")
            .margin_start(24)
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
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
        if icon.is_some() {
            item.set_icon(icon);
        } else {
            item.set_native_icon(native_icon);
        }
        item.set_accelerator(accelerator);

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

        image.set_icon_name(None);

        if let Some(icon) = icon {
            let texture = icon.inner.texture();
            image.set_paintable(Some(&texture));
        } else {
            image.set_paintable(gtk::gdk::Paintable::NONE);
        }
    }

    pub fn set_native_icon(&self, icon: Option<&str>) {
        let Some(image) = self.imp().image.get() else {
            return;
        };

        image.set_paintable(gtk::gdk::Paintable::NONE);
        image.set_icon_name(icon);
    }

    pub(super) fn set_accelerator(&self, accelerator: Option<&MenuAccelerator>) {
        let Some(accelerator_label) = self.imp().accelerator_label.get() else {
            return;
        };

        let accelerator = accelerator.and_then(|accelerator| {
            let (key, mods) = gtk::accelerator_parse(accelerator.to_gtk()?)?;
            let label = gtk::accelerator_get_label(key, mods);
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
        let focus = gtk::EventControllerFocus::new();
        focus.connect_enter({
            move |controller| {
                if let Some(widget) = controller.widget() {
                    clear_sibling_selected_visual_state(&widget);
                    widget.set_state_flags(gtk::StateFlags::PRELIGHT, false);
                }
            }
        });
        focus.connect_leave({
            move |controller| {
                if let Some(widget) = controller.widget() {
                    widget.unset_state_flags(gtk::StateFlags::PRELIGHT);
                }
            }
        });
        self.add_controller(focus);

        // Close nearest popover menu (aka submenu hosting this item and its parents) when clicked
        self.connect_clicked(|button| {
            if let Some(popover) = button
                .ancestor(gtk::Popover::static_type())
                .and_then(|widget| widget.downcast::<gtk::Popover>().ok())
            {
                // popdown instead of hide to cascade close all submenus
                popover.popdown();
            }
        });

        // Close nearest sibling popover menu (aka submenu of the same parent as this item) when mouse enters this item
        let motion = gtk::EventControllerMotion::new();
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

/// Heirarchy of widgets for menu items in gtk:
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
fn find_sibling_popover_menu(widget: &gtk::Widget) -> Option<gtk::PopoverMenu> {
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

fn find_child_popover_menu(widget: &gtk::Widget) -> Option<gtk::PopoverMenu> {
    let mut child = widget.first_child();
    while let Some(sibling) = child {
        if let Some(menu) = sibling.downcast_ref::<gtk::PopoverMenu>() {
            return Some(menu.clone());
        }

        child = sibling.next_sibling();
    }

    None
}

fn clear_sibling_selected_visual_state(widget: &gtk::Widget) {
    let Some(row) = widget.parent() else {
        return;
    };
    let Some(parent) = row.parent() else {
        return;
    };

    let mut child = parent.first_child();
    while let Some(sibling) = child {
        if sibling != row {
            sibling.unset_state_flags(gtk::StateFlags::PRELIGHT | gtk::StateFlags::SELECTED);
        }

        child = sibling.next_sibling();
    }
}
