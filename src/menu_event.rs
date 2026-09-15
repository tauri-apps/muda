// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::{Lazy, OnceCell};

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
type MenuEventHandler = Box<dyn Fn(MenuEvent) + Send + Sync + 'static>;

static MENU_CHANNEL: Lazy<(Sender<MenuEvent>, MenuEventReceiver)> = Lazy::new(unbounded);
static MENU_EVENT_HANDLER: OnceCell<Option<MenuEventHandler>> = OnceCell::new();

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
    /// This will not receive any events if [`MenuEvent::set_event_handler`] has been called with a `Some` value.
    pub fn receiver<'a>() -> &'a MenuEventReceiver {
        &MENU_CHANNEL.1
    }

    /// Set a handler to be called for new events. Useful for implementing custom event sender.
    ///
    /// ## Note
    ///
    /// Calling this function with a `Some` value,
    /// will not send new events to the channel associated with [`MenuEvent::receiver`]
    pub fn set_event_handler<F: Fn(MenuEvent) + Send + Sync + 'static>(f: Option<F>) {
        if let Some(f) = f {
            let _ = MENU_EVENT_HANDLER.set(Some(Box::new(f)));
        } else {
            let _ = MENU_EVENT_HANDLER.set(None);
        }
    }

    pub(crate) fn send(event: MenuEvent) {
        if let Some(handler) = MENU_EVENT_HANDLER.get_or_init(|| None) {
            handler(event);
        } else {
            let _ = MENU_CHANNEL.0.send(event);
        }
    }
}
