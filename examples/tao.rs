use menu_rs::Menu;
#[cfg(target_os = "linux")]
use tao::platform::unix::WindowExtUnix;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

enum UserEvent {
    MenuEvent(u64),
}

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event();

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window2 = WindowBuilder::new().build(&event_loop).unwrap();

    let mut menu_bar = Menu::new();
    let mut file_menu = menu_bar.add_submenu("File", true);
    let mut edit_menu = menu_bar.add_submenu("Edit", true);

    let mut open_item = file_menu.add_text_item("Open", true, |_| {});

    let proxy = event_loop.create_proxy();
    let mut counter = 0;
    let save_item = file_menu.add_text_item("Save", true, move |i| {
        counter += 1;
        i.set_label(format!("Save triggered {} times", counter));
        let _ = proxy.send_event(UserEvent::MenuEvent(i.id()));
    });
    let _quit_item = file_menu.add_text_item("Quit", true, |_| {});

    let _copy_item = edit_menu.add_text_item("Copy", true, |_| {});
    let _cut_item = edit_menu.add_text_item("Cut", true, |_| {});

    #[cfg(target_os = "windows")]
    {
        menu_bar.init_for_hwnd(window.hwnd() as _);
        menu_bar.init_for_hwnd(window2.hwnd() as _);
    }
    #[cfg(target_os = "linux")]
    {
        menu_bar.init_for_gtk_window(window.gtk_window());
        menu_bar.init_for_gtk_window(window2.gtk_window());
    }

    let mut open_item_disabled = false;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }

            Event::UserEvent(e) => match e {
                UserEvent::MenuEvent(id) => {
                    if id == save_item.id() {
                        println!("Save menu item triggered");

                        if !open_item_disabled {
                            println!("Open item disabled!");
                            open_item.set_enabled(false);
                            open_item_disabled = true;
                        }
                    }
                }
            },
            _ => (),
        }
    })
}
