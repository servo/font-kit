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

use descriptor::Spec;
use family::Family;
use font::Font;
use source::Source;

pub struct MemSource {
    families: Vec<FamilyEntry>,
}

impl MemSource {
    pub fn from_fonts<I>(fonts: I) -> MemSource where I: Iterator<Item = Font> {
        let mut families: Vec<_> = fonts.map(|font| {
            let family_name = font.family_name();
            FamilyEntry {
                family_name,
                font,
            }
        }).collect();
        families.sort_by(|a, b| a.family_name.cmp(&b.family_name));
        MemSource {
            families,
        }
    }

    pub fn all_families(&self) -> Vec<String> {
        self.families
            .iter()
            .map(|family| &*family.family_name)
            .dedup()
            .map(|name| name.to_owned())
            .collect()
    }

    // FIXME(pcwalton): Case-insensitive comparison.
    pub fn select_family(&self, family_name: &str) -> Family {
        let mut first_family_index = match self.families.binary_search_by(|family| {
            (&*family.family_name).cmp(family_name)
        }) {
            Err(_) => return Family::new(),
            Ok(family_index) => family_index,
        };
        while first_family_index > 0 &&
                self.families[first_family_index - 1].family_name == family_name {
            first_family_index -= 1
        }
        let mut last_family_index = first_family_index;
        while last_family_index + 1 < self.families.len() &&
                self.families[last_family_index + 1].family_name == family_name {
            last_family_index += 1
        }
        Family::from_fonts(self.families[first_family_index..(last_family_index + 1)]
                               .iter()
                               .map(|family| family.font.clone()))
    }

    pub fn find_by_postscript_name(&self, postscript_name: &str) -> Result<Font, ()> {
        self.families
            .iter()
            .filter(|family_entry| family_entry.font.postscript_name() == postscript_name)
            .map(|family_entry| family_entry.font.clone())
            .next()
            .ok_or(())
    }

    pub fn find(&self, spec: &Spec) -> Result<Font, ()> {
        <Self as Source>::find(self, spec)
    }
}

impl Source for MemSource {
    #[inline]
    fn all_families(&self) -> Vec<String> {
        self.all_families()
    }

    fn select_family(&self, family_name: &str) -> Family {
        self.select_family(family_name)
    }

    fn find_by_postscript_name(&self, postscript_name: &str) -> Result<Font, ()> {
        self.find_by_postscript_name(postscript_name)
    }
}

struct FamilyEntry {
    family_name: String,
    font: Font,
}
