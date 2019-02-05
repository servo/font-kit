// font-kit/src/canvas.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! An in-memory bitmap surface for glyph rasterization.

use euclid::Size2D;
use std::cmp;

use utils;

lazy_static! {
    static ref BITMAP_1BPP_TO_8BPP_LUT: [[u8; 8]; 256] = {
        let mut lut = [[0; 8]; 256];
        for byte in 0..0x100 {
            let mut value = [0; 8];
            for bit in 0..8 {
                if (byte & (0x80 >> bit)) != 0 {
                    value[bit] = 0xff;
                }
            }
            lut[byte] = value
        }
        lut
    };
}

/// An in-memory bitmap surface for glyph rasterization.
pub struct Canvas {
    /// The raw pixel data.
    pub pixels: Vec<u8>,
    /// The size of the buffer, in pixels.
    pub size: Size2D<u32>,
    /// The number of *bytes* between successive rows.
    pub stride: usize,
    /// The image format of the canvas.
    pub format: Format,
}

impl Canvas {
    /// Creates a new blank canvas with the given pixel size and format.
    ///
    /// Stride is automatically calculated from width.
    ///
    /// The canvas is initialized with transparent black (all values 0).
    #[inline]
    pub fn new(size: &Size2D<u32>, format: Format) -> Canvas {
        Canvas::with_stride(size, size.width as usize * format.bytes_per_pixel() as usize, format)
    }

    /// Creates a new blank canvas with the given pixel size, stride (number of bytes between
    /// successive rows), and format.
    ///
    /// The canvas is initialized with transparent black (all values 0).
    pub fn with_stride(size: &Size2D<u32>, stride: usize, format: Format) -> Canvas {
        Canvas {
            pixels: vec![0; stride * size.height as usize],
            size: *size,
            stride,
            format,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn blit_from_canvas(&mut self, src: &Canvas) {
        self.blit_from(&src.pixels, &src.size, src.stride, src.format)
    }

    #[allow(dead_code)]
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
            (Format::Rgb24, Format::A8) => {
                self.blit_from_with::<BlitA8ToRgb24>(src_bytes, &size, src_stride, src_format)
            }
            (Format::Rgb24, Format::Rgba32) => {
                self.blit_from_with::<BlitRgba32ToRgb24>(src_bytes, &size, src_stride, src_format)
            }
            (Format::Rgba32, Format::Rgb24) |
            (Format::Rgba32, Format::A8) |
            (Format::A8, Format::Rgba32) => unimplemented!(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn blit_from_bitmap_1bpp(&mut self,
                                        src_bytes: &[u8],
                                        src_size: &Size2D<u32>,
                                        src_stride: usize) {
        if self.format != Format::A8 {
            unimplemented!()
        }

        let width = cmp::min(src_size.width as usize, self.size.width as usize);
        let height = cmp::min(src_size.height as usize, self.size.height as usize);
        let size = Size2D::new(width, height);

        let dest_bytes_per_pixel = self.format.bytes_per_pixel() as usize;
        let dest_row_stride = size.width * dest_bytes_per_pixel;
        let src_row_stride = utils::div_round_up(size.width, 8);

        for y in 0..size.height {
            let (dest_row_start, src_row_start) = (y * self.stride, y * src_stride);
            let dest_row_end = dest_row_start + dest_row_stride;
            let src_row_end = src_row_start + src_row_stride;
            let dest_row_pixels = &mut self.pixels[dest_row_start..dest_row_end];
            let src_row_pixels = &src_bytes[src_row_start..src_row_end];
            for x in 0..src_row_stride {
                let pattern = &BITMAP_1BPP_TO_8BPP_LUT[src_row_pixels[x] as usize];
                let dest_start = x * 8;
                let dest_end = cmp::min(dest_start + 8, dest_row_stride);
                let src = &pattern[0..(dest_end - dest_start)];
                dest_row_pixels[dest_start..dest_end].clone_from_slice(src);
            }
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

/// The image format for the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    /// Premultiplied R8G8B8A8, little-endian.
    Rgba32,
    /// R8G8B8, little-endian.
    Rgb24,
    /// A8.
    A8,
}

impl Format {
    /// Returns the number of bits per pixel that this image format corresponds to.
    #[inline]
    pub fn bits_per_pixel(self) -> u8 {
        match self {
            Format::Rgba32 => 32,
            Format::Rgb24 => 24,
            Format::A8 => 8,
        }
    }

    /// Returns the number of color channels per pixel that this image format corresponds to.
    #[inline]
    pub fn components_per_pixel(self) -> u8 {
        match self {
            Format::Rgba32 => 4,
            Format::Rgb24 => 3,
            Format::A8 => 1,
        }
    }

    /// Returns the number of bits per color channel that this image format contains.
    #[inline]
    pub fn bits_per_component(self) -> u8 {
        self.bits_per_pixel() / self.components_per_pixel()
    }

    /// Returns the number of bytes per pixel that this image format corresponds to.
    #[inline]
    pub fn bytes_per_pixel(self) -> u8 {
        self.bits_per_pixel() / 8
    }
}

/// The antialiasing strategy that should be used when rasterizing glyphs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RasterizationOptions {
    /// "Black-and-white" rendering. Each pixel is either entirely on or off.
    Bilevel,
    /// Grayscale antialiasing. Only one channel is used.
    GrayscaleAa,
    /// Subpixel RGB antialiasing, for LCD screens.
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

struct BlitA8ToRgb24;

impl Blit for BlitA8ToRgb24 {
    #[inline]
    fn blit(dest: &mut [u8], src: &[u8]) {
        for (dest, src) in dest.chunks_mut(3).zip(src.iter()) {
            dest[0] = *src;
            dest[1] = *src;
            dest[2] = *src;
        }
    }
}

struct BlitRgba32ToRgb24;

impl Blit for BlitRgba32ToRgb24 {
    #[inline]
    fn blit(dest: &mut [u8], src: &[u8]) {
        // TODO(pcwalton): SIMD.
        for (dest, src) in dest.chunks_mut(3).zip(src.chunks(4)) {
            dest.copy_from_slice(&src[0..3])
        }
    }
}
