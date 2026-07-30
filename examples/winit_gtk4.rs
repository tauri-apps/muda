// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![allow(unused)]
use std::collections::HashMap;

use crossbeam_channel::{unbounded, Receiver};
use gtk4::prelude::*;
use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    dpi::{PhysicalPosition, Position},
    AboutMetadata, CheckMenuItem, ContextMenu, IconMenuItem, Menu, MenuEvent, MenuItem, NativeIcon,
    PredefinedMenuItem, Submenu,
};
use winit_gtk4::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    platform::gtk4::{EventLoopBuilderExtGtk4, WindowExtGtk4},
    window::{Window, WindowAttributes, WindowId},
};

fn main() {
    let mut event_loop_builder = EventLoop::builder();
    event_loop_builder
        .with_gtk4()
        .with_application_id("com.github.tauri.muda.winit-gtk4".to_string());

    let menu_bar = Menu::new();
    let event_loop = event_loop_builder.build().unwrap();

    let (menu_sender, menu_receiver) = unbounded();
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_sender.send(event);
        proxy.wake_up();
    }));

    event_loop
        .run_app(App {
            app_menu: AppMenu::new(menu_bar),
            windows: HashMap::default(),
            redraw_window: None,
            cursor_position: PhysicalPosition::new(0., 0.),
            use_window_pos: false,
            menu_receiver,
        })
        .unwrap();
}

struct App {
    app_menu: AppMenu,
    windows: HashMap<WindowId, Box<dyn Window>>,
    redraw_window: Option<WindowId>,
    cursor_position: PhysicalPosition<f64>,
    use_window_pos: bool,
    menu_receiver: Receiver<MenuEvent>,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        let window = event_loop
            .create_window(WindowAttributes::default().with_title("Window 1"))
            .unwrap();
        let gtk_window = window.gtk_window().unwrap();

        let window2 = event_loop
            .create_window(WindowAttributes::default().with_title("Window 2"))
            .unwrap();
        let gtk_window2 = window2.gtk_window().unwrap();

        attach_menubar_window(&gtk_window, &self.app_menu.menu_bar);
        attach_menubar_window(&gtk_window2, &self.app_menu.menu_bar);

        self.redraw_window = Some(window.id());
        self.windows.insert(window.id(), window);
        self.windows.insert(window2.id(), window2);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }

            WindowEvent::PointerMoved { position, .. } => {
                self.cursor_position = PhysicalPosition::new(position.x, position.y);
            }

            WindowEvent::PointerButton { state, button, .. } => {
                if state == ElementState::Released
                    && button.mouse_button() == Some(MouseButton::Right)
                {
                    show_context_menu(
                        self.windows.get(&window_id).unwrap().as_ref(),
                        &self.app_menu.file_menu,
                        if self.use_window_pos {
                            Some(self.cursor_position.into())
                        } else {
                            None
                        },
                    );
                    self.use_window_pos = !self.use_window_pos;
                }
            }

            _ => {}
        }
    }

    fn proxy_wake_up(&mut self, _event_loop: &dyn ActiveEventLoop) {
        while let Ok(event) = self.menu_receiver.try_recv() {
            println!("{event:?}");

            if event.id == self.app_menu.custom_item.id() {
                let new_item = MenuItem::new("New Menu Item", true, None);
                self.app_menu.file_menu.insert(&new_item, 2).unwrap();
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &dyn ActiveEventLoop) {
        if let Some(window_id) = self.redraw_window.as_ref() {
            if let Some(window) = self.windows.get(window_id) {
                window.request_redraw();
            }
        }
    }
}

struct AppMenu {
    menu_bar: Menu,
    file_menu: Submenu,
    edit_menu: Submenu,
    window_menu: Submenu,
    custom_help_menu: Submenu,
    custom_item: MenuItem,
}

impl AppMenu {
    fn new(menu_bar: Menu) -> Self {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/icon.png");
        let icon = load_icon(std::path::Path::new(path));

        let file_menu = Submenu::new("&File", true);
        let edit_menu = Submenu::new("&Edit", true);
        let window_menu = Submenu::new("&Window", true);
        let help_menu = Submenu::new("&Custom Help", true);

        window_menu.set_icon(Some(icon.clone()));

        menu_bar
            .append_items(&[&file_menu, &edit_menu, &window_menu, &help_menu])
            .unwrap();

        let custom_i_1 = MenuItem::with_id(
            "custom-i-1",
            "C&ustom 1",
            true,
            Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyC)),
        );

        let image_item = IconMenuItem::with_id(
            "image-custom-1",
            "&Image custom 1",
            true,
            Some(icon),
            Some(Accelerator::new(Some(Modifiers::CONTROL), Code::KeyC)),
        );
        let native_icon_item = IconMenuItem::with_id_and_native_icon(
            "native-icon-1",
            "Native icon",
            true,
            Some(NativeIcon::Folder),
            None,
        );

        let check_custom_i_1 =
            CheckMenuItem::with_id("check-custom-1", "Check Custom 1", true, true, None);
        let check_custom_i_2 =
            CheckMenuItem::with_id("check-custom-2", "Check Custom 2", false, true, None);
        let check_custom_i_3 = CheckMenuItem::with_id(
            "check-custom-3",
            "Check Custom 3",
            true,
            true,
            Some(Accelerator::new(Some(Modifiers::SHIFT), Code::KeyD)),
        );

        let copy_i = PredefinedMenuItem::copy(None);
        let cut_i = PredefinedMenuItem::cut(None);
        let paste_i = PredefinedMenuItem::paste(None);

        file_menu
            .append_items(&[
                &custom_i_1,
                &image_item,
                &native_icon_item,
                &window_menu,
                &PredefinedMenuItem::separator(),
                &check_custom_i_1,
                &check_custom_i_2,
            ])
            .unwrap();

        window_menu
            .append_items(&[
                &PredefinedMenuItem::minimize(None),
                &PredefinedMenuItem::maximize(None),
                &PredefinedMenuItem::close_window(Some("Close")),
                &PredefinedMenuItem::fullscreen(None),
                &PredefinedMenuItem::bring_all_to_front(None),
                &PredefinedMenuItem::about(
                    None,
                    Some(AboutMetadata {
                        name: Some("tao".to_string()),
                        version: Some("1.2.3".to_string()),
                        copyright: Some("Copyright tao".to_string()),
                        ..Default::default()
                    }),
                ),
                &check_custom_i_3,
                &image_item,
                &custom_i_1,
            ])
            .unwrap();

        help_menu
            .append_items(&[&MenuItem::new("Supposed to show search", true, None)])
            .unwrap();

        edit_menu
            .append_items(&[&copy_i, &PredefinedMenuItem::separator(), &paste_i])
            .unwrap();

        Self {
            menu_bar,
            file_menu,
            edit_menu,
            window_menu,
            custom_help_menu: help_menu,
            custom_item: custom_i_1,
        }
    }
}

fn attach_menubar_window(window: &gtk4::ApplicationWindow, menu_bar: &Menu) {
    let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    menu_bar
        .init_for_gtk_window(window, Some(&container))
        .unwrap();
    window.set_child(Some(&container));
}

fn show_context_menu(window: &dyn Window, menu: &dyn ContextMenu, position: Option<Position>) {
    println!("Show context menu at position {position:?}");
    if let Some(gtk_window) = window.gtk_window() {
        menu.show_context_menu_for_gtk_window(gtk_window.upcast_ref::<gtk4::Window>(), position);
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
