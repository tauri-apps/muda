use crate::{accelerator::Accelerator, AboutMetadata};
use keyboard_types::{Code, Modifiers};

#[cfg(target_os = "macos")]
pub const CMD_OR_CTRL: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
pub const CMD_OR_CTRL: Modifiers = Modifiers::CONTROL;

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
            PredfinedMenuItemType::Copy => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyC))
            }
            PredfinedMenuItemType::Cut => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyX))
            }
            PredfinedMenuItemType::Paste => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyV))
            }
            PredfinedMenuItemType::Undo => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyZ))
            }
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Redo => {
                Some(Accelerator::new(Some(CMD_OR_CTRL | Modifiers::SHIFT), Code::KeyZ))
            }
            #[cfg(not(target_os = "macos"))]
            PredfinedMenuItemType::Redo => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyY))
            }
            PredfinedMenuItemType::SelectAll => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyA))
            }
            PredfinedMenuItemType::Minimize => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyM))
            }
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Fullscreen => {
                Some(Accelerator::new(Some(Modifiers::META | Modifiers::CONTROL), Code::KeyF))
            }
            PredfinedMenuItemType::Hide =>  {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyH))
            }
            PredfinedMenuItemType::HideOthers =>  {
                Some(Accelerator::new(Some(CMD_OR_CTRL | Modifiers::ALT), Code::KeyH))
            }
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::CloseWindow => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyW))
            }
            #[cfg(not(target_os = "macos"))]
            PredfinedMenuItemType::CloseWindow => {
                Some(Accelerator::new(Some(Modifiers::ALT), Code::F4))
            }
            #[cfg(target_os = "macos")]
            PredfinedMenuItemType::Quit => {
                Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyQ))
            }
            _ => None,
        }
    }
}
