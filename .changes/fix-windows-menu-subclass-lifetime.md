---
"muda": patch
---

On Windows, detach menu and submenu window subclass handlers when their owners are dropped to prevent dangling callbacks.
