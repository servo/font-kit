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

/// Reasons why a loader might fail to load a font.
#[derive(Debug, Fail)]
pub enum FontLoadingError {
    /// The data was of a format the loader didn't recognize.
    #[fail(display = "unknown format")]
    UnknownFormat,

    /// Attempted to load an invalid index in a TrueType or OpenType font collection.
    ///
    /// For example, if a `.ttc` file has 2 fonts in it, and you ask for the 5th one, you'll get
    /// this error.
    #[fail(display = "no such font in the collection")]
    NoSuchFontInCollection,

    /// Attempted to load a malformed or corrupted font.
    #[fail(display = "parse error")]
    Parse,

    /// A disk or similar I/O error occurred while attempting to load the font.
    #[fail(display = "I/O error")]
    Io(io::Error),
}

impl From<io::Error> for FontLoadingError {
    fn from(error: io::Error) -> FontLoadingError {
        FontLoadingError::Io(error)
    }
}

/// Reasons why a font might fail to load a glyph.
#[derive(PartialEq, Debug, Fail)]
pub enum GlyphLoadingError {
    /// The font didn't contain a glyph with that ID.
    #[fail(display = "no such glyph")]
    NoSuchGlyph,
}

/// Reasons why a source might fail to look up a font or fonts.
#[derive(PartialEq, Debug, Fail)]
pub enum SelectionError {
    /// No font matching the given query was found.
    #[fail(display = "no font found")]
    NotFound,
    /// The source was inaccessible because of an I/O or similar error.
    #[fail(display = "failed to access source")]
    CannotAccessSource,
}
