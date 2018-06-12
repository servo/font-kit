// font-kit/src/descriptor.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub const WEIGHT_THIN: f32 = 100.0;
pub const WEIGHT_EXTRA_LIGHT: f32 = 200.0;
pub const WEIGHT_LIGHT: f32 = 300.0;
pub const WEIGHT_NORMAL: f32 = 400.0;
pub const WEIGHT_MEDIUM: f32 = 500.0;
pub const WEIGHT_SEMIBOLD: f32 = 600.0;
pub const WEIGHT_BOLD: f32 = 700.0;
pub const WEIGHT_EXTRA_BOLD: f32 = 800.0;
pub const WEIGHT_BLACK: f32 = 900.0;

pub const STRETCH_ULTRA_CONDENSED: f32 = 0.5;
pub const STRETCH_EXTRA_CONDENSED: f32 = 0.625;
pub const STRETCH_CONDENSED: f32 = 0.75;
pub const STRETCH_SEMI_CONDENSED: f32 = 0.875;
pub const STRETCH_NORMAL: f32 = 1.0;
pub const STRETCH_SEMI_EXPANDED: f32 = 1.125;
pub const STRETCH_EXPANDED: f32 = 1.25;
pub const STRETCH_EXTRA_EXPANDED: f32 = 1.5;
pub const STRETCH_ULTRA_EXPANDED: f32 = 2.0;

#[derive(Clone, Debug, Default)]
pub struct Descriptor {
    /// PostScript name of the font.
    pub postscript_name: String,
    /// Display name of the font.
    pub display_name: String,
    /// Name of the font family.
    pub family_name: String,
    /// Designer's description of the font's style.
    pub style_name: String,
    /// The font weight, as defined in CSS.
    pub weight: f32,
    /// The font stretchiness, as defined in CSS.
    pub stretch: f32,
    /// Various flags.
    pub flags: Flags,
}

impl Descriptor {
    #[inline]
    pub fn new() -> Descriptor {
        Descriptor::default()
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Flags: u8 {
        const ITALIC = 0x01;
        const MONOSPACE = 0x02;
        const VERTICAL = 0x04;
    }
}

#[derive(Clone, Debug, Default)]
pub struct Query {
    pub(crate) descriptor: Descriptor,
    pub(crate) fields: QueryFields,
}

impl Query {
    #[inline]
    pub fn new() -> Query {
        Query::default()
    }

    #[inline]
    pub fn is_universal(&self) -> bool {
        self.fields.is_empty()
    }

    #[inline]
    pub fn postscript_name<'a, 'b>(&'a mut self, name: &'b str) -> &'a mut Query {
        self.descriptor.postscript_name = name.to_owned();
        self.fields |= QueryFields::POSTSCRIPT_NAME;
        self
    }

    #[inline]
    pub fn display_name<'a, 'b>(&'a mut self, name: &'b str) -> &'a mut Query {
        self.descriptor.display_name = name.to_owned();
        self.fields |= QueryFields::DISPLAY_NAME;
        self
    }

    #[inline]
    pub fn family_name<'a, 'b>(&'a mut self, name: &'b str) -> &'a mut Query {
        self.descriptor.family_name = name.to_owned();
        self.fields |= QueryFields::FAMILY_NAME;
        self
    }

    #[inline]
    pub fn style_name<'a, 'b>(&'a mut self, name: &'b str) -> &'a mut Query {
        self.descriptor.style_name = name.to_owned();
        self.fields |= QueryFields::STYLE_NAME;
        self
    }

    #[inline]
    pub fn weight(&mut self, weight: f32) -> &mut Query {
        self.descriptor.weight = weight;
        self.fields |= QueryFields::WEIGHT;
        self
    }

    #[inline]
    pub fn stretch(&mut self, stretch: f32) -> &mut Query {
        self.descriptor.stretch = stretch;
        self.fields |= QueryFields::STRETCH;
        self
    }

    #[inline]
    pub fn italic(&mut self, italic: bool) -> &mut Query {
        self.descriptor.flags.set(Flags::ITALIC, italic);
        self.fields |= QueryFields::ITALIC;
        self
    }

    #[inline]
    pub fn monospace(&mut self, monospace: bool) -> &mut Query {
        self.descriptor.flags.set(Flags::MONOSPACE, monospace);
        self.fields |= QueryFields::MONOSPACE;
        self
    }

    #[inline]
    pub fn vertical(&mut self, vertical: bool) -> &mut Query {
        self.descriptor.flags.set(Flags::VERTICAL, vertical);
        self.fields |= QueryFields::VERTICAL;
        self
    }
}

bitflags! {
    #[derive(Default)]
    pub struct QueryFields: u16 {
        const POSTSCRIPT_NAME = 0x001;
        const DISPLAY_NAME = 0x002;
        const FAMILY_NAME = 0x004;
        const STYLE_NAME = 0x008;
        const WEIGHT = 0x010;
        const STRETCH = 0x020;
        const ITALIC = 0x040;
        const MONOSPACE = 0x080;
        const VERTICAL = 0x100;
    }
}
