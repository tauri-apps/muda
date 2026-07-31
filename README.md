### muda

Menu Utilities library for Desktop Applications.

[![Documentation](https://img.shields.io/docsrs/muda)](https://docs.rs/muda/latest/muda/)

## Platforms supported:

- Windows
- macOS
- Linux/BSD with GTK 3
- Linux/BSD with GTK 4

## Platform-specific notes:

- On Windows, accelerators don't work unless the win32 message loop calls
  [`TranslateAcceleratorW`](https://docs.rs/windows-sys/latest/windows_sys/Win32/UI/WindowsAndMessaging/fn.TranslateAcceleratorW.html).
  See [`Menu::init_for_hwnd`](https://docs.rs/muda/latest/x86_64-pc-windows-msvc/muda/struct.Menu.html#method.init_for_hwnd) for more details

### Cargo Features

- `common-controls-v6`: Use `TaskDialogIndirect` API from `ComCtl32.dll` v6 on Windows for showing the predefined `About` menu item dialog.
- `libxdo`: Enables linking to `libxdo` for the GTK 3 backend. This is used by the predefined `Copy`, `Cut`, `Paste` and `SelectAll` menu items, and is enabled by default. It is not used by GTK 4.
- `serde`: Enables de/serializing the dpi types.
- `gtk`: Enables the GTK 3 backend on Linux or BSD platforms. This is enabled by default.
- `gtk4`: Enables the GTK 4 backend on Linux or BSD platforms. Use `default-features = false` when enabling this feature because the default features include `gtk`.

The `gtk` and `gtk4` features are mutually exclusive.

## Dependencies (Linux/BSD)

The `gtk` feature uses GTK 3 for menus. The `gtk4` feature uses GTK 4 for menus. `libxdo` is only used by the GTK 3 backend to make the predefined `Copy`, `Cut`, `Paste` and `SelectAll` menu items work when the `libxdo` feature is enabled.

Be sure to install the packages for the GTK backend you enabled before building:

#### Arch Linux / Manjaro:

```sh
# GTK 3 backend
pacman -S gtk3 xdotool

# GTK 4 backend
pacman -S gtk4
```

#### Debian / Ubuntu:

```sh
# GTK 3 backend
sudo apt install libgtk-3-dev libxdo-dev

# GTK 4 backend
sudo apt install libgtk-4-dev
```

## Dependencies in FreeBSD

Install these dependencies in order to compile `muda`. Instructions using `pkg`:

```sh
# GTK 3 backend
pkg install -y rust glib pkgconf gtk3 xdotool

# GTK 4 backend
pkg install -y rust glib pkgconf gtk4
```

## Example

Create the menu and add your items

```rs
let menu = Menu::new();
let menu_item2 = MenuItem::new("Menu item #2", false, None);
let submenu = Submenu::with_items("Submenu Outer", true,&[
  &MenuItem::new("Menu item #1", true, Some(Accelerator::new(Modifiers::ALT, Code::KeyD))),
  &PredefinedMenuItem::separator(),
  &menu_item2,
  &MenuItem::new("Menu item #3", true, None),
  &PredefinedMenuItem::separator(),
  &Submenu::with_items("Submenu Inner", true,&[
    &MenuItem::new("Submenu item #1", true, None),
    &PredefinedMenuItem::separator(),
    &menu_item2,
  ])
]);

```

Then add your root menu to a window on Windows, GTK 3, or GTK 4
or use it as your global app menu on macOS

```rs
// --snip--
#[cfg(target_os = "windows")]
unsafe { menu.init_for_hwnd(window.hwnd() as isize) };
#[cfg(target_os = "linux")]
menu.init_for_gtk_window(&gtk_window, Some(&vertical_gtk_box));
#[cfg(target_os = "macos")]
menu.init_for_nsapp();
```

## Context menus (Popup menus)

You can also use a [`Menu`] or a [`Submenu`] to show a context menu.

```rs
// --snip--
let position = muda::dpi::PhysicalPosition { x: 100., y: 120. };
#[cfg(target_os = "windows")]
unsafe { menu.show_context_menu_for_hwnd(window.hwnd() as isize, Some(position.into())) };
#[cfg(target_os = "linux")]
menu.show_context_menu_for_gtk_window(&gtk_window, Some(position.into()));
#[cfg(target_os = "macos")]
unsafe { menu.show_context_menu_for_nsview(nsview, Some(position.into())) };
```

## Processing menu events

You can use `MenuEvent::receiver` to get a reference to the `MenuEventReceiver`
which you can use to listen to events when a menu item is activated

```rs
if let Ok(event) = MenuEvent::receiver().try_recv() {
    match event.id {
        _ if event.id == save_item.id() => {
            println!("Save menu item activated");
        },
        _ => {}
    }
}
```

### Note for [winit] or [tao] users:

You should use [`MenuEvent::set_event_handler`] and forward
the menu events to the event loop by using [`EventLoopProxy`]
so that the event loop is awakened on each menu event.

```rust
enum UserEvent {
  MenuEvent(muda::MenuEvent)
}

let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();

let proxy = event_loop.create_proxy();
muda::MenuEvent::set_event_handler(Some(move |event| {
    proxy.send_event(UserEvent::MenuEvent(event));
}));
```

[`EventLoopProxy`]: https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html
[winit]: https://docs.rs/winit
[tao]: https://docs.rs/tao

## License

Apache-2.0/MIT
