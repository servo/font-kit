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
#[cfg(all(target_family = "unix", any(not(target_os = "macos"), feature = "backend-fontconfig")))]
extern crate fontconfig;
#[cfg(all(target_family = "unix", any(not(target_os = "macos"), feature = "backend-freetype")))]
extern crate freetype;

#[cfg(target_family = "windows")]
#[path = "platform/windows.rs"]
mod platform;
#[cfg(all(target_os = "macos", not(feature = "backend-fontconfig")))]
#[path = "platform/macos.rs"]
mod platform;
#[cfg(all(target_family = "unix", any(not(target_os = "macos"),
                                      any(feature = "backend-fontconfig",
                                          feature = "backend-freetype"))))]
#[path = "platform/unix.rs"]
mod platform;

pub mod descriptor;
pub mod family;
pub mod font;
pub mod set;

#[cfg(test)]
pub mod test;
