// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use objc2::{rc::Retained, AllocAnyThread};
use objc2_app_kit::NSImage;
use objc2_core_foundation::CGFloat;
use objc2_foundation::{NSData, NSSize};

use crate::icon::{BadIcon, RgbaIcon};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct PlatformIcon(RgbaIcon);

impl PlatformIcon {
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
        Ok(PlatformIcon(RgbaIcon::from_rgba(rgba, width, height)?))
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.0.width, self.0.height)
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

    /// Renders the icon as an [`NSImage`].
    ///
    /// `max_height` scales the image down to that many points if it is taller, keeping its
    /// aspect ratio. An icon that already fits is left at its natural size rather than scaled
    /// up.
    pub fn to_nsimage(&self, max_height: Option<f64>) -> Retained<NSImage> {
        let (width, height) = self.get_size();
        let icon = self.to_png();

        let mut icon_width = width as CGFloat;
        let mut icon_height = height as CGFloat;

        if let Some(max_height) = max_height.map(|max_height| max_height as CGFloat) {
            if icon_height > max_height {
                icon_width /= icon_height / max_height;
                icon_height = max_height;
            }
        }

        let nsdata = NSData::with_bytes(&icon);

        let nsimage = NSImage::initWithData(NSImage::alloc(), &nsdata).unwrap();
        let new_size = NSSize::new(icon_width, icon_height);
        nsimage.setSize(new_size);

        nsimage
    }
}
