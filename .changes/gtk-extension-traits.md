---
"muda": minor
---

Split GTK 3 and GTK 4 menu APIs into backend-specific extension traits:

- Removed `Menu::init_for_gtk_window`, use `MenuGtkExt::init_for_gtk_window` on GTK 3 or `MenuGtk4Ext::init_for_gtk_window` on GTK 4 instead.
- Removed `Menu::remove_for_gtk_window`, use `MenuGtkExt::remove_for_gtk_window` on GTK 3 or `MenuGtk4Ext::remove_for_gtk_window` on GTK 4 instead.
- Removed `Menu::hide_for_gtk_window`, use `MenuGtkExt::hide_for_gtk_window` on GTK 3 or `MenuGtk4Ext::hide_for_gtk_window` on GTK 4 instead.
- Removed `Menu::show_for_gtk_window`, use `MenuGtkExt::show_for_gtk_window` on GTK 3 or `MenuGtk4Ext::show_for_gtk_window` on GTK 4 instead.
- Removed `Menu::is_visible_on_gtk_window`, use `MenuGtkExt::is_visible_on_gtk_window` on GTK 3 or `MenuGtk4Ext::is_visible_on_gtk_window` on GTK 4 instead.
- Removed the GTK 3 `Menu::gtk_menubar_for_gtk_window`; the replacement borrows the menu instead of consuming it, use `MenuGtkExt::gtk_menubar_for_gtk_window` instead.
- Removed the GTK 4 `Menu::gtk_menubar_for_gtk_window`; the replacement borrows the menu instead of consuming it, use `MenuGtk4Ext::gtk_popover_menubar_for_gtk_window` instead.
- Removed `ContextMenu::show_context_menu_for_gtk_window`, use `ContextMenuGtkExt::show_context_menu_for_gtk_window` on GTK 3 or `ContextMenuGtk4Ext::show_context_menu_for_gtk_window` on GTK 4 instead.
- Removed the GTK 3 `ContextMenu::gtk_context_menu`, use `ContextMenuGtkExt::gtk_menu` instead.
- Removed the GTK 4 `ContextMenu::gtk_context_menu`, use `ContextMenuGtk4Ext::gtk_popover_menu` instead.

Muda no longer aliases the `gtk4` crate as `gtk`: GTK 3 signatures use `gtk` types and GTK 4 signatures use `gtk4` types.
