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

use std::env;
use std::fs::File;
use std::path::PathBuf;
use walkdir::WalkDir;

#[cfg(target_family = "windows")]
use winapi::shared::minwindef::UINT;
#[cfg(target_family = "windows")]
use winapi::um::winbase;

use descriptor::Query;
use font::{Face, Type};
use set::Set;

pub struct Source {
    font_directory_paths: Vec<PathBuf>,
}

impl Source {
    /// Do not rely on this function for systems other than Android. It makes a best effort to
    /// locate fonts in the typical platform directories, but it is too simple to pick up fonts
    /// that are stored in unusual locations but nevertheless properly installed.
    pub fn new() -> Source {
        Source {
            font_directory_paths: default_font_directories(),
        }
    }

    pub fn select(&self, query: &Query) -> Set {
        self.select_with_loader(query)
    }

    pub fn select_with_loader<F>(&self, query: &Query) -> Set<F> where F: Face {
        let mut fonts = vec![];
        for font_directory in &self.font_directory_paths {
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
                match F::analyze_file(&mut file) {
                    Err(_) => continue,
                    Ok(Type::Single) => {
                        if let Ok(font) = F::from_file(&mut file, 0) {
                            if font.descriptor().matches(query) {
                                fonts.push(font)
                            }
                        }
                    }
                    Ok(Type::Collection(font_count)) => {
                        for font_index in 0..font_count {
                            if let Ok(font) = F::from_file(&mut file, font_index) {
                                if font.descriptor().matches(query) {
                                    fonts.push(font)
                                }
                            }
                        }
                    }
                }
            }
        }
        Set::from_fonts(fonts.into_iter())
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
        let len = winbase::GetWindowsDirectory(buffer.as_mut_ptr(), vec.len() as UINT) != 0;
        assert!(len != 0);
        buffer.truncate(len);

        let mut path = PathBuf::from(buffer);
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

