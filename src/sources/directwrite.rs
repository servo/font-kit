// font-kit/src/sources/directwrite.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A source that contains the installed fonts on Windows.

use dwrote::Font as DWriteFont;
use dwrote::FontCollection as DWriteFontCollection;
use dwrote::FontSimulations as DWriteFontSimulations;
use dwrote::FontStyle as DWriteFontStyle;
use dwrote::InformationalStringId as DWriteInformationalStringId;
use std::ops::Deref;
use std::sync::{Arc, MutexGuard};

use error::SelectionError;
use family::Family;
use family_handle::FamilyHandle;
use family_name::FamilyName;
use font::Font;
use handle::Handle;
use properties::Properties;
use source::Source;

/// A source that contains the installed fonts on Windows.
pub struct DirectWriteSource {
    system_font_collection: DWriteFontCollection,
}

impl DirectWriteSource {
    pub fn new() -> DirectWriteSource {
        DirectWriteSource {
            system_font_collection: DWriteFontCollection::system(),
        }
    }

    pub fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        Ok(self.system_font_collection
               .families_iter()
               .map(|dwrite_family| dwrite_family.name())
               .collect())
    }

    // TODO(pcwalton): Case-insensitivity.
    pub fn select_family_by_name(&self, family_name: &str)
                                 -> Result<FamilyHandle, SelectionError> {
        let mut family = FamilyHandle::new();
        let dwrite_family = match self.system_font_collection
                                      .get_font_family_by_name(family_name) {
            Some(dwrite_family) => dwrite_family,
            None => return Err(SelectionError::NotFound),
        };
        for font_index in 0..dwrite_family.get_font_count() {
            unsafe {
                let dwrite_font = dwrite_family.get_font(font_index);
                family.push(self.create_handle_from_dwrite_font(dwrite_font))
            }
        }
        Ok(family)
    }

    pub fn select_by_postscript_name(&self, postscript_name: &str)
                                     -> Result<Handle, SelectionError> {
        <Self as Source>::select_by_postscript_name(self, postscript_name)
    }

    #[inline]
    pub fn select_best_match(&self, family_names: &[FamilyName], properties: &Properties)
                             -> Result<Handle, SelectionError> {
        <Self as Source>::select_best_match(self, family_names, properties)
    }

    fn create_handle_from_dwrite_font(&self, dwrite_font: DWriteFont) -> Handle {
        let dwrite_font_face = dwrite_font.create_font_face();
        let dwrite_font_files = dwrite_font_face.get_files();
        Handle::Path {
            path: dwrite_font_files[0].get_font_file_path().unwrap(),
            font_index: dwrite_font_face.get_index()
        }
    }
}

impl Source for DirectWriteSource {
    #[inline]
    fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.all_families()
    }

    #[inline]
    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        self.select_family_by_name(family_name)
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

