// font-kit/src/sources/directwrite.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dwrote::Font as DWriteFont;
use dwrote::FontCollection as DWriteFontCollection;
use dwrote::FontSimulations as DWriteFontSimulations;
use dwrote::FontStyle as DWriteFontStyle;
use dwrote::InformationalStringId as DWriteInformationalStringId;
use std::ops::Deref;
use std::sync::{Arc, MutexGuard};

use descriptor::{FONT_STRETCH_MAPPING, Spec};
use family::Family;
use font::Font;
use source::Source;

pub struct DirectWriteSource {
    system_font_collection: DWriteFontCollection,
}

impl DirectWriteSource {
    pub fn new() -> DirectWriteSource {
        DirectWriteSource {
            system_font_collection: DWriteFontCollection::system(),
        }
    }

    pub fn all_families(&self) -> Vec<String> {
        self.system_font_collection
            .families_iter()
            .map(|dwrite_family| dwrite_family.name())
            .collect()
    }

    // TODO(pcwalton): Case-insensitivity.
    pub fn select_family(&self, family_name: &str) -> Family {
        let mut family = Family::new();
        let dwrite_family = match self.system_font_collection
                                      .get_font_family_by_name(family_name) {
            Some(dwrite_family) => dwrite_family,
            None => return family,
        };
        for font_index in 0..dwrite_family.get_font_count() {
            unsafe {
                let dwrite_font = dwrite_family.get_font(font_index);
                family.push(Font::from_native_font(dwrite_font.create_font_face()))
            }
        }
        family
    }

    pub fn find(&self, spec: &Spec) -> Result<Font, ()> {
        <Self as Source>::find(self, spec)
    }
}

impl Source for DirectWriteSource {
    #[inline]
    fn all_families(&self) -> Vec<String> {
        self.all_families()
    }

    #[inline]
    fn select_family(&self, family_name: &str) -> Family {
        self.select_family(family_name)
    }
}

pub struct FontData<'a> {
    font_data: MutexGuard<'a, Option<Arc<Vec<u8>>>>,
}

impl<'a> Deref for FontData<'a> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        &***self.font_data.as_ref().unwrap()
    }
}

fn style_name_for_dwrite_style(style: DWriteFontStyle) -> &'static str {
    match style {
        DWriteFontStyle::Normal => "Regular",
        DWriteFontStyle::Oblique => "Oblique",
        DWriteFontStyle::Italic => "Italic",
    }
}

fn dwrite_style_is_italic(style: DWriteFontStyle) -> bool {
    match style {
        DWriteFontStyle::Normal => false,
        DWriteFontStyle::Oblique | DWriteFontStyle::Italic => true,
    }
}

