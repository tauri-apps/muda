#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use gtk::prelude::*;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use keyboard_types::{Code, Modifiers};
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use muda::{accelerator::Accelerator, MenuEvent};

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn main() {
    // Create a new application
    let application = gtk::Application::builder()
        .application_id("com.github.gtk4-rs.examples.menubar")
        .build();
    application.connect_startup(on_startup);
    application.connect_activate(on_activate);
    application.run();
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn on_startup(_: &gtk::Application) {
    MenuEvent::set_event_handler(Some(|event| {
        println!("{event:?}");
    }));
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn on_activate(application: &gtk::Application) {
    use muda::ContextMenu;

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Menubar Example")
        .default_width(350)
        .default_height(350)
        .show_menubar(true)
        .build();

    window.present();

    let about_menu_item = muda::MenuItem::new("About", true, None);

    let check = muda::CheckMenuItem::new(
        "Check",
        true,
        true,
        Some(Accelerator::new(Some(Modifiers::empty()), Code::KeyQ)),
    );

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/icon.png");
    let icon = load_icon(std::path::Path::new(path));
    let icon_menu_item = muda::IconMenuItem::new("Icon", true, Some(icon), None);

    let quit_menu_item = muda::MenuItem::with_id(
        "quit",
        "&Quit",
        true,
        Some(Accelerator::new(Some(Modifiers::CONTROL), Code::KeyQ)),
    );

    let file_menu = muda::Submenu::new("&File", true);
    file_menu.append(&about_menu_item).unwrap();
    file_menu.append(&check).unwrap();
    file_menu.append(&icon_menu_item).unwrap();
    file_menu.append(&quit_menu_item).unwrap();

    let menubar = muda::Menu::new();
    menubar.append(&file_menu).unwrap();

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    menubar.init_for_gtk_window(&window, Some(&vbox)).unwrap();

    let btn = gtk::Button::with_label("ASdasd");
    let w = window.clone();

    btn.connect_clicked(move |_| {
        file_menu.show_context_menu_for_gtk_window(w.dynamic_cast_ref().unwrap(), None);
    });
    vbox.append(&btn);

    window.set_child(Some(&vbox));
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn main() {}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
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
