#![allow(unused)]
use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    menu_event_receiver, AboutMetadata, CheckMenuItem, ContextMenu, Menu, MenuItem,
    PredefinedMenuItem, Submenu,
};
#[cfg(target_os = "macos")]
use winit::platform::macos::{EventLoopBuilderExtMacOS, WindowExtMacOS};
#[cfg(target_os = "windows")]
use winit::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

fn main() {
    let mut event_loop_builder = EventLoopBuilder::new();

    let menu_bar = Menu::new();

    #[cfg(target_os = "windows")]
    {
        let menu_bar_c = menu_bar.clone();
        event_loop_builder.with_msg_hook(move |msg| {
            use windows_sys::Win32::UI::WindowsAndMessaging::{TranslateAcceleratorW, MSG};
            unsafe {
                let msg = msg as *const MSG;
                let translated = TranslateAcceleratorW((*msg).hwnd, menu_bar_c.haccel(), msg);
                translated == 1
            }
        });
    }
    #[cfg(target_os = "macos")]
    event_loop_builder.with_default_menu(false);

    #[allow(unused_mut)]
    let mut event_loop = event_loop_builder.build();

    let window = WindowBuilder::new().with_title("Window 1").build(&event_loop).unwrap();
    let window2 = WindowBuilder::new().with_title("Window 2").build(&event_loop).unwrap();

    #[cfg(target_os = "macos")]
    {
        let app_m = Submenu::new("App", true);
        menu_bar.append(&app_m);
        app_m.append_items(&[
            &PredefinedMenuItem::about(None, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ]);
    }

    let file_m = Submenu::new("&File", true);
    let edit_m = Submenu::new("&Edit", true);
    let test_m = Submenu::new("&Test", true);
    let window_m = Submenu::new("&Window", true);

    menu_bar.append_items(&[&file_m, &edit_m, &window_m]);
    menu_bar.insert(&test_m, 3);

    let custom_i_1 = MenuItem::new(
        "C&ustom 1",
        true,
        Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyC)),
    );
    let custom_i_2 = MenuItem::new("Custom 2", false, None);
    let check_custom_i_1 = CheckMenuItem::new("Check Custom 1", true, true, None);
    let check_custom_i_2 = CheckMenuItem::new("Check Custom 2", false, true, None);
    let check_custom_i_3 = CheckMenuItem::new(
        "Check Custom 3",
        true,
        true,
        Some(Accelerator::new(Some(Modifiers::SHIFT), Code::KeyD)),
    );

    let copy_i = PredefinedMenuItem::copy(None);
    let cut_i = PredefinedMenuItem::cut(None);
    let paste_i = PredefinedMenuItem::paste(None);
    let select_i = PredefinedMenuItem::select_all(None);

    let submenu_m = Submenu::new("Submenu", true);
    submenu_m.append_items(&[
        &MenuItem::new("Submenu Item 1", true, None),
        &MenuItem::new("Submenu Item 2", true, None),
    ]);

    file_m.append_items(&[
        &custom_i_1,
        &custom_i_2,
        &window_m,
        &PredefinedMenuItem::separator(),
        &submenu_m,
        &check_custom_i_1,
    ]);

    window_m.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::maximize(None),
        &PredefinedMenuItem::close_window(Some("Close")),
        &PredefinedMenuItem::fullscreen(None),
    ]);

    test_m.append_items(&[
        &check_custom_i_1,
        &check_custom_i_2,
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
        &select_i,
        &select_i,
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("winit".to_string()),
                copyright: Some("Copyright winit".to_string()),
                ..Default::default()
            }),
        ),
        &check_custom_i_3,
        &custom_i_2,
        &custom_i_1,
    ]);

    test_m.insert(&cut_i, 2);
    test_m.remove(&select_i);

    edit_m.append_items(&[&copy_i, &paste_i, &PredefinedMenuItem::separator()]);
    edit_m.prepend(&cut_i);
    edit_m.append(&select_i);

    file_m.set_text("Hello World");

    custom_i_2.set_text("Foo && Bar");
    check_custom_i_2.set_checked(false);
    check_custom_i_3.set_enabled(false);

    #[cfg(target_os = "windows")]
    {
        menu_bar.init_for_hwnd(window.hwnd() as _);
        menu_bar.init_for_hwnd(window2.hwnd() as _);
    }
    #[cfg(target_os = "macos")]
    {
        menu_bar.init_for_nsapp();
        window_m.set_windows_menu_for_nsapp();
    }

    let menu_channel = menu_event_receiver();

    let mut x = 0_f64;
    let mut y = 0_f64;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            // #[cfg(target_os = "macos")]
            // Event::NewEvents(winit::event::StartCause::Init) => {
            //     menu_bar.init_for_nsapp();
            // }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                window_id,
                ..
            } => {
                if window_id == window.id() {
                    x = position.x;
                    y = position.y;
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Right,
                        ..
                    },
                window_id,
                ..
            } => {
                if window_id == window2.id() {
                    #[cfg(target_os = "windows")]
                    window_m.show_context_menu_for_hwnd(window2.hwnd(), x, y);
                    #[cfg(target_os = "macos")]
                    menu_bar.show_context_menu_for_nsview(window2.ns_view() as _);
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => (),
        }

        if let Ok(event) = menu_channel.try_recv() {
            if event.id == custom_i_1.id() {
                file_m.insert(&MenuItem::new("asdasd", false, None), 2);
            }
            println!("{:?}", event);
        }
    })
}
