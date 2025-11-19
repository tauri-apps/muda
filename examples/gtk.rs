#[cfg(target_os = "linux")]
use gtk4::prelude::*;
#[cfg(target_os = "linux")]
use keyboard_types::{Code, Modifiers};
#[cfg(target_os = "linux")]
use muda::{accelerator::Accelerator, ContextMenu, MenuEvent};

#[cfg(target_os = "linux")]
fn main() {
    let application = gtk4::Application::builder()
        .application_id("com.github.muda.example.gtk")
        .build();

    application.connect_startup(|_| {
        MenuEvent::set_event_handler(Some(|event| {
            println!("{event:?}");
        }))
    });

    application.connect_activate(move |application| {
        let window = gtk4::ApplicationWindow::builder()
            .application(application)
            .title("GTK Menubar Example")
            .default_width(350)
            .default_height(350)
            .show_menubar(true)
            .build();

            setup_ui(&window);
    });

    application.run();
}

#[cfg(target_os = "linux")]
fn setup_ui(window: &gtk4::ApplicationWindow) {
    let about_menu_item = muda::MenuItem::new("About", true, None);

    let check = muda::CheckMenuItem::new(
        "Check",
        true,
        true,
        Some(Accelerator::new(Modifiers::empty(), Code::KeyQ)),
    );

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/icon.png");
    let icon = load_icon(std::path::Path::new(path));
    let icon_menu_item = muda::IconMenuItem::new("Icon", true, Some(icon), None);

    let quit_menu_item = muda::MenuItem::with_id(
        "quit",
        "&Quit",
        true,
        Some(Accelerator::new(Modifiers::CONTROL, Code::KeyQ)),
    );

    let file_menu = muda::Submenu::new("&File", true);
    file_menu.append(&about_menu_item).unwrap();
    file_menu.append(&check).unwrap();
    file_menu.append(&icon_menu_item).unwrap();
    file_menu.append(&quit_menu_item).unwrap();

    let menubar = muda::Menu::new();
    menubar.append(&file_menu).unwrap();

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    menubar.init_for_gtk_window(window, Some(&vbox)).unwrap();

    let btn = gtk4::Button::with_label("ASdasd");
    let w = window.clone();

    btn.connect_clicked(move |_| {
        file_menu.show_context_menu_for_gtk_window(w.dynamic_cast_ref().unwrap(), None);
    });

    vbox.append(&btn);

    window.set_child(Some(&vbox));
    window.present();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This example is only available on Linux");
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
