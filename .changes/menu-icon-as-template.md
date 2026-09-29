---
"muda": minor
---

Add `IconMenuItem::set_icon_as_template`, `IconMenuItem::icon_as_template`,
`Submenu::set_icon_as_template` and `Submenu::icon_as_template` to opt an item's icon
into being drawn as a macOS template image, so the system recolours it to match the
menu the way the built-in items are. Defaults to `false`, leaving existing behaviour
unchanged. Native icons are left to the system. No-op on Windows and Linux.
