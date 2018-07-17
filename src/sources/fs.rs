// font-kit/src/sources/fs.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A source that loads fonts from a directory on the filesystem.
//!
//! This is the native source on Android.

use itertools::Itertools;
use std::fs::File;
use std::path::PathBuf;
use walkdir::WalkDir;

#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_family = "windows")]
use std::ffi::OsString;
#[cfg(target_family = "windows")]
use std::os::windows::ffi::OsStringExt;
#[cfg(target_family = "windows")]
use winapi::shared::minwindef::{MAX_PATH, UINT};
#[cfg(target_family = "windows")]
use winapi::um::sysinfoapi;

use descriptor::Spec;
use family::Family;
use font::{Font, Type};
use source::Source;

pub struct FsSource {
    families: Vec<FamilyEntry>,
}

impl FsSource {
    /// Do not rely on this function for systems other than Android. It makes a best effort to
    /// locate fonts in the typical platform directories, but it is too simple to pick up fonts
    /// that are stored in unusual locations but nevertheless properly installed.
    pub fn new() -> FsSource {
        let mut families = vec![];
        for font_directory in default_font_directories() {
            for directory_entry in WalkDir::new(font_directory).into_iter() {
                let directory_entry = match directory_entry {
                    Ok(directory_entry) => directory_entry,
                    Err(_) => continue,
                };
                let path = directory_entry.path();
                let mut file = match File::open(path) {
                    Err(_) => continue,
                    Ok(file) => file,
                };
                match Font::analyze_file(&mut file) {
                    Err(_) => continue,
                    Ok(Type::Single) => {
                        if let Ok(font) = Font::from_file(&mut file, 0) {
                            families.push(FamilyEntry {
                                family_name: font.family_name(),
                                font: font,
                            })
                        }
                    }
                    Ok(Type::Collection(font_count)) => {
                        for font_index in 0..font_count {
                            if let Ok(font) = Font::from_file(&mut file, font_index) {
                                families.push(FamilyEntry {
                                    family_name: font.family_name(),
                                    font: font,
                                })
                            }
                        }
                    }
                }
            }
        }
        families.sort_by(|a, b| a.family_name.cmp(&b.family_name));
        FsSource {
            families,
        }
    }

    pub fn all_families(&self) -> Vec<String> {
        self.families
            .iter()
            .map(|family| &*family.family_name)
            .dedup()
            .map(|name| name.to_owned())
            .collect()
    }

    // FIXME(pcwalton): Case-insensitive comparison.
    pub fn select_family(&self, family_name: &str) -> Family {
        let mut first_family_index = match self.families.binary_search_by(|family| {
            (&*family.family_name).cmp(family_name)
        }) {
            Err(_) => return Family::new(),
            Ok(family_index) => family_index,
        };
        while first_family_index > 0 &&
                self.families[first_family_index - 1].family_name == family_name {
            first_family_index -= 1
        }
        let mut last_family_index = first_family_index;
        while last_family_index + 1 < self.families.len() &&
                self.families[last_family_index + 1].family_name == family_name {
            last_family_index += 1
        }
        Family::from_fonts(self.families[first_family_index..(last_family_index + 1)]
                               .iter()
                               .map(|family| family.font.clone()))
    }

    pub fn find_by_postscript_name(&self, postscript_name: &str) -> Result<Font, ()> {
        self.families
            .iter()
            .filter(|family_entry| family_entry.font.postscript_name() == postscript_name)
            .map(|family_entry| family_entry.font.clone())
            .next()
            .ok_or(())
    }

    pub fn find(&self, spec: &Spec) -> Result<Font, ()> {
        <Self as Source>::find(self, spec)
    }
}

impl Source for FsSource {
    #[inline]
    fn all_families(&self) -> Vec<String> {
        self.all_families()
    }

    fn select_family(&self, family_name: &str) -> Family {
        self.select_family(family_name)
    }

    fn find_by_postscript_name(&self, postscript_name: &str) -> Result<Font, ()> {
        self.find_by_postscript_name(postscript_name)
    }
}

struct FamilyEntry {
    family_name: String,
    font: Font,
}

#[cfg(target_os = "android")]
fn default_font_directories() -> Vec<PathBuf> {
    vec![PathBuf::from("/system/fonts")]
}

#[cfg(target_family = "windows")]
fn default_font_directories() -> Vec<PathBuf> {
    unsafe {
        let mut buffer = vec![0; MAX_PATH];
        let len = sysinfoapi::GetWindowsDirectoryW(buffer.as_mut_ptr(), buffer.len() as UINT);
        assert!(len != 0);
        buffer.truncate(len as usize);

        let mut path = PathBuf::from(OsString::from_wide(&buffer));
        path.push("Fonts");
        vec![path]
    }
}

#[cfg(target_os = "macos")]
fn default_font_directories() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/Network/Library/Fonts"),
    ];
    if let Some(mut path) = env::home_dir() {
        path.push("Library");
        path.push("Fonts");
        directories.push(path);
    }
    directories
}

#[cfg(not(any(target_os = "android", target_family = "windows", target_os = "macos")))]
fn default_font_directories() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Some(mut path) = env::home_dir() {
        path.push(".fonts");
        directories.push(path);
    }
    directories
}

