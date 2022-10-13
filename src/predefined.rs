use crate::{accelerator::Accelerator, TextMenuItem};
use keyboard_types::{Code, Modifiers};

pub fn copy(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::Copy, text)
}

pub fn cut(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::Cut, text)
}

pub fn paste(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::Paste, text)
}

pub fn select_all(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::SelectAll, text)
}

/// A Separator in a menu
///
/// ## Platform-specific:
///
/// - **Windows**: Doesn't work when added in the [menu bar](crate::Menu)
pub fn separator() -> TextMenuItem {
    TextMenuItem::predefined::<&str>(PredfinedMenuItem::Separator, None)
}

pub fn minimize(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::Minimize, text)
}

pub fn close_window(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::CloseWindow, text)
}

pub fn quit(text: Option<&str>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::Quit, text)
}

pub fn about(text: Option<&str>, metadata: Option<AboutMetadata>) -> TextMenuItem {
    TextMenuItem::predefined(PredfinedMenuItem::About(metadata), text)
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[non_exhaustive]
pub(crate) enum PredfinedMenuItem {
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

impl Default for PredfinedMenuItem {
    fn default() -> Self {
        Self::None
    }
}

impl PredfinedMenuItem {
    pub(crate) fn text(&self) -> &str {
        match self {
            PredfinedMenuItem::Copy => "&Copy",
            PredfinedMenuItem::Cut => "Cu&t",
            PredfinedMenuItem::Paste => "&Paste",
            PredfinedMenuItem::SelectAll => "Select &All",
            PredfinedMenuItem::Separator => "",
            PredfinedMenuItem::Minimize => "&Minimize",
            #[cfg(windows)]
            PredfinedMenuItem::CloseWindow => "Close",
            #[cfg(not(windows))]
            PredfinedMenuItem::CloseWindow => "C&lose Window",
            PredfinedMenuItem::Quit => "&Quit",
            PredfinedMenuItem::About(_) => "&About",
            PredfinedMenuItem::None => "",
        }
    }

    pub(crate) fn accelerator(&self) -> Option<Accelerator> {
        match self {
            PredfinedMenuItem::Copy => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyC))
            }
            PredfinedMenuItem::Cut => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyX))
            }
            PredfinedMenuItem::Paste => {
                #[cfg(target_os = "macos")]
                let mods = Modifiers::META;
                #[cfg(not(target_os = "macos"))]
                let mods = Modifiers::CONTROL;
                Some(Accelerator::new(Some(mods), Code::KeyV))
            }

            PredfinedMenuItem::SelectAll => {
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

/// Application metadata for the [`NativeMenuItem::About`].
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
