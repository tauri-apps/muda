// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::ops::{Deref, DerefMut};

#[cfg(feature = "snapshot")]
use std::sync::{Arc, Mutex, MutexGuard, Weak};
#[cfg(not(feature = "snapshot"))]
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::{Rc, Weak},
};

/// Shared interior-mutable state backed by `Rc<RefCell<T>>` normally and `Arc<Mutex<T>>` when the
/// `snapshot` feature is enabled.
pub(crate) struct StateCell<T> {
    #[cfg(not(feature = "snapshot"))]
    inner: Rc<RefCell<T>>,
    #[cfg(feature = "snapshot")]
    inner: Arc<Mutex<T>>,
}

/// A non-owning handle to a [`StateCell`].
pub(crate) struct WeakStateCell<T> {
    #[cfg(not(feature = "snapshot"))]
    inner: Weak<RefCell<T>>,
    #[cfg(feature = "snapshot")]
    inner: Weak<Mutex<T>>,
}

pub(crate) struct StateRef<'a, T> {
    #[cfg(not(feature = "snapshot"))]
    inner: Ref<'a, T>,
    #[cfg(feature = "snapshot")]
    inner: MutexGuard<'a, T>,
}

pub(crate) struct StateRefMut<'a, T> {
    #[cfg(not(feature = "snapshot"))]
    inner: RefMut<'a, T>,
    #[cfg(feature = "snapshot")]
    inner: MutexGuard<'a, T>,
}

impl<T> Clone for StateCell<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Clone for WeakStateCell<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> StateCell<T> {
    pub(crate) fn new(state: T) -> Self {
        #[cfg(not(feature = "snapshot"))]
        let inner = Rc::new(RefCell::new(state));
        #[cfg(feature = "snapshot")]
        let inner = Arc::new(Mutex::new(state));

        Self { inner }
    }

    pub(crate) fn borrow(&self) -> StateRef<'_, T> {
        #[cfg(not(feature = "snapshot"))]
        let inner = self.inner.borrow();
        #[cfg(feature = "snapshot")]
        let inner = self.inner.lock().unwrap();

        StateRef { inner }
    }

    pub(crate) fn borrow_mut(&self) -> StateRefMut<'_, T> {
        #[cfg(not(feature = "snapshot"))]
        let inner = self.inner.borrow_mut();
        #[cfg(feature = "snapshot")]
        let inner = self.inner.lock().unwrap();

        StateRefMut { inner }
    }

    pub(crate) fn downgrade(&self) -> WeakStateCell<T> {
        #[cfg(not(feature = "snapshot"))]
        let inner = Rc::downgrade(&self.inner);
        #[cfg(feature = "snapshot")]
        let inner = Arc::downgrade(&self.inner);

        WeakStateCell { inner }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        #[cfg(not(feature = "snapshot"))]
        {
            Rc::ptr_eq(&self.inner, &other.inner)
        }
        #[cfg(feature = "snapshot")]
        {
            Arc::ptr_eq(&self.inner, &other.inner)
        }
    }
}

impl<T> WeakStateCell<T> {
    pub(crate) fn upgrade(&self) -> Option<StateCell<T>> {
        self.inner.upgrade().map(|inner| StateCell { inner })
    }
}

impl<T> Deref for StateRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Deref for StateRefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for StateRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(feature = "snapshot")]
impl<T> Drop for StateRefMut<'_, T> {
    fn drop(&mut self) {
        crate::MenuChangeEvent::send();
    }
}

#[cfg(test)]
mod tests {
    use super::StateCell;

    #[test]
    fn clones_and_weak_handles_share_state() {
        let state = StateCell::new(1);
        let weak = state.downgrade();
        let clone = weak.upgrade().unwrap();

        assert!(state.ptr_eq(&clone));
        *clone.borrow_mut() = 2;
        assert_eq!(*state.borrow(), 2);
    }

    #[cfg(feature = "snapshot")]
    #[test]
    fn snapshot_state_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<StateCell<usize>>();
    }
}
