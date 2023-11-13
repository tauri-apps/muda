---
"muda": "minor"
---

Changed `ContextMenu::show_context_menu_for_gtk_window` to take `gtk::Window` instead of `gtk::ApplicationWindow` and relaxed generic gtk constraints on the following methods:

- `MenuBar::init_for_gtk_window`
- `MenuBar::remove_for_gtk_window`
- `MenuBar::hide_for_gtk_window`
- `MenuBar::show_for_gtk_window`
- `MenuBar::is_visible_on_gtk_window`
- `MenuBar::gtk_menubar_for_gtk_window`
