---
"muda": minor
---

**Breaking Change:** On Linux, `Menu::inti_for_gtk_window` has been changed to require the second paramter to extend `gtk::Box`. This ensures that the menu bar is added at the beginning of the box instead of at the bottom.
