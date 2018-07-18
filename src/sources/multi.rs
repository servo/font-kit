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
use error::SelectionError;
use family::Family;
use font::Face;
use source::Source;

/// FIXME(pcwalton): This API is rather unwieldy for more than two sources. Thankfully, I think
/// only two sources should be needed in most cases.
pub struct MultiSource<SA, SB> where SA: Source, SB: Source {
    subsources: (SA, SB),
}

impl<SA, SB> MultiSource<SA, SB> where SA: Source, SB: Source {
    pub fn from_sources<SAX, SBX>(source_a: SAX, source_b: SBX) -> MultiSource<SAX, SBX>
                                  where SAX: Source, SBX: Source {
        MultiSource {
            subsources: (source_a, source_b)
        }
    }

    pub fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        let mut families = try!(self.subsources.0.all_families());
        families.append(&mut try!(self.subsources.1.all_families()));
        Ok(families)
    }

    // FIXME(pcwalton): Case-insensitive comparison.
    pub fn select_family_with_loader<F>(&self, family_name: &str)
                                        -> Result<Family<F>, SelectionError>
                                        where F: Face {
        match self.subsources.0.select_family_with_loader(family_name) {
            Ok(family) => Ok(family),
            Err(SelectionError::NotFound) => {
                self.subsources.1.select_family_with_loader(family_name)
            }
            Err(err) => Err(err),
        }
    }

    pub fn find_by_postscript_name_with_loader<F>(&self, postscript_name: &str)
                                                  -> Result<F, SelectionError>
                                                  where F: Face {
        match self.subsources.0.find_by_postscript_name_with_loader(postscript_name) {
            Ok(family) => Ok(family),
            Err(SelectionError::NotFound) => {
                self.subsources.1.find_by_postscript_name_with_loader(postscript_name)
            }
            Err(err) => Err(err),
        }
    }

    pub fn find_with_loader<F>(&self, spec: &Spec) -> Result<F, SelectionError> where F: Face {
        <Self as Source>::find_with_loader(self, spec)
    }
}

impl<SA, SB> Source for MultiSource<SA, SB> where SA: Source, SB: Source {
    #[inline]
    fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.all_families()
    }

    fn select_family_with_loader<F>(&self, family_name: &str) -> Result<Family<F>, SelectionError>
                                    where F: Face {
        self.select_family_with_loader(family_name)
    }

    fn find_by_postscript_name_with_loader<F>(&self, postscript_name: &str)
                                              -> Result<F, SelectionError>
                                              where F: Face {
        self.find_by_postscript_name_with_loader(postscript_name)
    }
}
