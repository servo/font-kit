// font-kit/src/sources/mem.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A source that keeps fonts in memory.

use itertools::Itertools;

#[cfg(target_family = "windows")]
use std::ffi::OsString;
#[cfg(target_family = "windows")]
use std::os::windows::ffi::OsStringExt;
#[cfg(target_family = "windows")]
use winapi::shared::minwindef::{MAX_PATH, UINT};
#[cfg(target_family = "windows")]
use winapi::um::sysinfoapi;

use error::{FontLoadingError, SelectionError};
use family_handle::FamilyHandle;
use family_name::FamilyName;
use font::Font;
use handle::Handle;
use properties::Properties;
use source::Source;

/// A source that keeps fonts in memory.
pub struct MemSource {
    families: Vec<FamilyEntry>,
}

impl MemSource {
    /// Creates a new memory source that contains the given set of font handles.
    ///
    /// The fonts referenced by the handles are eagerly loaded into memory.
    pub fn from_fonts<I>(fonts: I) -> Result<MemSource, FontLoadingError>
                         where I: Iterator<Item = Handle> {
        let mut families = vec![];
        for handle in fonts {
            let font = try!(Font::from_handle(&handle));
            if let Some(postscript_name) = font.postscript_name() {
                families.push(FamilyEntry {
                    family_name: font.family_name(),
                    postscript_name: postscript_name,
                    font: handle,
                })
            }
        }
        families.sort_by(|a, b| a.family_name.cmp(&b.family_name));
        Ok(MemSource {
            families,
        })
    }

    /// Returns paths of all fonts installed on the system.
    pub fn all_fonts(&self) -> Result<Vec<Handle>, SelectionError> {
        Ok(self.families
            .iter()
            .map(|family| family.font.clone())
            .collect())
    }

    /// Returns the names of all families installed on the system.
    pub fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        Ok(self.families
               .iter()
               .map(|family| &*family.family_name)
               .dedup()
               .map(|name| name.to_owned())
               .collect())
    }

    /// Looks up a font family by name and returns the handles of all the fonts in that family.
    ///
    /// FIXME(pcwalton): Case-insensitive comparison.
    pub fn select_family_by_name(&self, family_name: &str)
                                 -> Result<FamilyHandle, SelectionError> {
        let mut first_family_index = try!(self.families.binary_search_by(|family| {
            (&*family.family_name).cmp(family_name)
        }).map_err(|_| SelectionError::NotFound));

        while first_family_index > 0 &&
                self.families[first_family_index - 1].family_name == family_name {
            first_family_index -= 1
        }
        let mut last_family_index = first_family_index;
        while last_family_index + 1 < self.families.len() &&
                self.families[last_family_index + 1].family_name == family_name {
            last_family_index += 1
        }

        let families = &self.families[first_family_index..(last_family_index + 1)];
        Ok(FamilyHandle::from_font_handles(families.iter().map(|family| family.font.clone())))
    }

    /// Selects a font by PostScript name, which should be a unique identifier.
    ///
    /// The default implementation, which is used by the DirectWrite and the filesystem backends,
    /// does a brute-force search of installed fonts to find the one that matches.
    pub fn select_by_postscript_name(&self, postscript_name: &str)
                                     -> Result<Handle, SelectionError> {
        self.families
            .iter()
            .filter(|family_entry| family_entry.postscript_name == postscript_name)
            .map(|family_entry| family_entry.font.clone())
            .next()
            .ok_or(SelectionError::NotFound)
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and returns the
    /// handle.
    #[inline]
    pub fn select_best_match(&self, family_names: &[FamilyName], properties: &Properties)
                             -> Result<Handle, SelectionError> {
        <Self as Source>::select_best_match(self, family_names, properties)
    }
}

impl Source for MemSource {
    #[inline]
    fn all_fonts(&self) -> Result<Vec<Handle>, SelectionError> {
        self.all_fonts()
    }

    #[inline]
    fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.all_families()
    }

    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        self.select_family_by_name(family_name)
    }

    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<Handle, SelectionError> {
        self.select_by_postscript_name(postscript_name)
    }
}

struct FamilyEntry {
    family_name: String,
    postscript_name: String,
    font: Handle,
}
