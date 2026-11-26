// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use objc2::rc::Retained;
use objc2_core_foundation::CGFloat;
use objc2_foundation::NSData;
use objc2_ui_kit::UIImage;

use crate::icon::{BadIcon, RgbaIcon};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct PlatformIcon(RgbaIcon);

impl PlatformIcon {
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
        Ok(PlatformIcon(RgbaIcon::from_rgba(rgba, width, height)?))
    }

    pub fn to_png(&self) -> Vec<u8> {
        let mut png = Vec::new();

        {
            let mut encoder =
                png::Encoder::new(Cursor::new(&mut png), self.0.width as _, self.0.height as _);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&self.0.rgba).unwrap();
        }

        png
    }

    /// Renders the icon as a [`UIImage`].
    ///
    /// `max_height` scales the image down to that many points if it is taller, keeping its
    /// aspect ratio. An icon that already fits is left at its natural size rather than scaled
    /// up.
    pub fn to_uiimage(&self, max_height: f64) -> Option<Retained<UIImage>> {
        let height = self.0.height as CGFloat;
        let scale = (height / max_height as CGFloat).max(1.0);
        UIImage::imageWithData_scale(&NSData::with_bytes(&self.to_png()), scale)
    }
}
