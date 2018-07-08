// font-kit/src/loaders/core_text.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use byteorder::{BigEndian, ReadBytesExt};
use core_graphics::data_provider::CGDataProvider;
use core_graphics::font::CGFont;
use core_graphics::geometry::{CG_AFFINE_TRANSFORM_IDENTITY, CG_ZERO_SIZE, CGPoint};
use core_graphics::path::CGPathElementType;
use core_text::font::CTFont;
use core_text::font_descriptor::{SymbolicTraitAccessors, TraitAccessors};
use core_text::font_descriptor::{kCTFontDefaultOrientation};
use core_text;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::builder::PathBuilder;
use memmap::Mmap;
use std::f32;
use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use descriptor::{Descriptor, Flags, FONT_STRETCH_MAPPING};
use font::{Face, HintingOptions, Metrics, Type};
use sources;
use utils;

const TTC_TAG: [u8; 4] = [b't', b't', b'c', b'f'];

pub type NativeFont = CTFont;

#[derive(Clone)]
pub struct Font {
    core_text_font: CTFont,
    font_data: FontData<'static>,
}

impl Font {
    pub fn from_bytes(mut font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Font, ()> {
        // Sadly, there's no API to load OpenType collections on macOS, I don't believe…
        if font_is_collection(&**font_data) {
            let mut new_font_data = (*font_data).clone();
            try!(unpack_otc_font(&mut new_font_data, font_index));
            font_data = Arc::new(new_font_data);
        }

        let data_provider = CGDataProvider::from_buffer(font_data.clone());
        let core_graphics_font = try!(CGFont::from_data_provider(data_provider).map_err(drop));
        let core_text_font = core_text::font::new_from_CGFont(&core_graphics_font, 16.0);
        Ok(Font {
            core_text_font,
            font_data: FontData::Memory(font_data),
        })
    }

    pub fn from_file(file: &mut File, font_index: u32) -> Result<Font, ()> {
        let mut font_data = vec![];
        try!(file.seek(SeekFrom::Start(0)).map_err(drop));
        try!(file.read_to_end(&mut font_data).map_err(drop));
        Font::from_bytes(Arc::new(font_data), font_index)
    }

    #[inline]
    pub fn from_path<P>(path: P, font_index: u32) -> Result<Font, ()> where P: AsRef<Path> {
        <Font as Face>::from_path(path, font_index)
    }

    pub unsafe fn from_native_font(core_text_font: NativeFont) -> Font {
        Font::from_core_text_font(core_text_font)
    }

    pub unsafe fn from_core_text_font(core_text_font: NativeFont) -> Font {
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

    pub fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<Type, ()> {
        if let Ok(font_count) = read_number_of_fonts_from_otc_header(&font_data) {
            return Ok(Type::Collection(font_count))
        }
        let data_provider = CGDataProvider::from_buffer(font_data);
        match CGFont::from_data_provider(data_provider) {
            Ok(_) => Ok(Type::Single),
            Err(_) => Err(()),
        }
    }

    pub fn analyze_file(file: &mut File) -> Result<Type, ()> {
        let mut font_data = vec![];
        try!(file.seek(SeekFrom::Start(0)).map_err(drop));
        try!(file.read_to_end(&mut font_data).map_err(drop));
        Font::analyze_bytes(Arc::new(font_data))
    }

    #[inline]
    pub fn analyze_path<P>(path: P) -> Result<Type, ()> where P: AsRef<Path> {
        <Self as Face>::analyze_path(path)
    }

    #[inline]
    pub fn as_native_font(&self) -> NativeFont {
        self.core_text_font.clone()
    }

    pub fn descriptor(&self) -> Descriptor {
        let symbolic_traits = self.core_text_font.symbolic_traits();
        let all_traits = self.core_text_font.all_traits();

        let mut flags = Flags::empty();
        flags.set(Flags::MONOSPACE, symbolic_traits.is_monospace());
        flags.set(Flags::VERTICAL, symbolic_traits.is_vertical());
        flags.set(Flags::ITALIC, all_traits.normalized_slant() > 0.0);

        let weight = core_text_to_css_font_weight(all_traits.normalized_weight() as f32);
        let stretch = core_text_width_to_css_stretchiness(all_traits.normalized_width() as f32);

        Descriptor {
            postscript_name: self.core_text_font.postscript_name(),
            display_name: self.core_text_font.display_name(),
            family_name: self.core_text_font.family_name(),
            style_name: self.core_text_font.style_name(),
            weight,
            stretch,
            flags,
        }
    }

    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        unsafe {
            let (mut dest, mut src) = ([0, 0], [0, 0]);
            let src = character.encode_utf16(&mut src);
            self.core_text_font.get_glyphs_for_characters(src.as_ptr(), dest.as_mut_ptr(), 2);
            Some(dest[0] as u32)
        }
    }

    pub fn outline<B>(&self, glyph_id: u32, _: HintingOptions, path_builder: &mut B)
                      -> Result<(), ()>
                      where B: PathBuilder {
        let path = try!(self.core_text_font.create_path_for_glyph(glyph_id as u16,
                                                                  &CG_AFFINE_TRANSFORM_IDENTITY));
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

    pub fn typographic_bounds(&self, glyph_id: u32) -> Rect<f32> {
        let rect = self.core_text_font.get_bounding_rects_for_glyphs(kCTFontDefaultOrientation,
                                                                     &[glyph_id as u16]);
        let units_per_point = self.units_per_point();
        Rect::new(Point2D::new((rect.origin.x * units_per_point) as f32,
                               (rect.origin.y * units_per_point) as f32),
                  Size2D::new((rect.size.width * units_per_point) as f32,
                              (rect.size.height * units_per_point) as f32))
    }

    pub fn advance(&self, glyph_id: u32) -> Vector2D<f32> {
        let (glyph_id, mut advance) = (glyph_id as u16, CG_ZERO_SIZE);
        self.core_text_font
            .get_advances_for_glyphs(kCTFontDefaultOrientation, &glyph_id, &mut advance, 1);
        Vector2D::new((advance.width * self.units_per_point()) as f32,
                      (advance.height * self.units_per_point()) as f32)
    }

    pub fn origin(&self, glyph_id: u32) -> Point2D<f32> {
        let (glyph_id, mut translation) = (glyph_id as u16, CG_ZERO_SIZE);
        self.core_text_font.get_vertical_translations_for_glyphs(kCTFontDefaultOrientation,
                                                                 &glyph_id,
                                                                 &mut translation,
                                                                 1);
        Point2D::new((translation.width * self.units_per_point()) as f32,
                     (translation.height * self.units_per_point()) as f32)
    }

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

    #[inline]
    pub fn font_data(&self) -> Option<FontData> {
        match self.font_data {
            FontData::Unavailable => None,
            FontData::File(_) | FontData::Memory(_) => Some(self.font_data.clone()),
            FontData::Unused(_) => unreachable!(),
        }
    }

    #[inline]
    fn units_per_point(&self) -> f64 {
        (self.core_text_font.units_per_em() as f64) / self.core_text_font.pt_size()
    }
}

impl Face for Font {
    type NativeFont = NativeFont;

    #[inline]
    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, ()> {
        Font::from_bytes(font_data, font_index)
    }

    #[inline]
    fn from_file(file: &mut File, font_index: u32) -> Result<Font, ()> {
        Font::from_file(file, font_index)
    }

    #[inline]
    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self {
        Font::from_native_font(native_font)
    }

    #[cfg(target_os = "macos")]
    #[inline]
    unsafe fn from_core_text_font(core_text_font: CTFont) -> Font {
        Font::from_core_text_font(core_text_font)
    }

    fn analyze_file(file: &mut File) -> Result<Type, ()> {
        Font::analyze_file(file)
    }

    #[inline]
    fn descriptor(&self) -> Descriptor {
        self.descriptor()
    }

    #[inline]
    fn glyph_for_char(&self, character: char) -> Option<u32> {
        self.glyph_for_char(character)
    }

    #[inline]
    fn outline<B>(&self, glyph_id: u32, hinting_mode: HintingOptions, path_builder: &mut B)
                  -> Result<(), ()>
                  where B: PathBuilder {
        self.outline(glyph_id, hinting_mode, path_builder)
    }

    #[inline]
    fn typographic_bounds(&self, glyph_id: u32) -> Rect<f32> {
        self.typographic_bounds(glyph_id)
    }

    #[inline]
    fn advance(&self, glyph_id: u32) -> Vector2D<f32> {
        self.advance(glyph_id)
    }

    #[inline]
    fn origin(&self, origin: u32) -> Point2D<f32> {
        self.origin(origin)
    }

    #[inline]
    fn metrics(&self) -> Metrics {
        self.metrics()
    }
}

impl Debug for Font {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        self.descriptor().fmt(fmt)
    }
}

#[derive(Clone)]
pub enum FontData<'a> {
    Unavailable,
    Memory(Arc<Vec<u8>>),
    File(Arc<Mmap>),
    Unused(PhantomData<&'a u8>),
}

impl<'a> Deref for FontData<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            FontData::Unavailable => panic!("Font data unavailable!"),
            FontData::File(ref mmap) => &***mmap,
            FontData::Memory(ref data) => &***data,
            FontData::Unused(_) => unreachable!(),
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

fn core_text_to_css_font_weight(core_text_weight: f32) -> f32 {
    sources::core_text::piecewise_linear_find_index(core_text_weight,
                                                    &sources::core_text::FONT_WEIGHT_MAPPING) *
        100.0 + 100.0
}

fn core_text_width_to_css_stretchiness(core_text_width: f32) -> f32 {
    sources::core_text::piecewise_linear_lookup((core_text_width + 1.0) * 4.0,
                                                &FONT_STRETCH_MAPPING)
}

fn font_is_collection(header: &[u8]) -> bool {
    header.len() >= 4 && header[0..4] == TTC_TAG
}

fn read_number_of_fonts_from_otc_header(header: &[u8]) -> Result<u32, ()> {
    if !font_is_collection(header) {
        return Err(())
    }
    (&header[8..]).read_u32::<BigEndian>().map_err(drop)
}

// Unpacks an OTC font "in-place".
fn unpack_otc_font(data: &mut [u8], font_index: u32) -> Result<(), ()> {
    if font_index >= try!(read_number_of_fonts_from_otc_header(data)) {
        return Err(())
    }

    let offset_table_pos_pos = 12 + 4 * font_index as usize;
    let offset_table_pos = try!((&data[offset_table_pos_pos..]).read_u32::<BigEndian>()
                                                               .map_err(drop)) as usize;
    debug_assert!(utils::SFNT_VERSIONS.iter().any(|version| {
        data[offset_table_pos..(offset_table_pos + 4)] == *version
    }));
    let num_tables = try!((&data[(offset_table_pos + 4)..]).read_u16::<BigEndian>().map_err(drop));

    // Must copy forward in order to avoid problems with overlapping memory.
    let offset_table_and_table_record_size = 12 + (num_tables as usize) * 16;
    for offset in 0..offset_table_and_table_record_size {
        data[offset] = data[offset_table_pos + offset]
    }

    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_core_text_to_css_font_weight() {
        // Exact matches
        assert_eq!(super::core_text_to_css_font_weight(-0.7), 100.0);
        assert_eq!(super::core_text_to_css_font_weight(0.0), 400.0);
        assert_eq!(super::core_text_to_css_font_weight(0.4), 700.0);
        assert_eq!(super::core_text_to_css_font_weight(0.8), 900.0);

        // Linear interpolation
        assert_eq!(super::core_text_to_css_font_weight(0.1), 450.0);
    }

    #[test]
    fn test_core_text_to_css_font_stretch() {
        // Exact matches
        assert_eq!(super::core_text_width_to_css_stretchiness(0.0), 1.0);
        assert_eq!(super::core_text_width_to_css_stretchiness(-1.0), 0.5);
        assert_eq!(super::core_text_width_to_css_stretchiness(1.0), 2.0);

        // Linear interpolation
        assert_eq!(super::core_text_width_to_css_stretchiness(0.85), 1.7);
    }
}
