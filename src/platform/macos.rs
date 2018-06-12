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
use core_text::font_descriptor::{kCTFontDefaultOrientation, kCTFontItalicTrait};
use core_text::font_descriptor::{kCTFontMonoSpaceTrait, kCTFontVerticalTrait};
use core_text;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::builder::PathBuilder;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use descriptor::{Descriptor, Flags, Query, QueryFields};
use family::Family;
use font::Metrics;
use set::Set;

const ITALIC_SLANT: f64 = 1.0 / 15.0;

static FONT_WEIGHT_MAPPINGS: [f32; 11] =
    [-1.0, -0.7, -0.5, -0.23, 0.0, 0.2, 0.3, 0.4, 0.6, 0.8, 1.0];

pub type NativeFont = CTFont;

#[derive(Clone)]
pub struct Font {
    core_text_font: CTFont,
    font_data: Option<Arc<Vec<u8>>>,
}

impl Font {
    pub fn from_bytes(font_data: Arc<Vec<u8>>) -> Result<Font, ()> {
        let data_provider = CGDataProvider::from_buffer(font_data.clone());
        let core_graphics_font = try!(CGFont::from_data_provider(data_provider).map_err(drop));
        let core_text_font = core_text::font::new_from_CGFont(&core_graphics_font, 16.0);
        Ok(Font {
            core_text_font,
            font_data: Some(font_data),
        })
    }

    pub fn from_native_font(core_text_font: NativeFont) -> Font {
        let mut font_data = None;
        match core_text_font.url() {
            None => warn!("No URL found for Core Text font!"),
            Some(url) => {
                match url.to_path() {
                    Some(path) => {
                        match File::open(path) {
                            Ok(mut file) => {
                                let mut buffer = vec![];
                                match file.read_to_end(&mut buffer) {
                                    Err(_) => warn!("Could not read Core Text font from disk!"),
                                    Ok(_) => font_data = Some(Arc::new(buffer)),
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
        let stretch = core_text_font_width_to_css_stretchiness(all_traits.normalized_width() as
                                                               f32);

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
            descent: (self.core_text_font.descent() * units_per_point) as f32,
            leading: (self.core_text_font.leading() * units_per_point) as f32,
            underline_position: (self.core_text_font.underline_position() *
                                 units_per_point) as f32,
            underline_thickness: (self.core_text_font.underline_thickness() *
                                  units_per_point) as f32,
            slant_angle: (self.core_text_font.slant_angle() * units_per_point) as f32,
            cap_height: (self.core_text_font.cap_height() * units_per_point) as f32,
            x_height: (self.core_text_font.x_height() * units_per_point) as f32,
        }
    }

    #[inline]
    pub fn font_data(&self) -> Option<&[u8]> {
        self.font_data.as_ref().map(|font_data| &***font_data)
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
                /*let weight = css_to_core_text_font_weight(self.descriptor.weight);
                core_text_traits.set(CFString::new("NSCTFontWeightTrait"),
                                     CFNumber::from(weight).as_CFType());*/
                //let weight = css_to_core_text_font_weight(self.descriptor.weight);
                let weight = self.descriptor.weight;
                core_text_traits.set(CFString::new("CTFontCSSWeightAttribute"),
                                     CFNumber::from(weight).as_CFType());
            }
            if self.fields.contains(QueryFields::STRETCH) {
                /*let width = css_stretchiness_to_core_text_font_width(self.descriptor.stretch);
                core_text_traits.set(CFString::new("NSCTFontProportionTrait"),
                                     CFNumber::from(width).as_CFType());*/
                let width = self.descriptor.stretch;
                core_text_traits.set(CFString::new("CTFontCSSWidthAttribute"),
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

trait CGPointExt {
    fn to_euclid_point(&self) -> Point2D<f32>;
}

impl CGPointExt for CGPoint {
    #[inline]
    fn to_euclid_point(&self) -> Point2D<f32> {
        Point2D::new(self.x as f32, self.y as f32)
    }
}

fn css_to_core_text_font_weight(mut css_weight: f32) -> f32 {
    
    css_weight = clamp(css_weight, 100.0, 900.0) - 400.0;
    let factor = if css_weight <= 0.0 {
        1.0 / 400.0
    } else {
        1.0 / 500.0
    };
    factor * css_weight
}

fn core_text_to_css_font_weight(core_text_weight: f32) -> f32 {
    let factor = if core_text_weight <= 0.0 {
        300.0
    } else {
        500.0
    };
    factor * core_text_weight + 400.0
}

fn css_stretchiness_to_core_text_font_width(css_stretchiness: f32) -> f32 {
    let mut core_text_font_width = clamp(css_stretchiness, 0.5, 2.0) - 1.0;
    if core_text_font_width < 0.0 {
        core_text_font_width *= 2.0
    }
    core_text_font_width
}

fn core_text_font_width_to_css_stretchiness(core_text_font_width: f32) -> f32 {
    let mut css_stretchiness = core_text_font_width;
    if css_stretchiness < 0.0 {
        css_stretchiness *= 0.5
    }
    css_stretchiness + 1.0
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

fn symbolic_trait_fields() -> QueryFields {
    QueryFields::MONOSPACE | QueryFields::VERTICAL
}
