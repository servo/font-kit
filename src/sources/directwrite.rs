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

use descriptor::{Flags, FONT_STRETCH_MAPPING, Query, QueryFields};
use family::Family;
use font::Font;
use set::Set;

pub struct Source {
    system_font_collection: DWriteFontCollection,
}

impl Source {
    pub fn new() -> Source {
        Source {
            system_font_collection: DWriteFontCollection::system(),
        }
    }

    pub fn select(&self, query: &Query) -> Set {
        let mut set = Set::new();
        for dwrite_family in self.system_font_collection.families_iter() {
            let mut family = Family::new();
            for font_index in 0..dwrite_family.get_font_count() {
                unsafe {
                    let dwrite_font = dwrite_family.get_font(font_index);
                    if query.matches_dwrite_font(&dwrite_font) {
                        family.push(Font::from_native_font(dwrite_font.create_font_face()))
                    }
                }
            }
            if !family.fonts().is_empty() {
                set.push(family)
            }
        }
        set
    }
}

impl Query {
    fn matches_dwrite_font(&self, dwrite_font: &DWriteFont) -> bool {
        if dwrite_font.simulations() != DWriteFontSimulations::None {
            return false
        }

        if self.fields.contains(QueryFields::POSTSCRIPT_NAME) &&
                !self.matches_informational_string(dwrite_font,
                                                   &self.descriptor.postscript_name,
                                                   DWriteInformationalStringId::PostscriptName) {
            return false
        }
        if self.fields.contains(QueryFields::DISPLAY_NAME) &&
                !self.matches_informational_string(dwrite_font,
                                                   &self.descriptor.display_name,
                                                   DWriteInformationalStringId::FullName) {
            return false
        }
        if self.fields.contains(QueryFields::FAMILY_NAME) &&
                dwrite_font.family_name() != self.descriptor.family_name {
            return false
        }
        if self.fields.contains(QueryFields::STYLE_NAME) &&
                style_name_for_dwrite_style(dwrite_font.style()) != self.descriptor.style_name {
            return false
        }
        if self.fields.contains(QueryFields::WEIGHT) &&
                dwrite_font.weight() as u32 as f32 != self.descriptor.weight {
            return false
        }
        if self.fields.contains(QueryFields::STRETCH) &&
                FONT_STRETCH_MAPPING[(dwrite_font.stretch() as usize) - 1] !=
                self.descriptor.stretch {
            return false
        }
        if self.fields.contains(QueryFields::ITALIC) &&
                dwrite_style_is_italic(dwrite_font.style()) !=
                self.descriptor.flags.contains(Flags::ITALIC) {
            return false
        }
        // TODO(pcwalton): Monospace, once we have a `winapi` upgrade.
        // FIXME(pcwalton): How do we identify vertical fonts?
        true
    }

    fn matches_informational_string(&self,
                                    dwrite_font: &DWriteFont,
                                    query_name: &str,
                                    id: DWriteInformationalStringId)
                                    -> bool {
        match dwrite_font.informational_string(id) {
            None => false,
            Some(name) => name == query_name,
        }
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

