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
use std::cmp;

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

    pub(crate) fn blit_from(&mut self,
                            src_bytes: &[u8],
                            src_size: &Size2D<u32>,
                            src_stride: usize,
                            src_format: Format) {
        let width = cmp::min(src_size.width as usize, self.size.width as usize);
        let height = cmp::min(src_size.height as usize, self.size.height as usize);
        let size = Size2D::new(width, height);

        match (self.format, src_format) {
            (Format::A8, Format::A8) |
            (Format::Rgb24, Format::Rgb24) |
            (Format::Rgba32, Format::Rgba32) => {
                self.blit_from_with::<BlitMemcpy>(src_bytes, &size, src_stride, src_format)
            }
            (Format::A8, Format::Rgb24) => {
                self.blit_from_with::<BlitRgb24ToA8>(src_bytes, &size, src_stride, src_format)
            }
            _ => unimplemented!()
        }
    }

    fn blit_from_with<B>(&mut self,
                         src_bytes: &[u8],
                         size: &Size2D<usize>,
                         src_stride: usize,
                         src_format: Format)
                         where B: Blit {
        let src_bytes_per_pixel = src_format.bytes_per_pixel() as usize;
        let dest_bytes_per_pixel = self.format.bytes_per_pixel() as usize;

        for y in 0..size.height {
            let (dest_row_start, src_row_start) = (y * self.stride, y * src_stride);
            let dest_row_end = dest_row_start + size.width * dest_bytes_per_pixel;
            let src_row_end = src_row_start + size.width * src_bytes_per_pixel;
            let dest_row_pixels = &mut self.pixels[dest_row_start..dest_row_end];
            let src_row_pixels = &src_bytes[src_row_start..src_row_end];
            B::blit(dest_row_pixels, src_row_pixels)
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

trait Blit {
    fn blit(dest: &mut [u8], src: &[u8]);
}

struct BlitMemcpy;

impl Blit for BlitMemcpy {
    #[inline]
    fn blit(dest: &mut [u8], src: &[u8]) {
        dest.clone_from_slice(src)
    }
}

struct BlitRgb24ToA8;

impl Blit for BlitRgb24ToA8 {
    #[inline]
    fn blit(dest: &mut [u8], src: &[u8]) {
        // TODO(pcwalton): SIMD.
        for (dest, src) in dest.iter_mut().zip(src.chunks(3)) {
            *dest = src[1]
        }
    }
}
