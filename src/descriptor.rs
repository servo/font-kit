// font-kit/src/descriptor.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Mapping from `usWidthClass` values to CSS `font-stretch` values.
pub(crate) const FONT_STRETCH_MAPPING: [f32; 9] = [
    Stretch::ULTRA_CONDENSED.0,
    Stretch::EXTRA_CONDENSED.0,
    Stretch::CONDENSED.0,
    Stretch::SEMI_CONDENSED.0,
    Stretch::NORMAL.0,
    Stretch::SEMI_EXPANDED.0,
    Stretch::EXPANDED.0,
    Stretch::EXTRA_EXPANDED.0,
    Stretch::ULTRA_EXPANDED.0,
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Properties {
    /// The font style, as defined in CSS.
    pub style: Style,
    /// The font weight, as defined in CSS.
    pub weight: Weight,
    /// The font stretchiness, as defined in CSS.
    pub stretch: Stretch,
}

impl Properties {
    #[inline]
    pub fn new() -> Properties {
        Properties::default()
    }

    #[inline]
    pub fn style(&mut self, style: Style) -> &mut Properties {
        self.style = style;
        self
    }

    #[inline]
    pub fn weight(&mut self, weight: Weight) -> &mut Properties {
        self.weight = weight;
        self
    }

    #[inline]
    pub fn stretch(&mut self, stretch: Stretch) -> &mut Properties {
        self.stretch = stretch;
        self
    }
}

/// A possible value for the `font-family` CSS property.
///
/// TODO(pcwalton): `system-ui`, `emoji`, `math`, `fangsong`
#[derive(Clone, Debug, PartialEq)]
pub enum Class {
    Name(String),
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    Normal,
    Italic,
    Oblique,
}

impl Default for Style {
    fn default() -> Style {
        Style::Normal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Weight(pub f32);

impl Default for Weight {
    #[inline]
    fn default() -> Weight {
        Weight::NORMAL
    }
}

impl Weight {
    pub const THIN: Weight = Weight(100.0);
    pub const EXTRA_LIGHT: Weight = Weight(200.0);
    pub const LIGHT: Weight = Weight(300.0);
    pub const NORMAL: Weight = Weight(400.0);
    pub const MEDIUM: Weight = Weight(500.0);
    pub const SEMIBOLD: Weight = Weight(600.0);
    pub const BOLD: Weight = Weight(700.0);
    pub const EXTRA_BOLD: Weight = Weight(800.0);
    pub const BLACK: Weight = Weight(900.0);
}


#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Stretch(pub f32);

impl Default for Stretch {
    #[inline]
    fn default() -> Stretch {
        Stretch::NORMAL
    }
}

impl Stretch {
    pub const ULTRA_CONDENSED: Stretch = Stretch(0.5);
    pub const EXTRA_CONDENSED: Stretch = Stretch(0.625);
    pub const CONDENSED: Stretch = Stretch(0.75);
    pub const SEMI_CONDENSED: Stretch = Stretch(0.875);
    pub const NORMAL: Stretch = Stretch(1.0);
    pub const SEMI_EXPANDED: Stretch = Stretch(1.125);
    pub const EXPANDED: Stretch = Stretch(1.25);
    pub const EXTRA_EXPANDED: Stretch = Stretch(1.5);
    pub const ULTRA_EXPANDED: Stretch = Stretch(2.0);
}
