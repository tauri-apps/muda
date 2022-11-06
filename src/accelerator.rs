//! Accelerators describe keyboard shortcuts for menu items.
//!
//! [`Accelerator`s](crate::accelerator::Accelerator) are used to define a keyboard shortcut consisting
//! of an optional combination of modifier keys (provided by [`Modifiers`](crate::accelerator::Modifiers)) and
//! one key ([`Code`](crate::accelerator::Code)).
//!
//! # Examples
//! They can be created directly
//! ```no_run
//! # use muda::accelerator::{Accelerator, Modifiers, Code};
//! let accelerator = Accelerator::new(Some(Modifiers::SHIFT), Code::KeyQ);
//! let accelerator_without_mods = Accelerator::new(None, Code::KeyQ);
//! ```
//! or from `&str`, note that all modifiers
//! have to be listed before the non-modifier key, `shift+alt+KeyQ` is legal,
//! whereas `shift+q+alt` is not.
//! ```no_run
//! # use muda::accelerator::{Accelerator};
//! let accelerator: Accelerator = "shift+alt+KeyQ".parse().unwrap();
//! # // This assert exists to ensure a test breaks once the
//! # // statement above about ordering is no longer valid.
//! # assert!("shift+KeyQ+alt".parse::<Accelerator>().is_err());
//! ```
//!

pub use keyboard_types::{Code, Modifiers};
use std::{borrow::Borrow, hash::Hash, str::FromStr};

/// A keyboard shortcut that consists of an optional combination
/// of modifier keys (provided by [`Modifiers`](crate::accelerator::Modifiers)) and
/// one key ([`Code`](crate::accelerator::Code)).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Accelerator {
    pub(crate) mods: Modifiers,
    pub(crate) key: Code,
}

impl Accelerator {
    /// Creates a new accelerator to define keyboard shortcuts throughout your application.
    /// Only [`Modifiers::ALT`], [`Modifiers::SHIFT`], [`Modifiers::CONTROL`], and [`Modifiers::META`]/[`Modifiers::SUPER`]
    pub fn new(mods: Option<Modifiers>, key: Code) -> Self {
        Self {
            mods: mods.unwrap_or_else(Modifiers::empty),
            key,
        }
    }

    /// Returns `true` if this [`Code`] and [`Modifiers`] matches this `Accelerator`.
    pub fn matches(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Code>) -> bool {
        // Should be a const but const bit_or doesn't work here.
        let base_mods = Modifiers::SHIFT
            | Modifiers::CONTROL
            | Modifiers::ALT
            | Modifiers::META
            | Modifiers::SUPER;
        let modifiers = modifiers.borrow();
        let key = key.borrow();
        self.mods == *modifiers & base_mods && self.key == *key
    }
}

// Accelerator::from_str is available to be backward
// compatible with tauri and it also open the option
// to generate accelerator from string
impl FromStr for Accelerator {
    type Err = crate::Error;
    fn from_str(accelerator_string: &str) -> Result<Self, Self::Err> {
        parse_accelerator(accelerator_string)
    }
}

fn parse_accelerator(accelerator_string: &str) -> crate::Result<Accelerator> {
    let mut mods = Modifiers::empty();
    let mut key = Code::Unidentified;

    for raw in accelerator_string.split('+') {
        let token = raw.trim().to_string();
        if token.is_empty() {
            return Err(crate::Error::AcceleratorParseError(
                "Unexpected empty token while parsing accelerator".into(),
            ));
        }

        if key != Code::Unidentified {
            // at this point we already parsed the modifiers and found a main key but
            // the function received more then one main key or it is not in the right order
            // examples:
            // 1. "Ctrl+Shift+C+A" => only one main key should be allowd.
            // 2. "Ctrl+C+Shift" => wrong order
            return Err(crate::Error::AcceleratorParseError(format!(
                "Unexpected accelerator string format: \"{}\"",
                accelerator_string
            )));
        }

        match token.to_uppercase().as_str() {
            "OPTION" | "ALT" => {
                mods.set(Modifiers::ALT, true);
            }
            "CONTROL" | "CTRL" => {
                mods.set(Modifiers::CONTROL, true);
            }
            "COMMAND" | "CMD" | "SUPER" => {
                mods.set(Modifiers::META, true);
            }
            "SHIFT" => {
                mods.set(Modifiers::SHIFT, true);
            }
            "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
                #[cfg(target_os = "macos")]
                mods.set(Modifiers::META, true);
                #[cfg(not(target_os = "macos"))]
                mods.set(Modifiers::CONTROL, true);
            }
            _ => {
                if let Ok(code) = Code::from_str(token.as_str()) {
                    match code {
                        Code::Unidentified => {
                            return Err(crate::Error::AcceleratorParseError(format!(
                                "Couldn't identify \"{}\" as a valid `Code`",
                                token
                            )))
                        }
                        _ => key = code,
                    }
                } else {
                    return Err(crate::Error::AcceleratorParseError(format!(
                        "Couldn't identify \"{}\" as a valid `Code`",
                        token
                    )));
                }
            }
        }
    }

    Ok(Accelerator { key, mods })
}

#[test]
fn test_parse_accelerator() {
    assert_eq!(
        parse_accelerator("CTRL+KeyX").unwrap(),
        Accelerator {
            mods: Modifiers::CONTROL,
            key: Code::KeyX,
        }
    );
    assert_eq!(
        parse_accelerator("SHIFT+KeyC").unwrap(),
        Accelerator {
            mods: Modifiers::SHIFT,
            key: Code::KeyC,
        }
    );
    assert_eq!(
        parse_accelerator("CTRL+KeyZ").unwrap(),
        Accelerator {
            mods: Modifiers::CONTROL,
            key: Code::KeyZ,
        }
    );
    assert_eq!(
        parse_accelerator("super+ctrl+SHIFT+alt+ArrowUp").unwrap(),
        Accelerator {
            mods: Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
            key: Code::ArrowUp,
        }
    );
    assert_eq!(
        parse_accelerator("Digit5").unwrap(),
        Accelerator {
            mods: Modifiers::empty(),
            key: Code::Digit5,
        }
    );
    assert_eq!(
        parse_accelerator("KeyG").unwrap(),
        Accelerator {
            mods: Modifiers::empty(),
            key: Code::KeyG,
        }
    );

    let acc = parse_accelerator("+G");
    assert!(acc.is_err());

    let acc = parse_accelerator("SHGSH+G");
    assert!(acc.is_err());

    assert_eq!(
        parse_accelerator("SHiFT+F12").unwrap(),
        Accelerator {
            mods: Modifiers::SHIFT,
            key: Code::F12,
        }
    );
    assert_eq!(
        parse_accelerator("CmdOrCtrl+Space").unwrap(),
        Accelerator {
            #[cfg(target_os = "macos")]
            mods: Modifiers::SUPER,
            #[cfg(not(target_os = "macos"))]
            mods: Modifiers::CONTROL,
            key: Code::Space,
        }
    );

    let acc = parse_accelerator("CTRL+");
    assert!(acc.is_err());
}
