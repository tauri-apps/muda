// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

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

    pub unsafe fn to_nsimage(&self, fixed_height: Option<f64>) -> cocoa::base::id {
        use cocoa::{
            appkit::NSImage,
            base::nil,
            foundation::{NSData, NSSize},
        };

        let (width, height) = self.get_size();
        let icon = self.to_png();

        let (icon_width, icon_height) = match fixed_height {
            Some(fixed_height) => {
                let icon_height: f64 = fixed_height;
                let icon_width: f64 = (width as f64) / (height as f64 / icon_height);

                (icon_width, icon_height)
            }

            None => (width as f64, height as f64),
        };

        let nsdata = NSData::dataWithBytes_length_(
            nil,
            icon.as_ptr() as *const std::os::raw::c_void,
            icon.len() as u64,
        );

        let nsimage = NSImage::initWithData_(NSImage::alloc(nil), nsdata);
        let new_size = NSSize::new(icon_width, icon_height);
        let _: () = msg_send![nsimage, setSize: new_size];

        nsimage
    }
}
