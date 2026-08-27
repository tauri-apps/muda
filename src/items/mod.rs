// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod check;
mod icon;
mod normal;
mod predefined;
mod styled_text;
mod submenu;

pub use check::*;
pub use icon::*;
pub use normal::*;
pub use predefined::*;
pub use styled_text::*;
pub use submenu::*;

#[cfg(test)]
mod test {
    use crate::{
        CheckMenuItem, CheckMenuItemBuilder, IconMenuItem, IconMenuItemBuilder, MenuId, MenuItem,
        MenuItemBuilder, PredefinedMenuItem, Submenu, SubmenuBuilder, TextStyle,
    };

    #[test]
    #[cfg_attr(
        all(
            miri,
            not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))
        ),
        ignore
    )]
    fn it_returns_same_id() {
        let id = MenuId::new("1");
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

    #[test]
    #[cfg_attr(
        all(
            miri,
            not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))
        ),
        ignore
    )]
    fn test_convert_from_id_and_into_id() {
        let id = "TEST ID";
        let expected = MenuId(id.to_string());

        let item = CheckMenuItem::with_id(id, "test", true, true, None);
        assert_eq!(item.id(), &expected);
        assert_eq!(item.into_id(), expected);

        let item = IconMenuItem::with_id(id, "test", true, None, None);
        assert_eq!(item.id(), &expected);
        assert_eq!(item.into_id(), expected);

        let item = MenuItem::with_id(id, "test", true, None);
        assert_eq!(item.id(), &expected);
        assert_eq!(item.into_id(), expected);

        let item = Submenu::with_id(id, "test", true);
        assert_eq!(item.id(), &expected);
        assert_eq!(item.into_id(), expected);

        let item = PredefinedMenuItem::separator();
        assert_eq!(item.id().clone(), item.into_id());
    }

    #[test]
    #[cfg_attr(
        all(
            miri,
            not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))
        ),
        ignore
    )]
    fn set_styled_text_concatenates_label() {
        let item = MenuItem::new("Preview", true, None);
        item.set_styled_text([
            ("Preview", TextStyle::Normal),
            (" (default)", TextStyle::Secondary),
        ]);
        assert_eq!(item.text(), "Preview (default)");

        item.set_styled_text([("Preview", TextStyle::Normal)]);
        assert_eq!(item.text(), "Preview");

        // `set_text` reverts to a plain label.
        item.set_styled_text([
            ("Preview", TextStyle::Normal),
            (" (default)", TextStyle::Secondary),
        ]);
        item.set_text("Plain again");
        assert_eq!(item.text(), "Plain again");
    }

    #[test]
    #[cfg_attr(
        all(
            miri,
            not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))
        ),
        ignore
    )]
    fn every_item_type_takes_styled_text() {
        let parts = [
            ("Speakers", TextStyle::Normal),
            (" (current)", TextStyle::Secondary),
        ];

        let item = MenuItem::new("placeholder", true, None);
        item.set_styled_text(parts);
        assert_eq!(item.text(), "Speakers (current)");

        let icon_item = IconMenuItem::new("placeholder", true, None, None);
        icon_item.set_styled_text(parts);
        assert_eq!(icon_item.text(), "Speakers (current)");

        let check_item = CheckMenuItem::new("placeholder", true, true, None);
        check_item.set_styled_text(parts);
        assert_eq!(check_item.text(), "Speakers (current)");

        let submenu = Submenu::new("placeholder", true);
        submenu.set_styled_text(parts);
        assert_eq!(submenu.text(), "Speakers (current)");
    }

    #[test]
    #[cfg_attr(
        all(
            miri,
            not(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))
        ),
        ignore
    )]
    fn every_builder_takes_styled_text() {
        let parts = [
            ("Preview", TextStyle::Normal),
            (" (default)", TextStyle::Secondary),
        ];

        assert_eq!(
            MenuItemBuilder::new().styled_text(parts).build().text(),
            "Preview (default)"
        );
        assert_eq!(
            IconMenuItemBuilder::new().styled_text(parts).build().text(),
            "Preview (default)"
        );
        assert_eq!(
            CheckMenuItemBuilder::new()
                .styled_text(parts)
                .build()
                .text(),
            "Preview (default)"
        );
        assert_eq!(
            SubmenuBuilder::new()
                .styled_text(parts)
                .build()
                .unwrap()
                .text(),
            "Preview (default)"
        );

        // `styled_text` wins over a plain `text` set on the same builder.
        assert_eq!(
            MenuItemBuilder::new()
                .text("Ignored")
                .styled_text(parts)
                .build()
                .text(),
            "Preview (default)"
        );
    }
}
