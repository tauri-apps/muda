use crate::{accelerator::Accelerator, MenuItemExt, MenuItemType};
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
    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredfinedMenuItemType::Separator, None)
    }

    pub fn copy(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Copy, text)
    }

    pub fn cut(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Cut, text)
    }

    pub fn paste(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Paste, text)
    }

    pub fn select_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::SelectAll, text)
    }

    pub fn undo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Undo, text)
    }

    pub fn redo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Redo, text)
    }

    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Minimize, text)
    }

    pub fn maximize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Maximize, text)
    }

    pub fn fullscreen(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Fullscreen, text)
    }

    pub fn hide(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Hide, text)
    }

    pub fn hide_others(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::HideOthers, text)
    }

    pub fn show_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::ShowAll, text)
    }

    pub fn close_window(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::CloseWindow, text)
    }

    pub fn quit(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Quit, text)
    }

    pub fn about(text: Option<&str>, metadata: Option<AboutMetadata>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::About(metadata), text)
    }

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
///
/// ## Platform-specific
///
/// - **macOS:** The metadata is ignored.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct AboutMetadata {
    /// The application name.
    pub name: Option<String>,
    /// The application version.
    pub version: Option<String>,
    /// The authors of the application.
    pub authors: Option<Vec<String>>,
    /// Application comments.
    pub comments: Option<String>,
    /// The copyright of the application.
    pub copyright: Option<String>,
    /// The license of the application.
    pub license: Option<String>,
    /// The application website.
    pub website: Option<String>,
    /// The website label.
    pub website_label: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
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
            PredfinedMenuItemType::Maximize => "Maximize",
            PredfinedMenuItemType::Fullscreen => "Toggle Full Screen",
            PredfinedMenuItemType::Hide => "Hide",
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
