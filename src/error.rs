// font-kit/src/error.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Various types of errors that `font-kit` can return.

use std::convert::From;
use std::io;

#[derive(Debug, Fail)]
pub enum FontLoadingError {
    #[fail(display = "unknown format")]
    UnknownFormat,

    /// Attempted to load an invalid index in a TrueType or OpenType font collection.
    ///
    /// For example, if a `.ttc` file has 2 fonts in it, and you ask for the 5th one, you'll get
    /// this error.
    #[fail(display = "no such font in the collection")]
    NoSuchFontInCollection,

    #[fail(display = "parse error")]
    Parse,

    #[fail(display = "font data unavailable")]
    FontDataUnavailable,

    #[fail(display = "I/O error")]
    Io(io::Error),
}

impl From<io::Error> for FontLoadingError {
    fn from(error: io::Error) -> FontLoadingError {
        FontLoadingError::Io(error)
    }
}

#[derive(PartialEq, Debug, Fail)]
pub enum GlyphLoadingError {
    #[fail(display = "no such glyph")]
    NoSuchGlyph,
}

#[derive(PartialEq, Debug, Fail)]
pub enum SelectionError {
    #[fail(display = "no font found")]
    NotFound,
    #[fail(display = "failed to access source")]
    CannotAccessSource,
}
