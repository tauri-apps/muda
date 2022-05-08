use muda::{menu_event_receiver, Menu};
#[cfg(target_os = "windows")]
use winit::platform::windows::WindowExtWindows;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let _window2 = WindowBuilder::new().build(&event_loop).unwrap();

    let mut menu_bar = Menu::new();
    let mut file_menu = menu_bar.add_submenu("File", true);
    let mut edit_menu = menu_bar.add_submenu("Edit", true);

    let mut open_item = file_menu.add_text_item("Open", true);

    let mut save_item = file_menu.add_text_item("Save", true);
    let _quit_item = file_menu.add_text_item("Quit", true);

    let _copy_item = edit_menu.add_text_item("Copy", true);
    let _cut_item = edit_menu.add_text_item("Cut", true);

    #[cfg(target_os = "windows")]
    {
        menu_bar.init_for_hwnd(window.hwnd() as _);
        menu_bar.init_for_hwnd(_window2.hwnd() as _);
    }

    let menu_channel = menu_event_receiver();
    let mut open_item_disabled = false;
    let mut counter = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(event) = menu_channel.try_recv() {
            match event.id {
                _ if event.id == save_item.id() => {
                    println!("Save menu item activated!");
                    counter += 1;
                    save_item.set_label(format!("Save activated {counter} times"));

                    if !open_item_disabled {
                        println!("Open item disabled!");
                        open_item.set_enabled(false);
                        open_item_disabled = true;
                    }
                }
                _ => {}
            }
        }

        match event {
            #[cfg(target_os = "macos")]
            Event::NewEvents(winit::event::StartCause::Init) => {
                menu_bar.init_for_nsapp();
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => (),
        }
    })
}
