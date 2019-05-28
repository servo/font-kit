// font-kit/src/test.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate font_kit;

use font_kit::error::SelectionError;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

macro_rules! match_handle {
    ($handle:expr, $path:expr, $index:expr) => {
        match $handle {
            Handle::Path {
                ref path,
                font_index,
            } => {
                assert_eq!(path, path);
                assert_eq!(
                    font_index, $index,
                    "expecting font index {} not {}",
                    font_index, $index
                );
            }
            _ => unreachable!(),
        }
    };
}

#[cfg(target_os = "windows")]
mod test {
    use super::*;

    #[test]
    fn select_best_match_serif() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Serif], &Properties::default())
            .unwrap();
        match_handle!(handle, "C:\\WINDOWS\\FONTS\\TIMES.TTF", 0);
    }

    #[test]
    fn select_best_match_sans_serif() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::default())
            .unwrap();
        match_handle!(handle, ":\\WINDOWS\\FONTS\\ARIAL.TTF", 0);
    }

    #[test]
    fn select_best_match_monospace() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Monospace], &Properties::default())
            .unwrap();
        match_handle!(handle, "C:\\WINDOWS\\FONTS\\COUR.TTF", 0);
    }

    #[test]
    fn select_best_match_cursive() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Cursive], &Properties::default())
            .unwrap();
        match_handle!(handle, "C:\\WINDOWS\\FONTS\\COMIC.TTF", 0);
    }

    // TODO: no such font in a Travis CI instance.
    // https://github.com/pcwalton/font-kit/issues/62
    // #[test]
    // fn select_best_match_fantasy() {
    //     let handle = SystemSource::new()
    //         .select_best_match(&[FamilyName::Fantasy], &Properties::default())
    //         .unwrap();
    //     match_handle!(handle, "C:\\WINDOWS\\FONTS\\IMPACT.TTF", 0);
    // }

    #[test]
    fn select_best_match_bold_sans_serif() {
        let handle = SystemSource::new()
            .select_best_match(
                &[FamilyName::SansSerif],
                &Properties {
                    style: font_kit::properties::Style::Normal,
                    weight: font_kit::properties::Weight::BOLD,
                    stretch: font_kit::properties::Stretch::NORMAL,
                },
            )
            .unwrap();
        match_handle!(handle, "C:\\WINDOWS\\FONTS\\ARIALBD.TTF", 0);
    }

    #[test]
    fn select_best_match_by_name_after_invalid() {
        let handle = SystemSource::new()
            .select_best_match(
                &[
                    FamilyName::Title("Invalid".to_string()),
                    FamilyName::Title("Times New Roman".to_string()),
                ],
                &Properties::default(),
            )
            .unwrap();
        match_handle!(handle, "C:\\WINDOWS\\FONTS\\TIMES.TTF", 0);
    }

    #[test]
    fn select_family_by_name_arial() {
        let family = SystemSource::new().select_family_by_name("Arial").unwrap();
        assert_eq!(family.fonts().len(), 8);
        match_handle!(family.fonts()[0], "C:\\WINDOWS\\FONTS\\ARIAL.TTF", 0);
        match_handle!(family.fonts()[1], "C:\\WINDOWS\\FONTS\\ARIALI.TTF", 0);
        match_handle!(family.fonts()[2], "C:\\WINDOWS\\FONTS\\ARIALBD.TTF", 0);
        match_handle!(family.fonts()[3], "C:\\WINDOWS\\FONTS\\ARIALBI.TTF", 0);
        match_handle!(family.fonts()[4], "C:\\WINDOWS\\FONTS\\ARIBLK.TTF", 0);
        match_handle!(family.fonts()[5], "C:\\WINDOWS\\FONTS\\ARIAL.TTF", 0);
        match_handle!(family.fonts()[6], "C:\\WINDOWS\\FONTS\\ARIALBD.TTF", 0);
        match_handle!(family.fonts()[7], "C:\\WINDOWS\\FONTS\\ARIBLK.TTF", 0);
    }

    #[allow(non_snake_case)]
    #[test]
    fn select_by_postscript_name_ArialMT() {
        let font = SystemSource::new()
            .select_by_postscript_name("ArialMT")
            .unwrap()
            .load()
            .unwrap();

        assert_eq!(font.postscript_name().unwrap(), "ArialMT");
    }

    #[test]
    fn select_by_postscript_name_invalid() {
        match SystemSource::new().select_by_postscript_name("zxhjfgkadsfhg") {
            Err(SelectionError::NotFound) => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }
}

// Technically Ubuntu 16.04
#[cfg(target_os = "linux")]
mod test {
    use super::*;

    #[test]
    fn select_best_match_serif() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Serif], &Properties::default())
            .unwrap();
        match_handle!(
            handle,
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
            0
        );
    }

    #[test]
    fn select_best_match_sans_serif() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::default())
            .unwrap();
        match_handle!(handle, "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 0);
    }

    #[test]
    fn select_best_match_monospace() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Monospace], &Properties::default())
            .unwrap();
        match_handle!(
            handle,
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            0
        );
    }

    #[test]
    fn select_best_match_cursive() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Cursive], &Properties::default())
            .unwrap();
        match_handle!(
            handle,
            "/usr/share/fonts/truetype/msttcorefonts/Comic_Sans_MS.ttf",
            0
        );
    }

    #[test]
    fn select_best_match_fantasy() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Fantasy], &Properties::default())
            .unwrap();
        match_handle!(handle, "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 0);
    }

    #[test]
    fn select_best_match_bold_sans_serif() {
        let handle = SystemSource::new()
            .select_best_match(
                &[FamilyName::SansSerif],
                &Properties {
                    style: font_kit::properties::Style::Normal,
                    weight: font_kit::properties::Weight::BOLD,
                    stretch: font_kit::properties::Stretch::NORMAL,
                },
            )
            .unwrap();
        match_handle!(
            handle,
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            0
        );
    }

    #[test]
    fn select_best_match_by_name_after_invalid() {
        let handle = SystemSource::new()
            .select_best_match(
                &[
                    FamilyName::Title("Invalid".to_string()),
                    FamilyName::Title("Times New Roman".to_string()),
                ],
                &Properties::default(),
            )
            .unwrap();
        match_handle!(handle, "/usr/share/fonts/corefonts/times.ttf", 0);
    }

    #[test]
    fn select_family_by_name_arial() {
        let family = SystemSource::new().select_family_by_name("Arial").unwrap();
        assert_eq!(family.fonts().len(), 7);
        match_handle!(
            family.fonts()[0],
            "/usr/share/fonts/truetype/msttcorefonts/Arial_Bold.ttf",
            0
        );
        match_handle!(
            family.fonts()[1],
            "/usr/share/fonts/truetype/msttcorefonts/Arial.ttf",
            0
        );
        match_handle!(
            family.fonts()[2],
            "/usr/share/fonts/truetype/msttcorefonts/ariali.ttf",
            0
        );
        match_handle!(
            family.fonts()[3],
            "/usr/share/fonts/truetype/msttcorefonts/arialbi.ttf",
            0
        );

        match_handle!(
            family.fonts()[4],
            "/usr/share/fonts/truetype/msttcorefonts/arialbd.ttf",
            0
        );
        match_handle!(
            family.fonts()[5],
            "/usr/share/fonts/truetype/msttcorefonts/Arial_Italic.ttf",
            0
        );
        match_handle!(
            family.fonts()[6],
            "/usr/share/fonts/truetype/msttcorefonts/Arial_Bold_Italic.ttf",
            0
        );
    }

    #[allow(non_snake_case)]
    #[test]
    fn select_by_postscript_name_ArialMT() {
        let font = SystemSource::new()
            .select_by_postscript_name("ArialMT")
            .unwrap()
            .load()
            .unwrap();

        assert_eq!(font.postscript_name().unwrap(), "ArialMT");
    }

    #[test]
    fn select_by_postscript_name_invalid() {
        match SystemSource::new().select_by_postscript_name("zxhjfgkadsfhg") {
            Err(SelectionError::NotFound) => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }
}

#[cfg(target_os = "macos")]
mod test {
    use super::*;

    #[test]
    fn select_best_match_serif() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Serif], &Properties::default())
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Times New Roman.ttf", 0);
    }

    #[test]
    fn select_best_match_sans_serif() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::default())
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Arial.ttf", 0);
    }

    #[test]
    fn select_best_match_monospace() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Monospace], &Properties::default())
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Courier New.ttf", 0);
    }

    #[test]
    fn select_best_match_cursive() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Cursive], &Properties::default())
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Comic Sans MS.ttf", 0);
    }

    #[test]
    fn select_best_match_fantasy() {
        let handle = SystemSource::new()
            .select_best_match(&[FamilyName::Fantasy], &Properties::default())
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Papyrus.ttc", 1);
    }

    #[test]
    fn select_best_match_bold_sans_serif() {
        let handle = SystemSource::new()
            .select_best_match(
                &[FamilyName::SansSerif],
                &Properties {
                    style: font_kit::properties::Style::Normal,
                    weight: font_kit::properties::Weight::BOLD,
                    stretch: font_kit::properties::Stretch::NORMAL,
                },
            )
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Arial Bold.ttf", 0);
    }

    #[test]
    fn select_best_match_by_name_after_invalid() {
        let handle = SystemSource::new()
            .select_best_match(
                &[
                    FamilyName::Title("Invalid".to_string()),
                    FamilyName::Title("Times New Roman".to_string()),
                ],
                &Properties::default(),
            )
            .unwrap();
        match_handle!(handle, "/Library/Fonts/Times New Roman.ttf", 0);
    }

    #[test]
    fn select_family_by_name_arial() {
        let family = SystemSource::new().select_family_by_name("Arial").unwrap();
        assert_eq!(family.fonts().len(), 4);
        match_handle!(family.fonts()[0], "/Library/Fonts/Arial.ttf", 0);
        match_handle!(family.fonts()[1], "/Library/Fonts/Arial Bold.ttf", 0);
        match_handle!(family.fonts()[2], "/Library/Fonts/Arial Bold Italic.ttf", 0);
        match_handle!(family.fonts()[3], "/Library/Fonts/Arial Italic.ttf", 0);
    }

    #[allow(non_snake_case)]
    #[test]
    fn select_by_postscript_name_ArialMT() {
        let font = SystemSource::new()
            .select_by_postscript_name("ArialMT")
            .unwrap()
            .load()
            .unwrap();

        assert_eq!(font.postscript_name().unwrap(), "ArialMT");
    }

    #[test]
    fn select_by_postscript_name_invalid() {
        match SystemSource::new().select_by_postscript_name("zxhjfgkadsfhg") {
            Err(SelectionError::NotFound) => {}
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
