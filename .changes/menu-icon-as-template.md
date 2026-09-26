---
"muda": minor
---

Add `IconMenuItem::set_icon_as_template` and `IconMenuItem::icon_as_template` to
opt a menu item's custom icon into being drawn as a macOS template image, so the
system recolours it to match the menu the way the built-in items are. Defaults to
`false`, leaving existing behaviour unchanged. No-op on Windows and Linux.
