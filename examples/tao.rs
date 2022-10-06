use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    menu_event_receiver, predefined, CheckMenuItem, Menu, Submenu, TextMenuItem,
};
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

    let menu_bar = Menu::new();

    let file_m = Submenu::new("File", true);
    let edit_m = Submenu::new("Edit", true);
    let window_m = Submenu::new("Window", true);

    menu_bar.add_menu_item(&file_m);
    menu_bar.add_menu_item(&edit_m);
    menu_bar.add_menu_item(&window_m);

    let custom_i_1 = TextMenuItem::new("C&ustom 1", true, None);
    let custom_i_2 = TextMenuItem::new(
        "Custom 2",
        false,
        Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyC)),
    );
    let custom_i_3 = CheckMenuItem::new("Check Custom 1", true, true, None);
    let custom_i_4 = CheckMenuItem::new("Check Custom 2", true, false, None);
    let custom_i_5 = CheckMenuItem::new(
        "Check Custom 3",
        false,
        true,
        Some(Accelerator::new(Some(Modifiers::SHIFT), Code::KeyD)),
    );

    let copy_i = predefined::copy(None);
    let cut_i = predefined::cut(None);
    let paste_i = predefined::paste(None);

    file_m.add_menu_item(&custom_i_1);
    file_m.add_menu_item(&custom_i_2);
    file_m.add_menu_item(&predefined::separator());
    file_m.add_menu_item(&custom_i_3);
    window_m.add_menu_item(&custom_i_4);
    window_m.add_menu_item(&predefined::close_window(None));
    window_m.add_menu_item(&predefined::separator());
    window_m.add_menu_item(&predefined::quit(None));
    window_m.add_menu_item(&predefined::select_all(None));
    window_m.add_menu_item(&predefined::about(None, None));
    window_m.add_menu_item(&predefined::minimize(None));
    window_m.add_menu_item(&custom_i_5);
    window_m.add_menu_item(&custom_i_1);
    edit_m.add_menu_item(&copy_i);
    edit_m.add_menu_item(&predefined::separator());
    edit_m.add_menu_item(&cut_i);
    edit_m.add_menu_item(&paste_i);
    window_m.add_menu_item(&cut_i);

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
            dbg!(custom_i_3.is_checked());
            println!("{:?}", event);
        }
    })
}
