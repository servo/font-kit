// font-kit/src/sources/fontconfig.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Support for generic Unix systems via fontconfig.
//!
//! On macOS, the Cargo feature `loader-fontconfig` can be used to opt into this support for font
//! lookup.

use arrayvec::ArrayVec;
use byteorder::{BigEndian, ReadBytesExt};
use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::builder::PathBuilder;
use memmap::Mmap;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Cursor;
use std::iter;
use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;
use std::os::raw::{c_char, c_int, c_uchar, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;

use fontconfig::fontconfig::{FcBool, FcConfig, FcConfigGetFonts, FcConfigSubstitute};
use fontconfig::fontconfig::{FcDefaultSubstitute, FcFontList, FcInitLoadConfigAndFonts};
use fontconfig::fontconfig::{FcMatchPattern, FcObjectSet, FcObjectSetAdd, FcObjectSetCreate};
use fontconfig::fontconfig::{FcObjectSetDestroy, FcPattern, FcPatternAddInteger};
use fontconfig::fontconfig::{FcPatternAddString, FcPatternCreate, FcPatternDestroy};
use fontconfig::fontconfig::{FcPatternGetString, FcResultMatch, FcResultNoMatch, FcSetSystem};
use fontconfig::fontconfig::{FcType, FcTypeString};

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

const FT_POINT_TAG_ON_CURVE: c_char = 0x01;
const FT_POINT_TAG_CUBIC_CONTROL: c_char = 0x02;

pub struct Source {
    fontconfig: *mut FcConfig,
}

impl Source {
    pub fn new() -> Source {
        unsafe {
            Source {
                fontconfig: FcInitLoadConfigAndFonts(),
            }
        }
    }

    pub fn select(&self, query: &Query) -> Set {
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
    assert_eq!(FT_Set_Char_Size(face, ((*face).units_per_EM as i64) << 6, 0, 0, 0), 0);
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

fn ft_fixed_26_6_to_f32(fixed: i64) -> f32 {
    (fixed as f32) / 64.0
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
