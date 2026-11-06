---
"muda": minor
---

Add `id()` to every menu item snapshot type and `accelerator()` to the menu item, check menu item and icon menu item snapshots, returning the new `SnapshotAccelerator` (a physical `Accelerator` or a logical `KeyAccelerator`). A consumer of `MenuSnapshotHandle` can now show shortcuts and address items by their `MenuId`.
