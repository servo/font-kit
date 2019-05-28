// font-kit/src/loaders/mod.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The different system services that can load and rasterize fonts.

#[cfg(all(target_os = "macos", not(feature = "loader-freetype-default")))]
pub use loaders::core_text::Font;

#[cfg(all(target_family = "windows", not(feature = "loader-freetype-default")))]
pub use loaders::directwrite::Font;

#[cfg(any(
    not(any(target_os = "macos", target_family = "windows")),
    feature = "loader-freetype-default"
))]
pub use loaders::freetype::Font;

#[cfg(all(target_os = "macos"))]
mod core_text;

#[cfg(all(target_family = "windows"))]
mod directwrite;

#[cfg(any(
    not(any(target_os = "macos", target_family = "windows")),
    feature = "loader-freetype"
))]
mod freetype;
