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

use descriptor::{WEIGHT_NORMAL, WEIGHT_BOLD, Flags, Query};
use font::Font;

// TODO(pcwalton): Change this to DejaVu or whatever on Linux.
static SANS_SERIF_FONT_FAMILY_NAME: &'static str = "Arial";
static SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME: &'static str = "ArialMT";
static SANS_SERIF_FONT_BOLD_POSTSCRIPT_NAME: &'static str = "Arial-BoldMT";
static SANS_SERIF_FONT_ITALIC_POSTSCRIPT_NAME: &'static str = "Arial-ItalicMT";

static TEST_FONT_FILE_PATH: &'static str = "resources/tests/EBGaramond12-Regular.otf";
static TEST_FONT_POSTSCRIPT_NAME: &'static str = "EBGaramond12-Regular";

#[test]
pub fn lookup_single_regular_font() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME)
                            .weight(WEIGHT_NORMAL)
                            .italic(false)
                            .lookup();
    assert_eq!(fonts.families().len(), 1);
    let family = &fonts.families()[0];
    assert_eq!(family.fonts().len(), 1);
    let font = &family.fonts()[0];
    assert_eq!(font.descriptor().postscript_name, SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME);
}

#[test]
pub fn lookup_single_bold_font() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME)
                            .weight(WEIGHT_BOLD)
                            .italic(false)
                            .lookup();
    assert_eq!(fonts.families().len(), 1);
    let family = &fonts.families()[0];
    assert_eq!(family.fonts().len(), 1);
    let font = &family.fonts()[0];
    assert_eq!(font.descriptor().postscript_name, SANS_SERIF_FONT_BOLD_POSTSCRIPT_NAME);
}

#[test]
pub fn lookup_single_italic_font() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME)
                            .weight(WEIGHT_NORMAL)
                            .italic(true)
                            .lookup();
    assert_eq!(fonts.families().len(), 1);
    let family = &fonts.families()[0];
    assert_eq!(family.fonts().len(), 1);
    let font = &family.fonts()[0];
    assert_eq!(font.descriptor().postscript_name, SANS_SERIF_FONT_ITALIC_POSTSCRIPT_NAME);
}

#[test]
pub fn lookup_all_fonts_in_a_family() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME).lookup();
    assert_eq!(fonts.families().len(), 1);
    let family = &fonts.families()[0];
    assert!(family.fonts().len() > 2);
}

#[test]
pub fn load_font_from_file() {
    let file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let font = Font::from_file(file).unwrap();
    assert_eq!(font.descriptor().postscript_name, TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn load_font_from_memory() {
    let mut file = File::open(TEST_FONT_FILE_PATH).unwrap();
    let mut font_data = vec![];
    file.read_to_end(&mut font_data).unwrap();
    let font = Font::from_bytes(Arc::new(font_data)).unwrap();
    assert_eq!(font.descriptor().postscript_name, TEST_FONT_POSTSCRIPT_NAME);
}

#[test]
pub fn get_glyph_for_char() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME).lookup();
    let font = &fonts.families()[0].fonts()[0];
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(glyph, 68);
}

#[test]
pub fn get_glyph_outline() {
    let mut path_builder = Path::builder();
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME).lookup();
    let font = &fonts.families()[0].fonts()[0];
    let glyph = font.glyph_for_char('i').expect("No glyph for char!");
    font.outline(glyph, &mut path_builder).unwrap();
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

#[test]
pub fn get_glyph_typographic_bounds() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME)
                            .weight(WEIGHT_NORMAL)
                            .italic(false)
                            .lookup();
    let font = &fonts.families()[0].fonts()[0];
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.typographic_bounds(glyph),
               Rect::new(Point2D::new(74.0, -24.0), Size2D::new(978.0, 1110.0)));
}

#[test]
pub fn get_glyph_advance_and_origin() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME)
                            .weight(WEIGHT_NORMAL)
                            .italic(false)
                            .lookup();
    let font = &fonts.families()[0].fonts()[0];
    let glyph = font.glyph_for_char('a').expect("No glyph for char!");
    assert_eq!(font.advance(glyph), Vector2D::new(1139.0, 0.0));
    assert_eq!(font.origin(glyph), Point2D::zero());
}

#[test]
pub fn get_font_metrics() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME)
                            .weight(WEIGHT_NORMAL)
                            .italic(false)
                            .lookup();
    let font = &fonts.families()[0].fonts()[0];
    let metrics = font.metrics();
    assert_eq!(metrics.units_per_em, 2048);
    assert_eq!(metrics.ascent, 1854.0);
    assert_eq!(metrics.descent, 434.0);
    assert_eq!(metrics.leading, 67.0);
    assert_eq!(metrics.underline_position, -217.0);
    assert_eq!(metrics.underline_thickness, 150.0);
    assert_eq!(metrics.cap_height, 1467.0);
    assert_eq!(metrics.x_height, 1062.0);
}

#[test]
pub fn get_font_descriptor() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME).lookup();
    let font = &fonts.families()[0].fonts()[0];
    let descriptor = font.descriptor();
    assert_eq!(descriptor.postscript_name, SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME);
    assert_eq!(descriptor.display_name, "Arial");
    assert_eq!(descriptor.family_name, "Arial");
    assert_eq!(descriptor.style_name, "Regular");
    assert_eq!(descriptor.weight, 400.0);
    assert_eq!(descriptor.stretch, 1.0);
    assert_eq!(descriptor.flags, Flags::empty());
}

#[test]
pub fn get_font_data() {
    let fonts = Query::new().family_name(SANS_SERIF_FONT_FAMILY_NAME).lookup();
    let font = &fonts.families()[0].fonts()[0];
    let data = font.font_data().unwrap();
    let magic = &data[0..4];
    assert!(magic == &[0, 1, 0, 0] || magic == b"OTTO");
}
