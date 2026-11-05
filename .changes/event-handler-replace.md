---
"muda": patch
---

Fix `MenuEvent::set_event_handler` having no effect when called again, or after a menu event was already emitted. It now replaces the current handler, and passing `None` removes it, so events are sent to `MenuEvent::receiver` again.
