// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Remembers where the last touch landed, so a context menu shown without a position opens
//! where the user tapped, as it opens at the cursor on desktop. UIKit has no API to read the
//! location of the current touch or pointer.

use std::{cell::Cell, ptr::NonNull};

use block2::RcBlock;
use objc2::{
    define_class, msg_send, rc::Retained, runtime::NSObject, DefinedClass, MainThreadMarker,
    MainThreadOnly,
};
use objc2_core_foundation::CGPoint;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSSet};
use objc2_ui_kit::{
    UIApplication, UIEvent, UIGestureRecognizer, UIGestureRecognizerState, UITouch, UIView,
    UIWindow, UIWindowDidBecomeVisibleNotification, UIWindowScene,
};

define_class!(
    /// Records where touches begin in its window, then fails so it never gets in the way of
    /// other gestures.
    ///
    /// Holds the location of the last touch in its window, in window coordinates.
    #[unsafe(super(UIGestureRecognizer, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Cell<Option<CGPoint>>]
    struct TouchLocationRecorder;

    impl TouchLocationRecorder {
        #[unsafe(method(touchesBegan:withEvent:))]
        fn touches_began(&self, touches: &NSSet<UITouch>, event: &UIEvent) {
            let _: () = unsafe { msg_send![super(self), touchesBegan: touches, withEvent: event] };

            if let Some(touch) = touches.anyObject() {
                self.ivars().set(Some(touch.locationInView(None)));
            }
            self.setState(UIGestureRecognizerState::Failed);
        }
    }
);

impl TouchLocationRecorder {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Cell::new(None));
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

/// The location in `view` of the last touch in the view's window.
pub(super) fn last_touch_location(view: &UIView) -> Option<CGPoint> {
    let window = view.window()?;
    let location = recorder(&window)?.ivars().get()?;
    Some(view.convertPoint_fromView(location, None))
}
