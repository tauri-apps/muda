// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Remembers where the last touch landed, so a context menu shown without a position opens
//! where the user tapped, as it opens at the cursor on desktop. UIKit has no API to read the
//! location of the current touch or pointer.

use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
};

use block2::RcBlock;
use objc2::{
    define_class, msg_send,
    rc::{Retained, Weak},
    runtime::NSObject,
    ClassType, MainThreadMarker, MainThreadOnly,
};
use objc2_core_foundation::CGPoint;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSSet};
use objc2_ui_kit::{
    UIApplication, UIEvent, UIGestureRecognizer, UIGestureRecognizerState, UITouch, UIView,
    UIWindow, UIWindowDidBecomeVisibleNotification, UIWindowScene,
};

thread_local! {
    /// The window and the location in it of the last touch.
    static LAST_TOUCH: RefCell<Option<(Weak<UIWindow>, CGPoint)>> = const { RefCell::new(None) };
    /// Whether windows are being given a [`TouchLocationRecorder`].
    static TRACKING: Cell<bool> = const { Cell::new(false) };
}

define_class!(
    /// Records where touches begin in its window, then fails so it never gets in the way of
    /// other gestures.
    #[unsafe(super(UIGestureRecognizer, NSObject))]
    #[thread_kind = MainThreadOnly]
    struct TouchLocationRecorder;

    impl TouchLocationRecorder {
        #[unsafe(method(touchesBegan:withEvent:))]
        fn touches_began(&self, touches: &NSSet<UITouch>, event: &UIEvent) {
            let _: () = unsafe { msg_send![super(self), touchesBegan: touches, withEvent: event] };

            if let Some(touch) = touches.anyObject() {
                if let Some(window) = touch.window() {
                    let location = touch.locationInView(None);
                    LAST_TOUCH.set(Some((Weak::from_retained(&window), location)));
                }
            }
            self.setState(UIGestureRecognizerState::Failed);
        }
    }
);

impl TouchLocationRecorder {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        this.setCancelsTouchesInView(false);
        this.setDelaysTouchesEnded(false);
        this
    }
}

/// Starts recording touches in every window, the current ones and the ones shown later.
pub(super) fn track_touches(mtm: MainThreadMarker) {
    if TRACKING.replace(true) {
        return;
    }

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

    // Not launched yet when this is nil, and then no window is visible yet either.
    let application: Option<Retained<UIApplication>> =
        unsafe { msg_send![UIApplication::class(), sharedApplication] };
    for scene in application.iter().flat_map(|app| app.connectedScenes()) {
        if let Some(scene) = scene.downcast_ref::<UIWindowScene>() {
            for window in scene.windows() {
                add_recorder(&window, mtm);
            }
        }
    }
}

fn add_recorder(window: &UIWindow, mtm: MainThreadMarker) {
    let has_recorder = window.gestureRecognizers().is_some_and(|recognizers| {
        recognizers
            .iter()
            .any(|recognizer| recognizer.downcast_ref::<TouchLocationRecorder>().is_some())
    });
    if !has_recorder {
        window.addGestureRecognizer(&TouchLocationRecorder::new(mtm));
    }
}

/// The location of the last touch in `view`, if it landed in the view's window.
pub(super) fn last_touch_location(view: &UIView) -> Option<CGPoint> {
    let window = view.window()?;
    let location = LAST_TOUCH.with_borrow(|last_touch| {
        let (touch_window, location) = last_touch.as_ref()?;
        let touch_window = touch_window.load()?;
        std::ptr::eq(&*touch_window, &*window).then_some(*location)
    })?;
    Some(view.convertPoint_fromView(location, None))
}
