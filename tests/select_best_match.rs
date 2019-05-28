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

use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

#[cfg(target_os = "windows")]
mod test {
    use super::*;

    #[test]
    fn select_best_match_serif() {
        assert_eq!(
            font_path_by_family(FamilyName::Serif, Properties::new()),
            "C:\\WINDOWS\\FONTS\\TIMES.TTF"
        );
    }

    #[test]
    fn select_best_match_sans_serif() {
        assert_eq!(
            font_path_by_family(FamilyName::SansSerif, Properties::new()),
            "C:\\WINDOWS\\FONTS\\ARIAL.TTF"
        );
    }

    #[test]
    fn select_best_match_monospace() {
        assert_eq!(
            font_path_by_family(FamilyName::Monospace, Properties::new()),
            "C:\\WINDOWS\\FONTS\\COUR.TTF"
        );
    }

    #[test]
    fn select_best_match_cursive() {
        assert_eq!(
            font_path_by_family(FamilyName::Cursive, Properties::new()),
            "C:\\WINDOWS\\FONTS\\COMIC.TTF"
        );
    }

    // TODO: no such font in a Travis CI instance.
    // https://github.com/pcwalton/font-kit/issues/62
    // #[test]
    // fn select_best_match_fantasy() {
    //     assert_eq!(
    //         font_path_by_family(FamilyName::Fantasy, Properties::new()),
    //         "C:\\WINDOWS\\FONTS\\IMPACT.TTF"
    //     );
    // }

    #[test]
    fn select_best_match_bold_sans_serif() {
        assert_eq!(
            font_path_by_family(
                FamilyName::SansSerif,
                Properties {
                    style: font_kit::properties::Style::Normal,
                    weight: font_kit::properties::Weight::BOLD,
                    stretch: font_kit::properties::Stretch::NORMAL,
                }
            ),
            "C:\\WINDOWS\\FONTS\\ARIALBD.TTF"
        );
    }
}

// Technically Ubuntu 16.04
#[cfg(target_os = "linux")]
mod test {
    use super::*;

    #[test]
    fn select_best_match_serif() {
        assert_eq!(
            font_path_by_family(FamilyName::Serif, Properties::new()),
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"
        );
    }

    #[test]
    fn select_best_match_sans_serif() {
        assert_eq!(
            font_path_by_family(FamilyName::SansSerif, Properties::new()),
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
        );
    }

    #[test]
    fn select_best_match_monospace() {
        assert_eq!(
            font_path_by_family(FamilyName::Monospace, Properties::new()),
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
        );
    }

    #[test]
    fn select_best_match_cursive() {
        assert_eq!(
            font_path_by_family(FamilyName::Cursive, Properties::new()),
            "/usr/share/fonts/truetype/msttcorefonts/Comic_Sans_MS.ttf"
        );
    }

    #[test]
    fn select_best_match_fantasy() {
        assert_eq!(
            font_path_by_family(FamilyName::Fantasy, Properties::new()),
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
        );
    }

    #[test]
    fn select_best_match_bold_sans_serif() {
        assert_eq!(
            font_path_by_family(
                FamilyName::SansSerif,
                Properties {
                    style: font_kit::properties::Style::Normal,
                    weight: font_kit::properties::Weight::BOLD,
                    stretch: font_kit::properties::Stretch::NORMAL,
                }
            ),
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
        );
    }
}

#[cfg(target_os = "macos")]
mod test {
    use super::*;

    #[test]
    fn select_best_match_serif() {
        assert_eq!(
            font_path_by_family(FamilyName::Serif, Properties::new()),
            "/Library/Fonts/Times New Roman.ttf"
        );
    }

    #[test]
    fn select_best_match_sans_serif() {
        assert_eq!(
            font_path_by_family(FamilyName::SansSerif, Properties::new()),
            "/Library/Fonts/Arial.ttf"
        );
    }

    #[test]
    fn select_best_match_monospace() {
        assert_eq!(
            font_path_by_family(FamilyName::Monospace, Properties::new()),
            "/Library/Fonts/Courier New.ttf"
        );
    }

    #[test]
    fn select_best_match_cursive() {
        assert_eq!(
            font_path_by_family(FamilyName::Cursive, Properties::new()),
            "/Library/Fonts/Comic Sans MS.ttf"
        );
    }

    #[test]
    fn select_best_match_fantasy() {
        assert_eq!(
            font_path_by_family(FamilyName::Fantasy, Properties::new()),
            "/Library/Fonts/Papyrus.ttc"
        );
    }

    #[test]
    fn select_best_match_bold_sans_serif() {
        assert_eq!(
            font_path_by_family(
                FamilyName::SansSerif,
                Properties {
                    style: font_kit::properties::Style::Normal,
                    weight: font_kit::properties::Weight::BOLD,
                    stretch: font_kit::properties::Stretch::NORMAL,
                }
            ),
            "/Library/Fonts/Arial Bold.ttf"
        );
    }
}

fn font_path_by_family(family: FamilyName, properties: Properties) -> String {
    let handle = SystemSource::new()
        .select_best_match(&[family], &properties)
        .unwrap();
    match handle {
        Handle::Path { path, .. } => path.display().to_string(),
        _ => unreachable!(),
    }
}
