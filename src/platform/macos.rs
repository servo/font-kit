// font-kit/src/platform/macos.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFMutableDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::data_provider::CGDataProvider;
use core_graphics::font::CGFont;
use core_graphics::geometry::{CG_AFFINE_TRANSFORM_IDENTITY, CG_ZERO_SIZE, CGPoint};
use core_graphics::path::CGPathElementType;
use core_text::font::CTFont;
use core_text::font_collection;
use core_text::font_descriptor::{self, CTFontDescriptor, SymbolicTraitAccessors, TraitAccessors};
use core_text::font_descriptor::{kCTFontDefaultOrientation, kCTFontMonoSpaceTrait};
use core_text::font_descriptor::{kCTFontVerticalTrait};
use core_text;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::builder::PathBuilder;
use memmap::Mmap;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::f32;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use descriptor::{Descriptor, Flags, FONT_STRETCH_MAPPING, Query, QueryFields};
use family::Family;
use font::Metrics;
use set::Set;

const ITALIC_SLANT: f64 = 1.0 / 15.0;

static FONT_WEIGHT_MAPPING: [f32; 9] = [-0.7, -0.5, -0.23, 0.0, 0.2, 0.3, 0.4, 0.6, 0.8];

pub type NativeFont = CTFont;

#[derive(Clone)]
pub struct Font {
    core_text_font: CTFont,
    font_data: FontData<'static>,
}

impl Font {
    pub fn from_bytes(font_data: Arc<Vec<u8>>) -> Result<Font, ()> {
        let data_provider = CGDataProvider::from_buffer(font_data.clone());
        let core_graphics_font = try!(CGFont::from_data_provider(data_provider).map_err(drop));
        let core_text_font = core_text::font::new_from_CGFont(&core_graphics_font, 16.0);
        Ok(Font {
            core_text_font,
            font_data: FontData::Memory(font_data),
        })
    }

    pub fn from_file(mut file: File) -> Result<Font, ()> {
        let mut font_data = vec![];
        try!(file.read_to_end(&mut font_data).map_err(drop));
        Font::from_bytes(Arc::new(font_data))
    }

    pub unsafe fn from_native_font(core_text_font: NativeFont) -> Font {
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

    pub fn outline<B>(&self, glyph_id: u32, path_builder: &mut B) -> Result<(), ()>
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

impl Query {
    pub fn lookup(&self) -> Set {
        let collection = if self.is_universal() {
            font_collection::create_for_all_families()
        } else {
            let descriptor = self.as_core_text_font_descriptor();
            font_collection::new_from_descriptors(&CFArray::from_CFTypes(&[descriptor]))
        };

        let mut families = HashMap::new();
        if let Some(descriptors) = collection.get_descriptors() {
            for index in 0..descriptors.len() {
                unsafe {
                    let descriptor = (*descriptors.get(index).unwrap()).clone();
                    let core_text_font = core_text::font::new_from_descriptor(&descriptor, 12.0);
                    let family_name = core_text_font.family_name();
                    let font = Font::from_native_font(core_text_font);
                    match families.entry(family_name) {
                        Entry::Vacant(entry) => {
                            entry.insert(vec![font]);
                        }
                        Entry::Occupied(mut entry) => entry.get_mut().push(font),
                    }
                }
            }
        }

        Set::from_families(families.into_iter().map(|(_, fonts)| {
            Family::from_fonts(fonts.into_iter())
        }))
    }

    fn as_core_text_font_descriptor(&self) -> CTFontDescriptor {
        let mut attributes: CFMutableDictionary<CFString, CFType> = CFMutableDictionary::new();

        if self.fields.contains(QueryFields::POSTSCRIPT_NAME) {
            attributes.set(CFString::new("NSFontNameAttribute"),
                           CFString::new(&self.descriptor.postscript_name).as_CFType());
        }
        if self.fields.contains(QueryFields::DISPLAY_NAME) {
            attributes.set(CFString::new("NSFontVisibleNameAttribute"),
                           CFString::new(&self.descriptor.display_name).as_CFType());
        }
        if self.fields.contains(QueryFields::FAMILY_NAME) {
            attributes.set(CFString::new("NSFontFamilyAttribute"),
                           CFString::new(&self.descriptor.family_name).as_CFType());
        }
        if self.fields.contains(QueryFields::STYLE_NAME) {
            attributes.set(CFString::new("NSFontFaceAttribute"),
                           CFString::new(&self.descriptor.style_name).as_CFType());
        }

        if self.fields.intersects(QueryFields::WEIGHT | QueryFields::STRETCH |
                                  QueryFields::ITALIC | symbolic_trait_fields()) {
            let mut core_text_traits: CFMutableDictionary<CFString, CFType> =
                CFMutableDictionary::new();

            if self.fields.contains(QueryFields::WEIGHT) {
                let weight = css_to_core_text_font_weight(self.descriptor.weight);
                core_text_traits.set(CFString::new("NSCTFontWeightTrait"),
                                     CFNumber::from(weight).as_CFType());
            }
            if self.fields.contains(QueryFields::STRETCH) {
                let width = css_stretchiness_to_core_text_width(self.descriptor.stretch);
                core_text_traits.set(CFString::new("NSCTFontProportionTrait"),
                                     CFNumber::from(width).as_CFType());
            }
            if self.fields.contains(QueryFields::ITALIC) {
                let slant = if self.descriptor.flags.contains(Flags::ITALIC) {
                    ITALIC_SLANT
                } else {
                    0.0
                };
                core_text_traits.set(CFString::new("NSCTFontSlantTrait"),
                                     CFNumber::from(slant).as_CFType());
            }

            if self.fields.intersects(symbolic_trait_fields()) {
                let mut symbolic_traits = 0;
                if self.fields.contains(QueryFields::MONOSPACE) &&
                        self.descriptor.flags.contains(Flags::MONOSPACE) {
                    symbolic_traits |= kCTFontMonoSpaceTrait
                }
                if self.fields.contains(QueryFields::VERTICAL) &&
                        self.descriptor.flags.contains(Flags::VERTICAL) {
                    symbolic_traits |= kCTFontVerticalTrait
                }
                core_text_traits.set(CFString::new("NSCTFontSymbolicTrait"),
                                    CFNumber::from(symbolic_traits as i64).as_CFType());
            }

            attributes.set(CFString::new("NSCTFontTraitsAttribute"), core_text_traits.as_CFType());
        }

        font_descriptor::new_from_attributes(&attributes.as_dictionary())
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

fn piecewise_linear_lookup(index: f32, mapping: &[f32]) -> f32 {
    let lower_value = mapping[f32::floor(index) as usize];
    let upper_value = mapping[f32::ceil(index) as usize];
    lerp(lower_value, upper_value, f32::fract(index))
}

fn piecewise_linear_find_index(query_value: f32, mapping: &[f32]) -> f32 {
    let upper_index = match mapping.binary_search_by(|value| {
        value.partial_cmp(&query_value).unwrap_or(Ordering::Less)
    }) {
        Ok(index) => return index as f32,
        Err(upper_index) => upper_index,
    };
    let lower_index = upper_index - 1;
    let (upper_value, lower_value) = (mapping[upper_index], mapping[lower_index]);
    let t = (query_value - lower_value) / (upper_value - lower_value);
    lower_index as f32 + t
}

fn css_to_core_text_font_weight(css_weight: f32) -> f32 {
    piecewise_linear_lookup(f32::max(100.0, css_weight) / 100.0 - 1.0, &FONT_WEIGHT_MAPPING)
}

fn core_text_to_css_font_weight(core_text_weight: f32) -> f32 {
    piecewise_linear_find_index(core_text_weight, &FONT_WEIGHT_MAPPING) * 100.0 + 100.0
}

fn css_stretchiness_to_core_text_width(css_stretchiness: f32) -> f32 {
    let css_stretchiness = clamp(css_stretchiness, 0.5, 2.0);
    0.25 * piecewise_linear_find_index(css_stretchiness, &FONT_STRETCH_MAPPING) - 1.0
}

fn core_text_width_to_css_stretchiness(core_text_width: f32) -> f32 {
    piecewise_linear_lookup((core_text_width + 1.0) * 4.0, &FONT_STRETCH_MAPPING)
}

fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn symbolic_trait_fields() -> QueryFields {
    QueryFields::MONOSPACE | QueryFields::VERTICAL
}

#[cfg(test)]
mod test {
    #[test]
    fn test_css_to_core_text_font_weight() {
        // Exact matches
        assert_eq!(super::css_to_core_text_font_weight(100.0), -0.7);
        assert_eq!(super::css_to_core_text_font_weight(400.0), 0.0);
        assert_eq!(super::css_to_core_text_font_weight(700.0), 0.4);
        assert_eq!(super::css_to_core_text_font_weight(900.0), 0.8);

        // Linear interpolation
        assert_eq!(super::css_to_core_text_font_weight(450.0), 0.1);
    }

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
    fn test_css_to_core_text_font_stretch() {
        // Exact matches
        assert_eq!(super::css_stretchiness_to_core_text_width(1.0), 0.0);
        assert_eq!(super::css_stretchiness_to_core_text_width(0.5), -1.0);
        assert_eq!(super::css_stretchiness_to_core_text_width(2.0), 1.0);

        // Linear interpolation
        assert_eq!(super::css_stretchiness_to_core_text_width(1.7), 0.85);
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
