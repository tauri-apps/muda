use gtk4::{
    gio,
    glib::{self, object::ObjectExt, subclass::prelude::*},
    prelude::WidgetExt,
    StateFlags,
};

mod imp {
    use std::cell::RefCell;

    use crate::gtk_widgets::MenuBar;

    use super::*;
    use gtk4::{glib::Properties, prelude::*, subclass::prelude::*, BinLayout, Orientation};

    #[derive(Default, Properties, Debug)]
    #[properties(wrapper_type = super::MenuBarItem)]
    pub struct MenuBarItem {
        pub image: gtk4::Image,
        pub label: gtk4::Label,

        #[property(get, set)]
        pub icon: RefCell<Option<gio::Icon>>,

        #[property(get, set)]
        pub label_text: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuBarItem {
        const NAME: &'static str = "MudaMenuBarItem";
        type Type = super::MenuBarItem;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("item");
            klass.set_accessible_role(gtk4::AccessibleRole::MenuItem);
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MenuBarItem {
        fn constructed(&self) {
            self.parent_constructed();

            let hbox = gtk4::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(4)
                .build();

            let label = &self.label;
            label.set_use_underline(true);

            hbox.append(&self.image);
            hbox.append(label);

            let obj = self.obj();

            obj.set_focusable(true);
            obj.set_cursor_from_name(Some("pointer"));

            let click_gesture = gtk4::GestureClick::new();
            click_gesture.connect_pressed(move |gesture, _, _, _| {
                let item = gesture
                    .widget()
                    .expect("Cannot get gesture widget")
                    .downcast::<super::MenuBarItem>()
                    .unwrap();

                let menu_bar = item
                    .ancestor(MenuBar::static_type())
                    .expect("Menu bar doesn't exist")
                    .downcast::<MenuBar>()
                    .unwrap();

                menu_bar.set_active_item(Some(item), true);
            });

            obj.add_controller(click_gesture);

            let motion_controller = gtk4::EventControllerMotion::new();
            motion_controller.connect_enter(move |motion, _, _| {
                let item = motion
                    .widget()
                    .expect("Cannot get motion widget")
                    .downcast::<super::MenuBarItem>()
                    .unwrap();
                let menu_bar = item
                    .ancestor(MenuBar::static_type())
                    .expect("Menu bar doesn't exist")
                    .downcast::<MenuBar>()
                    .unwrap();

                menu_bar.set_active_item(Some(item), false);
            });

            obj.add_controller(motion_controller);

            obj.bind_property("icon", &self.image, "gicon")
                .sync_create()
                .build();

            obj.bind_property("icon", &self.image, "visible")
                .sync_create()
                .transform_to(|_, value: Option<gio::Icon>| {
                    Some(value.is_some())
                })
                .build();

            obj.bind_property("label-text", &self.label, "label")
                .sync_create()
                .build();

            obj.bind_property("label-text", label, "visible")
                .sync_create()
                .transform_to(|_, value: Option<String>| {
                    Some(value.as_ref().map(|s| !s.is_empty()).unwrap_or(false))
                })
                .build();

            obj.set_layout_manager(Some(BinLayout::new()));

            hbox.set_parent(obj.upcast_ref::<gtk4::Widget>());
        }
    }

    impl WidgetImpl for MenuBarItem {}
}

glib::wrapper! {
    pub struct MenuBarItem(ObjectSubclass<imp::MenuBarItem>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl MenuBarItem {
    pub fn new(label: &str, icon: Option<&gio::Icon>) -> Self {
        glib::Object::builder()
            .property("label-text", label)
            .property("icon", icon)
            .build()
    }

    pub(crate) fn set_selected(&self, active: bool) {
        if active {
            self.set_state_flags(StateFlags::SELECTED, false);
        } else {
            self.unset_state_flags(StateFlags::SELECTED);
        }
    }
}
