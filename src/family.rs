// font-kit/src/family.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::iter;

use font::Font;

#[derive(Debug)]
pub struct Family {
    pub fonts: Vec<Font>,
}

impl Family {
    #[inline]
    pub fn from_fonts<I>(fonts: I) -> Family where I: Iterator<Item = Font> {
        Family {
            fonts: fonts.collect(),
        }
    }

    /// A convenience method to create a family with a single font.
    #[inline]
    pub fn from_font(font: Font) -> Family {
        Family::from_fonts(iter::once(font))
    }

    #[inline]
    pub fn fonts(&self) -> &[Font] {
        &self.fonts
    }
}
