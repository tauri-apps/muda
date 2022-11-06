use crate::{accelerator::Accelerator, MenuItemExt, MenuItemType};
use keyboard_types::{Code, Modifiers};

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

    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredfinedMenuItemType::Separator, None)
    }

    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Minimize, text)
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
    Copy,
    Cut,
    Paste,
    SelectAll,
    Separator,
    Minimize,
    CloseWindow,
    Quit,
    About(Option<AboutMetadata>),
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
            PredfinedMenuItemType::Copy => "&Copy",
            PredfinedMenuItemType::Cut => "Cu&t",
            PredfinedMenuItemType::Paste => "&Paste",
            PredfinedMenuItemType::SelectAll => "Select &All",
            PredfinedMenuItemType::Separator => "",
            PredfinedMenuItemType::Minimize => "&Minimize",
            #[cfg(windows)]
            PredfinedMenuItemType::CloseWindow => "Close",
            #[cfg(not(windows))]
            PredfinedMenuItemType::CloseWindow => "C&lose Window",
            #[cfg(windows)]
            PredfinedMenuItemType::Quit => "&Exit",
            #[cfg(not(windows))]
            PredfinedMenuItemType::Quit => "&Quit",
            PredfinedMenuItemType::About(_) => "&About",
            PredfinedMenuItemType::None => "",
        }
    }

    pub(crate) fn accelerator(&self) -> Option<Accelerator> {
        match self {
            PredfinedMenuItemType::Copy => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyC))
            }
            PredfinedMenuItemType::Cut => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyX))
            }
            PredfinedMenuItemType::Paste => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyV))
            }

            PredfinedMenuItemType::SelectAll => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyA))
            }
            _ => None,
        }
    }
}
