use crate::{accelerator::Accelerator, AboutMetadata};
use keyboard_types::{Code, Modifiers};

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
