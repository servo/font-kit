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
//! On macOS and Windows, the Cargo feature `source-fontconfig` can be used to opt into fontconfig
//! support. To prefer it over the native font source (only if you know what you're doing), use the
//! `source-fontconfig-default` feature.

use fontconfig::fontconfig::{FcBool, FcConfig, FcFontList, FcInitLoadConfigAndFonts, FcObjectSet};
use fontconfig::fontconfig::{FcObjectSetAdd, FcObjectSetCreate, FcObjectSetDestroy, FcPattern};
use fontconfig::fontconfig::{FcPatternAddString, FcPatternCreate, FcPatternDestroy};
use fontconfig::fontconfig::{FcPatternGetInteger, FcPatternGetString, FcResultMatch};
use memmap::Mmap;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::marker::PhantomData;
use std::ops::Deref;
use std::os::raw::{c_char, c_uchar};
use std::ptr;
use std::slice;
use std::sync::Arc;

use descriptor::Spec;
use family::Family;
use font::Font;
use source::Source;

#[allow(dead_code, non_upper_case_globals)]
const FcFalse: FcBool = 0;
#[allow(non_upper_case_globals)]
const FcTrue: FcBool = 1;

const FC_FAMILY: &'static [u8] = b"family\0";
const FC_FILE: &'static [u8] = b"file\0";
const FC_INDEX: &'static [u8] = b"index\0";

pub struct FontconfigSource {
    fontconfig: *mut FcConfig,
}

impl FontconfigSource {
    pub fn new() -> FontconfigSource {
        unsafe {
            FontconfigSource {
                fontconfig: FcInitLoadConfigAndFonts(),
            }
        }
    }

    pub fn all_families(&self) -> Vec<String> {
        unsafe {
            let pattern = FcPatternObject::new();

            // We want the file path and the font index.
            let mut object_set = FcObjectSetObject::new();
            object_set.push_string(FC_FAMILY);

            let font_set = FcFontList(self.fontconfig, pattern.pattern, object_set.object_set);
            assert!(!font_set.is_null());

            let font_patterns = slice::from_raw_parts((*font_set).fonts,
                                                      (*font_set).nfont as usize);

            let mut result_families = vec![];
            for font_pattern in font_patterns {
                let family = match fc_pattern_get_string(*font_pattern, FC_FAMILY) {
                    None => continue,
                    Some(family) => family,
                };
                result_families.push(family);
            }

            result_families.sort();
            result_families.dedup();
            result_families
        }
    }

    pub fn select_family(&self, family_name: &str) -> Family {
        unsafe {
            let mut pattern = FcPatternObject::new();
            pattern.push_string(FC_FAMILY, family_name.to_owned());

            // We want the file path and the font index.
            let mut object_set = FcObjectSetObject::new();
            object_set.push_string(FC_FILE);
            object_set.push_string(FC_INDEX);

            let font_set = FcFontList(self.fontconfig, pattern.pattern, object_set.object_set);
            assert!(!font_set.is_null());

            let font_patterns = slice::from_raw_parts((*font_set).fonts,
                                                      (*font_set).nfont as usize);

            let mut result_fonts = vec![];
            for font_pattern in font_patterns {
                let font_path = match fc_pattern_get_string(*font_pattern, FC_FILE) {
                    None => continue,
                    Some(font_path) => font_path,
                };
                let font_index = match fc_pattern_get_integer(*font_pattern, FC_INDEX) {
                    None => continue,
                    Some(font_index) => font_index,
                };
                let mut file = match File::open(font_path) {
                    Err(_) => continue,
                    Ok(file) => file,
                };
                let font = match Font::from_file(&mut file, font_index as u32) {
                    Err(_) => continue,
                    Ok(font) => font,
                };
                result_fonts.push(font);
            }

            Family::from_fonts(result_fonts.into_iter())
        }
    }

    pub fn find(&self, spec: &Spec) -> Result<Font, ()> {
        <Self as Source>::find(self, spec)
    }
}

impl Source for FontconfigSource {
    #[inline]
    fn all_families(&self) -> Vec<String> {
        self.all_families()
    }

    #[inline]
    fn select_family(&self, family_name: &str) -> Family {
        self.select_family(family_name)
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

unsafe fn fc_pattern_get_integer(pattern: *mut FcPattern, object: &'static [u8]) -> Option<i32> {
    let mut integer = 0;
    if FcPatternGetInteger(pattern,
                           object.as_ptr() as *const c_char,
                           0,
                           &mut integer) != FcResultMatch {
        return None
    }
    Some(integer)
}
