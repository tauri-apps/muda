use keyboard_types::Code;
use muda::{
    accelerator::{Accelerator, Mods},
    menu_event_receiver, Menu, NativeMenuItem,
};
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;
#[cfg(target_os = "windows")]
use winit::platform::windows::{EventLoopBuilderExtWindows, WindowExtWindows};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

fn main() {
    let mut event_loop_builder = EventLoopBuilder::new();

    let mut menu_bar = Menu::new();

    #[cfg(target_os = "windows")]
    {
        let menu_bar_c = menu_bar.clone();
        event_loop_builder.with_msg_hook(move |msg| {
            use windows_sys::Win32::UI::WindowsAndMessaging::{TranslateAcceleratorW, MSG};
            unsafe {
                let msg = msg as *mut MSG;
                let translated = TranslateAcceleratorW((*msg).hwnd, menu_bar_c.haccel(), msg);
                translated == 1
            }
        });
    }
    #[cfg(target_os = "macos")]
    event_loop_builder.with_default_menu(false);

    #[allow(unused_mut)]
    let mut event_loop = event_loop_builder.build();

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let _window2 = WindowBuilder::new().build(&event_loop).unwrap();

    let mut file_menu = menu_bar.add_submenu("&File", true);
    let mut open_item = file_menu.add_item("&Open", true, None);
    let mut save_item = file_menu.add_item(
        "&Save",
        true,
        Some(Accelerator::new(Mods::CtrlMeta, Code::KeyS)),
    );
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
        menu_bar.init_for_hwnd(_window2.hwnd() as _);
    }

    #[cfg(target_os = "macos")]
    {
        menu_bar.init_for_nsapp();
    }

    let menu_channel = menu_event_receiver();
    let mut open_item_disabled = false;
    let mut counter = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            #[cfg(target_os = "macos")]
            Event::NewEvents(winit::event::StartCause::Init) => {
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
