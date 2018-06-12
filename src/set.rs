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

use family::Family;

#[derive(Debug)]
pub struct Set {
    families: Vec<Family>,
}

impl Set {
    pub fn from_families<I>(families: I) -> Set where I: Iterator<Item = Family> {
        Set {
            families: families.collect(),
        }
    }

    pub fn families(&self) -> &[Family] {
        &self.families
    }
}
