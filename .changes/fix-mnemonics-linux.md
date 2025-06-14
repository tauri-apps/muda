---
"muda": patch
---

On Linux, fix `&&` resulting in `&&` when it should be just `&`. Also fix `_` not visible and actually adding a mnemonic. This makes the behavior on Linux match the behavior on Windows. 
