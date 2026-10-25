// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crossbeam_channel::{unbounded, Receiver, Sender};
use std::ptr;
use std::sync::{
    atomic::{AtomicPtr, Ordering},
    LazyLock,
};

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

static MENU_CHANNEL: LazyLock<(Sender<MenuEvent>, MenuEventReceiver)> = LazyLock::new(unbounded);
// A published handler is never freed, as a `send` on this or another thread may still be running it.
static MENU_EVENT_HANDLER: AtomicPtr<MenuEventHandler> = AtomicPtr::new(ptr::null_mut());

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

    /// Set a handler to be called for new events. Useful for implementing custom event sender.
    ///
    /// Only one handler can be set at a time. Calling this function with `None`
    /// (`None::<fn(MenuEvent)>`) removes the current one, so a new handler can be set.
    /// The removed handler is never dropped, so anything it captured stays alive.
    ///
    /// ## Errors
    ///
    /// Returns [`Error::EventHandlerAlreadySet`] and keeps the current handler
    /// if one is already set.
    ///
    /// ## Note
    ///
    /// - While a handler is set, new events are not sent to the channel associated with [`MenuEvent::receiver`].
    /// - The handler is only called for menus created by this version of muda, see
    ///   [multiple versions of muda](crate#note-for-multiple-versions-of-muda).
    ///
    /// [`Error::EventHandlerAlreadySet`]: crate::Error::EventHandlerAlreadySet
    pub fn set_event_handler<F: Fn(MenuEvent) + Send + Sync + 'static>(
        f: Option<F>,
    ) -> crate::Result<()> {
        let Some(f) = f else {
            MENU_EVENT_HANDLER.store(ptr::null_mut(), Ordering::Release);
            return Ok(());
        };
        let new = Box::into_raw(Box::new(Box::new(f) as MenuEventHandler));
        MENU_EVENT_HANDLER
            .compare_exchange(ptr::null_mut(), new, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| {
                // SAFETY: `new` was never published, so nothing else can be using it.
                drop(unsafe { Box::from_raw(new) });
                crate::Error::EventHandlerAlreadySet
            })
    }

    pub(crate) fn send(event: MenuEvent) {
        // SAFETY: a published handler is never freed.
        if let Some(handler) = unsafe { MENU_EVENT_HANDLER.load(Ordering::Acquire).as_ref() } {
            handler(event);
        } else {
            let _ = MENU_CHANNEL.0.send(event);
        }
    }
}
