// font-kit/src/loaders/directwrite.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A loader that uses the Windows DirectWrite API to load and rasterize fonts.

use dwrote::CustomFontCollectionLoaderImpl;
use dwrote::Font as DWriteFont;
use dwrote::FontCollection as DWriteFontCollection;
use dwrote::FontFace as DWriteFontFace;
use dwrote::FontFallback as DWriteFontFallback;
use dwrote::FontFile as DWriteFontFile;
use dwrote::FontStyle as DWriteFontStyle;
use dwrote::GlyphOffset as DWriteGlyphOffset;
use dwrote::GlyphRunAnalysis as DWriteGlyphRunAnalysis;
use dwrote::InformationalStringId as DWriteInformationalStringId;
use dwrote::{DWRITE_TEXTURE_ALIASED_1x1, DWRITE_RENDERING_MODE_NATURAL};
use dwrote::{DWRITE_TEXTURE_CLEARTYPE_3x1, OutlineBuilder};
use dwrote::{DWRITE_GLYPH_RUN, DWRITE_MEASURING_MODE_NATURAL, DWRITE_RENDERING_MODE_ALIASED};
use euclid::default::{Point2D, Rect, Size2D, Vector2D};
use euclid::point2;
use lyon_path::builder::PathBuilder;
use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use winapi::shared::minwindef::{FALSE, MAX_PATH};
use winapi::um::dwrite::{
    DWRITE_NUMBER_SUBSTITUTION_METHOD_NONE, DWRITE_READING_DIRECTION,
    DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
};
use winapi::um::fileapi;

use crate::canvas::{Canvas, Format, RasterizationOptions};
use crate::error::{FontLoadingError, GlyphLoadingError};
use crate::file_type::FileType;
use crate::handle::Handle;
use crate::hinting::HintingOptions;
use crate::loader::{FallbackFont, FallbackResult, FontTransform, Loader};
use crate::metrics::Metrics;
use crate::properties::{Properties, Stretch, Style, Weight};

const ERROR_BOUND: f32 = 0.0001;

/// DirectWrite's representation of a font.
pub struct NativeFont {
    /// The native DirectWrite font object.
    pub dwrite_font: DWriteFont,
    /// The native DirectWrite font face object.
    pub dwrite_font_face: DWriteFontFace,
}

/// A loader that uses the Windows DirectWrite API to load and rasterize fonts.
pub struct Font {
    dwrite_font: DWriteFont,
    dwrite_font_face: DWriteFontFace,
    cached_data: Mutex<Option<Arc<Vec<u8>>>>,
}

struct MyTextAnalysisSource {
    text_utf16_len: u32,
    locale: String,
}

impl dwrote::TextAnalysisSourceMethods for MyTextAnalysisSource {
    fn get_locale_name<'a>(&'a self, text_pos: u32) -> (Cow<'a, str>, u32) {
        (self.locale.as_str().into(), self.text_utf16_len - text_pos)
    }

    fn get_paragraph_reading_direction(&self) -> DWRITE_READING_DIRECTION {
        DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
    }
}

impl Font {
    fn from_dwrite_font_file(
        font_file: DWriteFontFile,
        mut font_index: u32,
        font_data: Option<Arc<Vec<u8>>>,
    ) -> Result<Font, FontLoadingError> {
        let collection_loader = CustomFontCollectionLoaderImpl::new(&[font_file.clone()]);
        let collection = DWriteFontCollection::from_loader(collection_loader);
        let families = collection.families_iter();
        for family in families {
            for family_font_index in 0..family.get_font_count() {
                if font_index > 0 {
                    font_index -= 1;
                    continue;
                }
                let dwrite_font = family.get_font(family_font_index);
                let dwrite_font_face = dwrite_font.create_font_face();
                return Ok(Font {
                    dwrite_font,
                    dwrite_font_face,
                    cached_data: Mutex::new(font_data),
                });
            }
        }
        Err(FontLoadingError::NoSuchFontInCollection)
    }

    /// Loads a font from raw font data (the contents of a `.ttf`/`.otf`/etc. file).
    ///
    /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index
    /// of the font to load from it. If the data represents a single font, pass 0 for `font_index`.
    pub fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Font, FontLoadingError> {
        let font_file =
            DWriteFontFile::new_from_data(font_data.clone()).ok_or(FontLoadingError::Parse)?;
        Font::from_dwrite_font_file(font_file, font_index, Some(font_data))
    }

    /// Loads a font from a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    pub fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        unsafe {
            let mut path = vec![0; MAX_PATH + 1];
            let path_len = fileapi::GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                path.as_mut_ptr(),
                path.len() as u32 - 1,
                0,
            );
            if path_len == 0 {
                return Err(FontLoadingError::Io(io::Error::last_os_error()));
            }
            path.truncate(path_len as usize);
            Font::from_path(PathBuf::from(OsString::from_wide(&path)), font_index)
        }
    }

    /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    #[inline]
    pub fn from_path<P: AsRef<Path>>(path: P, font_index: u32) -> Result<Font, FontLoadingError> {
        let font_file = DWriteFontFile::new_from_path(path).ok_or(FontLoadingError::Parse)?;
        Font::from_dwrite_font_file(font_file, font_index, None)
    }

    /// Creates a font from a native API handle.
    #[inline]
    pub unsafe fn from_native_font(native_font: NativeFont) -> Font {
        Font {
            dwrite_font: native_font.dwrite_font,
            dwrite_font_face: native_font.dwrite_font_face,
            cached_data: Mutex::new(None),
        }
    }

    /// Loads the font pointed to by a handle.
    #[inline]
    pub fn from_handle(handle: &Handle) -> Result<Self, FontLoadingError> {
        <Self as Loader>::from_handle(handle)
    }

    /// Determines whether a blob of raw font data represents a supported font, and, if so, what
    /// type of font it is.
    pub fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<FileType, FontLoadingError> {
        match DWriteFontFile::analyze_data(font_data) {
            0 => Err(FontLoadingError::Parse),
            1 => Ok(FileType::Single),
            font_count => Ok(FileType::Collection(font_count)),
        }
    }

    /// Determines whether a file represents a supported font, and, if so, what type of font it is.
    pub fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError> {
        let mut font_data = vec![];
        file.seek(SeekFrom::Start(0))
            .map_err(FontLoadingError::Io)?;
        match file.read_to_end(&mut font_data) {
            Err(io_error) => Err(FontLoadingError::Io(io_error)),
            Ok(_) => Font::analyze_bytes(Arc::new(font_data)),
        }
    }

    /// Returns the wrapped native font handle.
    pub fn native_font(&self) -> NativeFont {
        NativeFont {
            dwrite_font: self.dwrite_font.clone(),
            dwrite_font_face: self.dwrite_font_face.clone(),
        }
    }

    /// Determines whether a path points to a supported font, and, if so, what type of font it is.
    #[inline]
    pub fn analyze_path<P: AsRef<Path>>(path: P) -> Result<FileType, FontLoadingError> {
        <Self as Loader>::analyze_path(path)
    }

    /// Returns the PostScript name of the font. This should be globally unique.
    #[inline]
    pub fn postscript_name(&self) -> Option<String> {
        let dwrite_font = &self.dwrite_font;
        dwrite_font.informational_string(DWriteInformationalStringId::PostscriptName)
    }

    /// Returns the full name of the font (also known as "display name" on macOS).
    #[inline]
    pub fn full_name(&self) -> String {
        let dwrite_font = &self.dwrite_font;
        dwrite_font
            .informational_string(DWriteInformationalStringId::FullName)
            .unwrap_or_else(|| dwrite_font.family_name())
    }

    /// Returns the name of the font family.
    #[inline]
    pub fn family_name(&self) -> String {
        self.dwrite_font.family_name()
    }

    /// Returns true if and only if the font is monospace (fixed-width).
    #[inline]
    pub fn is_monospace(&self) -> bool {
        self.dwrite_font.is_monospace().unwrap_or(false)
    }

    /// Returns the values of various font properties, corresponding to those defined in CSS.
    pub fn properties(&self) -> Properties {
        let dwrite_font = &self.dwrite_font;
        Properties {
            style: style_for_dwrite_style(dwrite_font.style()),
            stretch: Stretch(Stretch::MAPPING[(dwrite_font.stretch() as usize) - 1]),
            weight: Weight(dwrite_font.weight().to_u32() as f32),
        }
    }

    /// Returns the usual glyph ID for a Unicode character.
    ///
    /// Be careful with this function; typographically correct character-to-glyph mapping must be
    /// done using a *shaper* such as HarfBuzz. This function is only useful for best-effort simple
    /// use cases like "what does character X look like on its own".
    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        let chars = [character as u32];
        self.dwrite_font_face
            .get_glyph_indices(&chars)
            .into_iter()
            .next()
            .map(|g| g as u32)
    }

    /// Returns the number of glyphs in the font.
    ///
    /// Glyph IDs range from 0 inclusive to this value exclusive.
    #[inline]
    pub fn glyph_count(&self) -> u32 {
        self.dwrite_font_face.get_glyph_count() as u32
    }

    /// Sends the vector path for a glyph to a path builder.
    ///
    /// If `hinting_mode` is not None, this function performs grid-fitting as requested before
    /// sending the hinding outlines to the builder.
    ///
    /// TODO(pcwalton): What should we do for bitmap glyphs?
    pub fn outline<B>(
        &self,
        glyph_id: u32,
        _: HintingOptions,
        path_builder: &mut B,
    ) -> Result<(), GlyphLoadingError>
    where
        B: PathBuilder,
    {
        let outline_buffer = OutlineBuffer::new();
        self.dwrite_font_face.get_glyph_run_outline(
            self.metrics().units_per_em as f32,
            &[glyph_id as u16],
            None,
            None,
            false,
            false,
            Box::new(outline_buffer.clone()),
        );
        outline_buffer.flush(path_builder);
        Ok(())
    }

    /// Returns the boundaries of a glyph in font units.
    pub fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError> {
        let metrics = self
            .dwrite_font_face
            .get_design_glyph_metrics(&[glyph_id as u16], false);

        let metrics = &metrics[0];
        let advance_width = metrics.advanceWidth as i32;
        let advance_height = metrics.advanceHeight as i32;
        let left_side_bearing = metrics.leftSideBearing as i32;
        let right_side_bearing = metrics.rightSideBearing as i32;
        let top_side_bearing = metrics.topSideBearing as i32;
        let bottom_side_bearing = metrics.bottomSideBearing as i32;
        let vertical_origin_y = metrics.verticalOriginY as i32;

        let y_offset = vertical_origin_y + bottom_side_bearing - advance_height;
        let width = advance_width - (left_side_bearing + right_side_bearing);
        let height = advance_height - (top_side_bearing + bottom_side_bearing);

        Ok(Rect::new(
            Point2D::new(left_side_bearing as f32, y_offset as f32),
            Size2D::new(width as f32, height as f32),
        ))
    }

    /// Returns the distance from the origin of the glyph with the given ID to the next, in font
    /// units.
    pub fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError> {
        let metrics = self
            .dwrite_font_face
            .get_design_glyph_metrics(&[glyph_id as u16], false);
        let metrics = &metrics[0];
        Ok(Vector2D::new(metrics.advanceWidth as f32, 0.0))
    }

    /// Returns the amount that the given glyph should be displaced from the origin.
    pub fn origin(&self, glyph: u32) -> Result<Point2D<f32>, GlyphLoadingError> {
        let metrics = self
            .dwrite_font_face
            .get_design_glyph_metrics(&[glyph as u16], false);
        Ok(Point2D::new(
            metrics[0].leftSideBearing as f32,
            (metrics[0].verticalOriginY + metrics[0].bottomSideBearing) as f32,
        ))
    }

    /// Retrieves various metrics that apply to the entire font.
    pub fn metrics(&self) -> Metrics {
        let dwrite_font = &self.dwrite_font;
        let dwrite_metrics = dwrite_font.metrics();
        Metrics {
            units_per_em: dwrite_metrics.designUnitsPerEm as u32,
            ascent: dwrite_metrics.ascent as f32,
            descent: -(dwrite_metrics.descent as f32),
            line_gap: dwrite_metrics.lineGap as f32,
            cap_height: dwrite_metrics.capHeight as f32,
            x_height: dwrite_metrics.xHeight as f32,
            underline_position: dwrite_metrics.underlinePosition as f32,
            underline_thickness: dwrite_metrics.underlineThickness as f32,
        }
    }

    /// Returns a handle to this font, if possible.
    ///
    /// This is useful if you want to open the font with a different loader.
    #[inline]
    pub fn handle(&self) -> Option<Handle> {
        <Self as Loader>::handle(self)
    }

    /// Attempts to return the raw font data (contents of the font file).
    ///
    /// If this font is a member of a collection, this function returns the data for the entire
    /// collection.
    pub fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        let mut font_data = self.cached_data.lock().unwrap();
        if font_data.is_none() {
            let files = self.dwrite_font_face.get_files();
            // FIXME(pcwalton): Is this right? When can a font have multiple files?
            if let Some(file) = files.get(0) {
                *font_data = Some(Arc::new(file.get_font_file_bytes()))
            }
        }
        (*font_data).clone()
    }

    /// Returns the pixel boundaries that the glyph will take up when rendered using this loader's
    /// rasterizer at the given size and origin.
    #[inline]
    pub fn raster_bounds(
        &self,
        glyph_id: u32,
        point_size: f32,
        transform: &FontTransform,
        origin: &Point2D<f32>,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<Rect<i32>, GlyphLoadingError> {
        let dwrite_analysis = self.build_glyph_analysis(
            glyph_id,
            point_size,
            transform,
            origin,
            hinting_options,
            rasterization_options,
        )?;

        let texture_type = match rasterization_options {
            RasterizationOptions::Bilevel => DWRITE_TEXTURE_ALIASED_1x1,
            RasterizationOptions::GrayscaleAa | RasterizationOptions::SubpixelAa => {
                DWRITE_TEXTURE_CLEARTYPE_3x1
            }
        };

        let texture_bounds = dwrite_analysis.get_alpha_texture_bounds(texture_type)?;
        let texture_width = texture_bounds.right - texture_bounds.left;
        let texture_height = texture_bounds.bottom - texture_bounds.top;

        Ok(Rect::new(
            Point2D::new(texture_bounds.left, -texture_height - texture_bounds.top),
            Size2D::new(texture_width, texture_height).to_i32(),
        ))
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
    pub fn rasterize_glyph(
        &self,
        canvas: &mut Canvas,
        glyph_id: u32,
        point_size: f32,
        transform: &FontTransform,
        origin: &Point2D<f32>,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<(), GlyphLoadingError> {
        // TODO(pcwalton): This is woefully incomplete. See WebRender's code for a more complete
        // implementation.

        let dwrite_analysis = self.build_glyph_analysis(
            glyph_id,
            point_size,
            transform,
            origin,
            hinting_options,
            rasterization_options,
        )?;

        let texture_type = match rasterization_options {
            RasterizationOptions::Bilevel => DWRITE_TEXTURE_ALIASED_1x1,
            RasterizationOptions::GrayscaleAa | RasterizationOptions::SubpixelAa => {
                DWRITE_TEXTURE_CLEARTYPE_3x1
            }
        };

        // TODO(pcwalton): Avoid a copy in some cases by writing directly to the canvas.
        let texture_bounds = dwrite_analysis.get_alpha_texture_bounds(texture_type)?;
        let texture_width = texture_bounds.right - texture_bounds.left;
        let texture_height = texture_bounds.bottom - texture_bounds.top;

        // 'Returns an empty rectangle if there are no glyphs of the specified texture type.'
        // https://docs.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwriteglyphrunanalysis-getalphatexturebounds
        if texture_width == 0 || texture_height == 0 {
            return Ok(());
        }

        let texture_format = if texture_type == DWRITE_TEXTURE_ALIASED_1x1 {
            Format::A8
        } else {
            Format::Rgb24
        };
        let texture_bits_per_pixel = texture_format.bits_per_pixel();
        let texture_bytes_per_pixel = texture_bits_per_pixel as usize / 8;
        let texture_size = Size2D::new(texture_width, texture_height).to_u32();
        let texture_stride = texture_width as usize * texture_bytes_per_pixel;

        let mut texture_bytes =
            dwrite_analysis.create_alpha_texture(texture_type, texture_bounds)?;
        canvas.blit_from(
            point2(texture_bounds.left, texture_bounds.top),
            &mut texture_bytes,
            &texture_size,
            texture_stride,
            texture_format,
        );

        Ok(())
    }

    /// Returns true if and only if the font loader can perform hinting in the requested way.
    ///
    /// Some APIs support only rasterizing glyphs with hinting, not retriving hinted outlines. If
    /// `for_rasterization` is false, this function returns true if and only if the loader supports
    /// retrieval of hinted *outlines*. If `for_rasterization` is true, this function returns true
    /// if and only if the loader supports *rasterizing* hinted glyphs.
    pub fn supports_hinting_options(
        &self,
        hinting_options: HintingOptions,
        for_rasterization: bool,
    ) -> bool {
        match (hinting_options, for_rasterization) {
            (HintingOptions::None, _)
            | (HintingOptions::Vertical(_), true)
            | (HintingOptions::VerticalSubpixel(_), true) => true,
            (HintingOptions::Vertical(_), false)
            | (HintingOptions::VerticalSubpixel(_), false)
            | (HintingOptions::Full(_), _) => false,
        }
    }

    fn build_glyph_analysis(
        &self,
        glyph_id: u32,
        point_size: f32,
        transform: &FontTransform,
        origin: &Point2D<f32>,
        _hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<DWriteGlyphRunAnalysis, GlyphLoadingError> {
        unsafe {
            let glyph_id = glyph_id as u16;
            let advance = 0.0;
            let offset = DWriteGlyphOffset {
                advanceOffset: 0.0,
                ascenderOffset: 0.0,
            };
            let glyph_run = DWRITE_GLYPH_RUN {
                fontFace: self.dwrite_font_face.as_ptr(),
                fontEmSize: point_size,
                glyphCount: 1,
                glyphIndices: &glyph_id,
                glyphAdvances: &advance,
                glyphOffsets: &offset,
                isSideways: FALSE,
                bidiLevel: 0,
            };

            let rendering_mode = match rasterization_options {
                RasterizationOptions::Bilevel => DWRITE_RENDERING_MODE_ALIASED,
                RasterizationOptions::GrayscaleAa | RasterizationOptions::SubpixelAa => {
                    DWRITE_RENDERING_MODE_NATURAL
                }
            };

            Ok(DWriteGlyphRunAnalysis::create(
                &glyph_run,
                1.0,
                Some(dwrote::DWRITE_MATRIX {
                    m11: transform.scale_x,
                    m12: transform.skew_y,
                    m21: transform.skew_x,
                    m22: transform.scale_y,
                    dx: origin.x,
                    dy: origin.y,
                }),
                rendering_mode,
                DWRITE_MEASURING_MODE_NATURAL,
                0.0,
                0.0,
            )?)
        }
    }

    /// Get font fallback results for the given text and locale.
    ///
    /// The `locale` argument is a language tag such as `"en-US"` or `"zh-Hans-CN"`.
    ///
    /// Note: on Windows 10, the result is a single font.
    fn get_fallbacks(&self, text: &str, locale: &str) -> FallbackResult<Font> {
        let sys_fallback = DWriteFontFallback::get_system_fallback();
        if sys_fallback.is_none() {
            unimplemented!("Need Windows 7 method for font fallbacks")
        }
        let text_utf16: Vec<u16> = text.encode_utf16().collect();
        let text_utf16_len = text_utf16.len() as u32;
        let number_subst =
            dwrote::NumberSubstitution::new(DWRITE_NUMBER_SUBSTITUTION_METHOD_NONE, locale, true);
        let text_analysis_source = MyTextAnalysisSource {
            text_utf16_len,
            locale: locale.to_owned(),
        };
        let text_analysis = dwrote::TextAnalysisSource::from_text_and_number_subst(
            Box::new(text_analysis_source),
            text_utf16,
            number_subst,
        );
        let sys_fallback = sys_fallback.unwrap();
        // TODO: I think the MapCharacters can take a null pointer, update
        // dwrote to accept an optional collection. This appears to be what
        // blink does.
        let collection = DWriteFontCollection::get_system(false);
        let fallback_result = sys_fallback.map_characters(
            &text_analysis,
            0,
            text_utf16_len,
            &collection,
            Some(&self.dwrite_font.family_name()),
            self.dwrite_font.weight(),
            self.dwrite_font.style(),
            self.dwrite_font.stretch(),
        );
        let valid_len = convert_len_utf16_to_utf8(text, fallback_result.mapped_length);
        //let face = fallback_result.mapped_font.
        let fonts = if let Some(dwrite_font) = fallback_result.mapped_font {
            let dwrite_font_face = dwrite_font.create_font_face();
            let font = Font {
                dwrite_font,
                dwrite_font_face,
                cached_data: Mutex::new(None),
            };
            let fallback_font = FallbackFont {
                font,
                scale: fallback_result.scale,
            };
            vec![fallback_font]
        } else {
            vec![]
        };
        FallbackResult { fonts, valid_len }
    }
}

// There might well be a more efficient impl that doesn't fully decode the text,
// just looks at the utf-8 bytes.
fn convert_len_utf16_to_utf8(text: &str, len_utf16: usize) -> usize {
    let mut l_utf8 = 0;
    let mut l_utf16 = 0;
    let mut chars = text.chars();
    while l_utf16 < len_utf16 {
        if let Some(c) = chars.next() {
            l_utf8 += c.len_utf8();
            l_utf16 += c.len_utf16();
        } else {
            break;
        }
    }
    l_utf8
}

impl Clone for Font {
    #[inline]
    fn clone(&self) -> Font {
        Font {
            dwrite_font: self.dwrite_font.clone(),
            dwrite_font_face: self.dwrite_font_face.clone(),
            cached_data: Mutex::new((*self.cached_data.lock().unwrap()).clone()),
        }
    }
}

impl Debug for Font {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        self.family_name().fmt(fmt)
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
    fn glyph_count(&self) -> u32 {
        self.glyph_count()
    }

    #[inline]
    fn outline<B>(
        &self,
        glyph_id: u32,
        hinting: HintingOptions,
        path_builder: &mut B,
    ) -> Result<(), GlyphLoadingError>
    where
        B: PathBuilder,
    {
        self.outline(glyph_id, hinting, path_builder)
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
    fn origin(&self, origin: u32) -> Result<Point2D<f32>, GlyphLoadingError> {
        self.origin(origin)
    }

    #[inline]
    fn metrics(&self) -> Metrics {
        self.metrics()
    }

    #[inline]
    fn supports_hinting_options(
        &self,
        hinting_options: HintingOptions,
        for_rasterization: bool,
    ) -> bool {
        self.supports_hinting_options(hinting_options, for_rasterization)
    }

    #[inline]
    fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        self.copy_font_data()
    }

    #[inline]
    fn rasterize_glyph(
        &self,
        canvas: &mut Canvas,
        glyph_id: u32,
        point_size: f32,
        transform: &FontTransform,
        origin: &Point2D<f32>,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<(), GlyphLoadingError> {
        self.rasterize_glyph(
            canvas,
            glyph_id,
            point_size,
            transform,
            origin,
            hinting_options,
            rasterization_options,
        )
    }

    #[inline]
    fn get_fallbacks(&self, text: &str, locale: &str) -> FallbackResult<Self> {
        self.get_fallbacks(text, locale)
    }
}

enum Event {
    MoveTo(Point2D<f32>),
    LineTo(Point2D<f32>),
    QuadraticTo {
        ctrl: Point2D<f32>,
        point: Point2D<f32>,
    },
    CubicTo {
        ctrl0: Point2D<f32>,
        ctrl1: Point2D<f32>,
        point: Point2D<f32>,
    },
    Close,
}

#[derive(Clone)]
struct OutlineBuffer {
    path_events: Arc<Mutex<Vec<Event>>>,
}

impl OutlineBuffer {
    fn new() -> OutlineBuffer {
        OutlineBuffer {
            path_events: Arc::new(Mutex::new(vec![])),
        }
    }

    fn flush<B>(&self, builder: &mut B)
    where
        B: PathBuilder,
    {
        let mut path_events = self.path_events.lock().unwrap();
        for event in path_events.drain(..) {
            match event {
                Event::MoveTo(p) => builder.move_to(p),
                Event::LineTo(p) => builder.line_to(p),
                Event::QuadraticTo { ctrl, point } => {
                    builder.quadratic_bezier_to(ctrl, point);
                }
                Event::CubicTo {
                    ctrl0,
                    ctrl1,
                    point,
                } => {
                    builder.cubic_bezier_to(ctrl0, ctrl1, point);
                }
                Event::Close => builder.close(),
            }
        }
    }
}

impl OutlineBuilder for OutlineBuffer {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path_events
            .lock()
            .unwrap()
            .push(Event::MoveTo(Point2D::new(x, -y)))
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path_events
            .lock()
            .unwrap()
            .push(Event::LineTo(Point2D::new(x, -y)))
    }

    fn curve_to(&mut self, cp0x: f32, cp0y: f32, cp1x: f32, cp1y: f32, x: f32, y: f32) {
        let (ctrl0, ctrl1) = (Point2D::new(cp0x, -cp0y), Point2D::new(cp1x, -cp1y));
        let to = Point2D::new(x, -y);

        // This might be a degree-elevated quadratic curve. Try to detect that.
        // See Sederberg § 2.6, "Distance Between Two Bézier Curves".
        let mut path_events = self.path_events.lock().unwrap();
        let from = match *path_events.last().unwrap() {
            Event::MoveTo(point)
            | Event::LineTo(point)
            | Event::QuadraticTo { point, .. }
            | Event::CubicTo { point, .. } => point,
            Event::Close => unreachable!(),
        };
        let approx_ctrl_0 = (ctrl0 * 3.0 - from) * 0.5;
        let approx_ctrl_1 = (ctrl1 * 3.0 - to) * 0.5;
        let delta_ctrl = (approx_ctrl_1 - approx_ctrl_0) * 2.0;
        let max_error = delta_ctrl.length() / 6.0;

        let event = if max_error < ERROR_BOUND {
            // Round to nearest 0.5.
            let mut approx_ctrl = approx_ctrl_0.lerp(approx_ctrl_1, 0.5).to_point();
            approx_ctrl = (approx_ctrl * 2.0).round() * 0.5;
            Event::QuadraticTo {
                ctrl: approx_ctrl,
                point: to,
            }
        } else {
            Event::CubicTo {
                ctrl0,
                ctrl1,
                point: to,
            }
        };
        path_events.push(event)
    }

    fn close(&mut self) {
        self.path_events.lock().unwrap().push(Event::Close)
    }
}

fn style_for_dwrite_style(style: DWriteFontStyle) -> Style {
    match style {
        DWriteFontStyle::Normal => Style::Normal,
        DWriteFontStyle::Oblique => Style::Oblique,
        DWriteFontStyle::Italic => Style::Italic,
    }
}
