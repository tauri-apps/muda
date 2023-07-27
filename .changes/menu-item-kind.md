---
"muda": "minor"
---

Removed `MenuItemType` enum and replaced with `MenuItemKind` enum. `Menu::items` and `Submenu::items` will now return `Vec<MenuItemKind>` instead of `Vec<Box<dyn MenuItemExt>>`
