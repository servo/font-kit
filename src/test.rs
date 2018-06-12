// font-kit/src/test.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use euclid::Point2D;
use lyon_path::PathEvent;
use lyon_path::builder::FlatPathBuilder;
use lyon_path::default::Path;

use descriptor::{WEIGHT_NORMAL, WEIGHT_BOLD, Query};

// TODO(pcwalton): Change this to DejaVu or whatever on Linux.
static SANS_SERIF_FONT_FAMILY_NAME: &'static str = "Arial";
static SANS_SERIF_FONT_REGULAR_POSTSCRIPT_NAME: &'static str = "ArialMT";
static SANS_SERIF_FONT_BOLD_POSTSCRIPT_NAME: &'static str = "Arial-BoldMT";
static SANS_SERIF_FONT_ITALIC_POSTSCRIPT_NAME: &'static str = "Arial-ItalicMT";

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
