---
"muda": "patch"
---

Wrapped the `id` field of the `Menu` struct in an `Rc` to be consistent with other menu structs and make it cheaper to clone.
