// Copyright 2014-2021 The winit contributors
// Copyright 2021-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::icon::BadIcon;

/// An icon used for the window titlebar, taskbar, etc.
#[derive(Debug, Clone)]
pub struct PlatformIcon(gtk4::gio::BytesIcon);

// Safety: `PlatformIcon` is used only on the same thread as the one created it
unsafe impl Send for PlatformIcon {}
unsafe impl Sync for PlatformIcon {}

impl PlatformIcon {
    /// Creates an `Icon` from 32bpp RGBA data.
    ///
    /// The length of `rgba` must be divisible by 4, and `width * height` must equal
    /// `rgba.len() / 4`. Otherwise, this will return a `BadIcon` error.
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
        let mut w = Vec::with_capacity(rgba.len());

        let mut encoder = png::Encoder::new(&mut w, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(BadIcon::PngEncodingError)?;
        writer
            .write_image_data(&rgba)
            .map_err(BadIcon::PngEncodingError)?;
        writer.finish().map_err(BadIcon::PngEncodingError)?;

        let bytes = gtk4::glib::Bytes::from_owned(w);

        Ok(Self(gtk4::gio::BytesIcon::new(&bytes)))
    }

    pub fn bytes_icon(&self) -> &gtk4::gio::BytesIcon {
        &self.0
    }
}
