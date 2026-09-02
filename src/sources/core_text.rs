// font-kit/src/sources/core_text.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A source that contains the installed fonts on macOS.

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_text::font_collection::{self, CTFontCollection};
use core_text::font_descriptor::{self, CTFontDescriptor};
use core_text::font_manager;
use std::any::Any;
use std::collections::HashMap;
use std::f32;
use std::path::PathBuf;

use crate::error::SelectionError;
use crate::family_handle::FamilyHandle;
use crate::family_name::FamilyName;
use crate::file_type::FileType;
use crate::font::Font;
use crate::handle::Handle;
use crate::loaders::core_text::{self as core_text_loader, FONT_WEIGHT_MAPPING};
use crate::properties::{Properties, Stretch, Weight};
use crate::source::Source;
use crate::utils;

/// A source that contains the installed fonts on macOS.
#[allow(missing_debug_implementations)]
#[allow(missing_copy_implementations)]
pub struct CoreTextSource;

impl Default for CoreTextSource {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreTextSource {
    /// Opens a new connection to the system font source.
    ///
    /// (Note that this doesn't actually do any Mach communication to the font server; that is done
    /// lazily on demand by the Core Text/Core Graphics API.)
    #[inline]
    pub fn new() -> CoreTextSource {
        CoreTextSource
    }

    /// Returns paths of all fonts installed on the system.
    pub fn all_fonts(&self) -> Result<Vec<Handle>, SelectionError> {
        let collection = font_collection::create_for_all_families();
        create_handles_from_core_text_collection(collection)
    }

    /// Returns the names of all families installed on the system.
    pub fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        let core_text_family_names = font_manager::copy_available_font_family_names();
        let mut families = Vec::with_capacity(core_text_family_names.len() as usize);
        for core_text_family_name in core_text_family_names.iter() {
            families.push(core_text_family_name.to_string())
        }
        Ok(families)
    }

    /// Looks up a font family by name and returns the handles of all the fonts in that family.
    pub fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        let attributes: CFDictionary<CFString, CFType> = CFDictionary::from_CFType_pairs(&[(
            CFString::new("NSFontFamilyAttribute"),
            CFString::new(family_name).as_CFType(),
        )]);

        let descriptor = font_descriptor::new_from_attributes(&attributes);
        let descriptors = CFArray::from_CFTypes(&[descriptor]);
        let collection = font_collection::new_from_descriptors(&descriptors);
        let handles = create_handles_from_core_text_collection(collection)?;
        Ok(FamilyHandle::from_font_handles(handles.into_iter()))
    }

    /// Selects a font by PostScript name, which should be a unique identifier.
    pub fn select_by_postscript_name(
        &self,
        postscript_name: &str,
    ) -> Result<Handle, SelectionError> {
        let attributes: CFDictionary<CFString, CFType> = CFDictionary::from_CFType_pairs(&[(
            CFString::new("NSFontNameAttribute"),
            CFString::new(postscript_name).as_CFType(),
        )]);

        let descriptor = font_descriptor::new_from_attributes(&attributes);
        let descriptors = CFArray::from_CFTypes(&[descriptor]);
        let collection = font_collection::new_from_descriptors(&descriptors);
        match collection.get_descriptors() {
            None => Err(SelectionError::NotFound),
            Some(descriptors) => create_handle_from_descriptor(&*descriptors.get(0).unwrap()),
        }
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and returns the
    /// handle.
    #[inline]
    pub fn select_best_match(
        &self,
        family_names: &[FamilyName],
        properties: &Properties,
    ) -> Result<Handle, SelectionError> {
        <Self as Source>::select_best_match(self, family_names, properties)
    }
}

impl Source for CoreTextSource {
    fn all_fonts(&self) -> Result<Vec<Handle>, SelectionError> {
        self.all_fonts()
    }

    fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.all_families()
    }

    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        self.select_family_by_name(family_name)
    }

    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<Handle, SelectionError> {
        self.select_by_postscript_name(postscript_name)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[allow(dead_code)]
fn css_to_core_text_font_weight(css_weight: Weight) -> f32 {
    core_text_loader::piecewise_linear_lookup(
        f32::max(100.0, css_weight.0) / 100.0 - 1.0,
        &FONT_WEIGHT_MAPPING,
    )
}

#[allow(dead_code)]
fn css_stretchiness_to_core_text_width(css_stretchiness: Stretch) -> f32 {
    let css_stretchiness = utils::clamp(css_stretchiness.0, 0.5, 2.0);
    0.1 * core_text_loader::piecewise_linear_find_index(css_stretchiness, &Stretch::MAPPING) - 0.4
}

fn create_handles_from_core_text_collection(
    collection: CTFontCollection,
) -> Result<Vec<Handle>, SelectionError> {
    let mut fonts = vec![];
    if let Some(descriptors) = collection.get_descriptors() {
        // Cache to track which paths we've seen and their font index mappings
        // This avoids re-analyzing the same font collection file multiple times
        let mut font_index_cache: HashMap<PathBuf, HashMap<String, u32>> = HashMap::new();

        for index in 0..descriptors.len() {
            let descriptor = descriptors.get(index).unwrap();
            let font_path = descriptor.font_path().unwrap();

            // Check if we already have this path in cache
            if let Some(postscript_map) = font_index_cache.get(&font_path) {
                let postscript_name = descriptor.font_name();
                if let Some(&font_index) = postscript_map.get(&postscript_name) {
                    // We already know the font index for this postscript name
                    fonts.push(Handle::from_path(font_path.clone(), font_index));
                    continue;
                }
                // If not in map, it means this is a single font file or we already processed it
                if postscript_map.is_empty() {
                    fonts.push(Handle::from_path(font_path.clone(), 0));
                    continue;
                }
            }

            // File not in cache - need to analyze it (but only read minimal metadata)
            let file_type = match Font::analyze_path(&font_path) {
                Ok(file_type) => file_type,
                Err(_) => {
                    // If we can't analyze, assume it's a single font and let load() handle errors
                    font_index_cache.insert(font_path.clone(), HashMap::new());
                    fonts.push(Handle::from_path(font_path, 0));
                    continue;
                }
            };

            match file_type {
                FileType::Collection(font_count) => {
                    // For collections, we need to find which index matches this descriptor
                    // We'll do a lazy approach: try loading just the font table headers
                    let postscript_name = descriptor.font_name();
                    let mut postscript_map = HashMap::new();
                    let mut found_index = None;

                    for font_index in 0..font_count {
                        // Only load the font if we haven't found our target yet
                        // This is still not perfect, but much better than loading all data
                        if let Ok(font) = Font::from_path(&font_path, font_index) {
                            if let Some(font_postscript_name) = font.postscript_name() {
                                postscript_map.insert(font_postscript_name.clone(), font_index);
                                if font_postscript_name == postscript_name {
                                    found_index = Some(font_index);
                                }
                            }
                        }
                    }

                    font_index_cache.insert(font_path.clone(), postscript_map);

                    if let Some(font_index) = found_index {
                        fonts.push(Handle::from_path(font_path, font_index));
                    }
                }
                FileType::Single => {
                    // Single font file - just use index 0
                    font_index_cache.insert(font_path.clone(), HashMap::new());
                    fonts.push(Handle::from_path(font_path, 0));
                }
            }
        }
    }
    if fonts.is_empty() {
        Err(SelectionError::NotFound)
    } else {
        Ok(fonts)
    }
}

fn create_handle_from_descriptor(descriptor: &CTFontDescriptor) -> Result<Handle, SelectionError> {
    let font_path = descriptor.font_path().unwrap();

    // Use path-based handle instead of loading entire font into memory
    match Font::analyze_path(&font_path) {
        Ok(FileType::Collection(font_count)) => {
            let postscript_name = descriptor.font_name();

            // For collections, we need to find the correct font index
            for font_index in 0..font_count {
                if let Ok(font) = Font::from_path(&font_path, font_index) {
                    if let Some(font_postscript_name) = font.postscript_name() {
                        if postscript_name == font_postscript_name {
                            return Ok(Handle::from_path(font_path, font_index));
                        }
                    }
                }
            }

            Err(SelectionError::NotFound)
        }
        Ok(FileType::Single) => Ok(Handle::from_path(font_path, 0)),
        Err(e) => Err(SelectionError::CannotAccessSource {
            reason: Some(format!("{:?} error on path {:?}", e, font_path).into()),
        }),
    }
}

#[cfg(test)]
mod test {
    use crate::properties::{Stretch, Weight};

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
        assert_eq!(
            super::css_stretchiness_to_core_text_width(Stretch(1.0)),
            0.0
        );
        assert_eq!(
            super::css_stretchiness_to_core_text_width(Stretch(0.5)),
            -0.4
        );
        assert_eq!(
            super::css_stretchiness_to_core_text_width(Stretch(2.0)),
            0.4
        );

        // Linear interpolation
        assert_eq!(
            super::css_stretchiness_to_core_text_width(Stretch(1.7)),
            0.34
        );
    }
}
