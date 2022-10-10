use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    menu_event_receiver,
    predefined::{self, AboutMetadata},
    CheckMenuItem, Menu, Submenu, TextMenuItem,
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

    menu_bar.append_list(&[&file_m, &edit_m, &window_m]);

    let custom_i_1 = TextMenuItem::new("C&ustom 1", true, None);
    let custom_i_2 = TextMenuItem::new(
        "Custom 2",
        false,
        Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyC)),
    );
    let check_custom_i_1 = CheckMenuItem::new("Check Custom 1", true, true, None);
    let check_custom_i_2 = CheckMenuItem::new("Check Custom 2", true, false, None);
    let check_custom_i_3 = CheckMenuItem::new(
        "Check Custom 3",
        false,
        true,
        Some(Accelerator::new(Some(Modifiers::SHIFT), Code::KeyD)),
    );

    let copy_i = predefined::copy(None);
    let cut_i = predefined::cut(None);
    let paste_i = predefined::paste(None);

    file_m.append_list(&[
        &custom_i_1,
        &custom_i_2,
        &predefined::separator(),
        &check_custom_i_1,
    ]);

    window_m.append_list(&[
        &check_custom_i_2,
        &predefined::close_window(None),
        &predefined::separator(),
        &predefined::quit(None),
        &predefined::select_all(None),
        &predefined::about(
            None,
            Some(AboutMetadata {
                name: Some("tao".to_string()),
                copyright: Some("Copyright TAO".to_string()),
                ..Default::default()
            }),
        ),
        &predefined::minimize(None),
        &check_custom_i_3,
        &custom_i_1,
    ]);

    edit_m.append_list(&[&copy_i, &predefined::separator(), &cut_i]);

    edit_m.prepend(&paste_i);
    window_m.insert(&cut_i, 2);

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
                window.request_redraw();
            }
            _ => (),
        }

        if let Ok(event) = menu_channel.try_recv() {
            if event.id == custom_i_1.id() {
                file_m.insert(&TextMenuItem::new("asdasd", false, None), 2);
            }
            println!("{:?}", event);
        }
    })
}
