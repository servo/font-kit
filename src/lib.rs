// font-kit/src/lib.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate arrayvec;
extern crate byteorder;
extern crate euclid;
extern crate lyon_path;
extern crate memmap;

#[macro_use]
extern crate bitflags;
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

#[cfg(all(target_family = "windows", not(feature = "loader-freetype")))]
#[path = "loaders/directwrite.rs"]
mod loader;
#[cfg(all(target_os = "macos", not(feature = "loader-freetype")))]
#[path = "loaders/core_text.rs"]
mod loader;
#[cfg(any(not(any(target_os = "macos", target_family = "windows")), feature = "loader-freetype"))]
#[path = "loaders/freetype.rs"]
mod loader;

pub mod descriptor;
pub mod family;
pub mod font;
pub mod sources;
pub mod set;

#[cfg(test)]
pub mod test;

mod utils;
