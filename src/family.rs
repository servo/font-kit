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

use error::FontLoadingError;
use font::{Face, Font};
use handle::Handle;

#[derive(Debug)]
pub struct FamilyHandle {
    pub fonts: Vec<Handle>,
}

impl FamilyHandle {
    #[inline]
    pub fn new() -> FamilyHandle {
        FamilyHandle {
            fonts: vec![],
        }
    }

    #[inline]
    pub fn from_font_handles<I>(fonts: I) -> FamilyHandle where I: Iterator<Item = Handle> {
        FamilyHandle {
            fonts: fonts.collect::<Vec<Handle>>(),
        }
    }

    #[inline]
    pub fn push(&mut self, font: Handle) {
        self.fonts.push(font)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    #[inline]
    pub fn fonts(&self) -> &[Handle] {
        &self.fonts
    }
}

#[derive(Debug)]
pub struct Family<F = Font> where F: Face {
    pub fonts: Vec<F>,
}

impl<F> Family<F> where F: Face {
    #[inline]
    pub fn new() -> Family<F> {
        Family {
            fonts: vec![],
        }
    }

    #[inline]
    pub fn from_fonts<I>(fonts: I) -> Family<F> where I: Iterator<Item = F> {
        Family {
            fonts: fonts.collect::<Vec<F>>(),
        }
    }

    /// A convenience method to create a family with a single font.
    #[inline]
    pub fn from_font(font: F) -> Family<F> {
        Family::from_fonts(iter::once(font))
    }

    pub fn from_font_handles<'a, I>(font_handles: I) -> Result<Family<F>, FontLoadingError>
                                    where I: Iterator<Item = &'a Handle> {
        let mut fonts = vec![];
        for font_handle in font_handles {
            fonts.push(try!(F::from_handle(font_handle)))
        }
        Ok(Family {
            fonts,
        })
    }

    #[inline]
    pub fn from_handle(family_handle: &FamilyHandle) -> Result<Family<F>, FontLoadingError> {
        Family::from_font_handles(family_handle.fonts.iter())
    }

    #[inline]
    pub fn fonts(&self) -> &[F] {
        &self.fonts
    }

    #[inline]
    pub fn push(&mut self, font: F) {
        self.fonts.push(font)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
}
