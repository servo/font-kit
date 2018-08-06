// font-kit/src/file_type.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! The type of a font file: either a single font or a TrueType/OpenType collection.

use byteorder::{BigEndian, ReadBytesExt};

const TTC_TAG: [u8; 4] = [b't', b't', b'c', b'f'];

/// The type of a font file: either a single font or a TrueType/OpenType collection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileType {
    /// The font file represents a single font (`.ttf`, `.otf`, `.woff`, etc.)
    Single,
    /// The font file represents a collection of fonts (`.ttc`, `.otc`, etc.)
    Collection(u32),
}

impl FileType {
    pub(crate) fn data_is_font_collection(data: &[u8]) -> bool {
        data.len() >= 4 && data[0..4] == TTC_TAG
    }

    pub(crate) fn get_number_of_fonts_from_data_if_collection(data: &[u8]) -> Option<u32> {
        if !FileType::data_is_font_collection(data) {
            None
        } else {
            (&data[8..]).read_u32::<BigEndian>().ok()
        }
    }
}
