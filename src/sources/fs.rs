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
use error::{FontLoadingError, SelectionError};
use family::{Family, FamilyHandle};
use font::{Font, Type};
use handle::Handle;
use source::Source;
use sources::mem::MemSource;

pub struct FsSource {
    mem_source: MemSource,
}

impl FsSource {
    /// Do not rely on this function for systems other than Android. It makes a best effort to
    /// locate fonts in the typical platform directories, but it is too simple to pick up fonts
    /// that are stored in unusual locations but nevertheless properly installed.
    pub fn new() -> FsSource {
        let mut fonts = vec![];
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
                    Ok(Type::Single) => fonts.push(Handle::from_path(path.to_owned(), 0)),
                    Ok(Type::Collection(font_count)) => {
                        for font_index in 0..font_count {
                            fonts.push(Handle::from_path(path.to_owned(), font_index))
                        }
                    }
                }
            }
        }

        FsSource {
            mem_source: MemSource::from_fonts(fonts.into_iter()).unwrap(),
        }
    }

    pub fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.mem_source.all_families()
    }

    pub fn select_family_by_name(&self, family_name: &str)
                                 -> Result<FamilyHandle, SelectionError> {
        self.mem_source.select_family_by_name(family_name)
    }

    pub fn select_by_postscript_name(&self, postscript_name: &str)
                                     -> Result<Handle, SelectionError> {
        self.mem_source.select_by_postscript_name(postscript_name)
    }
}

impl Source for FsSource {
    #[inline]
    fn all_families(&self) -> Result<Vec<String>, SelectionError> {
        self.all_families()
    }

    fn select_family_by_name(&self, family_name: &str) -> Result<FamilyHandle, SelectionError> {
        self.select_family_by_name(family_name)
    }

    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<Handle, SelectionError> {
        self.select_by_postscript_name(postscript_name)
    }
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

