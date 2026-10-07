---
"muda": patch
---

On macOS, let `objc2` auto-generate the Objective-C runtime names for muda's internal classes (previously hardcoded as `MudaMenuItem` and `MudaMenuDelegate`). The generated names include the crate version, so two SemVer-incompatible versions of muda in the same binary no longer panic with `could not create new class MudaMenuItem. Perhaps a class with that name already exists?`.
