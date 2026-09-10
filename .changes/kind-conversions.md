---
"muda": minor
---

Implement `From/TryFrom` traits from `MenuItem/CheckMenuItem/PredefinedMenuItem/IconMenuItem/Submenu` for `MenuItemKind`, and `From/TryFrom` traits for `Menu/Submenu` for `MenuKind`. Failed `TryFrom` conversions return the original kind value.
