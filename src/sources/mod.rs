// font-kit/src/sources/mod.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Various databases of installed fonts that can be queried.
//!
//! The system-specific sources (Core Text, DirectWrite, and Fontconfig) contain the fonts that are
//! installed on the system. The remaining databases (`fs`, `mem`, and `multi`) allow `font-kit` to
//! query fonts not installed on the system.

#[cfg(target_os = "macos")]
mod core_text;
#[cfg(target_os = "macos")]
pub use self::core_text::CoreTextSource;

#[cfg(target_family = "windows")]
mod directwrite;
#[cfg(target_family = "windows")]
pub use self::directwrite::DirectWriteSource;

#[cfg(any(
    not(any(target_os = "macos", target_family = "windows", target_arch = "wasm32")),
    feature = "source-fontconfig"
))]
mod fontconfig;
#[cfg(any(
    not(any(target_os = "macos", target_family = "windows", target_arch = "wasm32")),
    feature = "source-fontconfig"
))]
pub use self::fontconfig::FontconfigSource;

#[cfg(not(target_arch = "wasm32"))]
mod fs;
#[cfg(not(target_arch = "wasm32"))]
pub use self::fs::FsSource;

mod mem;
mod multi;

pub use self::mem::MemSource;
pub use self::multi::MultiSource;
