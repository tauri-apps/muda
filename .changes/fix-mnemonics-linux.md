---
"muda": patch
---

`&` and `_` in menu item labels are now handled on Linux the same way as
on Windows:

1. `&&` can be used for escaping and leads to a single displayed `&`.
2. `_` is displayed as is and no longer causes a mnemonic. Escaping is not
   needed.
3. The back conversion of `_` works correctly.
