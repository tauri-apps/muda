---
"muda": minor
---

Add `BadIcon::ZeroSized` error variant, returned by `Icon::from_rgba` when `width` or `height` is zero, instead of panicking at render time on some platforms. On Linux with the `gtk3` backend, `Icon::from_rgba` now also validates that `rgba` is divisible by 4 and matches `width * height`, as documented.
