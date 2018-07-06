// font-kit/src/set.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A collection of fonts.

use std::collections::HashMap;

use descriptor::Query;
use family::Family;
use font::Font;

#[derive(Debug)]
pub struct Set {
    families: Vec<Family>,
}

impl Set {
    pub fn new() -> Set {
        Set {
            families: vec![],
        }
    }

    pub fn from_families<I>(families: I) -> Set where I: Iterator<Item = Family> {
        Set {
            families: families.collect(),
        }
    }

    /// Creates a set from a group of fonts. The fonts are automatically sorted into families.
    pub fn from_fonts<I>(fonts: I) -> Set where I: Iterator<Item = Font> {
        let mut families = HashMap::new();
        for font in fonts {
            families.entry(font.descriptor().family_name)
                    .or_insert_with(|| Family::new())
                    .push(font)
        }
        Set::from_families(families.into_iter().map(|(_, family)| family))
    }

    pub fn families(&self) -> &[Family] {
        &self.families
    }

    pub fn push(&mut self, family: Family) {
        self.families.push(family)
    }

    pub fn filter(&mut self, query: &Query) {
        for family in &mut self.families {
            family.filter(query)
        }
        self.families.retain(|family| !family.is_empty())
    }
}
