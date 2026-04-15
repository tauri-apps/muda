---
"muda": minor
---

Add a new `KeyAccelerator` struct based on `keyboard_types::Key` alongside the existing `Accelerator` (based on `Code`).
This enables expressing logical key shortcuts like "Ctrl++", "Ctrl+€" that physical key codes cannot represent (see https://github.com/tauri-apps/muda/issues/333).
