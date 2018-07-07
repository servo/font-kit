// font-kit/src/sources/core_text.rs
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
use core_text::font_collection;
use core_text::font_descriptor::{self, CTFontDescriptor, kCTFontMonoSpaceTrait};
use core_text::font_descriptor::{kCTFontVerticalTrait};
use core_text;
use std::cmp::Ordering;
use std::f32;

use descriptor::{Flags, FONT_STRETCH_MAPPING, Query, QueryFields};
use font::Font;
use set::Set;
use utils;

const ITALIC_SLANT: f64 = 1.0 / 15.0;

pub static FONT_WEIGHT_MAPPING: [f32; 9] = [-0.7, -0.5, -0.23, 0.0, 0.2, 0.3, 0.4, 0.6, 0.8];

pub struct Source;

impl Source {
    #[inline]
    pub fn new() -> Source {
        Source
    }

    pub fn select(&self, query: &Query) -> Set {
        let collection = if query.is_universal() {
            font_collection::create_for_all_families()
        } else {
            let descriptor = query.as_core_text_font_descriptor();
            font_collection::new_from_descriptors(&CFArray::from_CFTypes(&[descriptor]))
        };

        let mut fonts = vec![];
        if let Some(descriptors) = collection.get_descriptors() {
            for index in 0..descriptors.len() {
                unsafe {
                    let descriptor = (*descriptors.get(index).unwrap()).clone();
                    let core_text_font = core_text::font::new_from_descriptor(&descriptor, 12.0);
                    fonts.push(Font::from_core_text_font(core_text_font));
                }
            }
        }
        Set::from_fonts(fonts.into_iter())
    }
}

impl Query {
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

pub(crate) fn piecewise_linear_lookup(index: f32, mapping: &[f32]) -> f32 {
    let lower_value = mapping[f32::floor(index) as usize];
    let upper_value = mapping[f32::ceil(index) as usize];
    utils::lerp(lower_value, upper_value, f32::fract(index))
}

pub(crate) fn piecewise_linear_find_index(query_value: f32, mapping: &[f32]) -> f32 {
    let upper_index = match mapping.binary_search_by(|value| {
        value.partial_cmp(&query_value).unwrap_or(Ordering::Less)
    }) {
        Ok(index) => return index as f32,
        Err(upper_index) => upper_index,
    };
    if upper_index == 0 {
        return upper_index as f32
    }
    let lower_index = upper_index - 1;
    let (upper_value, lower_value) = (mapping[upper_index], mapping[lower_index]);
    let t = (query_value - lower_value) / (upper_value - lower_value);
    lower_index as f32 + t
}

fn css_to_core_text_font_weight(css_weight: f32) -> f32 {
    piecewise_linear_lookup(f32::max(100.0, css_weight) / 100.0 - 1.0, &FONT_WEIGHT_MAPPING)
}

fn css_stretchiness_to_core_text_width(css_stretchiness: f32) -> f32 {
    let css_stretchiness = utils::clamp(css_stretchiness, 0.5, 2.0);
    0.25 * piecewise_linear_find_index(css_stretchiness, &FONT_STRETCH_MAPPING) - 1.0
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
    fn test_css_to_core_text_font_stretch() {
        // Exact matches
        assert_eq!(super::css_stretchiness_to_core_text_width(1.0), 0.0);
        assert_eq!(super::css_stretchiness_to_core_text_width(0.5), -1.0);
        assert_eq!(super::css_stretchiness_to_core_text_width(2.0), 1.0);

        // Linear interpolation
        assert_eq!(super::css_stretchiness_to_core_text_width(1.7), 0.85);
    }
}
