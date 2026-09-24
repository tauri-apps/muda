---
"muda": minor
---

**Breaking change** Remove the `libxdo` Cargo feature and the dependency on `libxdo`. The predefined `Copy`, `Cut`, `Paste` and `SelectAll` items now act on the application's focused widget with GTK itself.
