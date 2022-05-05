use menu_rs::Menu;
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

    let mut menu_bar = Menu::new();
    let mut file_menu = menu_bar.add_submenu("File", true);
    let mut edit_menu = menu_bar.add_submenu("Edit", true);

    let _open_item = file_menu.add_text_item("Open", true, |_| {});
    let _save_item = file_menu.add_text_item("Save", true, |i| {
        i.set_enabled(false);
        i.set_label("Save disabled");
    });
    let _quit_item = file_menu.add_text_item("Quit", true, |_| {});

    let _copy_item = edit_menu.add_text_item("Copy", true, |_| {});
    let _cut_item = edit_menu.add_text_item("Cut", true, |_| {});

    #[cfg(target_os = "windows")]
    menu_bar.init_for_hwnd(window.hwnd() as _);

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
            _ => (),
        }
    })
}
