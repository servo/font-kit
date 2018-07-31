// font-kit/src/loaders/core_text.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A loader that uses Apple's Core Text API to load and rasterize fonts.

use byteorder::{BigEndian, ReadBytesExt};
use core_graphics::base::{CGFloat, kCGImageAlphaPremultipliedLast};
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::{CGContext, CGTextDrawingMode};
use core_graphics::data_provider::{CGDataProvider, CustomData};
use core_graphics::font::{CGFont, CGGlyph};
use core_graphics::geometry::{CG_AFFINE_TRANSFORM_IDENTITY, CG_ZERO_POINT, CG_ZERO_SIZE, CGPoint};
use core_graphics::geometry::{CGRect, CGSize};
use core_graphics::path::CGPathElementType;
use core_text::font::CTFont;
use core_text::font_descriptor::{SymbolicTraitAccessors, TraitAccessors};
use core_text::font_descriptor::{kCTFontDefaultOrientation};
use core_text;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::builder::PathBuilder;
use memmap::{Mmap, MmapOptions};
use std::f32;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use canvas::{Canvas, Format, RasterizationOptions};
use error::{FontLoadingError, GlyphLoadingError};
use file_type::FileType;
use handle::Handle;
use hinting::HintingOptions;
use loader::Loader;
use metrics::Metrics;
use properties::{Properties, Stretch, Style, Weight};
use sources;
use utils;

const TTC_TAG: [u8; 4] = [b't', b't', b'c', b'f'];

#[allow(non_upper_case_globals)]
const kCGImageAlphaOnly: u32 = 7;

/// Core Text's representation of a font.
pub type NativeFont = CTFont;

/// A loader that uses Apple's Core Text API to load and rasterize fonts.
#[derive(Clone)]
pub struct Font {
    core_text_font: CTFont,
    font_data: FontData,
}

impl Font {
    /// Loads a font from raw font data (the contents of a `.ttf`/`.otf`/etc. file).
    ///
    /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index
    /// of the font to load from it. If the data represents a single font, pass 0 for `font_index`.
    pub fn from_bytes(mut font_data: Arc<Vec<u8>>, font_index: u32)
                      -> Result<Font, FontLoadingError> {
        // Sadly, there's no API to load OpenType collections on macOS, I don't believe…
        if font_is_collection(&**font_data) {
            let mut new_font_data = (*font_data).clone();
            try!(unpack_otc_font(&mut new_font_data, font_index));
            font_data = Arc::new(new_font_data);
        }

        let data_provider = CGDataProvider::from_buffer(font_data.clone());
        let core_graphics_font =
            try!(CGFont::from_data_provider(data_provider).map_err(|_| FontLoadingError::Parse));
        let core_text_font = core_text::font::new_from_CGFont(&core_graphics_font, 16.0);
        Ok(Font {
            core_text_font,
            font_data: FontData::Memory(font_data),
        })
    }

    /// Loads a font from a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    pub fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        try!(file.seek(SeekFrom::Start(0)));
        unsafe {
            let mut mmap = try!(MmapOptions::new().map_copy(file).map_err(FontLoadingError::Io));

            // Sadly, there's no API to load OpenType collections on macOS, I don't believe…
            if font_is_collection(&*mmap) {
                try!(unpack_otc_font(&mut *mmap, font_index));
            }

            let mmap = Arc::new(try!(mmap.make_read_only().map_err(FontLoadingError::Io)));
            let mmap_data = Box::new(Box::new(MmapData::new(mmap.clone())) as Box<CustomData>);
            let provider = CGDataProvider::from_custom_data(mmap_data);
            let core_graphics_font =
                try!(CGFont::from_data_provider(provider).map_err(|_| FontLoadingError::Parse));
            let core_text_font = core_text::font::new_from_CGFont(&core_graphics_font, 16.0);

            Ok(Font {
                core_text_font,
                font_data: FontData::File(mmap),
            })
        }
    }

    /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    #[inline]
    pub fn from_path<P>(path: P, font_index: u32) -> Result<Font, FontLoadingError>
                        where P: AsRef<Path> {
        <Font as Loader>::from_path(path, font_index)
    }

    /// Creates a font from a native API handle.
    pub unsafe fn from_native_font(core_text_font: NativeFont) -> Font {
        Font::from_core_text_font(core_text_font)
    }

    unsafe fn from_core_text_font(core_text_font: NativeFont) -> Font {
        let mut font_data = FontData::Unavailable;
        match core_text_font.url() {
            None => warn!("No URL found for Core Text font!"),
            Some(url) => {
                match url.to_path() {
                    Some(path) => {
                        match File::open(path) {
                            Ok(ref file) => {
                                match Mmap::map(file) {
                                    Ok(mmap) => font_data = FontData::File(Arc::new(mmap)),
                                    Err(_) => warn!("Could not map file for Core Text font!"),
                                }
                            }
                            Err(_) => warn!("Could not open file for Core Text font!"),
                        }
                    }
                    None => warn!("Could not convert URL from Core Text font to path!"),
                }
            }
        }

        Font {
            core_text_font,
            font_data,
        }
    }

    /// Loads the font pointed to by a handle.
    #[inline]
    pub fn from_handle(handle: &Handle) -> Result<Self, FontLoadingError> {
        <Self as Loader>::from_handle(handle)
    }

    /// Determines whether a file represents a supported font, and if so, what type of font it is.
    pub fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<FileType, FontLoadingError> {
        if let Ok(font_count) = read_number_of_fonts_from_otc_header(&font_data) {
            return Ok(FileType::Collection(font_count))
        }
        let data_provider = CGDataProvider::from_buffer(font_data);
        match CGFont::from_data_provider(data_provider) {
            Ok(_) => Ok(FileType::Single),
            Err(_) => Err(FontLoadingError::Parse),
        }
    }

    /// Determines whether a file represents a supported font, and if so, what type of font it is.
    pub fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError> {
        try!(file.seek(SeekFrom::Start(0)));
        unsafe {
            let mmap = try!(MmapOptions::new().map_copy(file).map_err(FontLoadingError::Io));
            let mmap = Arc::new(try!(mmap.make_read_only().map_err(FontLoadingError::Io)));
            if let Ok(font_count) = read_number_of_fonts_from_otc_header(&*mmap) {
                return Ok(FileType::Collection(font_count))
            }

            let mmap_data = Box::new(Box::new(MmapData::new(mmap.clone())) as Box<CustomData>);
            let provider = CGDataProvider::from_custom_data(mmap_data);
            match CGFont::from_data_provider(provider) {
                Ok(_) => Ok(FileType::Single),
                Err(_) => Err(FontLoadingError::Parse),
            }
        }
    }

    /// Determines whether a path points to a supported font, and if so, what type of font it is.
    #[inline]
    pub fn analyze_path<P>(path: P) -> Result<FileType, FontLoadingError> where P: AsRef<Path> {
        <Self as Loader>::analyze_path(path)
    }

    /// Returns the wrapped native font handle.
    #[inline]
    pub fn native_font(&self) -> NativeFont {
        self.core_text_font.clone()
    }

    /// Returns the PostScript name of the font. This should be globally unique.
    #[inline]
    pub fn postscript_name(&self) -> Option<String> {
        Some(self.core_text_font.postscript_name())
    }

    /// Returns the full name of the font (also known as "display name" on macOS).
    #[inline]
    pub fn full_name(&self) -> String {
        self.core_text_font.display_name()
    }

    /// Returns the name of the font family.
    #[inline]
    pub fn family_name(&self) -> String {
        self.core_text_font.family_name()
    }

    /// Returns the name of the font style, according to Core Text.
    ///
    /// NB: This function is only available on the Core Text backend.
    #[inline]
    pub fn style_name(&self) -> String {
        self.core_text_font.style_name()
    }

    /// Returns true if and only if the font is monospace (fixed-width).
    #[inline]
    pub fn is_monospace(&self) -> bool {
        self.core_text_font.symbolic_traits().is_monospace()
    }

    /// Returns the values of various font properties, corresponding to those defined in CSS.
    pub fn properties(&self) -> Properties {
        let symbolic_traits = self.core_text_font.symbolic_traits();
        let all_traits = self.core_text_font.all_traits();

        let style = if symbolic_traits.is_italic() {
            Style::Italic
        } else if all_traits.normalized_slant() > 0.0 {
            Style::Oblique
        } else {
            Style::Normal
        };

        let weight = core_text_to_css_font_weight(all_traits.normalized_weight() as f32);
        let stretch = core_text_width_to_css_stretchiness(all_traits.normalized_width() as f32);

        Properties {
            style,
            weight,
            stretch,
        }
    }

    /// Returns the usual glyph ID for a Unicode character.
    ///
    /// Be careful with this function; typographically correct character-to-glyph mapping must be
    /// done using a *shaper* such as HarfBuzz. This function is only useful for best-effort simple
    /// use cases like "what does character X look like on its own".
    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        unsafe {
            let (mut dest, mut src) = ([0, 0], [0, 0]);
            let src = character.encode_utf16(&mut src);
            self.core_text_font.get_glyphs_for_characters(src.as_ptr(), dest.as_mut_ptr(), 2);
            Some(dest[0] as u32)
        }
    }

    /// Sends the vector path for a glyph to a path builder.
    ///
    /// If `hinting_mode` is not None, this function performs grid-fitting as requested before
    /// sending the hinding outlines to the builder.
    ///
    /// TODO(pcwalton): What should we do for bitmap glyphs?
    pub fn outline<B>(&self, glyph_id: u32, _: HintingOptions, path_builder: &mut B)
                      -> Result<(), GlyphLoadingError>
                      where B: PathBuilder {
        let path = try!(self.core_text_font
                            .create_path_for_glyph(glyph_id as u16, &CG_AFFINE_TRANSFORM_IDENTITY)
                            .map_err(|_| GlyphLoadingError::NoSuchGlyph));
        let units_per_point = self.units_per_point() as f32;
        path.apply(&|element| {
            let points = element.points();
            match element.element_type {
                CGPathElementType::MoveToPoint => {
                    path_builder.move_to(points[0].to_euclid_point() * units_per_point)
                }
                CGPathElementType::AddLineToPoint => {
                    path_builder.line_to(points[0].to_euclid_point() * units_per_point)
                }
                CGPathElementType::AddQuadCurveToPoint => {
                    path_builder.quadratic_bezier_to(points[0].to_euclid_point() * units_per_point,
                                                     points[1].to_euclid_point() * units_per_point)
                }
                CGPathElementType::AddCurveToPoint => {
                    path_builder.cubic_bezier_to(points[0].to_euclid_point() * units_per_point,
                                                 points[1].to_euclid_point() * units_per_point,
                                                 points[2].to_euclid_point() * units_per_point)
                }
                CGPathElementType::CloseSubpath => path_builder.close(),
            }
        });
        Ok(())
    }

    /// Returns the boundaries of a glyph in font units.
    pub fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError> {
        let rect = self.core_text_font.get_bounding_rects_for_glyphs(kCTFontDefaultOrientation,
                                                                     &[glyph_id as u16]);
        let units_per_point = self.units_per_point();
        Ok(Rect::new(Point2D::new((rect.origin.x * units_per_point) as f32,
                                  (rect.origin.y * units_per_point) as f32),
                     Size2D::new((rect.size.width * units_per_point) as f32,
                                 (rect.size.height * units_per_point) as f32)))
    }

    /// Returns the distance from the origin of the glyph with the given ID to the next, in font
    /// units.
    pub fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError> {
        // FIXME(pcwalton): Apple's docs don't say what happens when the glyph is out of range!
        unsafe {
            let (glyph_id, mut advance) = (glyph_id as u16, CG_ZERO_SIZE);
            self.core_text_font
                .get_advances_for_glyphs(kCTFontDefaultOrientation, &glyph_id, &mut advance, 1);
            Ok(Vector2D::new((advance.width * self.units_per_point()) as f32,
                            (advance.height * self.units_per_point()) as f32))
        }
    }

    /// Returns the amount that the given glyph should be displaced from the origin.
    pub fn origin(&self, glyph_id: u32) -> Result<Point2D<f32>, GlyphLoadingError> {
        unsafe {
            // FIXME(pcwalton): Apple's docs don't say what happens when the glyph is out of range!
            let (glyph_id, mut translation) = (glyph_id as u16, CG_ZERO_SIZE);
            self.core_text_font.get_vertical_translations_for_glyphs(kCTFontDefaultOrientation,
                                                                    &glyph_id,
                                                                    &mut translation,
                                                                    1);
            Ok(Point2D::new((translation.width * self.units_per_point()) as f32,
                            (translation.height * self.units_per_point()) as f32))
        }
    }

    /// Retrieves various metrics that apply to the entire font.
    pub fn metrics(&self) -> Metrics {
        let units_per_em = self.core_text_font.units_per_em();
        let units_per_point = (units_per_em as f64) / self.core_text_font.pt_size();
        Metrics {
            units_per_em,
            ascent: (self.core_text_font.ascent() * units_per_point) as f32,
            descent: (-self.core_text_font.descent() * units_per_point) as f32,
            line_gap: (self.core_text_font.leading() * units_per_point) as f32,
            underline_position: (self.core_text_font.underline_position() *
                                 units_per_point) as f32,
            underline_thickness: (self.core_text_font.underline_thickness() *
                                  units_per_point) as f32,
            cap_height: (self.core_text_font.cap_height() * units_per_point) as f32,
            x_height: (self.core_text_font.x_height() * units_per_point) as f32,
        }
    }

    /// Attempts to return the raw font data (contents of the font file).
    ///
    /// If this font is a member of a collection, this function returns the data for the entire
    /// collection.
    pub fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        match self.font_data {
            FontData::Unavailable => None,
            FontData::File(ref file) => Some(Arc::new((*file).to_vec())),
            FontData::Memory(ref memory) => Some((*memory).clone()),
        }
    }

    /// Returns the pixel boundaries that the glyph will take up when rendered using this loader's
    /// rasterizer at the given size and origin.
    #[inline]
    pub fn raster_bounds(&self,
                         glyph_id: u32,
                         point_size: f32,
                         origin: &Point2D<f32>,
                         hinting_options: HintingOptions,
                         rasterization_options: RasterizationOptions)
                         -> Result<Rect<i32>, GlyphLoadingError> {
        <Self as Loader>::raster_bounds(self,
                                        glyph_id,
                                        point_size,
                                        origin,
                                        hinting_options,
                                        rasterization_options)
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
    ///
    /// TODO(pcwalton): This is woefully incomplete. See WebRender's code for a more complete
    /// implementation.
    pub fn rasterize_glyph(&self,
                           canvas: &mut Canvas,
                           glyph_id: u32,
                           point_size: f32,
                           origin: &Point2D<f32>,
                           hinting_options: HintingOptions,
                           rasterization_options: RasterizationOptions)
                           -> Result<(), GlyphLoadingError> {
        let (cg_color_space, cg_image_format) =
            match format_to_cg_color_space_and_image_format(canvas.format) {
                None => {
                    // Core Graphics doesn't support the requested image format. Allocate a
                    // temporary canvas, then perform color conversion.
                    //
                    // FIXME(pcwalton): Could improve this by only allocating a canvas with a tight
                    // bounding rect and blitting only that part.
                    let mut temp_canvas = Canvas::new(&canvas.size, Format::Rgba32);
                    try!(self.rasterize_glyph(&mut temp_canvas,
                                            glyph_id,
                                            point_size,
                                            origin,
                                            hinting_options,
                                            rasterization_options));
                    canvas.blit_from_canvas(&temp_canvas);
                    return Ok(());
                }
                Some(cg_color_space_and_format) => cg_color_space_and_format,
            };

        let core_graphics_context =
            CGContext::create_bitmap_context(Some(canvas.pixels.as_mut_ptr() as *mut _),
                                             canvas.size.width as usize,
                                             canvas.size.height as usize,
                                             canvas.format.bits_per_component() as usize,
                                             canvas.stride,
                                             &cg_color_space,
                                             cg_image_format);

        match canvas.format {
            Format::Rgba32 | Format::Rgb24 => {
                core_graphics_context.set_rgb_fill_color(0.0, 0.0, 0.0, 0.0);
            }
            Format::A8 => core_graphics_context.set_gray_fill_color(0.0, 0.0),
        }

        let core_graphics_size = CGSize::new(canvas.size.width as f64, canvas.size.height as f64);
        core_graphics_context.fill_rect(CGRect::new(&CG_ZERO_POINT, &core_graphics_size));

        match rasterization_options {
            RasterizationOptions::Bilevel => {
                core_graphics_context.set_allows_font_smoothing(false);
                core_graphics_context.set_should_smooth_fonts(false);
                core_graphics_context.set_should_antialias(false);
            }
            RasterizationOptions::GrayscaleAa | RasterizationOptions::SubpixelAa => {
                // FIXME(pcwalton): These shouldn't be handled the same!
                core_graphics_context.set_allows_font_smoothing(true);
                core_graphics_context.set_should_smooth_fonts(true);
                core_graphics_context.set_should_antialias(true);
            }
        }

        match canvas.format {
            Format::Rgba32 | Format::Rgb24 => {
                core_graphics_context.set_rgb_fill_color(1.0, 1.0, 1.0, 1.0);
            }
            Format::A8 => core_graphics_context.set_gray_fill_color(1.0, 1.0),
        }

        let origin = CGPoint::new(origin.x as CGFloat, origin.y as CGFloat);
        core_graphics_context.set_font(&self.core_text_font.copy_to_CGFont());
        core_graphics_context.set_font_size(point_size as CGFloat);
        core_graphics_context.set_text_drawing_mode(CGTextDrawingMode::CGTextFill);
        core_graphics_context.set_text_matrix(&CG_AFFINE_TRANSFORM_IDENTITY);
        core_graphics_context.show_glyphs_at_positions(&[glyph_id as CGGlyph], &[origin]);

        Ok(())
    }

    /// Returns true if and only if the font loader can perform hinting in the requested way.
    ///
    /// Some APIs support only rasterizing glyphs with hinting, not retriving hinted outlines. If
    /// `for_rasterization` is false, this function returns true if and only if the loader supports
    /// retrieval of hinted *outlines*. If `for_rasterization` is true, this function returns true
    /// if and only if the loader supports *rasterizing* hinted glyphs.
    #[inline]
    pub fn supports_hinting_options(&self, hinting_options: HintingOptions, _: bool) -> bool {
        match hinting_options {
            HintingOptions::None => true,
            HintingOptions::Vertical(..) |
            HintingOptions::VerticalSubpixel(..) |
            HintingOptions::Full(..) => false,
        }
    }

    #[inline]
    fn units_per_point(&self) -> f64 {
        (self.core_text_font.units_per_em() as f64) / self.core_text_font.pt_size()
    }
}

impl Loader for Font {
    type NativeFont = NativeFont;

    #[inline]
    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, FontLoadingError> {
        Font::from_bytes(font_data, font_index)
    }

    #[inline]
    fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        Font::from_file(file, font_index)
    }

    #[inline]
    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self {
        Font::from_native_font(native_font)
    }

    #[inline]
    fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<FileType, FontLoadingError> {
        Font::analyze_bytes(font_data)
    }

    #[inline]
    fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError> {
        Font::analyze_file(file)
    }

    #[inline]
    fn native_font(&self) -> Self::NativeFont {
        self.native_font()
    }

    #[inline]
    fn postscript_name(&self) -> Option<String> {
        self.postscript_name()
    }

    #[inline]
    fn full_name(&self) -> String {
        self.full_name()
    }

    #[inline]
    fn family_name(&self) -> String {
        self.family_name()
    }

    #[inline]
    fn is_monospace(&self) -> bool {
        self.is_monospace()
    }

    #[inline]
    fn properties(&self) -> Properties {
        self.properties()
    }

    #[inline]
    fn glyph_for_char(&self, character: char) -> Option<u32> {
        self.glyph_for_char(character)
    }

    #[inline]
    fn outline<B>(&self, glyph_id: u32, hinting_mode: HintingOptions, path_builder: &mut B)
                  -> Result<(), GlyphLoadingError>
                  where B: PathBuilder {
        self.outline(glyph_id, hinting_mode, path_builder)
    }

    #[inline]
    fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError> {
        self.typographic_bounds(glyph_id)
    }

    #[inline]
    fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError> {
        self.advance(glyph_id)
    }

    #[inline]
    fn origin(&self, glyph_id: u32) -> Result<Point2D<f32>, GlyphLoadingError> {
        self.origin(glyph_id)
    }

    #[inline]
    fn metrics(&self) -> Metrics {
        self.metrics()
    }

    #[inline]
    fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        self.copy_font_data()
    }

    #[inline]
    fn supports_hinting_options(&self, hinting_options: HintingOptions, for_rasterization: bool)
                                -> bool {
        self.supports_hinting_options(hinting_options, for_rasterization)
    }

    #[inline]
    fn rasterize_glyph(&self,
                       canvas: &mut Canvas,
                       glyph_id: u32,
                       point_size: f32,
                       origin: &Point2D<f32>,
                       hinting_options: HintingOptions,
                       rasterization_options: RasterizationOptions)
                       -> Result<(), GlyphLoadingError> {
        self.rasterize_glyph(canvas,
                             glyph_id,
                             point_size,
                             origin,
                             hinting_options,
                             rasterization_options)
    }
}

impl Debug for Font {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        self.full_name().fmt(fmt)
    }
}

#[derive(Clone)]
enum FontData {
    Unavailable,
    Memory(Arc<Vec<u8>>),
    File(Arc<Mmap>),
}

impl Deref for FontData {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            FontData::Unavailable => panic!("Font data unavailable!"),
            FontData::File(ref mmap) => &***mmap,
            FontData::Memory(ref data) => &***data,
        }
    }
}

trait CGPointExt {
    fn to_euclid_point(&self) -> Point2D<f32>;
}

impl CGPointExt for CGPoint {
    #[inline]
    fn to_euclid_point(&self) -> Point2D<f32> {
        Point2D::new(self.x as f32, self.y as f32)
    }
}

struct MmapData {
    mmap: Arc<Mmap>,
}

impl MmapData {
    fn new(mmap: Arc<Mmap>) -> MmapData {
        MmapData {
            mmap,
        }
    }
}

impl CustomData for MmapData {
    unsafe fn ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }
    unsafe fn len(&self) -> usize {
        self.mmap.len()
    }
}

fn core_text_to_css_font_weight(core_text_weight: f32) -> Weight {
    Weight(sources::core_text::piecewise_linear_find_index(
            core_text_weight,
            &sources::core_text::FONT_WEIGHT_MAPPING) * 100.0 + 100.0)
}

fn core_text_width_to_css_stretchiness(core_text_width: f32) -> Stretch {
    Stretch(sources::core_text::piecewise_linear_lookup((core_text_width + 1.0) * 4.0,
                                                        &Stretch::MAPPING))
}

fn font_is_collection(header: &[u8]) -> bool {
    header.len() >= 4 && header[0..4] == TTC_TAG
}

fn read_number_of_fonts_from_otc_header(header: &[u8]) -> Result<u32, FontLoadingError> {
    if !font_is_collection(header) {
        return Err(FontLoadingError::UnknownFormat)
    }
    Ok(try!((&header[8..]).read_u32::<BigEndian>()))
}

// Unpacks an OTC font "in-place".
fn unpack_otc_font(data: &mut [u8], font_index: u32) -> Result<(), FontLoadingError> {
    if font_index >= try!(read_number_of_fonts_from_otc_header(data)) {
        return Err(FontLoadingError::NoSuchFontInCollection)
    }

    let offset_table_pos_pos = 12 + 4 * font_index as usize;
    let offset_table_pos = try!((&data[offset_table_pos_pos..]).read_u32::<BigEndian>()) as usize;
    debug_assert!(utils::SFNT_VERSIONS.iter().any(|version| {
        data[offset_table_pos..(offset_table_pos + 4)] == *version
    }));
    let num_tables = try!((&data[(offset_table_pos + 4)..]).read_u16::<BigEndian>());

    // Must copy forward in order to avoid problems with overlapping memory.
    let offset_table_and_table_record_size = 12 + (num_tables as usize) * 16;
    for offset in 0..offset_table_and_table_record_size {
        data[offset] = data[offset_table_pos + offset]
    }

    Ok(())
}

// NB: This assumes little-endian, but that's true for all extant Apple hardware.
fn format_to_cg_color_space_and_image_format(format: Format) -> Option<(CGColorSpace, u32)> {
    match format {
        Format::Rgb24 => {
            // Unsupported by Core Graphics.
            None
        }
        Format::Rgba32 => {
            Some((CGColorSpace::create_device_rgb(), kCGImageAlphaPremultipliedLast))
        }
        Format::A8 => Some((CGColorSpace::create_device_gray(), kCGImageAlphaOnly)),
    }
}

#[cfg(test)]
mod test {
    use properties::{Stretch, Weight};

    #[test]
    fn test_core_text_to_css_font_weight() {
        // Exact matches
        assert_eq!(super::core_text_to_css_font_weight(-0.7), Weight(100.0));
        assert_eq!(super::core_text_to_css_font_weight(0.0), Weight(400.0));
        assert_eq!(super::core_text_to_css_font_weight(0.4), Weight(700.0));
        assert_eq!(super::core_text_to_css_font_weight(0.8), Weight(900.0));

        // Linear interpolation
        assert_eq!(super::core_text_to_css_font_weight(0.1), Weight(450.0));
    }

    #[test]
    fn test_core_text_to_css_font_stretch() {
        // Exact matches
        assert_eq!(super::core_text_width_to_css_stretchiness(0.0), Stretch(1.0));
        assert_eq!(super::core_text_width_to_css_stretchiness(-1.0), Stretch(0.5));
        assert_eq!(super::core_text_width_to_css_stretchiness(1.0), Stretch(2.0));

        // Linear interpolation
        assert_eq!(super::core_text_width_to_css_stretchiness(0.85), Stretch(1.7));
    }
}
