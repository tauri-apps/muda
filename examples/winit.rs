// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![allow(unused)]
use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    AboutMetadata, CheckMenuItem, ContextMenu, IconMenuItem, Menu, MenuEvent, MenuItem,
    PhysicalPosition, Position, PredefinedMenuItem, Submenu,
};
#[cfg(target_os = "macos")]
use winit::platform::macos::{EventLoopBuilderExtMacOS, WindowExtMacOS};
#[cfg(target_os = "windows")]
use winit::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

fn main() {
    let mut event_loop_builder = EventLoopBuilder::new();

    let menu_bar = Menu::new();

    #[cfg(target_os = "windows")]
    {
        let menu_bar = menu_bar.clone();
        event_loop_builder.with_msg_hook(move |msg| {
            use windows_sys::Win32::UI::WindowsAndMessaging::{TranslateAcceleratorW, MSG};
            unsafe {
                let msg = msg as *const MSG;
                let translated = TranslateAcceleratorW((*msg).hwnd, menu_bar.haccel(), msg);
                translated == 1
            }
        });
    }
    #[cfg(target_os = "macos")]
    event_loop_builder.with_default_menu(false);

    let event_loop = event_loop_builder.build().unwrap();

    let window = WindowBuilder::new()
        .with_title("Window 1")
        .build(&event_loop)
        .unwrap();
    let window2 = WindowBuilder::new()
        .with_title("Window 2")
        .build(&event_loop)
        .unwrap();

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
    let window_m = Submenu::new("&Window", true);

    menu_bar.append_items(&[&file_m, &edit_m, &window_m]);

    let custom_i_1 = MenuItem::new(
        "C&ustom 1",
        true,
        Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyC)),
    );

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/icon.png");
    let icon = load_icon(std::path::Path::new(path));
    let image_item = IconMenuItem::new("Image Custom 1", true, Some(icon), None);

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

    file_m.append_items(&[
        &custom_i_1,
        &image_item,
        &window_m,
        &PredefinedMenuItem::separator(),
        &check_custom_i_1,
        &check_custom_i_2,
    ]);

    window_m.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::maximize(None),
        &PredefinedMenuItem::close_window(Some("Close")),
        &PredefinedMenuItem::fullscreen(None),
        &PredefinedMenuItem::bring_all_to_front(None),
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("winit".to_string()),
                version: Some("1.2.3".to_string()),
                copyright: Some("Copyright winit".to_string()),
                ..Default::default()
            }),
        ),
        &check_custom_i_3,
        &image_item,
        &custom_i_1,
    ]);

    edit_m.append_items(&[&copy_i, &PredefinedMenuItem::separator(), &paste_i]);

    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::*;
        if let RawWindowHandle::Win32(handle) = window.window_handle().unwrap().as_raw() {
            menu_bar.init_for_hwnd(handle.hwnd.get());
        }
        if let RawWindowHandle::Win32(handle) = window2.window_handle().unwrap().as_raw() {
            menu_bar.init_for_hwnd(handle.hwnd.get());
        }
    }
    #[cfg(target_os = "macos")]
    {
        menu_bar.init_for_nsapp();
        window_m.set_as_windows_menu_for_nsapp();
    }

    let menu_channel = MenuEvent::receiver();
    let mut window_cursor_position = PhysicalPosition { x: 0., y: 0. };
    let mut use_window_pos = false;

    event_loop.run(move |event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => event_loop.exit(),
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                window_id,
                ..
            } => {
                window_cursor_position.x = position.x;
                window_cursor_position.y = position.y;
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
                show_context_menu(
                    if window_id == window.id() {
                        &window
                    } else {
                        &window2
                    },
                    &file_m,
                    if use_window_pos {
                        Some(window_cursor_position.into())
                    } else {
                        None
                    },
                );
                use_window_pos = !use_window_pos;
            }
            _ => (),
        }

        if let Ok(event) = menu_channel.try_recv() {
            if event.id == custom_i_1.id() {
                file_m.insert(&MenuItem::new("New Menu Item", true, None), 2);
            }
            println!("{event:?}");
        }
    });
}

fn show_context_menu(window: &Window, menu: &dyn ContextMenu, position: Option<Position>) {
    println!("Show context menu at position {position:?}");
    #[cfg(target_os = "windows")]
    {
        use winit::raw_window_handle::*;
        if let RawWindowHandle::Win32(handle) = window.window_handle().unwrap().as_raw() {
            menu.show_context_menu_for_hwnd(handle.hwnd.get(), position);
        }
    }
    #[cfg(target_os = "macos")]
    {
        use winit::raw_window_handle::*;
        if let RawWindowHandle::AppKit(handle) = window.window_handle().unwrap().as_raw() {
            menu.show_context_menu_for_nsview(handle.ns_view.as_ptr(), position);
        }
    }
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
