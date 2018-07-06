// font-kit/src/platform/windows.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dwrote::Font as DWriteFont;
use dwrote::FontCollection as DWriteFontCollection;
use dwrote::FontFace as DWriteFontFace;
use dwrote::FontFile as DWriteFontFile;
use dwrote::FontSimulations as DWriteFontSimulations;
use dwrote::FontStyle as DWriteFontStyle;
use dwrote::InformationalStringId as DWriteInformationalStringId;
use dwrote::OutlineBuilder;
use euclid::{Point2D, Rect, Size2D, Vector2D};
use lyon_path::PathEvent;
use lyon_path::builder::PathBuilder;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use winapi::dwrite::DWRITE_FONT_SIMULATIONS;

use descriptor::{Descriptor, Flags, FONT_STRETCH_MAPPING, Query, QueryFields};
use family::Family;
use font::Metrics;
use set::Set;

// TODO(pcwalton)
pub type NativeFont = DWriteFontFace;

pub struct Font {
    dwrite_font_face: DWriteFontFace,
    cached_data: Mutex<Option<Arc<Vec<u8>>>>,
}

impl Font {
    pub fn from_bytes(font_data: Arc<Vec<u8>>) -> Result<Font, ()> {
        let font_file = try!(DWriteFontFile::new_from_data(&**font_data).ok_or(()));
        // TODO(pcwalton): Support grabbing other faces from a single TrueType collection.
        let face = font_file.create_face(0, DWRITE_FONT_SIMULATIONS(0));
        Ok(Font {
            dwrite_font_face: face,
            cached_data: Mutex::new(Some(font_data)),
        })
    }

    pub fn from_file(mut file: File) -> Result<Font, ()> {
        let mut font_data = vec![];
        try!(file.read_to_end(&mut font_data).map_err(drop));
        Font::from_bytes(Arc::new(font_data))
    }

    // TODO(pcwalton)
    pub unsafe fn from_native_font(dwrite_font_face: NativeFont) -> Font {
        Font {
            dwrite_font_face,
            cached_data: Mutex::new(None),
        }
    }

    // TODO(pcwalton)
    pub fn descriptor(&self) -> Descriptor {
        let dwrite_font = self.dwrite_font_face.get_font();
        let family_name = dwrite_font.family_name();

        let mut flags = Flags::empty();
        if dwrite_style_is_italic(dwrite_font.style()) {
            flags.insert(Flags::ITALIC)
        }

        // TODO(pcwalton): Monospace, once we have a `winapi` upgrade.
        // FIXME(pcwalton): How do we identify vertical fonts?

        Descriptor {
            postscript_name:
                dwrite_font.informational_string(DWriteInformationalStringId::PostscriptName)
                           .unwrap_or_else(|| family_name.clone()),
            display_name:
                dwrite_font.informational_string(DWriteInformationalStringId::FullName)
                           .unwrap_or_else(|| family_name.clone()),
            family_name,
            style_name: style_name_for_dwrite_style(dwrite_font.style()).to_owned(),
            stretch: FONT_STRETCH_MAPPING[(dwrite_font.stretch() as usize) - 1],
            weight: dwrite_font.weight() as u32 as f32,
            flags,
        }
    }

    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        let chars = [character as u32];
        self.dwrite_font_face.get_glyph_indices(&chars).into_iter().next().map(|g| g as u32)
    }

    pub fn outline<B>(&self, glyph_id: u32, path_builder: &mut B) -> Result<(), ()>
                      where B: PathBuilder {
        let outline_buffer = OutlineBuffer::new();
        self.dwrite_font_face.get_glyph_run_outline(self.metrics().units_per_em as f32,
                                                    &[glyph_id as u16],
                                                    None,
                                                    None,
                                                    false,
                                                    false,
                                                    Box::new(outline_buffer.clone()));
        outline_buffer.flush(path_builder);
        Ok(())
    }

    pub fn typographic_bounds(&self, glyph_id: u32) -> Rect<f32> {
        let metrics = self.dwrite_font_face.get_design_glyph_metrics(&[glyph_id as u16], false);

        let metrics = &metrics[0];
        let advance_width = metrics.advanceWidth as i32;
        let advance_height = metrics.advanceHeight as i32;
        let left_side_bearing = metrics.leftSideBearing as i32;
        let right_side_bearing = metrics.rightSideBearing as i32;
        let top_side_bearing = metrics.topSideBearing as i32;
        let bottom_side_bearing = metrics.bottomSideBearing as i32;
        let vertical_origin_y = metrics.verticalOriginY as i32;

        let y_offset = vertical_origin_y + bottom_side_bearing - advance_height;
        let width = advance_width - (left_side_bearing + right_side_bearing);
        let height = advance_height - (top_side_bearing + bottom_side_bearing);

        Rect::new(Point2D::new(left_side_bearing as f32, y_offset as f32),
                  Size2D::new(width as f32, height as f32))
    }

    pub fn advance(&self, glyph_id: u32) -> Vector2D<f32> {
        let metrics = self.dwrite_font_face.get_design_glyph_metrics(&[glyph_id as u16], false);
        let metrics = &metrics[0];
        Vector2D::new(metrics.advanceWidth as f32, 0.0)
    }

    pub fn origin(&self, _: u32) -> Point2D<f32> {
        // FIXME(pcwalton): This can't be right!
        Point2D::zero()
    }

    pub fn metrics(&self) -> Metrics {
        let dwrite_font = self.dwrite_font_face.get_font();
        let dwrite_metrics = dwrite_font.metrics();
        Metrics {
            units_per_em: dwrite_metrics.designUnitsPerEm as u32,
            ascent: dwrite_metrics.ascent as f32,
            descent: -dwrite_metrics.descent as f32,
            line_gap: dwrite_metrics.lineGap as f32,
            cap_height: dwrite_metrics.capHeight as f32,
            x_height: dwrite_metrics.xHeight as f32,
            underline_position: dwrite_metrics.underlinePosition as f32,
            underline_thickness: dwrite_metrics.underlineThickness as f32,
        }
    }

    pub fn font_data(&self) -> Option<FontData> {
        let mut font_data = self.cached_data.lock().unwrap();
        if font_data.is_none() {
            let files = self.dwrite_font_face.get_files();
            // FIXME(pcwalton): Is this right? When can a font have multiple files?
            if let Some(file) = files.get(0) {
                *font_data = Some(Arc::new(file.get_font_file_bytes()))
            }
        }

        if font_data.is_none() {
            None
        } else {
            Some(FontData {
                font_data,
            })
        }
    }
}

impl Clone for Font {
    #[inline]
    fn clone(&self) -> Font {
        Font {
            dwrite_font_face: self.dwrite_font_face.clone(),
            cached_data: Mutex::new((*self.cached_data.lock().unwrap()).clone())
        }
    }
}

impl Query {
    pub fn lookup(&self) -> Set {
        let system_font_collection = DWriteFontCollection::system();
        let mut set = Set::new();
        for dwrite_family in system_font_collection.families_iter() {
            let mut family = Family::new();
            for font_index in 0..dwrite_family.get_font_count() {
                let dwrite_font = dwrite_family.get_font(font_index);
                if self.matches_dwrite_font(&dwrite_font) {
                    family.push(Font::from_native_font(dwrite_font.create_font_face()))
                }
            }
            if !family.fonts().is_empty() {
                set.push(family)
            }
        }
        set
    }

    fn matches_dwrite_font(&self, dwrite_font: &DWriteFont) -> bool {
        if dwrite_font.simulations() != DWriteFontSimulations::None {
            return false
        }

        if self.fields.contains(QueryFields::POSTSCRIPT_NAME) &&
                !self.matches_informational_string(dwrite_font,
                                                   &self.descriptor.postscript_name,
                                                   DWriteInformationalStringId::PostscriptName) {
            return false
        }
        if self.fields.contains(QueryFields::DISPLAY_NAME) &&
                !self.matches_informational_string(dwrite_font,
                                                   &self.descriptor.display_name,
                                                   DWriteInformationalStringId::FullName) {
            return false
        }
        if self.fields.contains(QueryFields::FAMILY_NAME) &&
                dwrite_font.family_name() != self.descriptor.family_name {
            return false
        }
        if self.fields.contains(QueryFields::STYLE_NAME) &&
                style_name_for_dwrite_style(dwrite_font.style()) != self.descriptor.style_name {
            return false
        }
        if self.fields.contains(QueryFields::WEIGHT) &&
                dwrite_font.weight() as u32 as f32 != self.descriptor.weight {
            return false
        }
        if self.fields.contains(QueryFields::STRETCH) &&
                FONT_STRETCH_MAPPING[(dwrite_font.stretch() as usize) - 1] !=
                self.descriptor.stretch {
            return false
        }
        if self.fields.contains(QueryFields::ITALIC) &&
                dwrite_style_is_italic(dwrite_font.style()) !=
                self.descriptor.flags.contains(Flags::ITALIC) {
            return false
        }
        // TODO(pcwalton): Monospace, once we have a `winapi` upgrade.
        // FIXME(pcwalton): How do we identify vertical fonts?
        true
    }

    fn matches_informational_string(&self,
                                    dwrite_font: &DWriteFont,
                                    query_name: &str,
                                    id: DWriteInformationalStringId)
                                    -> bool {
        match dwrite_font.informational_string(id) {
            None => false,
            Some(name) => name == query_name,
        }
    }
}

pub struct FontData<'a> {
    font_data: MutexGuard<'a, Option<Arc<Vec<u8>>>>,
}

impl<'a> Deref for FontData<'a> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &[u8] {
        &***self.font_data.as_ref().unwrap()
    }
}

#[derive(Clone)]
struct OutlineBuffer {
    path_events: Arc<Mutex<Vec<PathEvent>>>,
}

impl OutlineBuffer {
    pub fn new() -> OutlineBuffer {
        OutlineBuffer {
            path_events: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn flush<PB>(&self, path_builder: &mut PB) where PB: PathBuilder {
        let mut path_events = self.path_events.lock().unwrap();
        for path_event in path_events.drain(..) {
            path_builder.path_event(path_event)
        }
    }
}

impl OutlineBuilder for OutlineBuffer {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path_events.lock().unwrap().push(PathEvent::MoveTo(Point2D::new(x, -y)))
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path_events.lock().unwrap().push(PathEvent::LineTo(Point2D::new(x, -y)))

    }
    fn curve_to(&mut self, cp0x: f32, cp0y: f32, cp1x: f32, cp1y: f32, x: f32, y: f32) {
        self.path_events.lock().unwrap().push(PathEvent::CubicTo(Point2D::new(cp0x, -cp0y),
                                                                 Point2D::new(cp1x, -cp1y),
                                                                 Point2D::new(x, -y)))

    }
    fn close(&mut self) {
        self.path_events.lock().unwrap().push(PathEvent::Close)
    }
}

fn style_name_for_dwrite_style(style: DWriteFontStyle) -> &'static str {
    match style {
        DWriteFontStyle::Normal => "Regular",
        DWriteFontStyle::Oblique => "Oblique",
        DWriteFontStyle::Italic => "Italic",
    }
}

fn dwrite_style_is_italic(style: DWriteFontStyle) -> bool {
    match style {
        DWriteFontStyle::Normal => false,
        DWriteFontStyle::Oblique | DWriteFontStyle::Italic => true,
    }
}
