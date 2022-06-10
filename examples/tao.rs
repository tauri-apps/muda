use muda::{menu_event_receiver, Menu, NativeMenuItem};
#[cfg(target_os = "linux")]
use tao::platform::unix::WindowExtUnix;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window2 = WindowBuilder::new().build(&event_loop).unwrap();

    let mut menu_bar = Menu::new();

    let mut file_menu = menu_bar.add_submenu("&File", true);
    let mut open_item = file_menu.add_text_item("&Open", true, None);
    let mut save_item = file_menu.add_text_item("&Save", true, Some("CommandOrCtrl+S"));
    file_menu.add_native_item(NativeMenuItem::Minimize);
    file_menu.add_native_item(NativeMenuItem::CloseWindow);
    file_menu.add_native_item(NativeMenuItem::Quit);

    let mut edit_menu = menu_bar.add_submenu("&Edit", true);
    edit_menu.add_native_item(NativeMenuItem::Cut);
    edit_menu.add_native_item(NativeMenuItem::Copy);
    edit_menu.add_native_item(NativeMenuItem::Paste);
    edit_menu.add_native_item(NativeMenuItem::SelectAll);

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

    let menu_channel = menu_event_receiver();
    let mut open_item_disabled = false;
    let mut counter = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            #[cfg(target_os = "macos")]
            Event::NewEvents(tao::event::StartCause::Init) => {
                menu_bar.init_for_nsapp();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                // window.request_redraw();
            }
            _ => (),
        }

        if let Ok(event) = menu_channel.try_recv() {
            match event.id {
                _ if event.id == save_item.id() => {
                    println!("Save menu item activated!");
                    counter += 1;
                    save_item.set_label(format!("&Save activated {counter} times"));

                    if !open_item_disabled {
                        println!("Open item disabled!");
                        open_item.set_enabled(false);
                        open_item_disabled = true;
                    }
                }
                _ => {}
            }
        }
    })
}
