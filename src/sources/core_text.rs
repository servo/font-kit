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
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_text::font_collection::{self, CTFontCollection};
use core_text::font_descriptor;
use core_text::font_manager;
use core_text;
use std::cmp::Ordering;
use std::f32;

use descriptor::{FONT_STRETCH_MAPPING, Stretch, Spec, Weight};
use family::Family;
use font::Font;
use source::Source;
use utils;

pub static FONT_WEIGHT_MAPPING: [f32; 9] = [-0.7, -0.5, -0.23, 0.0, 0.2, 0.3, 0.4, 0.6, 0.8];

pub struct CoreTextSource;

impl CoreTextSource {
    #[inline]
    pub fn new() -> CoreTextSource {
        CoreTextSource
    }

    #[inline]
    pub fn all_families(&self) -> Vec<String> {
        let core_text_family_names = font_manager::copy_available_font_family_names();
        let mut families = Vec::with_capacity(core_text_family_names.len() as usize);
        for core_text_family_name in core_text_family_names.iter() {
            families.push(core_text_family_name.to_string())
        }
        families
    }

    #[inline]
    pub fn select_family(&self, family_name: &str) -> Family {
        let attributes: CFDictionary<CFString, CFType> = CFDictionary::from_CFType_pairs(&[
            (CFString::new("NSFontFamilyAttribute"), CFString::new(family_name).as_CFType()),
        ]);

        let descriptor = font_descriptor::new_from_attributes(&attributes);
        let descriptors = CFArray::from_CFTypes(&[descriptor]);
        let collection = font_collection::new_from_descriptors(&descriptors);
        Family::from_fonts(fonts_in_core_text_collection(collection).into_iter())
    }

    pub fn find(&self, spec: &Spec) -> Result<Font, ()> {
        <Self as Source>::find(self, spec)
    }
}

impl Source for CoreTextSource {
    fn all_families(&self) -> Vec<String> {
        self.all_families()
    }

    fn select_family(&self, family_name: &str) -> Family {
        self.select_family(family_name)
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

#[allow(dead_code)]
fn css_to_core_text_font_weight(css_weight: Weight) -> f32 {
    piecewise_linear_lookup(f32::max(100.0, css_weight.0) / 100.0 - 1.0, &FONT_WEIGHT_MAPPING)
}

#[allow(dead_code)]
fn css_stretchiness_to_core_text_width(css_stretchiness: Stretch) -> f32 {
    let css_stretchiness = utils::clamp(css_stretchiness.0, 0.5, 2.0);
    0.25 * piecewise_linear_find_index(css_stretchiness, &FONT_STRETCH_MAPPING) - 1.0
}

fn fonts_in_core_text_collection(collection: CTFontCollection) -> Vec<Font> {
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
    fonts
}

#[cfg(test)]
mod test {
    use descriptor::{Stretch, Weight};

    #[test]
    fn test_css_to_core_text_font_weight() {
        // Exact matches
        assert_eq!(super::css_to_core_text_font_weight(Weight(100.0)), -0.7);
        assert_eq!(super::css_to_core_text_font_weight(Weight(400.0)), 0.0);
        assert_eq!(super::css_to_core_text_font_weight(Weight(700.0)), 0.4);
        assert_eq!(super::css_to_core_text_font_weight(Weight(900.0)), 0.8);

        // Linear interpolation
        assert_eq!(super::css_to_core_text_font_weight(Weight(450.0)), 0.1);
    }

    #[test]
    fn test_css_to_core_text_font_stretch() {
        // Exact matches
        assert_eq!(super::css_stretchiness_to_core_text_width(Stretch(1.0)), 0.0);
        assert_eq!(super::css_stretchiness_to_core_text_width(Stretch(0.5)), -1.0);
        assert_eq!(super::css_stretchiness_to_core_text_width(Stretch(2.0)), 1.0);

        // Linear interpolation
        assert_eq!(super::css_stretchiness_to_core_text_width(Stretch(1.7)), 0.85);
    }
}
