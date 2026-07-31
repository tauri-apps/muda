// Copyright 2014-2021 The winit contributors
// Copyright 2021-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

use crate::icon::{BadIcon, RgbaIcon};

/// An icon used for the window titlebar, taskbar, etc.
#[derive(Debug, Clone)]
pub struct PlatformIcon(gtk::gdk_pixbuf::Pixbuf);

// Safety: `PlatformIcon` is used only on the same thread as the one created it
unsafe impl Send for PlatformIcon {}
unsafe impl Sync for PlatformIcon {}

impl PlatformIcon {
    /// Creates an `Icon` from 32bpp RGBA data.
    ///
    /// The length of `rgba` must be divisible by 4, and `width * height` must equal
    /// `rgba.len() / 4`. Otherwise, this will return a `BadIcon` error.
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
        let RgbaIcon {
            rgba,
            width,
            height,
        } = RgbaIcon::from_rgba(rgba, width, height)?;

        let bytes = gtk::glib::Bytes::from_owned(rgba);
        let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_bytes(
            &bytes,
            gtk::gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width as i32,
            height as i32,
            (width * 4) as i32,
        );

        Ok(Self(pixbuf))
    }

    pub fn texture(&self) -> gtk::gdk::Texture {
        gtk::gdk::Texture::for_pixbuf(&self.0)
    }

    pub fn to_pixbuf(&self) -> gtk::gdk_pixbuf::Pixbuf {
        self.0.clone()
    }
}
