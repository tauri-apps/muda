// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    accelerator::{Accelerator, Code, MenuAccelerator, Modifiers, CMD_OR_CTRL},
    sealed::IsMenuItemBase,
    AboutMetadata, IsMenuItem, MenuId, MenuItemKind,
};

/// A predefined (native) menu item which has a predefined behavior by the OS or by this crate.
#[derive(Clone)]
pub struct PredefinedMenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) inner: Rc<RefCell<crate::platform_impl::MenuChild>>,
}

impl IsMenuItemBase for PredefinedMenuItem {}
impl IsMenuItem for PredefinedMenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Predefined(self.clone())
    }

    fn id(&self) -> &MenuId {
        self.id()
    }

    fn into_id(self) -> MenuId {
        self.into_id()
    }
}

impl PredefinedMenuItem {
    /// Separator menu item
    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredefinedMenuItemType::Separator, None)
    }

    /// Copy menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Requires the `libxdo` feature.
    /// - **GTK 4:** Unsupported.
    pub fn copy(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Copy, text)
    }

    /// Cut menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Requires the `libxdo` feature.
    /// - **GTK 4:** Unsupported.
    pub fn cut(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Cut, text)
    }

    /// Paste menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Requires the `libxdo` feature.
    /// - **GTK 4:** Unsupported.
    pub fn paste(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Paste, text)
    }

    /// Paste and Match Style menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn paste_and_match_style(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::PasteAndMatchStyle, text)
    }

    /// Delete menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn delete(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Delete, text)
    }

    /// SelectAll menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Requires the `libxdo` feature.
    /// - **GTK 4:** Unsupported.
    pub fn select_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::SelectAll, text)
    }

    /// Undo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3 / GTK 4:** Unsupported.
    pub fn undo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Undo, text)
    }
    /// Redo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3 / GTK 4:** Unsupported.
    pub fn redo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Redo, text)
    }

    /// Minimize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Unsupported.
    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Minimize, text)
    }

    /// Maximize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Unsupported.
    pub fn maximize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Maximize, text)
    }

    /// Zoom window menu item
    ///
    /// This is an alias for [`Self::maximize`]. On macOS, the native maximize
    /// window action is conventionally named "Zoom".
    pub fn zoom(text: Option<&str>) -> PredefinedMenuItem {
        Self::maximize(text)
    }

    /// Actual Size menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn actual_size(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::ActualSize, text)
    }

    /// Zoom In menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn zoom_in(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::ZoomIn, text)
    }

    /// Zoom Out menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn zoom_out(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::ZoomOut, text)
    }

    /// Fullscreen menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3:** Unsupported.
    pub fn fullscreen(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Fullscreen, text)
    }

    /// Hide window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Unsupported.
    pub fn hide(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Hide, text)
    }

    /// Hide other windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn hide_others(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::HideOthers, text)
    }

    /// Show all app windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn show_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::ShowAll, text)
    }

    /// Close window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Unsupported.
    pub fn close_window(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::CloseWindow, text)
    }

    /// Quit app menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **GTK 3:** Unsupported.
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
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn services(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::Services, text)
    }

    /// 'Bring all to front' menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn bring_all_to_front(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::BringAllToFront, text)
    }

    /// Start Speaking menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn start_speaking(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::StartSpeaking, text)
    }

    /// Stop Speaking menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn stop_speaking(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::StopSpeaking, text)
    }

    /// Start Dictation menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn start_dictation(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::StartDictation, text)
    }

    /// Emoji & Symbols menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / GTK 3 / GTK 4:** Unsupported.
    pub fn emoji_and_symbols(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemType::EmojiAndSymbols, text)
    }

    fn new<S: AsRef<str>>(item: PredefinedMenuItemType, text: Option<S>) -> Self {
        let item = crate::platform_impl::MenuChild::new_predefined(
            item,
            text.map(|t| t.as_ref().to_string()),
        );
        Self {
            id: Rc::new(item.id().clone()),
            inner: Rc::new(RefCell::new(item)),
        }
    }

    /// Returns a unique identifier associated with this predefined menu item.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Get the text for this predefined menu item.
    pub fn text(&self) -> String {
        self.inner.borrow().text()
    }

    /// Set the text for this predefined menu item.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.inner.borrow_mut().set_text(text.as_ref())
    }

    /// Convert this menu item into its menu ID.
    pub fn into_id(mut self) -> MenuId {
        // Note: `Rc::into_inner` is available from Rust 1.70
        if let Some(id) = Rc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
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
            version: Some("Version: 1.inner".into()),
            ..Default::default()
        }
        .full_version(),
        Some("Version: 1.inner".into())
    );

    assert_eq!(
        AboutMetadata {
            version: Some("Version: 1.inner".into()),
            short_version: Some("Universal".into()),
            ..Default::default()
        }
        .full_version(),
        Some("Version: 1.inner (Universal)".into())
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
    PasteAndMatchStyle,
    Delete,
    SelectAll,
    Undo,
    Redo,
    Minimize,
    Maximize,
    ActualSize,
    ZoomIn,
    ZoomOut,
    Fullscreen,
    Hide,
    HideOthers,
    ShowAll,
    CloseWindow,
    Quit,
    About(Option<AboutMetadata>),
    Services,
    BringAllToFront,
    StartSpeaking,
    StopSpeaking,
    StartDictation,
    EmojiAndSymbols,
}

impl PredefinedMenuItemType {
    pub(crate) fn text(&self) -> &str {
        match self {
            PredefinedMenuItemType::Separator => "",
            PredefinedMenuItemType::Copy => "&Copy",
            PredefinedMenuItemType::Cut => "Cu&t",
            PredefinedMenuItemType::Paste => "&Paste",
            PredefinedMenuItemType::PasteAndMatchStyle => "Paste and Match Style",
            PredefinedMenuItemType::Delete => "&Delete",
            PredefinedMenuItemType::SelectAll => "Select &All",
            PredefinedMenuItemType::Undo => "Undo",
            PredefinedMenuItemType::Redo => "Redo",
            PredefinedMenuItemType::Minimize => "&Minimize",
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Maximize => "Zoom",
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemType::Maximize => "Ma&ximize",
            PredefinedMenuItemType::ActualSize => "Actual Size",
            PredefinedMenuItemType::ZoomIn => "Zoom In",
            PredefinedMenuItemType::ZoomOut => "Zoom Out",
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
            PredefinedMenuItemType::BringAllToFront => "Bring All to Front",
            PredefinedMenuItemType::StartSpeaking => "Start Speaking",
            PredefinedMenuItemType::StopSpeaking => "Stop Speaking",
            PredefinedMenuItemType::StartDictation => "Start Dictation…",
            PredefinedMenuItemType::EmojiAndSymbols => "Emoji & Symbols",
        }
    }

    pub(crate) fn accelerator(&self) -> Option<MenuAccelerator> {
        match self {
            PredefinedMenuItemType::Copy => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyC)),
            PredefinedMenuItemType::Cut => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyX)),
            PredefinedMenuItemType::Paste => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyV)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::PasteAndMatchStyle => Some(physical_accelerator(
                CMD_OR_CTRL | Modifiers::ALT | Modifiers::SHIFT,
                Code::KeyV,
            )),
            PredefinedMenuItemType::Undo => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyZ)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Redo => Some(physical_accelerator(
                CMD_OR_CTRL | Modifiers::SHIFT,
                Code::KeyZ,
            )),
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemType::Redo => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyY)),
            PredefinedMenuItemType::SelectAll => {
                Some(physical_accelerator(CMD_OR_CTRL, Code::KeyA))
            }
            PredefinedMenuItemType::Minimize => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyM)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::ActualSize => {
                Some(physical_accelerator(CMD_OR_CTRL, Code::Digit0))
            }
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::ZoomIn => Some(physical_accelerator(
                CMD_OR_CTRL | Modifiers::SHIFT,
                Code::Equal,
            )),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::ZoomOut => Some(physical_accelerator(CMD_OR_CTRL, Code::Minus)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Fullscreen => Some(physical_accelerator(
                Modifiers::META | Modifiers::CONTROL,
                Code::KeyF,
            )),
            PredefinedMenuItemType::Hide => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyH)),
            PredefinedMenuItemType::HideOthers => Some(physical_accelerator(
                CMD_OR_CTRL | Modifiers::ALT,
                Code::KeyH,
            )),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::CloseWindow => {
                Some(physical_accelerator(CMD_OR_CTRL, Code::KeyW))
            }
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemType::CloseWindow => {
                Some(physical_accelerator(Modifiers::ALT, Code::F4))
            }
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::Quit => Some(physical_accelerator(CMD_OR_CTRL, Code::KeyQ)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemType::EmojiAndSymbols => Some(physical_accelerator(
                Modifiers::META | Modifiers::CONTROL,
                Code::Space,
            )),
            _ => None,
        }
    }
}

fn physical_accelerator(modifiers: Modifiers, key: Code) -> MenuAccelerator {
    MenuAccelerator::Physical(Accelerator::new(modifiers, key))
}
