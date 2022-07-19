//! Accelerators describe keyboard shortcuts defined by the application.
//!
//! [`Accelerator`s](crate::accelerator::Accelerator) are used to define a keyboard shortcut consisting
//! of an optional combination of modifier keys (provided by [`SysMods`](crate::accelerator::SysMods),
//! [`RawMods`](crate::accelerator::RawMods) or [`Modifiers`](crate::accelerator::Modifiers)) and
//! one key ([`Code`](crate::accelerator::Code)).
//!
//! # Examples
//! They can be created directly
//! ```
//! # use muda::accelerator::{Accelerator, Mods, Modifiers, Code};
//! #
//! let accelerator = Accelerator::new(Mods::Shift, Code::KeyQ);
//! let accelerator_with_raw_mods = Accelerator::new(Mods::Shift, Code::KeyQ);
//! let accelerator_without_mods = Accelerator::new(None, Code::KeyQ);
//! # assert_eq!(accelerator, accelerator_with_raw_mods);
//! ```
//! or from `&str`, note that all modifiers
//! have to be listed before the non-modifier key, `shift+alt+q` is legal,
//! whereas `shift+q+alt` is not.
//! ```
//! # use muda::accelerator::{Accelerator, Mods};
//! #
//! let accelerator: Accelerator = "shift+alt+q".parse().unwrap();
//! #
//! # // This assert exists to ensure a test breaks once the
//! # // statement above about ordering is no longer valid.
//! # assert!("shift+q+alt".parse::<Accelerator>().is_err());
//! ```
//!

pub use keyboard_types::{Code, Modifiers};
use std::{borrow::Borrow, hash::Hash, str::FromStr};

/// Base `Accelerator` functions.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Accelerator {
    pub(crate) mods: Modifiers,
    pub(crate) key: Code,
}

impl Accelerator {
    /// Creates a new accelerator to define keyboard shortcuts throughout your application.
    pub fn new(mods: impl Into<Option<Modifiers>>, key: Code) -> Self {
        Self {
            mods: mods.into().unwrap_or_else(Modifiers::empty),
            key,
        }
    }

    /// Returns `true` if this [`Code`] and [`Modifiers`] matches this `Accelerator`.
    ///
    /// [`Code`]: Code
    /// [`Modifiers`]: crate::accelerator::Modifiers
    pub fn matches(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Code>) -> bool {
        // Should be a const but const bit_or doesn't work here.
        let base_mods = Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
        let modifiers = modifiers.borrow();
        let key = key.borrow();
        self.mods == *modifiers & base_mods && self.key == *key
    }
}

// Accelerator::from_str is available to be backward
// compatible with tauri and it also open the option
// to generate accelerator from string
impl FromStr for Accelerator {
    type Err = AcceleratorParseError;
    fn from_str(accelerator_string: &str) -> Result<Self, Self::Err> {
        parse_accelerator(accelerator_string)
    }
}

/// Represents the active modifier keys.
///
/// This is intended to be clearer than [`Modifiers`], when describing accelerators.
///
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Mods {
    None,
    Alt,
    Ctrl,
    Meta,
    Shift,
    AltCtrl,
    AltMeta,
    AltShift,
    CtrlShift,
    CtrlMeta,
    MetaShift,
    AltCtrlMeta,
    AltCtrlShift,
    AltMetaShift,
    CtrlMetaShift,
    AltCtrlMetaShift,
}

impl From<Mods> for Option<Modifiers> {
    fn from(src: Mods) -> Option<Modifiers> {
        Some(src.into())
    }
}

impl From<Mods> for Modifiers {
    fn from(src: Mods) -> Modifiers {
        let (alt, ctrl, meta, shift) = match src {
            Mods::None => (false, false, false, false),
            Mods::Alt => (true, false, false, false),
            Mods::Ctrl => (false, true, false, false),
            Mods::Meta => (false, false, true, false),
            Mods::Shift => (false, false, false, true),
            Mods::AltCtrl => (true, true, false, false),
            Mods::AltMeta => (true, false, true, false),
            Mods::AltShift => (true, false, false, true),
            Mods::CtrlMeta => (false, true, true, false),
            Mods::CtrlShift => (false, true, false, true),
            Mods::MetaShift => (false, false, true, true),
            Mods::AltCtrlMeta => (true, true, true, false),
            Mods::AltMetaShift => (true, false, true, true),
            Mods::AltCtrlShift => (true, true, false, true),
            Mods::CtrlMetaShift => (false, true, true, true),
            Mods::AltCtrlMetaShift => (true, true, true, true),
        };
        let mut mods = Modifiers::empty();
        mods.set(Modifiers::ALT, alt);
        mods.set(Modifiers::CONTROL, ctrl);
        mods.set(Modifiers::SUPER, meta);
        mods.set(Modifiers::SHIFT, shift);
        mods
    }
}

#[derive(Debug, Clone)]
pub struct AcceleratorParseError(String);

impl std::fmt::Display for AcceleratorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[AcceleratorParseError]: {}", self.0)
    }
}

fn parse_accelerator(accelerator_string: &str) -> Result<Accelerator, AcceleratorParseError> {
    let mut mods = Modifiers::empty();
    let mut key = Code::Unidentified;

    for raw in accelerator_string.split('+') {
        let token = raw.trim().to_string();
        if token.is_empty() {
            return Err(AcceleratorParseError(
                "Unexpected empty token while parsing accelerator".into(),
            ));
        }

        if key != Code::Unidentified {
            // at this point we already parsed the modifiers and found a main key but
            // the function received more then one main key or it is not in the right order
            // examples:
            // 1. "Ctrl+Shift+C+A" => only one main key should be allowd.
            // 2. "Ctrl+C+Shift" => wrong order
            return Err(AcceleratorParseError(format!(
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
                mods.set(Modifiers::SUPER, true);
            }
            "SHIFT" => {
                mods.set(Modifiers::SHIFT, true);
            }
            "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
                #[cfg(target_os = "macos")]
                mods.set(Modifiers::SUPER, true);
                #[cfg(not(target_os = "macos"))]
                mods.set(Modifiers::CONTROL, true);
            }
            _ => {
                if let Ok(code) = Code::from_str(token.as_str()) {
                    match code {
                        Code::Unidentified => {
                            return Err(AcceleratorParseError(format!(
                                "Couldn't identify \"{}\" as a valid `Code`",
                                token
                            )))
                        }
                        _ => key = code,
                    }
                } else {
                    return Err(AcceleratorParseError(format!(
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
            mods: Modifiers::SUPER | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
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
