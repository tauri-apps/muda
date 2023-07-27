// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use crate::{
    accelerator::{Accelerator, CMD_OR_CTRL},
    AboutMetadata, IsMenuItem, MenuItemKind,
};
use keyboard_types::{Code, Modifiers};

/// A predefined (native) menu item which has a predfined behavior by the OS or by this crate.
#[derive(Clone)]
pub struct PredefinedMenuItem(pub(crate) Rc<RefCell<crate::platform_impl::MenuChild>>);

unsafe impl IsMenuItem for PredefinedMenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Predefined(self.clone())
    }
}

impl PredefinedMenuItem {
    /// Separator menu item
    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredefinedMenuItemType::Separator, None)
    }

    /// Copy menu item
    pub fn copy(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Copy, text)
    }

    /// Cut menu item
    pub fn cut(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Cut, text)
    }

    /// Paste menu item
    pub fn paste(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Paste, text)
    }

    /// SelectAll menu item
    pub fn select_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::SelectAll, text)
    }

    /// Undo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn undo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Undo, text)
    }
    /// Redo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn redo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Redo, text)
    }

    /// Minimize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Minimize, text)
    }

    /// Maximize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn maximize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Maximize, text)
    }

    /// Fullscreen menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn fullscreen(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Fullscreen, text)
    }

    /// Hide window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn hide(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Hide, text)
    }

    /// Hide other windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn hide_others(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::HideOthers, text)
    }

    /// Show all app windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn show_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::ShowAll, text)
    }

    /// Close window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn close_window(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::CloseWindow, text)
    }

    /// Quit app menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn quit(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Quit, text)
    }

    /// About app menu item
    pub fn about(text: Option<&str>, metadata: Option<AboutMetadata>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::About(metadata), text)
    }

    /// Services menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn services(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Services, text)
    }

    fn new<S: AsRef<str>>(item: PredefinedMenuItemType, text: Option<S>) -> Self {
        Self(Rc::new(RefCell::new(
            crate::platform_impl::MenuChild::new_predefined(
                item,
                text.map(|t| t.as_ref().to_string()),
            ),
        )))
    }

    pub(crate) fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    /// Get the text for this predefined menu item.
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    /// Set the text for this predefined menu item.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.borrow_mut().set_text(text.as_ref())
    }
}

#[test]
fn test_about_metadata() {
    assert_eq!(
        AboutMetadata {
            ..Default::default()
        }
        .full_version(),
        None
    );

    assert_eq!(
        AboutMetadata {
            version: Some("Version: 1.0".into()),
            ..Default::default()
        }
        .full_version(),
        Some("Version: 1.0".into())
    );

    assert_eq!(
        AboutMetadata {
            version: Some("Version: 1.0".into()),
            short_version: Some("Universal".into()),
            ..Default::default()
        }
        .full_version(),
        Some("Version: 1.0 (Universal)".into())
    );
}

#[derive(Debug, Clone)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub(crate) enum PredefinedMenuItemType {
    Separator,
    Copy,
    Cut,
    Paste,
    SelectAll,
    Undo,
    Redo,
    Minimize,
    Maximize,
    Fullscreen,
    Hide,
    HideOthers,
    ShowAll,
    CloseWindow,
    Quit,
    About(Option<AboutMetadata>),
    Services,
    None,
}

impl Default for PredefinedMenuItemType {
    fn default() -> Self {
        Self::None
    }
}

impl PredefinedMenuItemType {
    pub(crate) fn text(&self) -> &str {
        match self {
            PredefinedMenuItemType::Separator => "",
            PredefinedMenuItemType::Copy => "&Copy",
            PredefinedMenuItemType::Cut => "Cu&t",
            PredefinedMenuItemType::Paste => "&Paste",
            PredefinedMenuItemType::SelectAll => "Select &All",
            PredefinedMenuItemType::Undo => "Undo",
            PredefinedMenuItemType::Redo => "Redo",
            PredefinedMenuItemType::Minimize => "&Minimize",
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Maximize => "Zoom",
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemType::Maximize => "Ma&ximize",
            PredefinedMenuItemType::Fullscreen => "Toggle Full Screen",
            PredefinedMenuItemType::Hide => "&Hide",
            PredefinedMenuItemType::HideOthers => "Hide Others",
            PredefinedMenuItemType::ShowAll => "Show All",
            #[cfg(windows)]
            PredefinedMenuItemType::CloseWindow => "Close",
            #[cfg(not(windows))]
            PredefinedMenuItemType::CloseWindow => "C&lose Window",
            #[cfg(windows)]
            PredefinedMenuItemType::Quit => "&Exit",
            #[cfg(not(windows))]
            PredefinedMenuItemType::Quit => "&Quit",
            PredefinedMenuItemType::About(_) => "&About",
            PredefinedMenuItemType::Services => "Services",
            PredefinedMenuItemType::None => "",
        }
    }

    pub(crate) fn accelerator(&self) -> Option<Accelerator> {
        match self {
            PredefinedMenuItemType::Copy => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyC)),
            PredefinedMenuItemType::Cut => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyX)),
            PredefinedMenuItemType::Paste => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyV)),
            PredefinedMenuItemType::Undo => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyZ)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Redo => Some(Accelerator::new(
                Some(CMD_OR_CTRL | Modifiers::SHIFT),
                Code::KeyZ,
            )),
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemType::Redo => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyY)),
            PredefinedMenuItemType::SelectAll => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyA))
            }
            PredefinedMenuItemType::Minimize => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyM))
            }
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Fullscreen => Some(Accelerator::new(
                Some(Modifiers::META | Modifiers::CONTROL),
                Code::KeyF,
            )),
            PredefinedMenuItemType::Hide => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyH)),
            PredefinedMenuItemType::HideOthers => Some(Accelerator::new(
                Some(CMD_OR_CTRL | Modifiers::ALT),
                Code::KeyH,
            )),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::CloseWindow => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyW))
            }
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemType::CloseWindow => {
                Some(Accelerator::new(Some(Modifiers::ALT), Code::F4))
            }
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Quit => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyQ)),
            _ => None,
        }
    }
}
