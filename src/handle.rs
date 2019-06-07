// font-kit/src/handle.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Encapsulates the information needed to locate and open a font.
//!
//! This is either the path to the font or the raw in-memory font data.
//!
//! To open the font referenced by a handle, use a loader.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::FontLoadingError;
use crate::font::Font;

/// Encapsulates the information needed to locate and open a font.
///
/// This is either the path to the font or the raw in-memory font data.
///
/// To open the font referenced by a handle, use a loader.
#[derive(Debug, Clone)]
pub enum Handle {
    /// A font on disk referenced by a path.
    Path {
        /// The path to the font.
        path: PathBuf,
        /// The index of the font, if the path refers to a collection.
        ///
        /// If the path refers to a single font, this value will be 0.
        font_index: u32,
    },
    /// A font in memory.
    Memory {
        /// The raw TrueType/OpenType/etc. data that makes up this font.
        bytes: Arc<Vec<u8>>,
        /// The index of the font, if the memory consists of a collection.
        ///
        /// If the memory consists of a single font, this value will be 0.
        font_index: u32,
    },
}

impl Handle {
    /// Creates a new handle from a path.
    ///
    /// `font_index` specifies the index of the font to choose if the path points to a font
    /// collection. If the path points to a single font file, pass 0.
    #[inline]
    pub fn from_path(path: PathBuf, font_index: u32) -> Handle {
        Handle::Path { path, font_index }
    }

    /// Creates a new handle from raw TTF/OTF/etc. data in memory.
    ///
    /// `font_index` specifies the index of the font to choose if the memory represents a font
    /// collection. If the memory represents a single font file, pass 0.
    #[inline]
    pub fn from_memory(bytes: Arc<Vec<u8>>, font_index: u32) -> Handle {
        Handle::Memory { bytes, font_index }
    }

    /// A convenience method to load this handle with the default loader, producing a Font.
    #[inline]
    pub fn load(&self) -> Result<Font, FontLoadingError> {
        Font::from_handle(self)
    }
}
