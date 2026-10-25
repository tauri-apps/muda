---
"muda": minor
---

**Breaking change** `MenuEvent::set_event_handler` now returns a `Result`, with the new `Error::EventHandlerAlreadySet` error when a handler is already set, instead of silently ignoring the new handler. Passing `None` now removes the current handler, so events are sent to `MenuEvent::receiver` again and a new handler can be set.
