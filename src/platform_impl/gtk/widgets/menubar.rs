use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib};

mod item;
use item::*;

mod imp {
    use crate::gtk_widgets::{get_icon_for_item, get_label_for_item};

    use super::*;
    use std::cell::RefCell;

    use glib::Properties;
    use gtk4::{prelude::*, BoxLayout, Orientation};

    #[derive(Default, Properties, Debug)]
    #[properties(wrapper_type = super::MenuBar)]
    pub struct MenuBar {
        #[property(get, set = set_menu_model)]
        pub menu_model: RefCell<gio::Menu>,
        pub active_item: RefCell<Option<MenuBarItem>>,
        items: RefCell<Vec<MenuBarItem>>,
    }

    fn set_menu_model(item: &MenuBar, menu: gio::Menu) {
        item.menu_model.replace(menu);

        item.build_menu_bar();
    }

    impl MenuBar {
        pub fn build_menu_bar(&self) {
            let mut items = self.items.borrow_mut();
            for item in items.drain(..) {
                item.unparent();
            }

            let menu = self.menu_model.borrow().clone();

            let obj = self.obj();
            for index in 0..menu.n_items() {
                if menu.item_link(index, "submenu").is_some() {
                    let item = MenuBarItem::new(
                        get_label_for_item(&menu, index)
                            .unwrap_or_default()
                            .as_ref(),
                        get_icon_for_item(&menu, index).as_ref(),
                    );

                    item.set_parent(obj.upcast_ref::<gtk4::Widget>());
                    items.push(item);
                }
            }
        }

        pub fn set_active_item(&self, item: Option<MenuBarItem>, _open_popup: bool) {
            let mut active_item = self.active_item.borrow_mut();

            let item_changed = active_item.as_ref() != item.as_ref();
            if item_changed {
                if let Some(item) = active_item.as_ref() {
                    item.set_selected(false);
                }

                if let Some(item) = item.as_ref() {
                    item.set_selected(true);
                }
            }

            if let Some(item) = item.as_ref() {
                if _open_popup {
                    item.grab_focus();
                }
            }

            *active_item = item;
        }
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

            let obj = self.obj();
            let motion_controller = gtk4::EventControllerMotion::new();

            motion_controller.connect_leave(|motion| {
                let menu_bar = motion
                    .widget()
                    .expect("Cannot get motion widget")
                    .downcast::<super::MenuBar>()
                    .unwrap();

                menu_bar.set_active_item(None, false);
            });

            obj.add_controller(motion_controller);

            obj.set_layout_manager(Some(BoxLayout::new(Orientation::Horizontal)));
        }
    }

    impl WidgetImpl for MenuBar {
        fn focus(&self, direction_type: gtk4::DirectionType) -> bool {
            let obj = self.obj();

            match direction_type {
                gtk4::DirectionType::Left => {
                    let previous = self
                        .active_item
                        .borrow()
                        .as_ref()
                        .and_then(|item| {
                            item.prev_sibling()
                                .and_then(|sibling| sibling.downcast_ref::<MenuBarItem>().cloned())
                        })
                        .or_else(|| {
                            obj.first_child()
                                .and_then(|child| child.downcast_ref::<MenuBarItem>().cloned())
                        });

                    if let Some(previous) = previous {
                        self.set_active_item(Some(previous), false);
                    }
                }

                gtk4::DirectionType::Right => {
                    let next = self
                        .active_item
                        .borrow()
                        .as_ref()
                        .and_then(|item| {
                            item.next_sibling()
                                .and_then(|sibling| sibling.downcast_ref::<MenuBarItem>().cloned())
                        })
                        .or_else(|| {
                            obj.last_child()
                                .and_then(|child| child.downcast_ref::<MenuBarItem>().cloned())
                        });

                    if let Some(next) = next {
                        self.set_active_item(Some(next), false);
                    }
                }
                _ => (),
            }

            true
        }
    }
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

    pub(crate) fn set_active_item(&self, item: Option<MenuBarItem>, _open_popup: bool) {
        self.imp().set_active_item(item, _open_popup);
    }
}
