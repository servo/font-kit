// font-kit/src/family.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use float_ord::FloatOrd;
use std::iter;

use descriptor::{Properties, Stretch, Style, Weight};
use font::{Face, Font};

#[derive(Debug)]
pub struct Family<F = Font> where F: Face {
    pub fonts: Vec<F>,
}

impl<F> Family<F> where F: Face {
    #[inline]
    pub fn new() -> Family<F> {
        Family {
            fonts: vec![],
        }
    }

    #[inline]
    pub fn from_fonts<I>(fonts: I) -> Family<F> where I: Iterator<Item = F> {
        Family {
            fonts: fonts.collect::<Vec<F>>(),
        }
    }

    /// A convenience method to create a family with a single font.
    #[inline]
    pub fn from_font(font: F) -> Family<F> {
        Family::from_fonts(iter::once(font))
    }

    #[inline]
    pub fn fonts(&self) -> &[F] {
        &self.fonts
    }

    #[inline]
    pub fn push(&mut self, font: F) {
        self.fonts.push(font)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// This follows CSS Fonts Level 3 § 5.2 [1].
    ///
    /// https://drafts.csswg.org/css-fonts-3/#font-style-matching
    ///
    /// FIXME(pcwalton): Cache font properties!
    pub fn find(&self, query: &Properties) -> Result<F, ()> {
        // Step 4.
        let mut matching_set = self.fonts.clone();
        if matching_set.is_empty() {
            return Err(())
        }

        // Step 4a (`font-stretch`).
        let matching_stretch =
            if matching_set.iter().any(|font| font.properties().stretch == query.stretch) {
                // Exact match.
                query.stretch
            } else if query.stretch <= Stretch::NORMAL {
                // Closest width, first checking narrower values and then wider values.
                match matching_set.iter()
                                  .filter(|font| font.properties().stretch < query.stretch)
                                  .min_by_key(|font| {
                                      FloatOrd(query.stretch.0 - font.properties().stretch.0)
                                  }) {
                    Some(matching_font) => matching_font.properties().stretch,
                    None => {
                        matching_set.iter()
                                    .min_by_key(|font| {
                                        FloatOrd(font.properties().stretch.0 - query.stretch.0)
                                    })
                                    .unwrap()
                                    .properties()
                                    .stretch
                    }
                }
            } else {
                // Closest width, first checking wider values and then narrower values.
                match matching_set.iter()
                                  .filter(|font| font.properties().stretch > query.stretch)
                                  .min_by_key(|font| {
                                      FloatOrd(font.properties().stretch.0 - query.stretch.0)
                                  }) {
                    Some(matching_font) => matching_font.properties().stretch,
                    None => {
                        matching_set.iter()
                                    .min_by_key(|font| {
                                        FloatOrd(query.stretch.0 - font.properties().stretch.0)
                                    })
                                    .unwrap()
                                    .properties()
                                    .stretch
                    }
                }
            };
        matching_set.retain(|font| font.properties().stretch == matching_stretch);

        // Step 4b (`font-style`).
        let style_preference = match query.style {
            Style::Italic => [Style::Italic, Style::Oblique, Style::Normal],
            Style::Oblique => [Style::Oblique, Style::Italic, Style::Normal],
            Style::Normal => [Style::Normal, Style::Oblique, Style::Italic],
        };
        let matching_style = *style_preference.iter().filter(|&query_style| {
            matching_set.iter().any(|font| font.properties().style == *query_style)
        }).next().unwrap();
        matching_set.retain(|font| font.properties().style == matching_style);

        // Step 4c (`font-weight`).
        //
        // The spec doesn't say what to do if the weight is between 400 and 500 exclusive, so we
        // just use 450 as the cutoff.
        let matching_weight =
            if query.weight >= Weight(400.0) && query.weight < Weight(450.0) &&
                    matching_set.iter().any(|font| font.properties().weight == Weight(500.0)) {
                // Check 500 first.
                Weight(500.0)
            } else if query.weight >= Weight(450.0) && query.weight <= Weight(500.0) &&
                    matching_set.iter().any(|font| font.properties().weight == Weight(400.0)) {
                // Check 400 first.
                Weight(400.0)
            } else if query.weight <= Weight(500.0) {
                // Closest weight, first checking thinner values and then fatter ones.
                match matching_set.iter()
                                  .filter(|font| font.properties().weight < query.weight)
                                  .min_by_key(|font| {
                                      FloatOrd(query.weight.0 - font.properties().weight.0)
                                  }) {
                    Some(matching_font) => matching_font.properties().weight,
                    None => {
                        matching_set.iter()
                                    .min_by_key(|font| {
                                        FloatOrd(font.properties().weight.0 - query.weight.0)
                                    })
                                    .unwrap()
                                    .properties()
                                    .weight
                    }
                }
            } else {
                // Closest weight, first checking fatter values and then thinner ones.
                match matching_set.iter()
                                  .filter(|font| font.properties().weight > query.weight)
                                  .min_by_key(|font| {
                                      FloatOrd(font.properties().weight.0 - query.weight.0)
                                  }) {
                    Some(matching_font) => matching_font.properties().weight,
                    None => {
                        matching_set.iter()
                                    .min_by_key(|font| {
                                        FloatOrd(query.weight.0 - font.properties().weight.0)
                                    })
                                    .unwrap()
                                    .properties()
                                    .weight
                    }
                }
            };
        matching_set.retain(|font| font.properties().weight == matching_weight);

        // Step 4d concerns `font-size`, but fonts in `font-kit` are unsized, so we ignore that.

        // Return the result.
        matching_set.into_iter().next().ok_or(())
    }
}
