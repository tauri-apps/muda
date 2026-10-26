---
"muda": minor
---

**Breaking change** `MenuEvent::set_event_handler` now takes and returns an `Option<MenuEventHandler>`, which is an `Arc<dyn Fn(MenuEvent) + Send + Sync>`, so wrap your closure in `Arc::new` and pass a plain `None` instead of `None::<fn(MenuEvent)>`. It replaces the current handler and returns it, instead of silently ignoring the new handler. Passing the returned handler back restores it, and passing `None` now removes the current handler, so events are sent to `MenuEvent::receiver` again.
