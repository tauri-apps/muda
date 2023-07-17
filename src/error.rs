// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use thiserror::Error;

/// Errors returned by muda.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error("This menu item is not a child of this `Menu` or `Submenu`")]
    NotAChildOfThisMenu,
    #[cfg(windows)]
    #[error("This menu has not been initialized for this hwnd`")]
    NotInitialized,
    #[cfg(target_os = "linux")]
    #[error("This menu has not been initialized for this gtk window`")]
    NotInitialized,
    #[cfg(windows)]
    #[error("This menu has already been initialized for this hwnd`")]
    AlreadyInitialized,
    #[cfg(target_os = "linux")]
    #[error("This menu has already been initialized for this gtk window`")]
    AlreadyInitialized,
    #[error("{0}")]
    AcceleratorParseError(String),
    #[error("Couldn't recognize \"{0}\" as a valid Accelerator Code, if you feel like it should be, please report this to https://github.com/tauri-apps/muda")]
    UnrecognizedAcceleratorCode(String),
    #[error("Unexpected empty token while parsing accelerator: \"{0}\"")]
    EmptyAcceleratorToken(String),
    #[error("Unexpected accelerator string format: \"{0}\", a accelerator should have the modifiers first and only contain one main key")]
    UnexpectedAcceleratorFormat(String),
}

/// Convenient type alias of Result type for muda.
pub type Result<T> = std::result::Result<T, Error>;
