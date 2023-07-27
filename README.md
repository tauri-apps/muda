muda is a Menu Utilities library for Desktop Applications.

## Platforms supported:

- Windows
- macOS
- Linux (gtk Only)

## Platform-specific notes:

- On Windows, accelerators don't work unless the win32 message loop calls
  [`TranslateAcceleratorW`](https://docs.rs/windows-sys/latest/windows_sys/Win32/UI/WindowsAndMessaging/fn.TranslateAcceleratorW.html).
  See [`Menu::init_for_hwnd`](https://docs.rs/muda/latest/x86_64-pc-windows-msvc/muda/struct.Menu.html#method.init_for_hwnd) for more details

### Cargo Features

- `common-controls-v6`: Use `TaskDialogIndirect` API from `ComCtl32.dll` v6 on Windows for showing the predefined `About` menu item dialog.
- `libxdo`: Enables linking to `libxdo` on Linux which is used for the predfined `Copy`, `Cut`, `Paste` and `SelectAll` menu item.
- `serde`: Enables de/serializing the dpi types.

## Dependencies (Linux Only)

`gtk` is used for menus and `libxdo` is used to make the predfined `Copy`, `Cut`, `Paste` and `SelectAll` menu items work. Be sure to install following packages before building:

#### Arch Linux / Manjaro:

```sh
pacman -S gtk3 xdotool
```

#### Debian / Ubuntu:

```sh
sudo apt install libgtk-3-dev libxdo-dev
```

## Example

Create the menu and add your items

```rs
let menu = Menu::new();
let menu_item2 = MenuItem::new("Menu item #2", false, None);
let submenu = Submenu::with_items("Submenu Outer", true,&[
  &MenuItem::new("Menu item #1", true, Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyD))),
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

Then Add your root menu to a Window on Windows and Linux Only or use it
as your global app menu on macOS

```rs
// --snip--
#[cfg(target_os = "windows")]
menu.init_for_hwnd(window.hwnd() as isize);
#[cfg(target_os = "linux")]
menu.init_for_gtk_window(&gtk_window);
#[cfg(target_os = "macos")]
menu.init_for_nsapp();
```

## Context menus (Popup menus)

You can also use a [`Menu`] or a [`Submenu`] show a context menu.

```rs
// --snip--
let x = 100;
let y = 120;
#[cfg(target_os = "windows")]
menu.show_context_menu_for_hwnd(window.hwnd() as isize, x, y);
#[cfg(target_os = "linux")]
menu.show_context_menu_for_gtk_window(&gtk_window, x, y);
#[cfg(target_os = "macos")]
menu.show_context_menu_for_nsview(nsview, x, y);
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

## License

Apache-2.0/MIT
