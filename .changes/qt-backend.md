---
"muda": minor
---

Add a Qt Widgets backend for Linux/BSD behind the `qt` feature, built on the `qtshim` crate: `Menu::init_for_qt_window`, `ContextMenu::show_context_menu_for_qt_window` and friends. When `gtk3` or `gtk4` is also enabled, the GTK backend is used and a build warning is emitted.
