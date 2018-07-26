// font-kit/src/test.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::PathEvent;
use lyon_path::builder::FlatPathBuilder;
use lyon_path::default::Path;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use canvas::{Canvas, Format, RasterizationOptions};
use family_name::FamilyName;
use file_type::FileType;
use font::Font;
use hinting::HintingOptions;
use properties::{Properties, Stretch, Style, Weight};
use source::SystemSource;
use sources::fs::FsSource;
use utils;

// TODO(pcwalton): Change this to DejaVu or whatever on Linux.
static SANS_SERIF_FONT_FAMILY_NAME: &'static str = "Arial";
static SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME: &'static str = "ArialMT";
static SANS_SERIF_FONT_BOLD_POSTSCRIPT_NAME: &'static str = "Arial-BoldMT";
static SANS_SERIF_FONT_ITALIC_POSTSCRIPT_NAME: &'static str = "Arial-ItalicMT";

static TEST_FONT_FILE_PATH: &'static str = "resources/tests/EBGaramond12-Regular.otf";
static TEST_FONT_POSTSCRIPT_NAME: &'static str = "EBGaramond12-Regular";

#[test]
pub fn lookup_single_regular_font() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    assert_eq!(font.postscript_name(), SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME);
}

#[test]
pub fn lookup_single_bold_font() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif],
                                                     Properties::new().weight(Weight::BOLD))
                                  .unwrap()
                                  .load()
                                  .unwrap();
    assert_eq!(font.postscript_name(), SANS_SERIF_FONT_BOLD_POSTSCRIPT_NAME);
}

#[test]
pub fn lookup_single_italic_font() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif],
                                                     Properties::new().style(Style::Italic))
                                  .unwrap()
                                  .load()
                                  .unwrap();
    assert_eq!(font.postscript_name(), SANS_SERIF_FONT_ITALIC_POSTSCRIPT_NAME);
}

#[test]
pub fn lookup_all_fonts_in_a_family() {
    let family = SystemSource::new().select_family_by_name(SANS_SERIF_FONT_FAMILY_NAME).unwrap();
    assert!(family.fonts().len() > 2);
}

// Ignored because the `fs` backend is *slow*.
#[ignore]
#[test]
pub fn lookup_all_fonts_in_a_family_in_system_font_directories() {
    let family = FsSource::new().select_family_by_name(SANS_SERIF_FONT_FAMILY_NAME).unwrap();
    assert!(family.fonts().len() > 0);
}

#[test]
pub fn lookup_font_by_postscript_name() {
    let font =
        SystemSource::new().select_by_postscript_name(SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME)
                           .unwrap()
                           .load()
                           .unwrap();
    assert_eq!(font.postscript_name(), SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME);
}

#[test]
pub fn load_font_from_file() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    assert_eq!(font.postscript_name(), TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn load_font_from_memory() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let mut font_data = vec![];
    file.read_to_end(&mut font_data).unwrap();
    let font = Font::from_bytes(Arc::new(font_data), 0).unwrap();
    assert_eq!(font.postscript_name(), TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn analyze_file() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    assert_eq!(Font::analyze_file(&mut file).unwrap(), FileType::Single);
}

#[test]
pub fn analyze_bytes() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let mut font_data = vec![];
    file.read_to_end(&mut font_data).unwrap();
    assert_eq!(Font::analyze_bytes(Arc::new(font_data)).unwrap(), FileType::Single);
}

#[test]
pub fn get_glyph_for_char() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(glyph, 68);
}

#[test]
pub fn get_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::None, &mut path_builder).unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(events.next(), Some(PathEvent::MoveTo(Point2D::new(136.0, 1259.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(136.0, 1466.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 1466.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 1259.0))));
    assert_eq!(events.next(), Some(PathEvent::Close));
    assert_eq!(events.next(), Some(PathEvent::MoveTo(Point2D::new(136.0, 0.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(136.0, 1062.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 1062.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 0.0))));
    assert_eq!(events.next(), Some(PathEvent::Close));
}

// Right now, only FreeType can do hinting.
#[cfg(any(not(any(target_os = "macos", target_family = "windows")),
          feature = "loader-freetype-default"))]
#[test]
pub fn get_vertically_hinted_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::Vertical(16.0), &mut path_builder).unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(events.next(), Some(PathEvent::MoveTo(Point2D::new(136.0, 1316.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(136.0, 1536.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 1536.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 1316.0))));
    assert_eq!(events.next(), Some(PathEvent::Close));
    assert_eq!(events.next(), Some(PathEvent::MoveTo(Point2D::new(136.0, 0.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(136.0, 1152.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 1152.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.0, 0.0))));
    assert_eq!(events.next(), Some(PathEvent::Close));
}

// Right now, only FreeType can do hinting.
#[cfg(any(not(any(target_os = "macos", target_family = "windows")),
          feature = "loader-freetype-default"))]
#[test]
pub fn get_fully_hinted_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::Full(10.0), &mut path_builder).unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(events.next(), Some(PathEvent::MoveTo(Point2D::new(137.6, 1228.8))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(137.6, 1433.6))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.80002, 1433.6))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.80002, 1228.8))));
    assert_eq!(events.next(), Some(PathEvent::Close));
    assert_eq!(events.next(), Some(PathEvent::MoveTo(Point2D::new(137.6, 0.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(137.6, 1024.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.80002, 1024.0))));
    assert_eq!(events.next(), Some(PathEvent::LineTo(Point2D::new(316.80002, 0.0))));
    assert_eq!(events.next(), Some(PathEvent::Close));
}

#[test]
pub fn get_glyph_typographic_bounds() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.typographic_bounds(glyph),
               Ok(Rect::new(Point2D::new(74.0, -24.0), Size2D::new(978.0, 1110.0))));
}

#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2D::new(1139.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Point2D::zero()));
}

#[test]
pub fn get_font_metrics() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let metrics = font.metrics();
    assert_eq!(metrics.units_per_em, 2048);
    assert_eq!(metrics.ascent, 1854.0);
    assert_eq!(metrics.descent, -434.0);
    assert_eq!(metrics.line_gap, 67.0);
    assert_eq!(metrics.underline_position, -217.0);
    assert_eq!(metrics.underline_thickness, 150.0);
    assert_eq!(metrics.cap_height, 1467.0);
    assert_eq!(metrics.x_height, 1062.0);
}

#[test]
pub fn get_font_full_name() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    assert_eq!(font.full_name(), "Arial");
}

#[test]
pub fn get_font_properties() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let properties = font.properties();
    assert_eq!(properties.weight, Weight(400.0));
    assert_eq!(properties.stretch, Stretch(1.0));
}

#[test]
pub fn get_font_data() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let data = font.copy_font_data().unwrap();
    debug_assert!(utils::SFNT_VERSIONS.iter().any(|version| data[0..4] == *version));
}

#[test]
pub fn rasterize_glyph_with_grayscale_aa() {
    let font = SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 32.0;
    let raster_rect = font.raster_bounds(glyph_id,
                                         size,
                                         &Point2D::zero(),
                                         HintingOptions::None,
                                         RasterizationOptions::GrayscaleAa)
                          .unwrap();
    let origin = Point2D::new(-raster_rect.origin.x, -raster_rect.origin.y).to_f32();
    let mut canvas = Canvas::new(&raster_rect.size.to_u32(), Format::A8);
    font.rasterize_glyph(&mut canvas,
                         glyph_id,
                         size,
                         &origin,
                         HintingOptions::None,
                         RasterizationOptions::GrayscaleAa)
        .unwrap();
    check_L_shape(&canvas);
}

// Makes sure that a canvas has an "L" shape in it.
#[allow(non_snake_case)]
fn check_L_shape(canvas: &Canvas) {
    // Find any empty rows at the start.
    let mut y = 0;
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        if canvas.pixels[row_start..row_end].iter().any(|&p| p != 0) {
            break
        }
    }

    // Find the top part of the L.
    let mut top_stripe_width = None;
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        let stripe_width = stripe_width(&canvas.pixels[row_start..row_end]);
        if let Some(top_stripe_width) = top_stripe_width {
            if stripe_width > top_stripe_width {
                break
            }
            assert_eq!(stripe_width, top_stripe_width);
        }
        top_stripe_width = Some(stripe_width);
    }

    // Find the bottom part of the L.
    let mut bottom_stripe_width = None;
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        let stripe_width = stripe_width(&canvas.pixels[row_start..row_end]);
        if let Some(bottom_stripe_width) = bottom_stripe_width {
            assert!(bottom_stripe_width > top_stripe_width.unwrap());
            assert_eq!(stripe_width, bottom_stripe_width);
        }
        bottom_stripe_width = Some(stripe_width);
    }

    // Find any empty rows at the end.
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        if canvas.pixels[row_start..row_end].iter().any(|&p| p != 0) {
            break
        }
    }

    // Make sure we made it to the end.
    assert_eq!(y, canvas.size.height);
}

fn stripe_width(pixels: &[u8]) -> u32 {
    let mut x = 0;
    // Find the initial empty part.
    while x < pixels.len() && pixels[x] == 0 {
        x += 1
    }
    // Find the stripe width.
    let mut stripe_width = 0;
    while x < pixels.len() && pixels[x] != 0 {
        x += 1;
        stripe_width += 1;
    }
    // Find the last empty part.
    while x < pixels.len() && pixels[x] == 0 {
        x += 1;
    }
    assert_eq!(x, pixels.len());
    stripe_width
}
