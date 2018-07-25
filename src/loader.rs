// font-kit/src/loader.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use euclid::{Point2D, Rect, Vector2D};
use lyon_path::builder::PathBuilder;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use canvas::{Canvas, RasterizationOptions};
use error::{FontLoadingError, GlyphLoadingError};
use file_type::FileType;
use handle::Handle;
use hinting::HintingOptions;
use metrics::Metrics;
use properties::Properties;

/// Provides a common interface to the platform-specific API that loads, parses, and rasterizes
/// fonts.
pub trait Loader: Clone + Sized {
    type NativeFont;

    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, FontLoadingError>;

    fn from_file(file: &mut File, font_index: u32) -> Result<Self, FontLoadingError>;

    fn from_path<P>(path: P, font_index: u32) -> Result<Self, FontLoadingError>
                    where P: AsRef<Path> {
        Loader::from_file(&mut try!(File::open(path)), font_index)
    }

    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self;

    fn from_handle(handle: &Handle) -> Result<Self, FontLoadingError> {
        match *handle {
            Handle::Memory {
                ref bytes,
                font_index,
            } => Self::from_bytes((*bytes).clone(), font_index),
            Handle::Path {
                ref path,
                font_index,
            } => Self::from_path(path, font_index),
        }
    }

    fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError>;

    #[inline]
    fn analyze_path<P>(path: P) -> Result<FileType, FontLoadingError> where P: AsRef<Path> {
        <Self as Loader>::analyze_file(&mut try!(File::open(path)))
    }

    /// PostScript name of the font.
    fn postscript_name(&self) -> String;

    /// Full name of the font (also known as "display name" on macOS).
    fn full_name(&self) -> String;

    /// Name of the font family.
    fn family_name(&self) -> String;

    /// Whether the font is monospace (fixed-width).
    fn is_monospace(&self) -> bool;

    /// Various font properties, corresponding to those defined in CSS.
    fn properties(&self) -> Properties;

    fn glyph_for_char(&self, character: char) -> Option<u32>;

    fn outline<B>(&self, glyph_id: u32, hinting_mode: HintingOptions, path_builder: &mut B)
                  -> Result<(), GlyphLoadingError>
                  where B: PathBuilder;

    fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError>;

    fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError>;

    fn origin(&self, glyph_id: u32) -> Result<Point2D<f32>, GlyphLoadingError>;

    fn metrics(&self) -> Metrics;

    fn copy_font_data(&self) -> Option<Arc<Vec<u8>>>;

    fn supports_hinting_options(&self, hinting_options: HintingOptions, for_rasterization: bool)
                                -> bool;

    fn raster_bounds(&self,
                     glyph_id: u32,
                     point_size: f32,
                     origin: &Point2D<f32>,
                     _: HintingOptions,
                     _: RasterizationOptions)
                     -> Result<Rect<i32>, GlyphLoadingError> {
        let typographic_bounds = try!(self.typographic_bounds(glyph_id));
        let typographic_raster_bounds = typographic_bounds * point_size /
            self.metrics().units_per_em as f32;
        Ok(typographic_raster_bounds.translate(&origin.to_vector()).round_out().to_i32())
    }

    fn rasterize_glyph(&self,
                       canvas: &mut Canvas,
                       glyph_id: u32,
                       point_size: f32,
                       origin: &Point2D<f32>,
                       hinting_options: HintingOptions,
                       rasterization_options: RasterizationOptions)
                       -> Result<(), GlyphLoadingError>;
}
