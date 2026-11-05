---
"muda": minor
---

**Breaking change** `MenuEvent::set_event_handler` now takes an `Option<MenuEventHandler>`, which is an `Arc<dyn Fn(MenuEvent) + Send + Sync>`, so wrap your closure in `Arc::new` and pass a plain `None` instead of `None::<fn(MenuEvent)>`. It also returns the handler it replaced, and passing that handler back restores it.
