// font-kit/src/handle.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Describes how to open a font. To open these, use a loader.

use std::path::PathBuf;
use std::sync::Arc;

use error::FontLoadingError;
use font::Font;

#[derive(Debug, Clone)]
pub enum Handle {
    Path {
        path: PathBuf,
        font_index: u32,
    },
    Memory {
        bytes: Arc<Vec<u8>>,
        font_index: u32,
    },
}

impl Handle {
    #[inline]
    pub fn from_path(path: PathBuf, font_index: u32) -> Handle {
        Handle::Path {
            path,
            font_index,
        }
    }

    #[inline]
    pub fn from_memory(bytes: Arc<Vec<u8>>, font_index: u32) -> Handle {
        Handle::Memory {
            bytes,
            font_index,
        }
    }

    /// A convenience method to load this handle with the default loader.
    #[inline]
    pub fn load(&self) -> Result<Font, FontLoadingError> {
        Font::from_handle(self)
    }
}
