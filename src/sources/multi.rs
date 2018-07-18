// font-kit/src/sources/multi.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A source that encapsulates multiple sources.

use descriptor::Spec;
use family::Family;
use font::Font;
use source::Source;

pub struct MultiSource {
    subsources: Vec<Box<Source>>,
}

impl MultiSource {
    pub fn from_sources<I>(subsources: I) -> MultiSource where I: Iterator<Item = Box<Source>> {
        MultiSource {
            subsources: subsources.collect(),
        }
    }

    pub fn all_families(&self) -> Vec<String> {
        let mut families = vec![];
        for subsource in &self.subsources {
            families.append(&mut subsource.all_families())
        }
        families
    }

    // FIXME(pcwalton): Case-insensitive comparison.
    pub fn select_family(&self, family_name: &str) -> Family {
        for subsource in &self.subsources {
            let family = subsource.select_family(family_name);
            if !family.is_empty() {
                return family
            }
        }
        Family::new()
    }

    pub fn find_by_postscript_name(&self, postscript_name: &str) -> Result<Font, ()> {
        for subsource in &self.subsources {
            if let Ok(font) = subsource.find_by_postscript_name(postscript_name) {
                return Ok(font)
            }
        }
        Err(())
    }

    pub fn find(&self, spec: &Spec) -> Result<Font, ()> {
        <Self as Source>::find(self, spec)
    }
}

impl Source for MultiSource {
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
