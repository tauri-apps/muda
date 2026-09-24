---
"muda": minor
---

Support the predefined `Undo` and `Redo` menu items on the GTK 3 and GTK 4 backends. On GTK 4 they act on the application's focused widget; on GTK 3 they act on a focused `WebKitWebView`, which is the only widget there with an undo stack.
