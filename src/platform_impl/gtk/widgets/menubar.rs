use gtk4::{gio, glib};

mod imp {
    use super::*;
    use std::cell::RefCell;

    use glib::Properties;
    use gtk4::{prelude::*, subclass::prelude::*, BoxLayout, Orientation};

    #[derive(Default, Properties, Debug)]
    #[properties(wrapper_type = super::MenuBar)]
    pub struct MenuBar {
        #[property(get, set)]
        pub menu_model: RefCell<gio::Menu>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuBar {
        const NAME: &'static str = "MudaMenuBar";
        type Type = super::MenuBar;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("menubar")
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MenuBar {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .set_layout_manager(Some(BoxLayout::new(Orientation::Horizontal)));
        }
    }

    impl WidgetImpl for MenuBar {}
}

glib::wrapper! {
    pub struct MenuBar(ObjectSubclass<imp::MenuBar>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl MenuBar {
    pub fn new(menu: gio::Menu) -> Self {
        glib::Object::builder().property("menu-model", menu).build()
    }
}
