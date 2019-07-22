// font-kit/src/loader.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Provides a common interface to the platform-specific API that loads, parses, and rasterizes
//! fonts.

use euclid::default::{Point2D, Rect, Transform2D, Vector2D};
use log::warn;
use lyon_path::builder::PathBuilder;
use std::sync::Arc;

use crate::canvas::{Canvas, RasterizationOptions};
use crate::error::{FontLoadingError, GlyphLoadingError};
use crate::file_type::FileType;
use crate::handle::Handle;
use crate::hinting::HintingOptions;
use crate::metrics::Metrics;
use crate::properties::Properties;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

/// The transform that glyphs will be transformed by.
#[derive(Debug, Clone, Copy)]
pub struct FontTransform {
    /// Scale in the x direction
    pub scale_x: f32,
    /// Skew in the x direction
    pub skew_x: f32,
    /// Skew in the y direction
    pub skew_y: f32,
    /// Scale in the y direction
    pub scale_y: f32,
}

impl FontTransform {
    /// Initializes a FontTransform to the provided values
    pub fn new(scale_x: f32, skew_x: f32, skew_y: f32, scale_y: f32) -> Self {
        FontTransform {
            scale_x,
            skew_x,
            skew_y,
            scale_y,
        }
    }

    /// Initializes a FontTransform to the identity transform
    pub fn identity() -> Self {
        FontTransform::new(1.0, 0.0, 0.0, 1.0)
    }
}

/// Provides a common interface to the platform-specific API that loads, parses, and rasterizes
/// fonts.
pub trait Loader: Clone + Sized {
    /// The handle that the API natively uses to represent a font.
    type NativeFont;

    /// Loads a font from raw font data (the contents of a `.ttf`/`.otf`/etc. file).
    ///
    /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index
    /// of the font to load from it. If the data represents a single font, pass 0 for `font_index`.
    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, FontLoadingError>;

    /// Loads a font from a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    #[cfg(not(target_arch = "wasm32"))]
    fn from_file(file: &mut File, font_index: u32) -> Result<Self, FontLoadingError>;

    /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    #[cfg(not(target_arch = "wasm32"))]
    fn from_path<P>(path: P, font_index: u32) -> Result<Self, FontLoadingError>
    where
        P: AsRef<Path>,
    {
        Loader::from_file(&mut File::open(path)?, font_index)
    }

    /// Creates a font from a native API handle.
    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self;

    /// Loads the font pointed to by a handle.
    fn from_handle(handle: &Handle) -> Result<Self, FontLoadingError> {
        match *handle {
            Handle::Memory {
                ref bytes,
                font_index,
            } => Self::from_bytes((*bytes).clone(), font_index),
            #[cfg(not(target_arch = "wasm32"))]
            Handle::Path {
                ref path,
                font_index,
            } => Self::from_path(path, font_index),
            #[cfg(target_arch = "wasm32")]
            Handle::Path { .. } => Err(FontLoadingError::NoFilesystem),
        }
    }

    /// Determines whether a blob of raw font data represents a supported font, and, if so, what
    /// type of font it is.
    fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<FileType, FontLoadingError>;

    /// Determines whether a file represents a supported font, and, if so, what type of font it is.
    #[cfg(not(target_arch = "wasm32"))]
    fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError>;

    /// Determines whether a path points to a supported font, and, if so, what type of font it is.
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    fn analyze_path<P>(path: P) -> Result<FileType, FontLoadingError>
    where
        P: AsRef<Path>,
    {
        <Self as Loader>::analyze_file(&mut File::open(path)?)
    }

    /// Returns the wrapped native font handle.
    fn native_font(&self) -> Self::NativeFont;

    /// Returns the PostScript name of the font. This should be globally unique.
    fn postscript_name(&self) -> Option<String>;

    /// Returns the full name of the font (also known as "display name" on macOS).
    fn full_name(&self) -> String;

    /// Returns the name of the font family.
    fn family_name(&self) -> String;

    /// Returns true if and only if the font is monospace (fixed-width).
    fn is_monospace(&self) -> bool;

    /// Returns the values of various font properties, corresponding to those defined in CSS.
    fn properties(&self) -> Properties;

    /// Returns the number of glyphs in the font.
    ///
    /// Glyph IDs range from 0 inclusive to this value exclusive.
    fn glyph_count(&self) -> u32;

    /// Returns the usual glyph ID for a Unicode character.
    ///
    /// Be careful with this function; typographically correct character-to-glyph mapping must be
    /// done using a *shaper* such as HarfBuzz. This function is only useful for best-effort simple
    /// use cases like "what does character X look like on its own".
    fn glyph_for_char(&self, character: char) -> Option<u32>;

    /// Returns the glyph ID for the specified glyph name.
    #[inline]
    fn glyph_by_name(&self, _name: &str) -> Option<u32> {
        warn!("unimplemented");
        None
    }

    /// Sends the vector path for a glyph to a path builder.
    ///
    /// If `hinting_mode` is not None, this function performs grid-fitting as requested before
    /// sending the hinding outlines to the builder.
    ///
    /// TODO(pcwalton): What should we do for bitmap glyphs?
    fn outline<B>(
        &self,
        glyph_id: u32,
        hinting_mode: HintingOptions,
        path_builder: &mut B,
    ) -> Result<(), GlyphLoadingError>
    where
        B: PathBuilder;

    /// Returns the boundaries of a glyph in font units.
    fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError>;

    /// Returns the distance from the origin of the glyph with the given ID to the next, in font
    /// units.
    fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError>;

    /// Returns the amount that the given glyph should be displaced from the origin.
    fn origin(&self, glyph_id: u32) -> Result<Point2D<f32>, GlyphLoadingError>;

    /// Retrieves various metrics that apply to the entire font.
    fn metrics(&self) -> Metrics;

    /// Returns a handle to this font, if possible.
    ///
    /// This is useful if you want to open the font with a different loader.
    fn handle(&self) -> Option<Handle> {
        // FIXME(pcwalton): This doesn't handle font collections!
        self.copy_font_data()
            .map(|font_data| Handle::from_memory(font_data, 0))
    }

    /// Attempts to return the raw font data (contents of the font file).
    ///
    /// If this font is a member of a collection, this function returns the data for the entire
    /// collection.
    fn copy_font_data(&self) -> Option<Arc<Vec<u8>>>;

    /// Returns true if and only if the font loader can perform hinting in the requested way.
    ///
    /// Some APIs support only rasterizing glyphs with hinting, not retriving hinted outlines. If
    /// `for_rasterization` is false, this function returns true if and only if the loader supports
    /// retrieval of hinted *outlines*. If `for_rasterization` is true, this function returns true
    /// if and only if the loader supports *rasterizing* hinted glyphs.
    fn supports_hinting_options(
        &self,
        hinting_options: HintingOptions,
        for_rasterization: bool,
    ) -> bool;

    /// Returns the pixel boundaries that the glyph will take up when rendered using this loader's
    /// rasterizer at the given `point_size`, `transform` and `origin`. `origin` is not transformed
    /// by `transform`.
    fn raster_bounds(
        &self,
        glyph_id: u32,
        point_size: f32,
        transform: &FontTransform,
        origin: &Point2D<f32>,
        _: HintingOptions,
        _: RasterizationOptions,
    ) -> Result<Rect<i32>, GlyphLoadingError> {
        let typographic_bounds = self.typographic_bounds(glyph_id)?;
        let typographic_raster_bounds =
            typographic_bounds * point_size / self.metrics().units_per_em as f32;
        let transform: Transform2D<f32> = Transform2D::column_major(
            transform.scale_x,
            transform.skew_x,
            origin.x,
            transform.skew_y,
            transform.scale_y,
            origin.y,
        );
        Ok(transform
            .transform_rect(&typographic_raster_bounds)
            .round_out()
            .to_i32())
    }

    /// Rasterizes a glyph to a canvas with the given size and origin.
    ///
    /// Format conversion will be performed if the canvas format does not match the rasterization
    /// options. For example, if bilevel (black and white) rendering is requested to an RGBA
    /// surface, this function will automatically convert the 1-bit raster image to the 32-bit
    /// format of the canvas. Note that this may result in a performance penalty, depending on the
    /// loader.
    ///
    /// If `hinting_options` is not None, the requested grid fitting is performed.
    /// `origin` is not transformed by `transform`.
    fn rasterize_glyph(
        &self,
        canvas: &mut Canvas,
        glyph_id: u32,
        point_size: f32,
        transform: &FontTransform,
        origin: &Point2D<f32>,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<(), GlyphLoadingError>;

    /// Get font fallback results for the given text and locale.
    ///
    /// The `locale` argument is a language tag such as `"en-US"` or `"zh-Hans-CN"`.
    fn get_fallbacks(&self, text: &str, locale: &str) -> FallbackResult<Self>;
}

/// The result of a fallback query.
#[derive(Debug)]
pub struct FallbackResult<Font> {
    /// A list of fallback fonts.
    pub fonts: Vec<FallbackFont<Font>>,
    /// The fallback list is valid for this slice of the given text.
    pub valid_len: usize,
}

/// A single font record for a fallback query result.
#[derive(Debug)]
pub struct FallbackFont<Font> {
    /// The font.
    pub font: Font,
    /// A scale factor that should be applied to the fallback font.
    pub scale: f32,
    // TODO: add font simulation data
}
