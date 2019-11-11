// font-kit/src/test.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use euclid::{
    default::{Point2D, Rect, Size2D, Vector2D},
    point2,
};
use lyon_path::{Path, PathEvent};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use crate::canvas::{Canvas, Format, RasterizationOptions};
use crate::family_name::FamilyName;
use crate::file_type::FileType;
use crate::font::Font;
use crate::hinting::HintingOptions;
use crate::loader::FontTransform;
use crate::properties::{Properties, Stretch, Weight};
use crate::source::SystemSource;
use crate::utils;

static TEST_FONT_FILE_PATH: &'static str = "resources/tests/eb-garamond/EBGaramond12-Regular.otf";
static TEST_FONT_POSTSCRIPT_NAME: &'static str = "EBGaramond12-Regular";
static TEST_FONT_COLLECTION_FILE_PATH: &'static str =
    "resources/tests/eb-garamond/EBGaramond12.otc";
static TEST_FONT_COLLECTION_POSTSCRIPT_NAME: [&'static str; 2] =
    ["EBGaramond12-Regular", "EBGaramond12-Italic"];

static FILE_PATH_EB_GARAMOND_TTF: &'static str =
    "resources/tests/eb-garamond/EBGaramond12-Regular.ttf";
static FILE_PATH_INCONSOLATA_TTF: &'static str =
    "resources/tests/inconsolata/Inconsolata-Regular.ttf";

#[test]
pub fn get_font_full_name() {
    let font = SystemSource::new()
        .select_best_match(
            &[FamilyName::Title("Arial".to_string())],
            &Properties::new(),
        )
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(font.full_name(), "Arial");
}

#[test]
pub fn load_font_from_file() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn load_font_from_memory() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let mut font_data = vec![];
    file.read_to_end(&mut font_data).unwrap();
    let font = Font::from_bytes(Arc::new(font_data), 0).unwrap();
    assert_eq!(font.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
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
    assert_eq!(
        Font::analyze_bytes(Arc::new(font_data)).unwrap(),
        FileType::Single
    );
}

#[test]
pub fn get_glyph_for_char() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(glyph, 68);
}

macro_rules! assert_line_to {
    ($event:expr, $pt:expr) => {
        match $event {
            Some(PathEvent::Line(ref segment)) => {
                assert_eq!(segment.to, $pt, "Expected a line to {:?}", $pt)
            }
            other => panic!("Expected line, got {:?}", other),
        }
    };
}

macro_rules! assert_quadratic_to {
    ($event:expr, $ctrl:expr, $to:expr) => {
        match $event {
            Some(PathEvent::Quadratic(ref segment)) => {
                assert_eq!(segment.ctrl, $ctrl);
                assert_eq!(segment.to, $to);
            }
            other => panic!("Expected quadratic segment, got {:?}", other),
        }
    };
}

macro_rules! assert_close {
    ($event:expr) => {
        match $event {
            Some(PathEvent::Close(..)) => {}
            other => panic!("Expected close, got {:?}", other),
        }
    };
}

#[cfg(any(target_family = "windows", target_os = "macos"))]
#[test]
pub fn get_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::None, &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(136.0, 1259.0)))
    );
    assert_line_to!(events.next(), Point2D::new(136.0, 1466.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 1466.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 1259.0));
    assert_close!(events.next());
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(136.0, 0.0)))
    );
    assert_line_to!(events.next(), Point2D::new(136.0, 1062.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 1062.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 0.0));
    assert_close!(events.next());
}

#[cfg(not(any(target_family = "windows", target_os = "macos", target_os = "ios")))]
#[test]
pub fn get_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::None, &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(193.0, 1120.0)))
    );
    assert_line_to!(events.next(), Point2D::new(377.0, 1120.0));
    assert_line_to!(events.next(), Point2D::new(377.0, 0.0));
    assert_line_to!(events.next(), Point2D::new(193.0, 0.0));
    assert_close!(events.next());
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(193.0, 1556.0)))
    );
    assert_line_to!(events.next(), Point2D::new(377.0, 1556.0));
    assert_line_to!(events.next(), Point2D::new(377.0, 1323.0));
    assert_line_to!(events.next(), Point2D::new(193.0, 1323.0));
    assert_close!(events.next());
}

// Right now, only FreeType can do hinting.
#[cfg(all(
    not(any(target_os = "macos", target_os = "ios", target_family = "windows")),
    feature = "loader-freetype-default"
))]
#[test]
pub fn get_vertically_hinted_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::Vertical(16.0), &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_line_to!(events.next(), Point2D::new(136.0, 1316.0));
    assert_line_to!(events.next(), Point2D::new(136.0, 1536.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 1536.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 1316.0));
    assert_close!(events.next());
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(136.0, 0.0)))
    );
    assert_line_to!(events.next(), Point2D::new(136.0, 1152.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 1152.0));
    assert_line_to!(events.next(), Point2D::new(316.0, 0.0));
    assert_close!(events.next());
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_family = "windows")))]
#[test]
pub fn get_vertically_hinted_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::Vertical(16.0), &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(194.0, 1152.0)))
    );
    assert_line_to!(events.next(), Point2D::new(378.0, 1152.0));
    assert_line_to!(events.next(), Point2D::new(378.0, 0.0));
    assert_line_to!(events.next(), Point2D::new(194.0, 0.0));
    assert_close!(events.next());
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(194.0, 1536.0)))
    );
    assert_line_to!(events.next(), Point2D::new(378.0, 1536.0));
    assert_line_to!(events.next(), Point2D::new(378.0, 1302.0));
    assert_line_to!(events.next(), Point2D::new(194.0, 1302.0));
    assert_close!(events.next());
}

// Right now, only FreeType can do hinting.
#[cfg(all(
    not(any(target_os = "macos", target_os = "ios", target_family = "windows")),
    feature = "loader-freetype-default"
))]
#[test]
pub fn get_fully_hinted_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::Full(10.0), &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(137.6, 1228.8)))
    );
    assert_line_to!(events.next(), Point2D::new(137.6, 1433.6));
    assert_line_to!(events.next(), Point2D::new(316.80002, 1433.6));
    assert_line_to!(events.next(), Point2D::new(316.80002, 1228.8));
    assert_close!(events.next());
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(137.6, 0.0)))
    );
    assert_line_to!(events.next(), Point2D::new(137.6, 1024.0));
    assert_line_to!(events.next(), Point2D::new(316.80002, 1024.0));
    assert_line_to!(events.next(), Point2D::new(316.80002, 0.0));
    assert_close!(events.next());
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_family = "windows")))]
#[test]
pub fn get_fully_hinted_glyph_outline() {
    let mut path_builder = Path::builder();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::Full(10.0), &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(204.8, 1024.0)))
    );
    assert_line_to!(events.next(), Point2D::new(409.6, 1024.0));
    assert_line_to!(events.next(), Point2D::new(409.6, 0.0));
    assert_line_to!(events.next(), Point2D::new(204.8, 0.0));
    assert_close!(events.next());
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(204.8, 1638.4)))
    );
    assert_line_to!(events.next(), Point2D::new(409.6, 1638.4));
    assert_line_to!(events.next(), Point2D::new(409.6, 1433.6));
    assert_line_to!(events.next(), Point2D::new(204.8, 1433.6));
    assert_close!(events.next());
}

#[test]
pub fn get_empty_glyph_outline() {
    let mut path_builder = Path::builder();
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char(' ').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::None, &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(events.next(), None);
}

#[cfg(any(target_family = "windows", target_os = "macos", target_os = "ios"))]
#[test]
pub fn get_glyph_typographic_bounds() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(
        font.typographic_bounds(glyph),
        Ok(Rect::new(
            Point2D::new(74.0, -24.0),
            Size2D::new(978.0, 1110.0)
        ))
    );
}

#[cfg(not(any(target_family = "windows", target_os = "macos", target_os = "ios")))]
#[test]
pub fn get_glyph_typographic_bounds() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(
        font.typographic_bounds(glyph),
        Ok(Rect::new(
            Point2D::new(123.0, -29.0),
            Size2D::new(946.0, 1176.0)
        ))
    );
}

#[cfg(target_family = "windows")]
#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2D::new(1139.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Point2D::new(74.0, 1898.0)));
}

#[cfg(target_os = "macos")]
#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2D::new(1139.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Point2D::zero()));
}

#[cfg(not(any(target_family = "windows", target_os = "macos", target_os = "ios")))]
#[test]
pub fn get_glyph_advance_and_origin() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Ok(Vector2D::new(1255.0, 0.0)));
    assert_eq!(font.origin(glyph), Ok(Point2D::zero()));
}

#[cfg(any(target_family = "windows", target_os = "macos"))]
#[test]
pub fn get_font_metrics() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
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

#[cfg(not(any(target_family = "windows", target_os = "macos", target_os = "ios")))]
#[test]
pub fn get_font_metrics() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let metrics = font.metrics();
    assert_eq!(metrics.units_per_em, 2048);
    assert_eq!(metrics.ascent, 1901.0);
    assert_eq!(metrics.descent, -483.0);
    assert_eq!(metrics.line_gap, 0.0); // FIXME(pcwalton): Huh?!
    assert_eq!(metrics.underline_position, -130.0);
    assert_eq!(metrics.underline_thickness, 90.0);
    assert_eq!(metrics.cap_height, 0.0); // FIXME(pcwalton): Huh?!
    assert_eq!(metrics.x_height, 0.0); // FIXME(pcwalton): Huh?!
}

#[test]
pub fn get_font_properties() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let properties = font.properties();
    assert_eq!(properties.weight, Weight(400.0));
    assert_eq!(properties.stretch, Stretch(1.0));
}

#[test]
pub fn get_font_data() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let data = font.copy_font_data().unwrap();
    debug_assert!(utils::SFNT_VERSIONS
        .iter()
        .any(|version| data[0..4] == *version));
}

#[test]
pub fn rasterize_glyph_with_grayscale_aa() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            &FontTransform::identity(),
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )
        .unwrap();
    let origin = Point2D::new(
        -raster_rect.origin.x,
        raster_rect.size.height + raster_rect.origin.y,
    )
    .to_f32();
    let mut canvas = Canvas::new(&raster_rect.size.to_u32(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        &FontTransform::identity(),
        &origin,
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    check_L_shape(&canvas);
}

#[test]
pub fn rasterize_glyph_bilevel() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 16.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            &FontTransform::identity(),
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let origin = Point2D::new(
        -raster_rect.origin.x,
        raster_rect.size.height + raster_rect.origin.y,
    )
    .to_f32();
    let mut canvas = Canvas::new(&raster_rect.size.to_u32(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        &FontTransform::identity(),
        &origin,
        HintingOptions::None,
        RasterizationOptions::Bilevel,
    )
    .unwrap();
    assert!(canvas
        .pixels
        .iter()
        .all(|&value| value == 0 || value == 0xff));
    check_L_shape(&canvas);
}

#[cfg(any(
    not(any(target_os = "macos", target_os = "ios", target_family = "windows")),
    feature = "loader-freetype-default"
))]
#[test]
pub fn rasterize_glyph_with_full_hinting() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            &FontTransform::identity(),
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let origin = Point2D::new(
        -raster_rect.origin.x,
        raster_rect.size.height + raster_rect.origin.y,
    )
    .to_f32();
    let mut canvas = Canvas::new(&raster_rect.size.to_u32(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        &FontTransform::identity(),
        &origin,
        HintingOptions::Full(size),
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    check_L_shape(&canvas);

    // Make sure the top and bottom (non-blank) rows have some fully black pixels in them.
    let mut top_row = &canvas.pixels[0..canvas.stride];
    if top_row.iter().all(|&value| value == 0) {
        top_row = &canvas.pixels[(1 * canvas.stride)..(2 * canvas.stride)];
    }

    assert!(top_row.iter().any(|&value| value == 0xff));
    for y in (0..(canvas.size.height as usize)).rev() {
        let bottom_row = &canvas.pixels[(y * canvas.stride)..((y + 1) * canvas.stride)];
        if bottom_row.iter().all(|&value| value == 0) {
            continue;
        }
        assert!(bottom_row.iter().any(|&value| value == 0xff));
        break;
    }
}

#[cfg(any(target_family = "windows"))]
#[test]
pub fn rasterize_glyph() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('{').unwrap();
    let size = 32.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            &FontTransform::identity(),
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )
        .unwrap();
    let origin = Point2D::new(
        -raster_rect.origin.x,
        raster_rect.size.height + raster_rect.origin.y,
    )
    .to_f32();
    let mut canvas = Canvas::new(&raster_rect.size.to_u32(), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        size,
        &FontTransform::identity(),
        &origin,
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    check_curly_shape(&canvas);
}

#[test]
pub fn font_transform() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let glyph_id = font.glyph_for_char('L').unwrap();
    let size = 16.0;
    let raster_rect = font
        .raster_bounds(
            glyph_id,
            size,
            &FontTransform::identity(),
            &point2(8., 8.),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    let raster_rect2 = font
        .raster_bounds(
            glyph_id,
            size,
            &FontTransform::new(3., 0., 0., 3.),
            &point2(8., 8.),
            HintingOptions::None,
            RasterizationOptions::Bilevel,
        )
        .unwrap();
    assert!((raster_rect2.size.width - raster_rect.size.width * 3).abs() <= 3);
    assert!((raster_rect2.size.height - raster_rect.size.height * 3).abs() <= 3);
    assert!((raster_rect2.origin.x - raster_rect.origin.x).abs() <= 3);
    assert!((raster_rect2.origin.y - raster_rect.origin.y).abs() <= 3);
}

#[test]
fn load_fonts_from_opentype_collection() {
    let mut file = File::open(TEST_FONT_COLLECTION_FILE_PATH).unwrap();
    {
        let font = Font::from_file(&mut file, 0).unwrap();
        assert_eq!(
            font.postscript_name().unwrap(),
            TEST_FONT_COLLECTION_POSTSCRIPT_NAME[0]
        );
    }
    let font = Font::from_file(&mut file, 1).unwrap();
    assert_eq!(
        font.postscript_name().unwrap(),
        TEST_FONT_COLLECTION_POSTSCRIPT_NAME[1]
    );
}

#[test]
fn get_glyph_count() {
    let font = Font::from_path(TEST_FONT_FILE_PATH, 0).unwrap();
    assert_eq!(font.glyph_count(), 3084);
}

// The initial off-curve point used to cause an assertion in the FreeType backend.
#[test]
fn get_glyph_outline_eb_garamond_exclam() {
    let mut path_builder = Path::builder();
    let mut file = File::open(FILE_PATH_EB_GARAMOND_TTF).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char('!').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::None, &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    // The TrueType spec doesn't specify the rounding method for midpoints, as far as I can tell.
    // So we are lenient and accept either values rounded down (what Core Text provides if the
    // first point is off-curve, it seems) or precise floating-point values (what our FreeType
    // loader provides).
    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(114.0, 598.0)))
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(114.0, 619.0),
        Point2D::new(127.5, 634.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(141.0, 649.0),
        Point2D::new(161.0, 649.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(181.0, 649.0),
        Point2D::new(193.5, 634.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(206.0, 619.0),
        Point2D::new(206.0, 598.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(206.0, 526.0),
        Point2D::new(176.0, 244.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(172.0, 205.0),
        Point2D::new(158.0, 205.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(144.0, 205.0),
        Point2D::new(140.0, 244.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(114.0, 491.0),
        Point2D::new(114.0, 598.0)
    );
    assert_close!(events.next());
    let event = events.next();
    assert!(
        event == Some(PathEvent::MoveTo(Point2D::new(117.0, 88.0)))
            || event == Some(PathEvent::MoveTo(Point2D::new(117.5, 88.5)))
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(135.0, 106.0),
        Point2D::new(160.0, 106.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(185.0, 106.0),
        Point2D::new(202.5, 88.5)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(220.0, 71.0),
        Point2D::new(220.0, 46.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(220.0, 21.0),
        Point2D::new(202.5, 3.5)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(185.0, -14.0),
        Point2D::new(160.0, -14.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(135.0, -14.0),
        Point2D::new(117.5, 3.5)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(100.0, 21.0),
        Point2D::new(100.0, 46.0)
    );
    match events.next() {
        Some(PathEvent::Quadratic(ref segment)) => {
            assert_eq!(segment.ctrl, Point2D::new(100.0, 71.0));
            assert!(
                segment.to == Point2D::new(117.0, 88.0) || segment.to == Point2D::new(117.5, 88.5)
            )
        }
        other => panic!("Expected quadratic got {:?}", other),
    }
    assert_close!(events.next());
}

// https://github.com/pcwalton/pathfinder/issues/84
#[allow(non_snake_case)]
#[test]
fn get_glyph_outline_inconsolata_J() {
    let mut path_builder = Path::builder();
    let mut file = File::open(FILE_PATH_INCONSOLATA_TTF).unwrap();
    let font = Font::from_file(&mut file, 0).unwrap();
    let glyph = font.glyph_for_char('J').expect("No glyph for char!");
    font.outline(glyph, HintingOptions::None, &mut path_builder)
        .unwrap();
    let path = path_builder.build();

    let mut events = path.into_iter();
    assert_eq!(
        events.next(),
        Some(PathEvent::MoveTo(Point2D::new(198.0, -11.0)))
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(106.0, -11.0),
        Point2D::new(49.0, 58.0)
    );
    assert_line_to!(events.next(), Point2D::new(89.0, 108.0));
    assert_line_to!(events.next(), Point2D::new(96.0, 116.0));
    assert_line_to!(events.next(), Point2D::new(101.0, 112.0));
    assert_quadratic_to!(
        events.next(),
        Point2D::new(102.0, 102.0),
        Point2D::new(106.0, 95.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(110.0, 88.0),
        Point2D::new(122.0, 78.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(157.0, 51.0),
        Point2D::new(196.0, 51.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(247.0, 51.0),
        Point2D::new(269.5, 86.5)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(292.0, 122.0),
        Point2D::new(292.0, 208.0)
    );
    assert_line_to!(events.next(), Point2D::new(292.0, 564.0));
    assert_line_to!(events.next(), Point2D::new(172.0, 564.0));
    assert_line_to!(events.next(), Point2D::new(172.0, 623.0));
    assert_line_to!(events.next(), Point2D::new(457.0, 623.0));
    assert_line_to!(events.next(), Point2D::new(457.0, 564.0));
    assert_line_to!(events.next(), Point2D::new(361.0, 564.0));
    assert_line_to!(events.next(), Point2D::new(361.0, 209.0));
    assert_quadratic_to!(
        events.next(),
        Point2D::new(363.0, 133.0),
        Point2D::new(341.0, 84.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(319.0, 35.0),
        Point2D::new(281.5, 12.0)
    );
    assert_quadratic_to!(
        events.next(),
        Point2D::new(244.0, -11.0),
        Point2D::new(198.0, -11.0)
    );
    assert_close!(events.next());
}

// Makes sure that a canvas has an "L" shape in it. This is used to test rasterization.
#[allow(non_snake_case)]
fn check_L_shape(canvas: &Canvas) {
    // Find any empty rows at the start.
    let mut y = 0;
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        if canvas.pixels[row_start..row_end].iter().any(|&p| p != 0) {
            break;
        }
        y += 1;
    }
    assert!(y < canvas.size.height);

    // Find the top part of the L.
    let mut top_stripe_width = None;
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        if let Some(stripe_width) = stripe_width(&canvas.pixels[row_start..row_end]) {
            if let Some(top_stripe_width) = top_stripe_width {
                if stripe_width > top_stripe_width {
                    break;
                }
                assert_eq!(stripe_width, top_stripe_width);
            }
            top_stripe_width = Some(stripe_width);
        }
        y += 1;
    }
    assert!(y < canvas.size.height);

    // Find the bottom part of the L.
    let mut bottom_stripe_width = None;
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        if let Some(stripe_width) = stripe_width(&canvas.pixels[row_start..row_end]) {
            if let Some(bottom_stripe_width) = bottom_stripe_width {
                assert!(bottom_stripe_width > top_stripe_width.unwrap());
                assert_eq!(stripe_width, bottom_stripe_width);
            }
            bottom_stripe_width = Some(stripe_width);
        }
    }

    // Find any empty rows at the end.
    while y < canvas.size.height {
        let (row_start, row_end) = (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        y += 1;
        if canvas.pixels[row_start..row_end].iter().any(|&p| p != 0) {
            break;
        }
    }

    // Make sure we made it to the end.
    assert_eq!(y, canvas.size.height);
}

// Makes sure that a canvas has an "{" shape in it. This is used to test rasterization.
fn check_curly_shape(canvas: &Canvas) {
    let mut y = 0;
    let height = canvas.size.height;
    // check the upper row and the lower rows are symmetrical
    while y < height / 2 {
        let (upper_row_start, upper_row_end) =
            (canvas.stride * y as usize, canvas.stride * (y + 1) as usize);
        let (lower_row_start, lower_row_end) = (
            canvas.stride * (height - y - 1) as usize,
            canvas.stride * (height - y) as usize,
        );
        let upper_row = &canvas.pixels[upper_row_start..upper_row_end];
        let lower_row = &canvas.pixels[lower_row_start..lower_row_end];
        let top_row_width = stripe_width(upper_row).unwrap();
        let lower_row_width = stripe_width(lower_row).unwrap();
        let upper_row_pixel_start = stride_pixel_start(upper_row).unwrap();
        let lower_row_pixel_start = stride_pixel_start(lower_row).unwrap();
        if top_row_width == lower_row_width {
            // check non-zero pixels start at the same index
            assert_eq!(upper_row_pixel_start, lower_row_pixel_start);
        } else {
            //if not, assert that the difference is not greater than 1
            assert_eq!((lower_row_width as i32 - top_row_width as i32).abs(), 1);
            // and assert that the non-zero pixel index difference is not greater than 1
            assert_eq!(
                (upper_row_pixel_start as i32 - lower_row_pixel_start as i32).abs(),
                1
            );
        }
        y += 1;
    }
}

// return the first non-zero pixel index
fn stride_pixel_start(pixels: &[u8]) -> Option<u32> {
    let mut index = 0;
    for x in pixels {
        if *x != 0 {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn stripe_width(pixels: &[u8]) -> Option<u32> {
    let mut x = 0;
    // Find the initial empty part.
    while x < pixels.len() && pixels[x] == 0 {
        x += 1
    }
    if x == pixels.len() {
        return None;
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
    Some(stripe_width)
}
