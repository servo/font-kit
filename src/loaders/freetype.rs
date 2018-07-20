// font-kit/src/loaders/freetype.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for font loading using FreeType.
//!
//! On macOS and Windows, the Cargo feature `loader-freetype` can be used to opt into this loader.
//! Because on these platforms FreeType can do everything the native loader can do and more, this
//! Cargo feature completely disables native font loading. In particular, this feature enables
//! support for retrieving hinted outlines.

use byteorder::{BigEndian, ReadBytesExt};
use canvas::{Canvas, Format, RasterizationOptions};
use euclid::{Point2D, Rect, Size2D, Vector2D};
use freetype::freetype::{FT_Byte, FT_Done_Face, FT_Error, FT_FACE_FLAG_FIXED_WIDTH, FT_Face};
use freetype::freetype::{FT_Get_Char_Index, FT_Get_Postscript_Name, FT_Get_Sfnt_Table};
use freetype::freetype::{FT_Init_FreeType, FT_LOAD_DEFAULT, FT_LOAD_NO_HINTING, FT_Library};
use freetype::freetype::{FT_Load_Glyph, FT_Long, FT_New_Memory_Face, FT_Reference_Face};
use freetype::freetype::{FT_Render_Glyph, FT_Render_Mode, FT_STYLE_FLAG_ITALIC, FT_Set_Char_Size};
use freetype::freetype::{FT_Set_Transform, FT_Sfnt_Tag, FT_UInt, FT_ULong, FT_UShort, FT_Vector};
use freetype::tt_os2::TT_OS2;
use lyon_path::builder::PathBuilder;
use memmap::Mmap;
use std::f32;
use std::ffi::CStr;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::iter;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::os::raw::{c_char, c_void};
use std::path::Path;
use std::ptr;
use std::slice;
use std::sync::Arc;

use descriptor::{FONT_STRETCH_MAPPING, Properties, Stretch, Style, Weight};
use error::{FontLoadingError, GlyphLoadingError};
use font::{Face, HintingOptions, Metrics, Type};
use handle::Handle;

const PS_DICT_FULL_NAME: u32 = 38;
const TT_NAME_ID_FULL_NAME: u16 = 4;

const TT_PLATFORM_APPLE_UNICODE: u16 = 0;

const FT_POINT_TAG_ON_CURVE: c_char = 0x01;
const FT_POINT_TAG_CUBIC_CONTROL: c_char = 0x02;

const FT_RENDER_MODE_NORMAL: u32 = 0;
const FT_RENDER_MODE_LIGHT: u32 = 1;
#[allow(dead_code)]
const FT_RENDER_MODE_MONO: u32 = 2;
const FT_RENDER_MODE_LCD: u32 = 3;

const FT_LOAD_TARGET_LIGHT: u32 = (FT_RENDER_MODE_LIGHT & 15) << 16;
const FT_LOAD_TARGET_LCD: u32 = (FT_RENDER_MODE_LCD & 15) << 16;
const FT_LOAD_TARGET_NORMAL: u32 = (FT_RENDER_MODE_NORMAL & 15) << 16;

const FT_PIXEL_MODE_MONO: u8 = 1;
const FT_PIXEL_MODE_GRAY: u8 = 2;
const FT_PIXEL_MODE_LCD: u8 = 5;
const FT_PIXEL_MODE_LCD_V: u8 = 6;

const OS2_FS_SELECTION_OBLIQUE: u16 = 1 << 9;

thread_local! {
    static FREETYPE_LIBRARY: FT_Library = {
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(FT_Init_FreeType(&mut library), 0);
            library
        }
    };
}

pub type NativeFont = FT_Face;

pub struct Font {
    freetype_face: FT_Face,
    font_data: FontData<'static>,
}

impl Font {
    pub fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Font, FontLoadingError> {
        FREETYPE_LIBRARY.with(|freetype_library| {
            unsafe {
                let mut freetype_face = ptr::null_mut();
                if FT_New_Memory_Face(*freetype_library,
                                      (*font_data).as_ptr(),
                                      font_data.len() as i64,
                                      font_index as FT_Long,
                                      &mut freetype_face) != 0 {
                    return Err(FontLoadingError::Parse)
                }

                setup_freetype_face(freetype_face);

                Ok(Font {
                    freetype_face,
                    font_data: FontData::Memory(font_data),
                })
            }
        })
    }

    pub fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        unsafe {
            let mmap = try!(Mmap::map(&file));
            FREETYPE_LIBRARY.with(|freetype_library| {
                let mut freetype_face = ptr::null_mut();
                if FT_New_Memory_Face(*freetype_library,
                                      (*mmap).as_ptr(),
                                      mmap.len() as i64,
                                      font_index as FT_Long,
                                      &mut freetype_face) != 0 {
                    return Err(FontLoadingError::Parse)
                }

                setup_freetype_face(freetype_face);

                Ok(Font {
                    freetype_face,
                    font_data: FontData::File(Arc::new(mmap)),
                })
            })
        }
    }

    #[inline]
    pub fn from_path<P>(path: P, font_index: u32) -> Result<Font, FontLoadingError>
                        where P: AsRef<Path> {
        // TODO(pcwalton): Perhaps use the native FreeType support for opening paths?
        <Font as Face>::from_path(path, font_index)
    }

    pub unsafe fn from_native_font(freetype_face: NativeFont) -> Font {
        // We make an in-memory copy of the underlying font data. This is because the native font
        // does not necessarily hold a strong reference to the memory backing it.
        const CHUNK_SIZE: usize = 4096;
        let mut font_data = vec![];
        loop {
            font_data.extend(iter::repeat(0).take(CHUNK_SIZE));
            let freetype_stream = (*freetype_face).stream;
            let n_read = ((*freetype_stream).read.unwrap())(freetype_stream,
                                                            font_data.len() as u64,
                                                            font_data.as_mut_ptr(),
                                                            CHUNK_SIZE as u64);
            if n_read < CHUNK_SIZE as u64 {
                break
            }
        }

        Font::from_bytes(Arc::new(font_data), (*freetype_face).face_index as u32).unwrap()
    }

    #[inline]
    pub fn from_handle(handle: &Handle) -> Result<Self, FontLoadingError> {
        <Self as Face>::from_handle(handle)
    }

    pub fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<Type, FontLoadingError> {
        FREETYPE_LIBRARY.with(|freetype_library| {
            unsafe {
                let mut freetype_face = ptr::null_mut();
                if FT_New_Memory_Face(*freetype_library,
                                      (*font_data).as_ptr(),
                                      font_data.len() as i64,
                                      0,
                                      &mut freetype_face) != 0 {
                    return Err(FontLoadingError::Parse)
                }

                let font_type = match (*freetype_face).num_faces {
                    1 => Type::Single,
                    num_faces => Type::Collection(num_faces as u32),
                };
                FT_Done_Face(freetype_face);
                Ok(font_type)
            }
        })
    }

    pub fn analyze_file(file: &mut File) -> Result<Type, FontLoadingError> {
        FREETYPE_LIBRARY.with(|freetype_library| {
            unsafe {
                let mmap = try!(Mmap::map(&file));
                let mut freetype_face = ptr::null_mut();
                if FT_New_Memory_Face(*freetype_library,
                                      (*mmap).as_ptr(),
                                      mmap.len() as i64,
                                      0,
                                      &mut freetype_face) != 0 {
                    return Err(FontLoadingError::Parse)
                }

                let font_type = match (*freetype_face).num_faces {
                    1 => Type::Single,
                    num_faces => Type::Collection(num_faces as u32),
                };
                FT_Done_Face(freetype_face);
                Ok(font_type)
            }
        })
    }

    #[inline]
    pub fn analyze_path<P>(path: P) -> Result<Type, FontLoadingError> where P: AsRef<Path> {
        <Self as Face>::analyze_path(path)
    }

    pub fn postscript_name(&self) -> String {
        unsafe {
            let postscript_name = FT_Get_Postscript_Name(self.freetype_face);
            CStr::from_ptr(postscript_name).to_str().unwrap().to_owned()
        }
    }

    pub fn full_name(&self) -> String {
        self.get_type_1_or_sfnt_name(PS_DICT_FULL_NAME, TT_NAME_ID_FULL_NAME)
            .unwrap_or_else(|| self.family_name())
    }

    pub fn family_name(&self) -> String {
        unsafe {
            CStr::from_ptr((*self.freetype_face).family_name).to_str().unwrap().to_owned()
        }
    }

    pub fn is_monospace(&self) -> bool {
        unsafe {
            (*self.freetype_face).face_flags & (FT_FACE_FLAG_FIXED_WIDTH as i64) != 0
        }
    }

    pub fn properties(&self) -> Properties {
        unsafe {
            let os2_table = self.get_os2_table();
            let style = match os2_table {
                Some(os2_table) if ((*os2_table).fsSelection & OS2_FS_SELECTION_OBLIQUE) != 0 => {
                    Style::Oblique
                }
                _ if ((*self.freetype_face).style_flags & (FT_STYLE_FLAG_ITALIC) as i64) != 0 => {
                    Style::Italic
                }
                _ => Style::Normal,
            };
            let stretch = match os2_table {
                Some(os2_table) if (*os2_table).usWidthClass > 0 => {
                    Stretch(FONT_STRETCH_MAPPING[((*os2_table).usWidthClass as usize) - 1])
                }
                _ => Stretch::NORMAL,
            };
            let weight = match os2_table {
                None => Weight::NORMAL,
                Some(os2_table) => Weight((*os2_table).usWeightClass as u32 as f32),
            };
            Properties {
                style,
                stretch,
                weight,
            }
        }
    }

    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        unsafe {
            Some(FT_Get_Char_Index(self.freetype_face, character as FT_ULong))
        }
    }

    pub fn outline<B>(&self, glyph_id: u32, hinting: HintingOptions, path_builder: &mut B)
                      -> Result<(), GlyphLoadingError>
                      where B: PathBuilder {
        unsafe {
            let load_flags = self.hinting_options_to_load_flags(hinting);

            let units_per_em = (*self.freetype_face).units_per_EM;
            let grid_fitting_size = hinting.grid_fitting_size();
            if let Some(size) = grid_fitting_size {
                assert_eq!(FT_Set_Char_Size(self.freetype_face,
                                            f32_to_ft_fixed_26_6(size),
                                            0,
                                            0,
                                            0),
                           0);
            }

            if FT_Load_Glyph(self.freetype_face, glyph_id, load_flags as i32) != 0 {
                return Err(GlyphLoadingError::NoSuchGlyph)
            }

            let outline = &(*(*self.freetype_face).glyph).outline;
            let contours = slice::from_raw_parts((*outline).contours,
                                                 (*outline).n_contours as usize);
            let point_positions = slice::from_raw_parts((*outline).points,
                                                        (*outline).n_points as usize);
            let point_tags = slice::from_raw_parts((*outline).tags, (*outline).n_points as usize);

            let mut current_point_index = 0;
            for &last_point_index_in_contour in contours {
                let last_point_index_in_contour = last_point_index_in_contour as usize;
                let (point, _) = get_point(&mut current_point_index,
                                           point_positions,
                                           point_tags,
                                           last_point_index_in_contour,
                                           grid_fitting_size,
                                           units_per_em);
                path_builder.move_to(point);
                while current_point_index <= last_point_index_in_contour {
                    let (point0, tag) = get_point(&mut current_point_index,
                                                  point_positions,
                                                  point_tags,
                                                  last_point_index_in_contour,
                                                  grid_fitting_size,
                                                  units_per_em);
                    if (tag & FT_POINT_TAG_ON_CURVE) != 0 {
                        path_builder.line_to(point0)
                    } else {
                        let (point1, _) = get_point(&mut current_point_index,
                                                    point_positions,
                                                    point_tags,
                                                    last_point_index_in_contour,
                                                    grid_fitting_size,
                                                    units_per_em);
                        if (tag & FT_POINT_TAG_CUBIC_CONTROL) != 0 {
                            let (point2, _) = get_point(&mut current_point_index,
                                                        point_positions,
                                                        point_tags,
                                                        last_point_index_in_contour,
                                                        grid_fitting_size,
                                                        units_per_em);
                            path_builder.cubic_bezier_to(point0, point1, point2)
                        } else {
                            path_builder.quadratic_bezier_to(point0, point1)
                        }
                    }
                }
                path_builder.close();
            }

            if hinting.grid_fitting_size().is_some() {
                reset_freetype_face_char_size((*self).freetype_face)
            }
        }

        return Ok(());

        fn get_point(current_point_index: &mut usize,
                     point_positions: &[FT_Vector],
                     point_tags: &[c_char],
                     last_point_index_in_contour: usize,
                     grid_fitting_size: Option<f32>,
                     units_per_em: u16)
                     -> (Point2D<f32>, c_char) {
            assert!(*current_point_index <= last_point_index_in_contour);
            let point_position = point_positions[*current_point_index];
            let point_tag = point_tags[*current_point_index];
            *current_point_index += 1;

            let mut point_position = Point2D::new(ft_fixed_26_6_to_f32(point_position.x),
                                                  ft_fixed_26_6_to_f32(point_position.y));
            if let Some(grid_fitting_size) = grid_fitting_size {
                point_position *= (units_per_em as f32) / grid_fitting_size
            }

            (point_position, point_tag)
        }
    }

    pub fn typographic_bounds(&self, glyph_id: u32) -> Result<Rect<f32>, GlyphLoadingError> {
        unsafe {
            if FT_Load_Glyph(self.freetype_face,
                             glyph_id,
                             (FT_LOAD_DEFAULT | FT_LOAD_NO_HINTING) as i32) != 0 {
                return Err(GlyphLoadingError::NoSuchGlyph)
            }

            let metrics = &(*(*self.freetype_face).glyph).metrics;
            Ok(Rect::new(Point2D::new(ft_fixed_26_6_to_f32(metrics.horiBearingX),
                                      ft_fixed_26_6_to_f32(metrics.horiBearingY - metrics.height)),
                         Size2D::new(ft_fixed_26_6_to_f32(metrics.width),
                                     ft_fixed_26_6_to_f32(metrics.height))))
        }
    }

    pub fn advance(&self, glyph_id: u32) -> Result<Vector2D<f32>, GlyphLoadingError> {
        unsafe {
            if FT_Load_Glyph(self.freetype_face,
                             glyph_id,
                             (FT_LOAD_DEFAULT | FT_LOAD_NO_HINTING) as i32) != 0 {
                return Err(GlyphLoadingError::NoSuchGlyph)
            }

            let advance = (*(*self.freetype_face).glyph).advance;
            Ok(Vector2D::new(ft_fixed_26_6_to_f32(advance.x), ft_fixed_26_6_to_f32(advance.y)))
        }
    }

    pub fn origin(&self, _: u32) -> Result<Point2D<f32>, GlyphLoadingError> {
        // FIXME(pcwalton): This can't be right!
        Ok(Point2D::zero())
    }

    pub fn metrics(&self) -> Metrics {
        let os2_table = self.get_os2_table();
        unsafe {
            let ascender = (*self.freetype_face).ascender;
            let descender = (*self.freetype_face).descender;
            let underline_position = (*self.freetype_face).underline_position;
            let underline_thickness = (*self.freetype_face).underline_thickness;
            Metrics {
                units_per_em: (*self.freetype_face).units_per_EM as u32,
                ascent: ascender as f32,
                descent: descender as f32,
                line_gap: ((*self.freetype_face).height + descender - ascender) as f32,
                underline_position: (underline_position + underline_thickness / 2) as f32,
                underline_thickness: underline_thickness as f32,
                cap_height: os2_table.map(|table| (*table).sCapHeight as f32).unwrap_or(0.0),
                x_height: os2_table.map(|table| (*table).sxHeight as f32).unwrap_or(0.0),
            }
        }
    }

    #[inline]
    pub fn font_data(&self) -> Option<FontData> {
        match self.font_data {
            FontData::File(_) | FontData::Memory(_) => Some(self.font_data.clone()),
            FontData::Unused(_) => unreachable!(),
        }
    }

    #[inline]
    pub fn supports_hinting_options(&self,
                                    hinting_options: HintingOptions,
                                    for_rasterization: bool)
                                    -> bool {
        match (hinting_options, for_rasterization) {
            (HintingOptions::None, _) |
            (HintingOptions::Vertical(_), true) |
            (HintingOptions::VerticalSubpixel(_), true) |
            (HintingOptions::Full(_), true) => true,
            (HintingOptions::Vertical(_), false) |
            (HintingOptions::VerticalSubpixel(_), false) |
            (HintingOptions::Full(_), false) => false,
        }
    }

    fn get_type_1_or_sfnt_name(&self, type_1_id: u32, sfnt_id: u16) -> Option<String> {
        unsafe {
            let ps_value_size = FT_Get_PS_Font_Value(self.freetype_face,
                                                     type_1_id,
                                                     0,
                                                     ptr::null_mut(),
                                                     0);
            if ps_value_size > 0 {
                let mut buffer = vec![0; ps_value_size as usize];
                if FT_Get_PS_Font_Value(self.freetype_face,
                                        type_1_id,
                                        0,
                                        buffer.as_mut_ptr() as *mut c_void,
                                        buffer.len() as i64) == 0 {
                    return String::from_utf8(buffer).ok()
                }
            }

            let sfnt_name_count = FT_Get_Sfnt_Name_Count(self.freetype_face);
            let mut sfnt_name = mem::zeroed();
            for sfnt_name_index in 0..sfnt_name_count {
                assert_eq!(FT_Get_Sfnt_Name(self.freetype_face, sfnt_name_index, &mut sfnt_name),
                           0);
                if sfnt_name.name_id != sfnt_id {
                    continue
                }

                match (sfnt_name.platform_id, sfnt_name.encoding_id) {
                    (TT_PLATFORM_APPLE_UNICODE, _) => {
                        let mut sfnt_name_bytes =
                            slice::from_raw_parts(sfnt_name.string, sfnt_name.string_len as usize);
                        let mut sfnt_name_string = Vec::with_capacity(sfnt_name_bytes.len() / 2);
                        while !sfnt_name_bytes.is_empty() {
                            sfnt_name_string.push(sfnt_name_bytes.read_u16::<BigEndian>().unwrap())
                        }
                        if let Ok(result) = String::from_utf16(&sfnt_name_string) {
                            return Some(result)
                        }
                    }
                    (platform_id, _) => {
                        warn!("get_type_1_or_sfnt_name(): found invalid platform ID {}",
                              platform_id);
                        // TODO(pcwalton)
                    }
                }
            }

            None
        }
    }

    fn get_os2_table(&self) -> Option<*const TT_OS2> {
        unsafe {
            let table = FT_Get_Sfnt_Table(self.freetype_face, FT_Sfnt_Tag::FT_SFNT_OS2);
            if table.is_null() {
                None
            } else {
                Some(table as *const TT_OS2)
            }
        }
    }

    #[inline]
    pub fn raster_bounds(&self,
                         glyph_id: u32,
                         point_size: f32,
                         origin: &Point2D<f32>,
                         hinting_options: HintingOptions,
                         rasterization_options: RasterizationOptions)
                         -> Result<Rect<i32>, GlyphLoadingError> {
        <Self as Face>::raster_bounds(self,
                                      glyph_id,
                                      point_size,
                                      origin,
                                      hinting_options,
                                      rasterization_options)
    }

    // TODO(pcwalton): This is woefully incomplete. See WebRender's code for a more complete
    // implementation.
    pub fn rasterize_glyph(&self,
                           canvas: &mut Canvas,
                           glyph_id: u32,
                           point_size: f32,
                           origin: &Point2D<f32>,
                           hinting_options: HintingOptions,
                           rasterization_options: RasterizationOptions)
                           -> Result<(), GlyphLoadingError> {
        unsafe {
            let mut delta = FT_Vector {
                x: f32_to_ft_fixed_26_6(origin.x),
                y: f32_to_ft_fixed_26_6(origin.y),
            };
            FT_Set_Transform(self.freetype_face, ptr::null_mut(), &mut delta);

            assert_eq!(FT_Set_Char_Size(self.freetype_face,
                                        f32_to_ft_fixed_26_6(point_size),
                                        0,
                                        0,
                                        0),
                       0);

            let load_flags = self.hinting_options_to_load_flags(hinting_options);
            if FT_Load_Glyph(self.freetype_face, glyph_id, load_flags as i32) != 0 {
                return Err(GlyphLoadingError::NoSuchGlyph)
            }

            let render_mode = match rasterization_options {
                RasterizationOptions::Bilevel => FT_Render_Mode::FT_RENDER_MODE_MONO,
                RasterizationOptions::GrayscaleAa => FT_Render_Mode::FT_RENDER_MODE_NORMAL,
                RasterizationOptions::SubpixelAa => FT_Render_Mode::FT_RENDER_MODE_LCD,
            };
            assert_eq!(FT_Render_Glyph((*self.freetype_face).glyph, render_mode), 0);

            // TODO(pcwalton): Use the FreeType "direct" API to save a copy here. Note that we will
            // need to keep this around for bilevel rendering, as the direct API doesn't work with
            // that mode.
            let bitmap = &(*(*self.freetype_face).glyph).bitmap;
            let bitmap_stride = (*bitmap).pitch as usize;
            let bitmap_width = (*bitmap).width as u32;
            let bitmap_height = (*bitmap).rows as u32;
            let bitmap_size = Size2D::new(bitmap_width, bitmap_height);
            let bitmap_buffer = (*bitmap).buffer as *const i8 as *const u8;
            let bitmap_length = bitmap_stride * bitmap_height as usize;
            let buffer = slice::from_raw_parts(bitmap_buffer, bitmap_length);

            // FIXME(pcwalton): This function should return a Result instead.
            let bitmap_format = match (*bitmap).pixel_mode {
                FT_PIXEL_MODE_MONO => unimplemented!(),
                FT_PIXEL_MODE_GRAY => Format::A8,
                FT_PIXEL_MODE_LCD | FT_PIXEL_MODE_LCD_V => Format::Rgb24,
                _ => panic!("Unexpected FreeType pixel mode!"),
            };

            canvas.blit_from(buffer, &bitmap_size, bitmap_stride, bitmap_format);

            FT_Set_Transform(self.freetype_face, ptr::null_mut(), ptr::null_mut());

            Ok(())
        }
    }

    fn hinting_options_to_load_flags(&self, hinting: HintingOptions) -> u32 {
        match hinting {
            HintingOptions::None => FT_LOAD_DEFAULT | FT_LOAD_NO_HINTING,
            HintingOptions::Vertical(_) => FT_LOAD_DEFAULT | FT_LOAD_TARGET_LIGHT,
            HintingOptions::VerticalSubpixel(_) => FT_LOAD_DEFAULT | FT_LOAD_TARGET_LCD,
            HintingOptions::Full(_) => FT_LOAD_DEFAULT | FT_LOAD_TARGET_NORMAL,
        }
    }

    pub fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        match self.font_data {
            FontData::File(ref file) => Some(Arc::new((*file).to_vec())),
            FontData::Memory(ref memory) => Some((*memory).clone()),
            FontData::Unused(_) => unreachable!(),
        }
    }
}

impl Clone for Font {
    fn clone(&self) -> Font {
        unsafe {
            assert_eq!(FT_Reference_Face(self.freetype_face), 0);
            Font {
                freetype_face: self.freetype_face,
                font_data: self.font_data.clone(),
            }
        }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            if !self.freetype_face.is_null() {
                assert_eq!(FT_Done_Face(self.freetype_face), 0);
            }
        }
    }
}

impl Debug for Font {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        self.family_name().fmt(fmt)
    }
}

impl Face for Font {
    type NativeFont = NativeFont;

    #[inline]
    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, FontLoadingError> {
        Font::from_bytes(font_data, font_index)
    }

    #[inline]
    fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        Font::from_file(file, font_index)
    }

    fn analyze_file(file: &mut File) -> Result<Type, FontLoadingError> {
        Font::analyze_file(file)
    }

    #[inline]
    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self {
        Font::from_native_font(native_font)
    }

    #[inline]
    fn postscript_name(&self) -> String {
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
    fn origin(&self, origin: u32) -> Result<Point2D<f32>, GlyphLoadingError> {
        self.origin(origin)
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

#[derive(Clone)]
pub enum FontData<'a> {
    Memory(Arc<Vec<u8>>),
    File(Arc<Mmap>),
    Unused(PhantomData<&'a u8>),
}

impl<'a> Deref for FontData<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            FontData::File(ref mmap) => &***mmap,
            FontData::Memory(ref data) => &***data,
            FontData::Unused(_) => unreachable!(),
        }
    }
}

unsafe fn setup_freetype_face(face: FT_Face) {
    reset_freetype_face_char_size(face);
}

unsafe fn reset_freetype_face_char_size(face: FT_Face) {
    // Apple Color Emoji has 0 units per em. Whee!
    let units_per_em = (*face).units_per_EM as i64;
    if units_per_em > 0 {
        assert_eq!(FT_Set_Char_Size(face, ((*face).units_per_EM as i64) << 6, 0, 0, 0), 0);
    }
}

#[repr(C)]
struct FT_SfntName {
    platform_id: FT_UShort,
    encoding_id: FT_UShort,
    language_id: FT_UShort,
    name_id: FT_UShort,
    string: *mut FT_Byte,
    string_len: FT_UInt,
}

fn ft_fixed_26_6_to_f32(fixed: i64) -> f32 {
    (fixed as f32) / 64.0
}

fn f32_to_ft_fixed_26_6(float: f32) -> i64 {
    f32::round(float * 64.0) as i64
}

extern "C" {
    fn FT_Get_PS_Font_Value(face: FT_Face,
                            key: u32,
                            idx: FT_UInt,
                            value: *mut c_void,
                            value_len: FT_Long)
                            -> FT_Long;
    fn FT_Get_Sfnt_Name(face: FT_Face, idx: FT_UInt, aname: *mut FT_SfntName) -> FT_Error;
    fn FT_Get_Sfnt_Name_Count(face: FT_Face) -> FT_UInt;
}
