// font-kit/src/platform/unix.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for generic Unix systems, via fontconfig and FreeType.
//!
//! On macOS, the Cargo feature `backend-fontconfig` can be used to opt into this support instead
//! of the native APIs.'
//!
//!
//! On macOS and Windows, the Cargo feature `backend-freetype` can be used to opt into this
//! support. This enables support for retrieving hinted outlines.

use euclid::{Point2D, Rect, Vector2D};
#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
use fontconfig::fontconfig::{FcPattern, FcPatternDestroy};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::freetype::{FT_Done_Face, FT_Face, FT_Init_FreeType, FT_Library, FT_New_Memory_Face};
use freetype::freetype::{FT_Reference_Face};
use lyon_path::builder::PathBuilder;
use memmap::Mmap;
use std::fs::File;
use std::iter;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr;
use std::sync::Arc;

use descriptor::{Descriptor, Query};
use font::Metrics;
use set::Set;

thread_local! {
    static FREETYPE_LIBRARY: FT_Library = {
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(FT_Init_FreeType(&mut library), 0);
            library
        }
    };
}

pub type NativeFont = FT_Face;

pub struct Font {
    freetype_face: FT_Face,
    font_data: FontData<'static>,
}

impl Clone for Font {
    fn clone(&self) -> Font {
        unsafe {
            assert_eq!(FT_Reference_Face(self.freetype_face), 0);
            Font {
                freetype_face: self.freetype_face,
                font_data: self.font_data.clone(),
            }
        }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            if !self.freetype_face.is_null() {
                assert_eq!(FT_Done_Face(self.freetype_face), 0);
            }
        }
    }
}

impl Font {
    // TODO(pcwalton): Allow the font index to be selected.
    pub fn from_bytes(font_data: Arc<Vec<u8>>) -> Result<Font, ()> {
        FREETYPE_LIBRARY.with(|freetype_library| {
            unsafe {
                let mut freetype_face = ptr::null_mut();
                assert_eq!(FT_New_Memory_Face(*freetype_library,
                                              (*font_data).as_ptr(),
                                              font_data.len() as i64,
                                              0,
                                              &mut freetype_face),
                           0);
                Ok(Font {
                    freetype_face,
                    font_data: FontData::Memory(font_data),
                })
            }
        })
    }

    pub fn from_file(mut file: File) -> Result<Font, ()> {
        unsafe {
            let mmap = try!(Mmap::map(&file).map_err(drop));
            FREETYPE_LIBRARY.with(|freetype_library| {
                let mut freetype_face = ptr::null_mut();
                assert_eq!(FT_New_Memory_Face(*freetype_library,
                                              (*mmap).as_ptr(),
                                              mmap.len() as i64,
                                              0,
                                              &mut freetype_face),
                           0);
                Ok(Font {
                    freetype_face,
                    font_data: FontData::File(Arc::new(mmap)),
                })
            })
        }
    }

    pub unsafe fn from_native_font(freetype_face: NativeFont) -> Font {
        // We make an in-memory copy of the underlying font data. This is because the native font
        // does not necessarily hold a strong reference to the memory backing it.
        const CHUNK_SIZE: usize = 4096;
        let mut font_data = vec![];
        loop {
            font_data.extend(iter::repeat(0).take(CHUNK_SIZE));
            let freetype_stream = (*freetype_face).stream;
            let n_read = ((*freetype_stream).read.unwrap())(freetype_stream,
                                                            font_data.len() as u64,
                                                            font_data.as_mut_ptr(),
                                                            CHUNK_SIZE as u64);
            if n_read < CHUNK_SIZE as u64 {
                break
            }
        }

        Font::from_bytes(Arc::new(font_data)).unwrap()
    }

    pub fn descriptor(&self) -> Descriptor {
        unimplemented!()
    }

    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        unimplemented!()
    }

    pub fn outline<B>(&self, glyph_id: u32, path_builder: &mut B) -> Result<(), ()>
                      where B: PathBuilder {
        unimplemented!()
    }

    pub fn typographic_bounds(&self, glyph_id: u32) -> Rect<f32> {
        unimplemented!()
    }

    pub fn advance(&self, glyph_id: u32) -> Vector2D<f32> {
        unimplemented!()
    }

    pub fn origin(&self, glyph_id: u32) -> Point2D<f32> {
        unimplemented!()
    }

    pub fn metrics(&self) -> Metrics {
        unimplemented!()
    }

    #[inline]
    pub fn font_data(&self) -> Option<FontData> {
        unimplemented!()
    }
}

impl Query {
    pub fn lookup(&self) -> Set {
        unimplemented!()
    }
}

#[derive(Clone)]
pub enum FontData<'a> {
    Unavailable,
    Memory(Arc<Vec<u8>>),
    File(Arc<Mmap>),
    Unused(PhantomData<&'a u8>),
}

impl<'a> Deref for FontData<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            FontData::Unavailable => panic!("Font data unavailable!"),
            FontData::File(ref mmap) => &***mmap,
            FontData::Memory(ref data) => &***data,
            FontData::Unused(_) => unreachable!(),
        }
    }
}
