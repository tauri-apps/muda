// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    accelerator::{Accelerator, Code, MenuAccelerator, Modifiers, CMD_OR_CTRL},
    platform_impl::PlatformMenuItem,
    sealed::IsMenuItemBase,
    util, AboutMetadata, ClickAction, IsMenuItem, MenuId, MenuItemKind,
};

/// A predefined (native) menu item which has a predefined behavior by the OS or by this crate.
#[derive(Clone)]
pub struct PredefinedMenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) state: Rc<RefCell<PredefinedMenuItemState>>,
    pub(crate) platform: Rc<RefCell<PlatformMenuItem>>,
}

/// Shared state of a [`PredefinedMenuItem`].
#[derive(Debug, Clone)]
pub(crate) struct PredefinedMenuItemState {
    pub text: String,
    pub predefined_item_type: PredefinedMenuItemType,
    pub enabled: bool,
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
        let id = util::next_id(None);

        let resolved_text = text
            .as_ref()
            .map(|text| text.as_ref().to_string())
            .unwrap_or_else(|| item.default_text(app_name().as_deref()));
        let enabled = item.is_supported();
        let state = Rc::new(RefCell::new(PredefinedMenuItemState {
            text: resolved_text,
            predefined_item_type: item,
            enabled,
        }));

        // A predefined item emits no event; what it does instead is decided from its kind at
        // click time, which is why the action needs a handle to state rather than the id.
        let click = ClickAction::Predefined(Rc::downgrade(&state));
        let platform = PlatformMenuItem::new(click, crate::MenuItemType::Predefined);

        Self {
            id: Rc::new(id),
            state,
            platform: Rc::new(RefCell::new(platform)),
        }
    }

    /// Returns a unique identifier associated with this predefined menu item.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Get the text for this predefined menu item.
    pub fn text(&self) -> String {
        self.platform
            .borrow()
            .text()
            .unwrap_or_else(|| self.state.borrow().text.clone())
    }

    /// Set the text for this predefined menu item.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        let accelerator = {
            let mut state = self.state.borrow_mut();
            state.text = text.as_ref().to_string();
            state.predefined_item_type.accelerator()
        };

        self.platform
            .borrow_mut()
            .set_text(text.as_ref(), accelerator.as_ref())
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

/// The running application's name, for the macOS items that splice it into their label.
///
/// The one construction-time platform call left in the crate, and the reason
/// [`PredefinedMenuItemState::new`] takes the name as an argument instead of fetching it: the
/// other three platforms' labels never mention it, so everywhere else this is a constant.
fn app_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        crate::platform_impl::app_name()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
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

    /// The label this kind carries when the caller supplies none.
    ///
    /// Mnemonic-encoded, like every other label in shared state: `&` marks the mnemonic and
    /// `&&` is a literal ampersand. Backends that have no mnemonics strip on the way out.
    ///
    /// `app_name` is consulted on macOS alone, where three of the kinds name the running
    /// application. It is passed in rather than read here because fetching it is a native
    /// call, and this table has to stay answerable without one.
    pub(crate) fn default_text(&self, app_name: Option<&str>) -> String {
        #[cfg(target_os = "macos")]
        {
            // An empty (or absent) name degrades to the bare verb, matching what
            // `format!("About {}", "").trim()` used to produce.
            let named = |verb: &str| match app_name {
                Some(name) if !name.trim().is_empty() => {
                    format!("{verb} {}", escape_mnemonic(name.trim()))
                }
                _ => verb.to_string(),
            };

            match self {
                PredefinedMenuItemType::About(_) => return named("About"),
                PredefinedMenuItemType::Hide => return named("Hide"),
                PredefinedMenuItemType::Quit => return named("Quit"),
                _ => {}
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = app_name;

        self.text().to_string()
    }

    /// Whether this kind does anything on the platform being compiled for.
    ///
    /// An unsupported kind is not rejected at construction — it is created and left
    /// disabled, which is why this feeds `enabled` rather than an error.
    #[cfg(target_os = "windows")]
    pub(crate) fn is_supported(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Copy
                | PredefinedMenuItemType::Cut
                | PredefinedMenuItemType::Paste
                | PredefinedMenuItemType::SelectAll
                | PredefinedMenuItemType::Undo
                | PredefinedMenuItemType::Redo
                | PredefinedMenuItemType::Minimize
                | PredefinedMenuItemType::Maximize
                | PredefinedMenuItemType::Hide
                | PredefinedMenuItemType::CloseWindow
                | PredefinedMenuItemType::Quit
                | PredefinedMenuItemType::About(_)
        )
    }

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        feature = "gtk"
    ))]
    pub(crate) fn is_supported(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Copy
                | PredefinedMenuItemType::Cut
                | PredefinedMenuItemType::Paste
                | PredefinedMenuItemType::SelectAll
                | PredefinedMenuItemType::About(_)
        )
    }

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        feature = "gtk4"
    ))]
    pub(crate) fn is_supported(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Minimize
                | PredefinedMenuItemType::Maximize
                | PredefinedMenuItemType::Fullscreen
                | PredefinedMenuItemType::Hide
                | PredefinedMenuItemType::CloseWindow
                | PredefinedMenuItemType::Quit
                | PredefinedMenuItemType::About(_)
        )
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn is_supported(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Copy
                | PredefinedMenuItemType::Cut
                | PredefinedMenuItemType::Paste
                | PredefinedMenuItemType::PasteAndMatchStyle
                | PredefinedMenuItemType::Delete
                | PredefinedMenuItemType::SelectAll
                | PredefinedMenuItemType::Undo
                | PredefinedMenuItemType::Redo
                | PredefinedMenuItemType::Minimize
                | PredefinedMenuItemType::Maximize
                | PredefinedMenuItemType::ActualSize
                | PredefinedMenuItemType::ZoomIn
                | PredefinedMenuItemType::ZoomOut
                | PredefinedMenuItemType::Fullscreen
                | PredefinedMenuItemType::Hide
                | PredefinedMenuItemType::HideOthers
                | PredefinedMenuItemType::ShowAll
                | PredefinedMenuItemType::CloseWindow
                | PredefinedMenuItemType::Quit
                | PredefinedMenuItemType::About(_)
                | PredefinedMenuItemType::Services
                | PredefinedMenuItemType::BringAllToFront
                | PredefinedMenuItemType::StartSpeaking
                | PredefinedMenuItemType::StopSpeaking
                | PredefinedMenuItemType::StartDictation
                | PredefinedMenuItemType::EmojiAndSymbols
        )
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

/// Make `text` survive mnemonic decoding unchanged.
///
/// Application names are not labels the caller wrote, so an ampersand in one is a literal
/// ampersand — `Foo & Bar` has to reach the user as `Foo & Bar`, not as `Foo  Bar` with the
/// space swallowed as a mnemonic marker.
#[cfg(target_os = "macos")]
fn escape_mnemonic(text: &str) -> String {
    text.replace('&', "&&")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
