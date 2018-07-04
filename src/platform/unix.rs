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
//! On macOS, the Cargo feature `backend-fontconfig` can be used to opt into this support for font
//! lookup instead of the native APIs.
//!
//! On macOS and Windows, the Cargo feature `backend-freetype` can be used to opt into this support
//! for lookup. This enables support for retrieving hinted outlines.

use euclid::{Point2D, Rect, Vector2D};
use lyon_path::builder::PathBuilder;
use memmap::Mmap;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::iter;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::os::raw::{c_char, c_int, c_uchar, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;

#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
use fontconfig::fontconfig::{FcBool, FcConfig, FcConfigGetFonts, FcConfigSubstitute, FcDefaultSubstitute, FcFontList};
#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
use fontconfig::fontconfig::{FcInitLoadConfigAndFonts, FcMatchPattern, FcObjectSet, FcObjectSetAdd, FcObjectSetCreate, FcObjectSetDestroy, FcPattern, FcPatternAddInteger, FcPatternAddString};
#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
use fontconfig::fontconfig::{FcPatternCreate, FcPatternDestroy, FcPatternGetString, FcResultMatch, FcResultNoMatch};
#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
use fontconfig::fontconfig::{FcSetSystem, FcType, FcTypeString};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::freetype::{FT_Byte, FT_Done_Face, FT_Encoding, FT_Error, FT_FACE_FLAG_FIXED_WIDTH, FT_FACE_FLAG_VERTICAL, FT_Face, FT_Get_Char_Index};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::freetype::{FT_Get_Postscript_Name, FT_Get_Sfnt_Table, FT_Init_FreeType};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::freetype::{FT_LOAD_NO_HINTING, FT_Long, FT_Library, FT_Load_Glyph};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::freetype::{FT_New_Memory_Face, FT_Reference_Face, FT_Select_Charmap, FT_Sfnt_Tag};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::freetype::{FT_STYLE_FLAG_ITALIC, FT_UInt, FT_ULong, FT_UShort};
#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
use freetype::tt_os2::TT_OS2;

use descriptor::{Descriptor, FONT_STRETCH_MAPPING, Flags, Query, QueryFields};
use family::Family;
use font::Metrics;
use set::Set;

const PS_DICT_FULL_NAME: u32 = 38;
const TT_NAME_ID_FULL_NAME: u16 = 4;

const PANOSE_PROPORTION: usize = 3;
const PAN_PROP_MONOSPACED: u8 = 9;

const FcFalse: FcBool = 0;
const FcTrue: FcBool = 1;
const FcDontCare: FcBool = 2;

const FC_FAMILY: &'static [u8] = b"family\0";
const FC_STYLE: &'static [u8] = b"style\0";
const FC_SLANT: &'static [u8] = b"slant\0";
const FC_WEIGHT: &'static [u8] = b"weight\0";
const FC_WIDTH: &'static [u8] = b"width\0";
const FC_FILE: &'static [u8] = b"file\0";
const FC_FULLNAME: &'static [u8] = b"fullname\0";
const FC_POSTSCRIPT_NAME: &'static [u8] = b"postscriptname\0";

#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
thread_local! {
    static FREETYPE_LIBRARY: FT_Library = {
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(FT_Init_FreeType(&mut library), 0);
            library
        }
    };
}

#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
thread_local! {
    static FONTCONFIG: *mut FcConfig = {
        unsafe {
            FcInitLoadConfigAndFonts()
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
                setup_freetype_face(freetype_face);
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
                setup_freetype_face(freetype_face);
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
        unsafe {
            let postscript_name = FT_Get_Postscript_Name(self.freetype_face);
            let postscript_name = CStr::from_ptr(postscript_name).to_str().unwrap().to_owned();
            let family_name = CStr::from_ptr((*self.freetype_face).family_name).to_str()
                                                                               .unwrap()
                                                                               .to_owned();
            let style_name = CStr::from_ptr((*self.freetype_face).style_name).to_str()
                                                                             .unwrap()
                                                                             .to_owned();
            let display_name = self.get_type_1_or_sfnt_name(PS_DICT_FULL_NAME,
                                                            TT_NAME_ID_FULL_NAME)
                                   .unwrap_or_else(|| family_name.clone());
            let os2_table = self.get_os2_table();

            let mut flags = Flags::empty();
            flags.set(Flags::ITALIC,
                      ((*self.freetype_face).style_flags | (FT_STYLE_FLAG_ITALIC as i64)) != 0);
            flags.set(Flags::MONOSPACE,
                      (*self.freetype_face).face_flags | (FT_FACE_FLAG_FIXED_WIDTH as i64) != 0);
            flags.set(Flags::VERTICAL,
                      (*self.freetype_face).face_flags | (FT_FACE_FLAG_VERTICAL as i64) != 0);

            Descriptor {
                postscript_name,
                display_name,
                family_name,
                style_name,
                stretch: FONT_STRETCH_MAPPING[((*os2_table).usWidthClass as usize) - 1],
                weight: (*os2_table).usWeightClass as u32 as f32,
                flags,
            }
        }
    }

    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        unsafe {
            Some(FT_Get_Char_Index(self.freetype_face, character as FT_ULong))
        }
    }

    pub fn outline<B>(&self, glyph_id: u32, path_builder: &mut B) -> Result<(), ()>
                      where B: PathBuilder {
        unimplemented!()
    }

    pub fn typographic_bounds(&self, glyph_id: u32) -> Rect<f32> {
        unimplemented!()
    }

    pub fn advance(&self, glyph_id: u32) -> Vector2D<f32> {
        unsafe {
            FT_Load_Glyph(self.freetype_face, glyph_id, FT_LOAD_NO_HINTING as i32);
            let advance = (*(*self.freetype_face).glyph).advance;
            Vector2D::new(advance.x as f32, advance.y as f32)
        }
    }

    pub fn origin(&self, glyph_id: u32) -> Point2D<f32> {
        // FIXME(pcwalton): This can't be right!
        Point2D::zero()
    }

    pub fn metrics(&self) -> Metrics {
        unimplemented!()
    }

    #[inline]
    pub fn font_data(&self) -> Option<FontData> {
        match self.font_data {
            FontData::File(_) | FontData::Memory(_) => Some(self.font_data.clone()),
            FontData::Unused(_) => unreachable!(),
        }
    }

    fn get_type_1_or_sfnt_name(&self, type_1_id: u32, sfnt_id: u16) -> Option<String> {
        unsafe {
            let mut buffer = vec![0; 1024];
            if FT_Get_PS_Font_Value(self.freetype_face,
                                    type_1_id,
                                    0,
                                    buffer.as_mut_ptr() as *mut c_void,
                                    buffer.len() as i64) == 0 {
                return String::from_utf8(buffer).ok()
            }

            let sfnt_name_count = FT_Get_Sfnt_Name_Count(self.freetype_face);
            let mut sfnt_name = mem::zeroed();
            for sfnt_name_index in 0..sfnt_name_count {
                assert_eq!(FT_Get_Sfnt_Name(self.freetype_face, sfnt_name_index, &mut sfnt_name),
                           0);
                // FIXME(pcwalton): Check encoding, platform, language.
                if sfnt_name.name_id != sfnt_id {
                    continue
                }
                let sfnt_name_string =
                    slice::from_raw_parts(sfnt_name.string,
                                          sfnt_name.string_len as usize).to_owned();
                if let Ok(result) = String::from_utf8(sfnt_name_string) {
                    return Some(result)
                }
            }

            None
        }
    }

    fn get_os2_table(&self) -> *const TT_OS2 {
        unsafe {
            FT_Get_Sfnt_Table(self.freetype_face, FT_Sfnt_Tag::FT_SFNT_OS2) as *const TT_OS2
        }
    }
}

impl Query {
    pub fn lookup(&self) -> Set {
        FONTCONFIG.with(|fontconfig| {
            unsafe {
                let mut pattern = FcPatternObject::new();
                if self.fields.contains(QueryFields::POSTSCRIPT_NAME) {
                    pattern.push_string(FC_POSTSCRIPT_NAME,
                                        self.descriptor.postscript_name.clone());
                }
                if self.fields.contains(QueryFields::DISPLAY_NAME) {
                    pattern.push_string(FC_FULLNAME, self.descriptor.display_name.clone());
                }
                if self.fields.contains(QueryFields::FAMILY_NAME) {
                    pattern.push_string(FC_FAMILY, self.descriptor.family_name.clone());
                }
                if self.fields.contains(QueryFields::STYLE_NAME) {
                    pattern.push_string(FC_STYLE, self.descriptor.style_name.clone());
                }
                if self.fields.contains(QueryFields::WEIGHT) {
                    let weight = FcWeightFromOpenType(self.descriptor.weight as i32);
                    pattern.push_int(FC_WEIGHT, weight)
                }
                if self.fields.contains(QueryFields::STRETCH) {
                    pattern.push_int(FC_WIDTH, (self.descriptor.stretch * 100.0) as i32)
                }
                if self.fields.contains(QueryFields::ITALIC) {
                    // FIXME(pcwalton): Really we want >=0 here. How do we request that?
                    let slant = if self.descriptor.flags.contains(Flags::ITALIC) {
                        100
                    } else {
                        0
                    };
                    pattern.push_int(FC_SLANT, slant);
                }

                // We want the file path and the family name for grouping.
                let mut object_set = FcObjectSetObject::new();
                object_set.push_string(FC_FAMILY);
                object_set.push_string(FC_FILE);

                let font_set = FcFontList(*fontconfig, pattern.pattern, object_set.object_set);
                assert!(!font_set.is_null());

                let font_patterns = slice::from_raw_parts((*font_set).fonts,
                                                          (*font_set).nfont as usize);

                let mut results = HashMap::new();
                for font_pattern in font_patterns {
                    let family_name = match fc_pattern_get_string(*font_pattern, FC_FAMILY) {
                        None => continue,
                        Some(family_name) => family_name,
                    };
                    let font_path = match fc_pattern_get_string(*font_pattern, FC_FILE) {
                        None => continue,
                        Some(font_path) => font_path,
                    };
                    let file = match File::open(font_path) {
                        Err(_) => continue,
                        Ok(file) => file,
                    };
                    let font = match Font::from_file(file) {
                        Err(_) => continue,
                        Ok(font) => font,
                    };
                    let mut family = results.entry(family_name).or_insert_with(|| Family::new());
                    family.push(font)
                }

                let mut result_set = Set::new();
                for (_, family) in results.into_iter() {
                    result_set.push(family)
                }
                result_set
            }
        })
    }
}

struct FcPatternObject {
    pattern: *mut FcPattern,
    c_strings: Vec<CString>,
}

impl Drop for FcPatternObject {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            FcPatternDestroy(self.pattern)
        }
    }
}

impl FcPatternObject {
    fn new() -> FcPatternObject {
        unsafe {
            FcPatternObject {
                pattern: FcPatternCreate(),
                c_strings: vec![],
            }
        }
    }

    unsafe fn push_string(&mut self, object: &'static [u8], value: String) {
        let c_string = CString::new(value).unwrap();
        FcPatternAddString(self.pattern,
                           object.as_ptr() as *const c_char,
                           c_string.as_ptr() as *const c_uchar);
        self.c_strings.push(c_string)
    }

    unsafe fn push_int(&mut self, object: &'static [u8], value: i32) {
        FcPatternAddInteger(self.pattern, object.as_ptr() as *const c_char, value);
    }
}

struct FcObjectSetObject {
    object_set: *mut FcObjectSet,
}

impl Drop for FcObjectSetObject {
    fn drop(&mut self) {
        unsafe {
            FcObjectSetDestroy(self.object_set)
        }
    }
}

impl FcObjectSetObject {
    fn new() -> FcObjectSetObject {
        unsafe {
            FcObjectSetObject {
                object_set: FcObjectSetCreate(),
            }
        }
    }

    unsafe fn push_string(&mut self, object: &'static [u8]) {
        assert_eq!(FcObjectSetAdd(self.object_set, object.as_ptr() as *const c_char), FcTrue);
    }
}

#[derive(Clone)]
pub enum FontData<'a> {
    Memory(Arc<Vec<u8>>),
    File(Arc<Mmap>),
    Unused(PhantomData<&'a u8>),
}

impl<'a> Deref for FontData<'a> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            FontData::File(ref mmap) => &***mmap,
            FontData::Memory(ref data) => &***data,
            FontData::Unused(_) => unreachable!(),
        }
    }
}

unsafe fn setup_freetype_face(face: FT_Face) {
    //assert_eq!(FT_Select_Charmap(face, FT_Encoding::FT_ENCODING_UNICODE), 0);
}

#[repr(C)]
struct FT_SfntName {
    platform_id: FT_UShort,
    encoding_id: FT_UShort,
    language_id: FT_UShort,
    name_id: FT_UShort,
    string: *mut FT_Byte,
    string_len: FT_UInt,
}

unsafe fn fc_pattern_get_string(pattern: *mut FcPattern, object: &'static [u8]) -> Option<String> {
    let mut string = ptr::null_mut();
    if FcPatternGetString(pattern,
                          object.as_ptr() as *const c_char,
                          0,
                          &mut string) != FcResultMatch {
        return None
    }
    if string.is_null() {
        return None
    }
    CStr::from_ptr(string as *const c_char).to_str().ok().map(|string| string.to_owned())
}

#[cfg(any(not(target_os = "macos"), feature = "backend-fontconfig"))]
extern "C" {
    fn FcWeightFromOpenType(fc_weight: c_int) -> c_int;
}

#[cfg(any(not(target_os = "macos"), feature = "backend-freetype"))]
extern "C" {
    fn FT_Get_PS_Font_Value(face: FT_Face,
                            key: u32,
                            idx: FT_UInt,
                            value: *mut c_void,
                            value_len: FT_Long)
                            -> FT_Long;
    fn FT_Get_Sfnt_Name(face: FT_Face, idx: FT_UInt, aname: *mut FT_SfntName) -> FT_Error;
    fn FT_Get_Sfnt_Name_Count(face: FT_Face) -> FT_UInt;
}
