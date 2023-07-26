---
"muda": "minor"
---

**Breaking Change**: Changed `Menu::init_for_gtk_window` to accept a second argument for the container to which the menu bar should be added, if none was provided it will add it to the window directly. The method will no longer create a `gtk::Box` and append it to the window, instead you should add the box to the window yourself, then pass a reference to it the method so it can be used as the container for the menu bar.
