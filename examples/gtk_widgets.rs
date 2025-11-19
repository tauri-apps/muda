#[cfg(target_os = "linux")]
use gtk4::{
    gio::{self, prelude::*, File, FileIcon},
    glib,
    prelude::*,
};

const SPACING: i32 = 8;
const SPACING_LARGE: i32 = SPACING * 2;
const SPACING_XLARGE: i32 = SPACING * 4;

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This example is only available on Linux");
}

#[cfg(target_os = "linux")]
fn main() {
    let application = gtk4::Application::builder()
        .application_id("com.github.muda.example.gtk")
        .build();

    application.connect_activate(move |application| {
        let window = gtk4::ApplicationWindow::builder()
            .application(application)
            .title("GTK Menubar Example")
            .default_width(350)
            .default_height(350)
            .show_menubar(true)
            .build();

        let icon = FileIcon::new(&File::for_path(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/icon.png"
        ))));

        let menu = {
            let file_menu = {
                let icon_section = gio::Menu::new();
                let icon_with_label_menu_item =
                    gio::MenuItem::new(Some("Icon With Label"), Some("muda.icon"));

                icon_with_label_menu_item.set_icon(&icon);

                icon_section.append_item(&icon_with_label_menu_item); 

                let submenu_item = gio::MenuItem::new(Some("Icon Submenu"), None);

                let icon_submenu = gio::Menu::new();
                let icon_menu_item = gio::MenuItem::new(Some("Icon Accelerator"), Some("muda.icon-accel"));
                    
                icon_submenu.append_item(&icon_menu_item);

                submenu_item.set_submenu(Some(&icon_submenu));

                icon_section.append_item(&submenu_item); 

                let cool_section = gtk4::gio::Menu::new();
                let cool_menu_item = gtk4::gio::MenuItem::new(Some("Be Cool"), Some("muda.cool"));
                let cool_menu_shortcut_item =
                    gtk4::gio::MenuItem::new(Some("Shortcut"), Some("muda.cool-shortcut"));

                cool_section.append_item(&cool_menu_item);
                cool_section.append_item(&cool_menu_shortcut_item);

                let program_section = gio::Menu::new();
                let about_menu_item = gio::MenuItem::new(Some("About"), Some("muda.about"));
                let quit_menu_item = gio::MenuItem::new(Some("Quit"), Some("muda.quit"));

                program_section.append_item(&about_menu_item);
                program_section.append_item(&quit_menu_item);

                let group = gio::SimpleActionGroup::new();

                group.add_action(&gio::SimpleAction::new_stateful(
                    "cool",
                    None,
                    &true.to_variant(),
                ));

                group.add_action(&gio::SimpleAction::new_stateful(
                    "cool-shortcut",
                    None,
                    &true.to_variant(),
                ));

                group.add_action(&gio::SimpleAction::new("quit", None));
                group.add_action(&gio::SimpleAction::new("about", None));
                group.add_action(&gio::SimpleAction::new("icon", None));
                group.add_action(&gio::SimpleAction::new("icon-accel", None));

                window.insert_action_group("muda", Some(&group));

                let file_menu = gtk4::gio::Menu::new();

                file_menu.append_section(None, &icon_section);
                file_menu.append_section(Some("Cool Header"), &cool_section);
                file_menu.append_section(None, &program_section);

                file_menu
            };

            let menu = gio::Menu::new();

            menu.append_submenu(Some("File"), &file_menu);

            menu
        };

        application.set_accels_for_action("muda.about", &["<Control>C"]);
        application.set_accels_for_action("muda.quit", &["<Control>Q"]);
        application.set_accels_for_action("muda.cool-shortcut", &["<Control>W"]);

        application.set_accels_for_action("muda.icon-accel", &["<Control>I"]);

        setup_ui(&window, &icon, &menu);

        window.present();
    });

    application.run();
}

#[cfg(target_os = "linux")]
fn setup_ui(window: &gtk4::ApplicationWindow, icon: &FileIcon, menu: &gio::Menu) {
    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(SPACING)
        .margin_top(SPACING_LARGE)
        .margin_bottom(SPACING_LARGE)
        .hexpand(true)
        .halign(gtk4::Align::Fill)
        .build();

    let hbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(SPACING)
        .margin_start(SPACING)
        .margin_end(SPACING)
        .hexpand(true)
        .halign(gtk4::Align::Fill)
        .build();

    setup_gtk_popover_menu(&hbox, menu);
    setup_gtk_popover_menu(&hbox, menu);

    let image = gtk4::Image::builder()
        .gicon(icon)
        .icon_size(gtk4::IconSize::Large)
        .build();

    vbox.append(&hbox);
    vbox.append(&image);
    vbox.append(&gtk4::Label::new(Some(
        "Some menu items should have this icon",
    )));

    window.set_child(Some(&vbox));
}

#[cfg(target_os = "linux")]
fn setup_gtk_popover_menu(container: &gtk4::Box, menu: &gio::Menu) {
    use gtk4::PopoverMenuBar;

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, SPACING);
    let frame = gtk4::Frame::builder().hexpand(true).vexpand(true).build();
    let menu_bar = PopoverMenuBar::builder()
        .menu_model(menu)
        .valign(gtk4::Align::Start)
        .build();

    vbox.append(&menu_bar);
    vbox.append(
        &gtk4::Label::builder()
            .label("GTK Widgets\nRight Click to See Context Menu")
            .justify(gtk4::Justification::Center)
            .margin_top(SPACING_LARGE)
            .margin_bottom(SPACING_LARGE)
            .margin_start(SPACING_XLARGE)
            .margin_end(SPACING_XLARGE)
            .valign(gtk4::Align::Center)
            .xalign(0.5)
            .vexpand(true)
            .build(),
    );

    frame.set_child(Some(&vbox));

    container.append(&frame);
}

fn load_icon(path: &std::path::Path) -> muda::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    muda::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
