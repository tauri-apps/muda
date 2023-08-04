// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod check;
mod icon;
mod normal;
mod predefined;
mod submenu;

pub use check::*;
pub use icon::*;
pub use normal::*;
pub use predefined::*;
pub use submenu::*;

#[cfg(test)]
mod test {
    use crate::{CheckMenuItem, IconMenuItem, MenuId, MenuItem, Submenu};

    #[test]
    fn it_returns_same_id() {
        let id = MenuId("1".into());
        assert_eq!(id, MenuItem::with_id(id.clone(), "", true, None).id());
        assert_eq!(id, Submenu::with_id(id.clone(), "", true).id());
        assert_eq!(
            id,
            CheckMenuItem::with_id(id.clone(), "", true, true, None).id()
        );
        assert_eq!(
            id,
            IconMenuItem::with_id(id.clone(), "", true, None, None).id()
        );
    }
}
