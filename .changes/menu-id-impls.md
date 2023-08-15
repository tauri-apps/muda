---
"muda": "patch"
---

Add `PartialEq<&str> for &MenuId` and `PartialEq<String> for &MenuId` implementations. Also add a blanket `From<T> for MenuId` where `T: ToString` implementation.
