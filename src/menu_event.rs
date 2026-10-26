// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex};

use crate::MenuId;

/// Describes a menu event emitted when a menu item is activated
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MenuEvent {
    /// Id of the menu item which triggered this event
    pub id: MenuId,
}

/// A receiver that could be used to listen to menu events.
pub type MenuEventReceiver = Receiver<MenuEvent>;

/// A handler for menu events, see [`MenuEvent::set_event_handler`].
pub type MenuEventHandler = Arc<dyn Fn(MenuEvent) + Send + Sync + 'static>;

static MENU_CHANNEL: LazyLock<(Sender<MenuEvent>, MenuEventReceiver)> = LazyLock::new(unbounded);
static MENU_EVENT_HANDLER: Mutex<Option<MenuEventHandler>> = Mutex::new(None);

impl MenuEvent {
    /// Returns the id of the menu item which triggered this event
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Gets a reference to the event channel's [`MenuEventReceiver`]
    /// which can be used to listen for menu events.
    ///
    /// ## Note
    ///
    /// This will not receive any events while a handler is set with [`MenuEvent::set_event_handler`].
    pub fn receiver<'a>() -> &'a MenuEventReceiver {
        &MENU_CHANNEL.1
    }

    /// Set a handler to be called for new events.
    ///
    /// This is intended for routing menu events through an event loop, using something like
    /// winit's [`EventLoopProxy`], so that the event loop wakes up on each menu event.
    ///
    /// Only one handler can be set at a time, so this replaces the current handler and returns it.
    /// Passing the returned handler back to this function restores it. Calling this function with
    /// `None` removes the current handler.
    ///
    /// ## Examples
    ///
    /// Route menu events through a winit event loop:
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use muda::MenuEvent;
    /// # use winit::event_loop::EventLoop;
    /// enum UserEvent {
    ///     MenuEvent(MenuEvent),
    /// }
    ///
    /// let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    ///
    /// let proxy = event_loop.create_proxy();
    /// MenuEvent::set_event_handler(Some(Arc::new(move |event| {
    ///     proxy.send_event(UserEvent::MenuEvent(event));
    /// })));
    /// ```
    ///
    /// When this function is called more than once, only the handler from the last call receives
    /// events. To have the replaced handler still receive the events that the new one doesn't
    /// handle, pass them to it:
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use muda::{MenuEvent, MenuItem};
    /// let item = MenuItem::new("Item", true, None);
    /// let item_id = item.id().clone();
    ///
    /// // Take the current handler first, so that the new one can call it.
    /// let previous = MenuEvent::set_event_handler(None);
    /// MenuEvent::set_event_handler(Some(Arc::new(move |event| {
    ///     if event.id == item_id {
    ///         println!("Item activated");
    ///     } else if let Some(previous) = &previous {
    ///         previous(event);
    ///     }
    /// })));
    /// ```
    ///
    /// ## Note
    ///
    /// - While a handler is set, new events are not sent to the channel associated with [`MenuEvent::receiver`].
    /// - The handler is only called for menus created by this version of muda, see
    ///   [multiple versions of muda](crate#note-for-multiple-versions-of-muda).
    ///
    /// [`EventLoopProxy`]: https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html
    pub fn set_event_handler(handler: Option<MenuEventHandler>) -> Option<MenuEventHandler> {
        std::mem::replace(&mut *MENU_EVENT_HANDLER.lock().unwrap(), handler)
    }

    pub(crate) fn send(event: MenuEvent) {
        // Don't hold the lock while running the handler,
        // as it may set a new handler or send another event.
        let handler = MENU_EVENT_HANDLER.lock().unwrap().clone();
        if let Some(handler) = handler {
            handler(event);
        } else {
            let _ = MENU_CHANNEL.0.send(event);
        }
    }
}
