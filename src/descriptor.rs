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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spec {
    pub families: Vec<FamilySpec>,
    pub properties: Properties,
}

impl Spec {
    #[inline]
    pub fn new() -> Spec {
        Spec::default()
    }

    #[inline]
    pub fn family(&mut self, family_name: &str) -> &mut Spec {
        self.families.push(FamilySpec::Name(family_name.to_owned()));
        self
    }

    #[inline]
    pub fn serif(&mut self) -> &mut Spec {
        self.families.push(FamilySpec::Serif);
        self
    }

    #[inline]
    pub fn sans_serif(&mut self) -> &mut Spec {
        self.families.push(FamilySpec::SansSerif);
        self
    }

    #[inline]
    pub fn monospace(&mut self) -> &mut Spec {
        self.families.push(FamilySpec::Monospace);
        self
    }

    #[inline]
    pub fn cursive(&mut self) -> &mut Spec {
        self.families.push(FamilySpec::Cursive);
        self
    }

    #[inline]
    pub fn fantasy(&mut self) -> &mut Spec {
        self.families.push(FamilySpec::Fantasy);
        self
    }

    #[inline]
    pub fn style(&mut self, style: Style) -> &mut Spec {
        self.properties.style = style;
        self
    }

    #[inline]
    pub fn weight(&mut self, weight: Weight) -> &mut Spec {
        self.properties.weight = weight;
        self
    }

    #[inline]
    pub fn stretch(&mut self, stretch: Stretch) -> &mut Spec {
        self.properties.stretch = stretch;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Properties {
    /// The font style, as defined in CSS.
    pub style: Style,
    /// The font weight, as defined in CSS.
    pub weight: Weight,
    /// The font stretchiness, as defined in CSS.
    pub stretch: Stretch,
}

// TODO(pcwalton): `system-ui`, `emoji`, `math`, `fangsong`
#[derive(Clone, Debug, PartialEq)]
pub enum FamilySpec {
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

#[derive(Clone, Copy, Debug, PartialEq)]
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


#[derive(Clone, Copy, Debug, PartialEq)]
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
