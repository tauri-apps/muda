---
"muda": minor
---

**Breaking change** `Accelerator::new` and `KeyAccelerator::new` now take `Modifiers` directly instead of `Option<Modifiers>`, use `Modifiers::empty()` instead.
