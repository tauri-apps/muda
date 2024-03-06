---
"muda": "minor"
---

Refactored the errors when parsing accelerator from string:

- Added `AcceleratorParseError` error enum.
- Removed `Error::UnrecognizedAcceleratorCode` enum variant
- Removed `Error::EmptyAcceleratorToken` enum variant
- Removed `Error::UnexpectedAcceleratorFormat` enum variant
- Changed `Error::AcceleratorParseError` inner value from `String` to the newly added `AcceleratorParseError` enum.
