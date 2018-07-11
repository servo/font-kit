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
use family::Family;
use font::Font;

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
    // TODO(pcwalton): Make this a default impl that redirects to `select_with_loader`.
    fn all_families(&self) -> Vec<String>;

    fn select_family(&self, family_name: &str) -> Family;

    // FIXME(pcwalton): This should be private, because it only returns one family for the generic
    // family names.
    fn select_family_spec(&self, family: &FamilySpec) -> Family {
        match *family {
            FamilySpec::Name(ref name) => self.select_family(name),
            FamilySpec::Serif => self.select_family(DEFAULT_FONT_FAMILY_SERIF),
            FamilySpec::SansSerif => self.select_family(DEFAULT_FONT_FAMILY_SANS_SERIF),
            FamilySpec::Monospace => self.select_family(DEFAULT_FONT_FAMILY_MONOSPACE),
            FamilySpec::Cursive => self.select_family(DEFAULT_FONT_FAMILY_CURSIVE),
            FamilySpec::Fantasy => self.select_family(DEFAULT_FONT_FAMILY_FANTASY),
        }
    }

    // TODO(pcwalton): Last resort font.
    fn find(&self, spec: &Spec) -> Result<Font, ()> {
        for family in &spec.families {
            if let Ok(font) = self.select_family_spec(family).find(&spec.properties) {
                return Ok(font)
            }
        }
        Err(())
    }
}
