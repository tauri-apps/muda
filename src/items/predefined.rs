// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use std::sync::Arc;
#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use arc_swap::ArcSwap;

use crate::{
    accelerator::{Accelerator, CMD_OR_CTRL},
    sealed::IsMenuItemBase,
    AboutMetadata, IsMenuItem, MenuId, MenuItemKind,
};
use keyboard_types::{Code, Modifiers};

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use super::compat::{CompatMenuItem, CompatStandardItem, strip_mnemonic};

/// A predefined (native) menu item which has a predfined behavior by the OS or by this crate.
#[derive(Clone)]
pub struct PredefinedMenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) inner: Rc<RefCell<crate::platform_impl::MenuChild>>,
    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
    pub(crate) compat: Arc<ArcSwap<CompatMenuItem>>,
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
        PredefinedMenuItem::new::<&str>(PredefinedMenuItemKind::Separator, None)
    }

    /// Copy menu item
    pub fn copy(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Copy, text)
    }

    /// Cut menu item
    pub fn cut(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Cut, text)
    }

    /// Paste menu item
    pub fn paste(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Paste, text)
    }

    /// SelectAll menu item
    pub fn select_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::SelectAll, text)
    }

    /// Undo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn undo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Undo, text)
    }
    /// Redo menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn redo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Redo, text)
    }

    /// Minimize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Minimize, text)
    }

    /// Maximize window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn maximize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Maximize, text)
    }

    /// Fullscreen menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn fullscreen(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Fullscreen, text)
    }

    /// Hide window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn hide(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Hide, text)
    }

    /// Hide other windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn hide_others(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::HideOthers, text)
    }

    /// Show all app windows menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn show_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::ShowAll, text)
    }

    /// Close window menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn close_window(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::CloseWindow, text)
    }

    /// Quit app menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Linux:** Unsupported.
    pub fn quit(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Quit, text)
    }

    /// About app menu item
    pub fn about(text: Option<&str>, metadata: Option<AboutMetadata>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::About(metadata), text)
    }

    /// Services menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn services(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::Services, text)
    }

    /// 'Bring all to front' menu item
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    pub fn bring_all_to_front(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredefinedMenuItemKind::BringAllToFront, text)
    }

    fn new<S: AsRef<str>>(item_type: PredefinedMenuItemKind, text: Option<S>) -> Self {
        #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
        let is_separator = matches!(item_type, PredefinedMenuItemKind::Separator);
        let item = crate::platform_impl::MenuChild::new_predefined(
            item_type,
            text.as_ref().map(|t| t.as_ref().to_string()),
        );
        let id = item.id().clone();
        #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
        let label = text.as_ref().map(|t| strip_mnemonic(t.as_ref())).unwrap_or_else(|| item.text());
        Self {
            id: Rc::new(id.clone()),
            inner: Rc::new(RefCell::new(item)),
            #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
            compat: Arc::new(ArcSwap::from_pointee(if is_separator {
                CompatMenuItem::Separator
            } else {
                CompatMenuItem::Standard(CompatStandardItem {
                    id: id.0.clone(),
                    label,
                    enabled: true, // Predefined items are always enabled
                    icon: None,
                    predefined_item_id: None, // TODO: populate from predefined type
                    about_metadata: None,
                })
            })),
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

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
#[allow(clippy::large_enum_variant)]
pub enum PredefinedMenuItemKind {
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
    BringAllToFront,
    #[default]
    None,
}

impl PredefinedMenuItemType {
    pub(crate) fn text(&self) -> &str {
        match self {
            PredefinedMenuItemKind::Separator => "",
            PredefinedMenuItemKind::Copy => "&Copy",
            PredefinedMenuItemKind::Cut => "Cu&t",
            PredefinedMenuItemKind::Paste => "&Paste",
            PredefinedMenuItemKind::SelectAll => "Select &All",
            PredefinedMenuItemKind::Undo => "Undo",
            PredefinedMenuItemKind::Redo => "Redo",
            PredefinedMenuItemKind::Minimize => "&Minimize",
            #[cfg(target_os = "macos")]
            PredefinedMenuItemKind::Maximize => "Zoom",
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemKind::Maximize => "Ma&ximize",
            PredefinedMenuItemKind::Fullscreen => "Toggle Full Screen",
            PredefinedMenuItemKind::Hide => "&Hide",
            PredefinedMenuItemKind::HideOthers => "Hide Others",
            PredefinedMenuItemKind::ShowAll => "Show All",
            #[cfg(windows)]
            PredefinedMenuItemKind::CloseWindow => "Close",
            #[cfg(not(windows))]
            PredefinedMenuItemKind::CloseWindow => "C&lose Window",
            #[cfg(windows)]
            PredefinedMenuItemKind::Quit => "&Exit",
            #[cfg(not(windows))]
            PredefinedMenuItemKind::Quit => "&Quit",
            PredefinedMenuItemKind::About(_) => "&About",
            PredefinedMenuItemKind::Services => "Services",
            PredefinedMenuItemKind::BringAllToFront => "Bring All to Front",
            PredefinedMenuItemKind::None => "",
        }
    }

    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub(crate) fn accelerator(&self) -> Option<Accelerator> {
        match self {
            PredefinedMenuItemKind::Copy => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyC)),
            PredefinedMenuItemKind::Cut => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyX)),
            PredefinedMenuItemKind::Paste => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyV)),
            PredefinedMenuItemKind::Undo => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyZ)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemKind::Redo => Some(Accelerator::new(
                Some(CMD_OR_CTRL | Modifiers::SHIFT),
                Code::KeyZ,
            )),
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemKind::Redo => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyY)),
            PredefinedMenuItemKind::SelectAll => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyA)),
            PredefinedMenuItemKind::Minimize => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyM)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemKind::Fullscreen => Some(Accelerator::new(
                Some(Modifiers::META | Modifiers::CONTROL),
                Code::KeyF,
            )),
            PredefinedMenuItemKind::Hide => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyH)),
            PredefinedMenuItemKind::HideOthers => {
                Some(Accelerator::new(CMD_OR_CTRL | Modifiers::ALT, Code::KeyH))
            }
            #[cfg(target_os = "macos")]
            PredefinedMenuItemKind::CloseWindow => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyW)),
            #[cfg(not(target_os = "macos"))]
            PredefinedMenuItemKind::CloseWindow => Some(Accelerator::new(Modifiers::ALT, Code::F4)),
            #[cfg(target_os = "macos")]
            PredefinedMenuItemKind::Quit => Some(Accelerator::new(CMD_OR_CTRL, Code::KeyQ)),
            _ => None,
        }
    }
}
