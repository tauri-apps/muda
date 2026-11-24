---
"muda": minor
---

`Icon::from_rgba` now returns the new `BadIcon::ZeroSized` error when `width` or `height` is zero, instead of creating an icon that panics when it is rendered.
