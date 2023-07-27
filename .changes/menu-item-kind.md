---
"muda": "minor"
---

**Breaking Change**: Removed `MenuItemType` enum and replaced with `MenuItemKind` enum. `Menu::items` and `Submenu::items` methods will now return `Vec<MenuItemKind>` instead of `Vec<Box<dyn MenuItemExt>>`
