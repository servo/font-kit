// font-kit/src/lib.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! `font-kit` provides a common interface to the various system font libraries and provides
//! services such as finding fonts on the system, performing nearest-font matching, and rasterizing
//! glyphs.
//!
//! ## Synopsis
//!
//! ```rust
//! # extern crate euclid;
//! # extern crate font_kit;
//! #
//!
//!    use euclid::{Point2D, Size2D};
//!    use font_kit::{
//!        Canvas, FamilyName, Format, HintingOptions, Properties, RasterizationOptions,
//!        SystemSource,
//!    };
//!
//!    let font = SystemSource::new()
//!        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
//!        .unwrap()
//!        .load()
//!        .unwrap();
//!    let glyph_id = font.glyph_for_char('A').unwrap();
//!    let mut canvas = Canvas::new(&Size2D::new(32, 32), Format::A8);
//!    font.rasterize_glyph(
//!        &mut canvas,
//!        glyph_id,
//!        32.0,
//!        &Point2D::zero(),
//!        HintingOptions::None,
//!        RasterizationOptions::GrayscaleAa,
//!    )
//! ```
//!
//! ## Backends
//!
//! `font-kit` delegates to system libraries to perform tasks. It has two types of backends: a
//! *source* and a *loader*. Sources are platform font databases; they allow lookup of installed
//! fonts by name or attributes. Loaders are font loading libraries; they allow font files (TTF,
//! OTF, etc.) to be loaded from a file on disk or from bytes in memory. Sources and loaders can be
//! freely intermixed at runtime; fonts can be looked up via DirectWrite and rendered via FreeType,
//! for example.
//!
//! Available loaders:
//!
//! * Core Text (macOS): The system font loader on macOS. Does not do hinting except when bilevel
//!   rendering is in use.
//!
//! * DirectWrite (Windows): The newer system framework for text rendering on Windows. Does
//!   vertical hinting but not full hinting.
//!
//! * FreeType (cross-platform): A full-featured font rendering framework.
//!
//! Available sources:
//!
//! * Core Text (macOS): The system font database on macOS.
//!
//! * DirectWrite (Windows): The newer API to query the system font database on Windows.
//!
//! * Fontconfig (cross-platform): A technically platform-neutral, but in practice Unix-specific,
//!   API to query and match fonts.
//!
//! * Filesystem (cross-platform): A simple source that reads fonts from a path on disk. This is
//!   the default on Android.
//!
//! * Memory (cross-platform): A source that reads from a fixed set of fonts in memory.
//!
//! * Multi (cross-platform): A source that allows multiple sources to be queried at once.
//!
//! On Windows and macOS, the FreeType loader and the Fontconfig source are not built by default.
//! To build them, use the `loader-freetype` and `source-fontconfig` Cargo features respectively.
//! If you want them to be the default, instead use the `loader-freetype-default` and
//! `source-fontconfig-default` Cargo features respectively. Beware that
//! `source-fontconfig-default` is rarely what you want on those two platforms!
//!
//! ## Features
//!
//! `font-kit` is capable of doing the following:
//!
//! * Loading fonts from files or memory.
//!
//! * Determining whether files on disk or in memory represent fonts.
//!
//! * Interoperating with native font APIs.
//!
//! * Querying various metadata about fonts.
//!
//! * Doing simple glyph-to-character mapping. (For more complex use cases, a shaper is required;
//!   proper shaping is beyond the scope of `font-kit`.)
//!
//! * Reading unhinted or hinted vector outlines from glyphs.
//!
//! * Calculating glyph and font metrics.
//!
//! * Looking up glyph advances and origins.
//!
//! * Rasterizing glyphs using the native rasterizer, optionally using hinting. (Custom
//!   rasterizers, such as Pathfinder, can be used in conjuction with the outline API.)
//!
//! * Looking up all fonts on the system.
//!
//! * Searching for specific fonts by family or PostScript name.
//!
//! * Performing font matching according to the [CSS Fonts Module Level 3] specification.
//!
//! ## License
//!
//! `font-kit` is licensed under the same terms as Rust itself.
//!
//! [CSS Fonts Module Level 3]: https://drafts.csswg.org/css-fonts-3/#font-matching-algorithm

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]

extern crate byteorder;
extern crate euclid;
extern crate float_ord;
extern crate libc;
extern crate lyon_path;

#[allow(unused_imports)]
#[macro_use]
extern crate lazy_static;
#[allow(unused_imports)]
#[macro_use]
extern crate log;

#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_graphics;
#[cfg(target_os = "macos")]
extern crate core_text;
#[cfg(not(target_arch = "wasm32"))]
#[allow(unused_imports)]
extern crate dirs;
#[cfg(target_family = "windows")]
extern crate dwrote;
#[cfg(any(
    not(any(target_os = "macos", target_family = "windows", target_arch = "wasm32")),
    feature = "source-fontconfig"
))]
extern crate fontconfig;
#[cfg(any(
    not(any(target_os = "macos", target_family = "windows")),
    feature = "loader-freetype"
))]
extern crate freetype;
#[cfg(not(target_arch = "wasm32"))]
extern crate memmap;
#[cfg(not(target_arch = "wasm32"))]
extern crate walkdir;
#[cfg(target_family = "windows")]
extern crate winapi;

mod canvas;
mod error;
mod family;
mod family_handle;
mod family_name;
mod file_type;
mod handle;
mod hinting;
mod loader;
mod loaders;
mod matching;
mod metrics;
mod properties;
mod source;
mod sources;
mod utils;

#[cfg(test)]
mod test;

pub use canvas::{Canvas, Format, RasterizationOptions};
pub use error::{FontLoadingError, GlyphLoadingError, SelectionError};
pub use family::Family;
pub use family_handle::FamilyHandle;
pub use family_name::FamilyName;
pub use file_type::FileType;
pub use handle::Handle;
pub use hinting::HintingOptions;
pub use loader::{FallbackFont, FallbackResult, Loader};
pub use loaders::Font;
pub use metrics::Metrics;
pub use properties::{Properties, Stretch, Style, Weight};
pub use source::{Source, SystemSource};
pub use sources::*;
