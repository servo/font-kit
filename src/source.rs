// font-kit/src/source.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use descriptor::{FamilySpec, Spec};
use error::SelectionError;
use family::Family;
use font::{Face, Font};

#[cfg(all(target_os = "macos", not(feature = "source-fontconfig-default")))]
pub use sources::core_text::CoreTextSource as SystemSource;
#[cfg(all(target_family = "windows", not(feature = "source-fontconfig-default")))]
pub use sources::directwrite::DirectWriteSource as SystemSource;
#[cfg(any(not(any(target_os = "android", target_os = "macos", target_family = "windows")),
          feature = "source-fontconfig-default"))]
pub use sources::fontconfig::FontconfigSource as SystemSource;
#[cfg(all(target_os = "android", not(feature = "source-fontconfig-default")))]
pub use sources::fs::FsSource as SystemSource;

// FIXME(pcwalton): These could expand to multiple fonts, and they could be language-specific.
const DEFAULT_FONT_FAMILY_SERIF: &'static str = "Times New Roman";
const DEFAULT_FONT_FAMILY_SANS_SERIF: &'static str = "Arial";
const DEFAULT_FONT_FAMILY_MONOSPACE: &'static str = "Courier New";
const DEFAULT_FONT_FAMILY_CURSIVE: &'static str = "Comic Sans MS";
const DEFAULT_FONT_FAMILY_FANTASY: &'static str = "Papyrus";

pub trait Source {
    fn all_families(&self) -> Result<Vec<String>, SelectionError>;

    fn select_family(&self, family_name: &str) -> Result<Family, SelectionError> {
        self.select_family_with_loader(family_name)
    }

    fn select_family_with_loader<F>(&self, family_name: &str) -> Result<Family<F>, SelectionError>
                                    where F: Face;

    // FIXME(pcwalton): This only returns one family instead of multiple families for the generic
    // family names.
    #[doc(hidden)]
    fn select_family_spec_with_loader<F>(&self, family: &FamilySpec)
                                         -> Result<Family<F>, SelectionError>
                                         where F: Face {
        match *family {
            FamilySpec::Name(ref name) => self.select_family_with_loader(name),
            FamilySpec::Serif => self.select_family_with_loader(DEFAULT_FONT_FAMILY_SERIF),
            FamilySpec::SansSerif => {
                self.select_family_with_loader(DEFAULT_FONT_FAMILY_SANS_SERIF)
            }
            FamilySpec::Monospace => self.select_family_with_loader(DEFAULT_FONT_FAMILY_MONOSPACE),
            FamilySpec::Cursive => self.select_family_with_loader(DEFAULT_FONT_FAMILY_CURSIVE),
            FamilySpec::Fantasy => self.select_family_with_loader(DEFAULT_FONT_FAMILY_FANTASY),
        }
    }

    #[inline]
    fn find(&self, spec: &Spec) -> Result<Font, SelectionError> {
        self.find_with_loader(spec)
    }

    // TODO(pcwalton): Last resort font.
    fn find_with_loader<F>(&self, spec: &Spec) -> Result<F, SelectionError> where F: Face {
        for family in &spec.families {
            if let Ok(family) = self.select_family_spec_with_loader(family) {
                if let Ok(font) = family.find(&spec.properties) {
                    return Ok(font)
                }
            }
        }
        Err(SelectionError::NotFound)
    }

    #[inline]
    fn find_by_postscript_name(&self, postscript_name: &str) -> Result<Font, SelectionError> {
        self.find_by_postscript_name_with_loader(postscript_name)
    }

    /// The default implementation, which is used by the DirectWrite and the filesystem backends,
    /// does a brute-force search of installed fonts to find the one that matches.
    fn find_by_postscript_name_with_loader<F>(&self, postscript_name: &str)
                                              -> Result<F, SelectionError>
                                              where F: Face {
        // TODO(pcwalton): Optimize this by searching for families with similar names first.
        for family_name in try!(self.all_families()) {
            if let Ok(family) = self.select_family_with_loader(&family_name) {
                for font in family.fonts() {
                    // Have to help type inference along…
                    let font: &F = font;
                    if font.postscript_name() == postscript_name {
                        return Ok((*font).clone())
                    }
                }
            }
        }
        Err(SelectionError::NotFound)
    }
}
