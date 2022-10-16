use crate::{accelerator::Accelerator, AboutMetadata};
use keyboard_types::{Code, Modifiers};

#[cfg(target_os = "macos")]
pub const CMD_OR_CTRL: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
pub const CMD_OR_CTRL: Modifiers = Modifiers::CONTROL;

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
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyC))
            }
            PredfinedMenuItemType::Cut => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyX))
            }
            PredfinedMenuItemType::Paste => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyV))
            }
            PredfinedMenuItemType::SelectAll => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyA))
            }
            _ => None,
        }
    }
}
