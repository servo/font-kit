// font-kit/examples/render-glyph.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate euclid;
extern crate font_kit;

use euclid::Point2D;
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::hinting::HintingOptions;
use font_kit::source::SystemSource;
use std::env;
use std::process;

fn usage() -> ! {
    eprintln!("usage: render-glyph POSTSCRIPT-NAME CHARACTER SIZE");
    eprintln!("    example: render-glyph ArialMT a 32");
    process::exit(0)
}

fn main() {
    let postscript_name = match env::args().skip(1).next() {
        None => usage(),
        Some(postscript_name) => postscript_name,
    };
    let character = match env::args().skip(2).next() {
        Some(ref character) if character.len() > 0 => character.as_bytes()[0] as char,
        Some(_) | None => usage(),
    };
    let size: f32 = match env::args().skip(3).next().and_then(|size| size.parse().ok()) {
        Some(size) => size,
        None => usage(),
    };

    let font = SystemSource::new().select_by_postscript_name(&postscript_name)
                                  .unwrap()
                                  .load()
                                  .unwrap();
    let glyph_id = font.glyph_for_char(character).unwrap();

    let raster_rect = font.raster_bounds(glyph_id,
                                         size,
                                         &Point2D::zero(),
                                         HintingOptions::None,
                                         RasterizationOptions::GrayscaleAa)
                          .unwrap();

    let stride = raster_rect.size.width as usize;
    let mut canvas = Canvas::new(&raster_rect.size.to_u32(), stride, Format::A8);

    font.rasterize_glyph(&mut canvas,
                         glyph_id,
                         size,
                         &Point2D::new(-raster_rect.origin.x, -raster_rect.origin.y).to_f32(),
                         HintingOptions::None,
                         RasterizationOptions::GrayscaleAa)
        .unwrap();

    println!("glyph {}:", glyph_id);
    for y in 0..raster_rect.size.height {
        let mut line = String::new();
        for x in 0..raster_rect.size.width {
            let character = match canvas.pixels[y as usize * stride + x as usize] {
                0 => ' ',
                1...84 => '░',
                85...169 => '▒',
                170...254 => '▓',
                _ => '█',
            };
            line.push(character);
            line.push(character);
        }
        println!("{}", line);
    }
}
