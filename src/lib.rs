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
//! services such as finding fonts on the system, performing nearest-font matching, and
//! glyph rasterization.

extern crate arrayvec;
extern crate byteorder;
extern crate euclid;
extern crate float_ord;
extern crate itertools;
extern crate libc;
extern crate lyon_path;
extern crate memmap;
extern crate walkdir;

#[allow(unused_imports)]
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

#[cfg(target_family = "windows")]
extern crate dwrote;
#[cfg(target_family = "windows")]
extern crate winapi;
#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_graphics;
#[cfg(target_os = "macos")]
extern crate core_text;
#[cfg(any(not(any(target_os = "macos", target_family = "windows")),
          feature = "source-fontconfig"))]
extern crate fontconfig;
#[cfg(any(not(any(target_os = "macos", target_family = "windows")), feature = "loader-freetype"))]
extern crate freetype;

pub mod canvas;
pub mod error;
pub mod family;
pub mod family_handle;
pub mod family_name;
pub mod file_type;
pub mod font;
pub mod handle;
pub mod hinting;
pub mod loader;
pub mod loaders;
pub mod metrics;
pub mod properties;
pub mod source;
pub mod sources;

#[cfg(test)]
pub mod test;

mod matching;
mod utils;
