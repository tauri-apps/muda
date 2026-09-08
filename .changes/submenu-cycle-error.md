---
"muda": minor
---

Adding a submenu to itself, or to one of its own descendants, now fails with the new `Error::WouldCreateCycle` instead of creating a cycle in the menu tree.
