// font-kit/src/canvas.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use euclid::Size2D;

pub struct Canvas {
    pub pixels: Vec<u8>,
    pub size: Size2D<u32>,
    pub stride: usize,
    pub format: Format,
}

impl Canvas {
    pub fn new(size: &Size2D<u32>, stride: usize, format: Format) -> Canvas {
        Canvas {
            pixels: vec![0; stride * size.height as usize * format.bytes_per_pixel() as usize],
            size: *size,
            stride,
            format,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    /// Premultiplied R8G8B8A8, little-endian.
    Rgba32,
    Rgb24,
    A8,
}

impl Format {
    #[inline]
    pub fn bits_per_pixel(self) -> u8 {
        match self {
            Format::Rgba32 => 32,
            Format::Rgb24 => 24,
            Format::A8 => 8,
        }
    }

    #[inline]
    pub fn components_per_pixel(self) -> u8 {
        match self {
            Format::Rgba32 => 4,
            Format::Rgb24 => 3,
            Format::A8 => 1,
        }
    }

    #[inline]
    pub fn bits_per_component(self) -> u8 {
        self.bits_per_pixel() / self.components_per_pixel()
    }

    #[inline]
    pub fn bytes_per_pixel(self) -> u8 {
        self.bits_per_pixel() / 8
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RasterizationOptions {
    Bilevel,
    GrayscaleAa,
    SubpixelAa,
}
