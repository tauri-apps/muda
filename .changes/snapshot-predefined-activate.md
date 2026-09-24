---
"muda": minor
---

Add `PredefinedMenuItemSnapshot::activate`, which runs the item's predefined action from a menu snapshot: the about dialog, the edit commands, and on GTK 4 the window items. The ones that act on a window use the application's active one, as a snapshot has no menu of this process to take it from.
