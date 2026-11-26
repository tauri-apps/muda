// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Remembers where the last touch landed, so a context menu shown without a position opens
//! where the user tapped, as it opens at the cursor on desktop. UIKit has no API to read the
//! location of the current touch or pointer.

use std::{cell::RefCell, ptr::NonNull};

use block2::RcBlock;
use objc2::{
    define_class, msg_send,
    rc::{Retained, Weak},
    runtime::NSObject,
    DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_core_foundation::CGPoint;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSProcessInfo, NSSet};
use objc2_ui_kit::{
    UIApplication, UIEvent, UIGestureRecognizer, UIGestureRecognizerState, UITouch, UITouchPhase,
    UIView, UIWindow, UIWindowDidBecomeVisibleNotification, UIWindowScene,
};

/// The last touch that began in a window.
struct LastTouch {
    /// The touch itself, while UIKit keeps it alive, to follow it as it moves and ends.
    ///
    /// Only weak: a touch references its window, which owns the recorder holding this.
    touch: Weak<UITouch>,
    /// Where the touch began, in window coordinates.
    location: CGPoint,
    /// When the touch began, in seconds since system startup.
    timestamp: f64,
}

define_class!(
    /// Records the touches that begin in its window, then fails so it never gets in the way of
    /// other gestures.
    #[unsafe(super(UIGestureRecognizer, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RefCell<Option<LastTouch>>]
    struct TouchLocationRecorder;

    impl TouchLocationRecorder {
        #[unsafe(method(touchesBegan:withEvent:))]
        fn touches_began(&self, touches: &NSSet<UITouch>, event: &UIEvent) {
            let _: () = unsafe { msg_send![super(self), touchesBegan: touches, withEvent: event] };

            if let Some(touch) = touches.anyObject() {
                *self.ivars().borrow_mut() = Some(LastTouch {
                    touch: Weak::from_retained(&touch),
                    location: touch.locationInView(None),
                    timestamp: touch.timestamp(),
                });
            }
            self.setState(UIGestureRecognizerState::Failed);
        }
    }
);

impl TouchLocationRecorder {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RefCell::new(None));
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        this.setCancelsTouchesInView(false);
        this.setDelaysTouchesEnded(false);
        this
    }
}

/// Starts recording touches in every window of `application`, the current ones and the ones
/// shown later. Must only be called once per application.
pub(super) fn track_touches(application: &UIApplication, mtm: MainThreadMarker) {
    let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
        // SAFETY: The notification center hands a valid notification to the block.
        let notification = unsafe { notification.as_ref() };
        if let Some(window) = notification
            .object()
            .and_then(|object| object.downcast::<UIWindow>().ok())
        {
            add_recorder(&window, mtm);
        }
    });
    // The notification center keeps the observer alive until it is removed, which it never is.
    let _observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(UIWindowDidBecomeVisibleNotification),
            None,
            None,
            &block,
        )
    };

    for scene in application.connectedScenes() {
        if let Some(scene) = scene.downcast_ref::<UIWindowScene>() {
            for window in scene.windows() {
                add_recorder(&window, mtm);
            }
        }
    }
}

fn recorder(window: &UIWindow) -> Option<Retained<TouchLocationRecorder>> {
    window
        .gestureRecognizers()?
        .iter()
        .find_map(|recognizer| recognizer.downcast::<TouchLocationRecorder>().ok())
}

fn add_recorder(window: &UIWindow, mtm: MainThreadMarker) {
    if recorder(window).is_none() {
        window.addGestureRecognizer(&TouchLocationRecorder::new(mtm));
    }
}

/// How long after a touch lifts it can still place a context menu, long enough for the menu
/// request to go through a webview or IPC.
const MAX_TOUCH_AGE: f64 = 2.0;

/// The location in `view` of the last touch in the view's window, if the finger is still down
/// or lifted recently, and it is inside the view.
pub(super) fn last_touch_location(view: &UIView) -> Option<CGPoint> {
    let window = view.window()?;
    let recorder = recorder(&window)?;
    let (location, timestamp) = {
        let last = recorder.ivars().borrow();
        let last = last.as_ref()?;
        match last.touch.load() {
            Some(touch) => {
                let down = matches!(
                    touch.phase(),
                    UITouchPhase::Began | UITouchPhase::Moved | UITouchPhase::Stationary
                );
                // A finger still down is current however long it has been held, and the touch
                // timestamp is when it last moved or lifted.
                let timestamp = if down { None } else { Some(touch.timestamp()) };
                (touch.locationInView(None), timestamp)
            }
            None => (last.location, Some(last.timestamp)),
        }
    };
    if timestamp.is_some_and(|timestamp| {
        NSProcessInfo::processInfo().systemUptime() - timestamp > MAX_TOUCH_AGE
    }) {
        return None;
    }
    let location = view.convertPoint_fromView(location, None);
    let bounds = view.bounds();
    let inside = location.x >= bounds.origin.x
        && location.x <= bounds.origin.x + bounds.size.width
        && location.y >= bounds.origin.y
        && location.y <= bounds.origin.y + bounds.size.height;
    inside.then_some(location)
}
