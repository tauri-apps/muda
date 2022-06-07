# muda

Menu Utilities for Desktop Applications.

## Example

Create the root menu and add submenus and men items.
```rs
let mut menu = Menu::new();

let file_menu = menu.add_submenu("File", true);
let open_item = file_menu.add_text_item("Open", true);
let save_item = file_menu.add_text_item("Save", true);

let edit_menu = menu.add_submenu("Edit", true);
let copy_item = file_menu.add_text_item("Copy", true);
let cut_item = file_menu.add_text_item("Cut", true);

#[cfg(target_os = "windows")]
menu.init_for_hwnd(window.hwnd() as isize);
#[cfg(target_os = "linux")]
menu.init_for_gtk_window(&gtk_window);
#[cfg(target_os = "macos")]
menu.init_for_nsapp();
```
Then listen for the events
```rs
if let Ok(event) = menu_event_receiver().try_recv() {
    match event.id {
        _ if event.id == save_item.id() => {
            println!("Save menu item activated");
        },
        _ => {}
    }
}
```
