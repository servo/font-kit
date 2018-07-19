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
use family::{Family, FamilyHandle};
use font::{Face, Font};
use handle::Handle;
use loaders;

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

    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError>;

    /// The default implementation, which is used by the DirectWrite and the filesystem backends,
    /// does a brute-force search of installed fonts to find the one that matches.
    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<Handle, SelectionError> {
        // TODO(pcwalton): Optimize this by searching for families with similar names first.
        for family_name in try!(self.all_families()) {
            if let Ok(family_handle) = self.select_family_by_name(&family_name) {
                if let Ok(family) = Family::<Font>::from_handle(&family_handle) {
                    for (handle, font) in family_handle.fonts().iter().zip(family.fonts().iter()) {
                        if font.postscript_name() == postscript_name {
                            return Ok((*handle).clone())
                        }
                    }
                }
            }
        }
        Err(SelectionError::NotFound)
    }

    // FIXME(pcwalton): This only returns one family instead of multiple families for the generic
    // family names.
    #[doc(hidden)]
    fn select_family_by_spec(&self, family: &FamilySpec) -> Result<FamilyHandle, SelectionError> {
        match *family {
            FamilySpec::Name(ref name) => self.select_family_by_name(name),
            FamilySpec::Serif => self.select_family_by_name(DEFAULT_FONT_FAMILY_SERIF),
            FamilySpec::SansSerif => self.select_family_by_name(DEFAULT_FONT_FAMILY_SANS_SERIF),
            FamilySpec::Monospace => self.select_family_by_name(DEFAULT_FONT_FAMILY_MONOSPACE),
            FamilySpec::Cursive => self.select_family_by_name(DEFAULT_FONT_FAMILY_CURSIVE),
            FamilySpec::Fantasy => self.select_family_by_name(DEFAULT_FONT_FAMILY_FANTASY),
        }
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and loads the font
    /// using the default loader.
    #[inline]
    fn find(&self, spec: &Spec) -> Result<Font, SelectionError> {
        find_with_loader::<_, Font>(self, spec)
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and loads the font
    /// using the Core Text loader.
    #[inline]
    #[cfg(all(target_os = "macos"))]
    fn find_with_core_text_loader(&self, spec: &Spec) -> Result<Font, SelectionError> {
        find_with_loader::<_, loaders::core_text::Font>(self, spec)
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and loads the font
    /// using the DirectWrite loader.
    #[inline]
    #[cfg(all(target_family = "windows"))]
    fn find_with_directwrite_loader(&self, spec: &Spec) -> Result<Font, SelectionError> {
        find_with_loader::<_, loaders::directwrite::Font>(self, spec)
    }

    /// Performs font matching according to the CSS Fonts Level 3 specification and loads the font
    /// using the FreeType loader.
    #[inline]
    #[cfg(any(not(any(target_os = "macos", target_family = "windows")),
              feature = "loader-freetype"))]
    fn find_with_freetype_loader(&self, spec: &Spec) -> Result<Font, SelectionError> {
        find_with_loader::<_, loaders::freetype::Font>(self, spec)
    }
}

// Performs font matching according to the CSS Fonts Level 3 specification, and loads the font
// using the supplied loader.
fn find_with_loader<S, F>(source: &S, spec: &Spec) -> Result<F, SelectionError>
                          where S: ?Sized + Source, F: Face {
    for family in &spec.families {
        if let Ok(family_handle) = source.select_family_by_spec(family) {
            if let Ok(family) = Family::from_handle(&family_handle) {
                if let Ok(font) = family.find(&spec.properties) {
                    return Ok(font)
                }
            }
        }
    }
    Err(SelectionError::NotFound)
}
