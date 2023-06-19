// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{accelerator::Accelerator, icon::Icon, MenuItemExt, MenuItemType};
use keyboard_types::{Code, Modifiers};

#[cfg(target_os = "macos")]
pub const CMD_OR_CTRL: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
pub const CMD_OR_CTRL: Modifiers = Modifiers::CONTROL;

/// A predefined (native) menu item which has a predfined behavior by the OS or by this crate.
pub struct PredefinedMenuItem(pub(crate) crate::platform_impl::PredefinedMenuItem);

unsafe impl MenuItemExt for PredefinedMenuItem {
    fn type_(&self) -> MenuItemType {
        MenuItemType::Predefined
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn id(&self) -> u32 {
        self.id()
    }
}

impl PredefinedMenuItem {
    /// Separator menu item
    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredfinedMenuItemType::Separator, None)
    }

    /// Copy menu item
    pub fn copy(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Copy, text)
    }

    /// Cut menu item
    pub fn cut(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Cut, text)
    }

    /// Paste menu item
    pub fn paste(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Paste, text)
    }

    /// SelectAll menu item
    pub fn select_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::SelectAll, text)
    }

    /// Undo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn undo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Undo, text)
    }
    /// Redo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn redo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Redo, text)
    }

    /// Minimize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Minimize, text)
    }

    /// Maximize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn maximize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Maximize, text)
    }

    /// Fullscreen menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn fullscreen(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Fullscreen, text)
    }

    /// Hide window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn hide(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Hide, text)
    }

    /// Hide other windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn hide_others(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::HideOthers, text)
    }

    /// Show all app windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn show_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::ShowAll, text)
    }

    /// Close window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn close_window(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::CloseWindow, text)
    }

    /// Quit app menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn quit(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Quit, text)
    }

    /// About app menu item
    pub fn about(text: Option<&str>, metadata: Option<AboutMetadata>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::About(metadata), text)
    }

    /// Services menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn services(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Services, text)
    }

    fn new<S: AsRef<str>>(item: PredfinedMenuItemType, text: Option<S>) -> Self {
        Self(crate::platform_impl::PredefinedMenuItem::new(
            item,
            text.map(|t| t.as_ref().to_string()),
        ))
    }

    fn id(&self) -> u32 {
        self.0.id()
    }

    /// Get the text for this predefined menu item.
    pub fn text(&self) -> String {
        self.0.text()
    }

    /// Set the text for this predefined menu item.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }
}

/// Application metadata for the [`PredefinedMenuItem::about`].
#[derive(Debug, Clone, Default)]
pub struct AboutMetadata {
    /// The application name.
    pub name: Option<String>,
    /// The application version.
    pub version: Option<String>,
    /// The short version, e.g. "1.0"
    ///
    /// ## Platform-specific
    ///
    /// - **Windows / Linux:** This is ignored
    pub short_version: Option<String>,
    /// The authors of the application.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This is ignored
    pub authors: Option<Vec<String>>,
    /// Application comments.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This is ignored
    pub comments: Option<String>,
    /// The copyright of the application.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This is ignored
    pub copyright: Option<String>,
    /// The license of the application.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This is ignored
    pub license: Option<String>,
    /// The application website.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This is ignored
    pub website: Option<String>,
    /// The website label.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** This is ignored
    pub website_label: Option<String>,
    /// The credits.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows / Linux:** This is ignored
    pub credits: Option<String>,
    /// The application icon.
    ///
    /// ## Platform-specific
    ///
    /// - **Windows / Linux:** This is ignored
    pub icon: Option<Icon>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) enum PredfinedMenuItemType {
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

impl Default for PredfinedMenuItemType {
    fn default() -> Self {
        Self::None
    }
}

impl PredfinedMenuItemType {
    pub(crate) fn text(&self) -> &str {
        match self {
            PredfinedMenuItemType::Separator => "",
            PredfinedMenuItemType::Copy => "&Copy",
            PredfinedMenuItemType::Cut => "Cu&t",
            PredfinedMenuItemType::Paste => "&Paste",
            PredfinedMenuItemType::SelectAll => "Select &All",
            PredfinedMenuItemType::Undo => "Undo",
            PredfinedMenuItemType::Redo => "Redo",
            PredfinedMenuItemType::Minimize => "&Minimize",
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Maximize => "Zoom",
            #[cfg(not(target_os = "macos"))]
            PredfinedMenuItemType::Maximize => "Ma&ximize",
            PredfinedMenuItemType::Fullscreen => "Toggle Full Screen",
            PredfinedMenuItemType::Hide => "&Hide",
            PredfinedMenuItemType::HideOthers => "Hide Others",
            PredfinedMenuItemType::ShowAll => "Show All",
            #[cfg(windows)]
            PredfinedMenuItemType::CloseWindow => "Close",
            #[cfg(not(windows))]
            PredfinedMenuItemType::CloseWindow => "C&lose Window",
            #[cfg(windows)]
            PredfinedMenuItemType::Quit => "&Exit",
            #[cfg(not(windows))]
            PredfinedMenuItemType::Quit => "&Quit",
            PredfinedMenuItemType::About(_) => "&About",
            PredfinedMenuItemType::Services => "Services",
            PredfinedMenuItemType::None => "",
        }
    }

    pub(crate) fn accelerator(&self) -> Option<Accelerator> {
        match self {
            PredfinedMenuItemType::Copy => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyC)),
            PredfinedMenuItemType::Cut => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyX)),
            PredfinedMenuItemType::Paste => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyV)),
            PredfinedMenuItemType::Undo => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyZ)),
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Redo => Some(Accelerator::new(
                Some(CMD_OR_CTRL | Modifiers::SHIFT),
                Code::KeyZ,
            )),
            #[cfg(not(target_os = "macos"))]
            PredfinedMenuItemType::Redo => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyY)),
            PredfinedMenuItemType::SelectAll => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyA))
            }
            PredfinedMenuItemType::Minimize => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyM))
            }
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Fullscreen => Some(Accelerator::new(
                Some(Modifiers::META | Modifiers::CONTROL),
                Code::KeyF,
            )),
            PredfinedMenuItemType::Hide => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyH)),
            PredfinedMenuItemType::HideOthers => Some(Accelerator::new(
                Some(CMD_OR_CTRL | Modifiers::ALT),
                Code::KeyH,
            )),
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::CloseWindow => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyW))
            }
            #[cfg(not(target_os = "macos"))]
            PredfinedMenuItemType::CloseWindow => {
                Some(Accelerator::new(Some(Modifiers::ALT), Code::F4))
            }
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Quit => Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyQ)),
            _ => None,
        }
    }
}
