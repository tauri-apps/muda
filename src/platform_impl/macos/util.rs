// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use objc2_app_kit::NSRunningApplication;

pub(crate) fn app_name() -> Option<String> {
    NSRunningApplication::currentApplication()
        .localizedName()
        .map(|name| name.to_string())
}
