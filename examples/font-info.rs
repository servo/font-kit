// font-kit/examples/font-info.rs
//
// A simple example that loads a system font and prints detailed information
// about it: metadata, CSS properties, font metrics, and a glyph inspection.

use font_kit::family_name::FamilyName;
use font_kit::hinting::HintingOptions;
use font_kit::outline::OutlineSink;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;

// ---------------------------------------------------------------------------
// A simple OutlineSink that counts the path commands in a glyph outline.
// ---------------------------------------------------------------------------
struct OutlineCounter {
    moves: u32,
    lines: u32,
    quads: u32,
    cubics: u32,
}

impl OutlineCounter {
    fn new() -> Self {
        OutlineCounter { moves: 0, lines: 0, quads: 0, cubics: 0 }
    }

    fn total(&self) -> u32 {
        self.moves + self.lines + self.quads + self.cubics
    }
}

impl OutlineSink for OutlineCounter {
    fn move_to(&mut self, _to: Vector2F) {
        self.moves += 1;
    }
    fn line_to(&mut self, _to: Vector2F) {
        self.lines += 1;
    }
    fn quadratic_curve_to(&mut self, _ctrl: Vector2F, _to: Vector2F) {
        self.quads += 1;
    }
    fn cubic_curve_to(&mut self, _ctrl: LineSegment2F, _to: Vector2F) {
        self.cubics += 1;
    }
    fn close(&mut self) {}
}

// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = SystemSource::new();

    // -----------------------------------------------------------------------
    // 1. Select the best-matching sans-serif font (normal weight + style).
    // -----------------------------------------------------------------------
    let properties = Properties::new();
    let handle = source.select_best_match(&[FamilyName::SansSerif], &properties)?;
    let font = handle.load()?;

    println!("=== Font Metadata ===");
    println!("  Full name      : {}", font.full_name());
    println!("  Family name    : {}", font.family_name());
    println!(
        "  PostScript name: {}",
        font.postscript_name().unwrap_or_else(|| "(none)".into())
    );
    println!("  Monospace      : {}", font.is_monospace());
    println!("  Glyph count    : {}", font.glyph_count());

    // -----------------------------------------------------------------------
    // 2. CSS-style properties.
    // -----------------------------------------------------------------------
    let props = font.properties();
    println!("\n=== CSS Properties ===");
    println!("  Style          : {:?}", props.style);
    println!("  Weight         : {:.0}", props.weight.0);
    println!("  Stretch        : {:.3}", props.stretch.0);

    // -----------------------------------------------------------------------
    // 3. Font-level metrics (from the OS/2 / head tables).
    // -----------------------------------------------------------------------
    let metrics = font.metrics();
    println!("\n=== Font Metrics (in font units, 1 em = {} units) ===", metrics.units_per_em);
    println!("  Ascent             : {:.2}", metrics.ascent);
    println!("  Descent            : {:.2}", metrics.descent);
    println!("  Line gap           : {:.2}", metrics.line_gap);
    println!("  Cap height         : {:.2}", metrics.cap_height);
    println!("  x-height           : {:.2}", metrics.x_height);
    println!("  Underline position : {:.2}", metrics.underline_position);
    println!("  Underline thickness: {:.2}", metrics.underline_thickness);
    let bb = metrics.bounding_box;
    println!(
        "  Bounding box       : ({:.0}, {:.0}) – ({:.0}, {:.0})",
        bb.origin_x(),
        bb.origin_y(),
        bb.max_x(),
        bb.max_y(),
    );

    // -----------------------------------------------------------------------
    // 4. Inspect a few individual glyphs.
    // -----------------------------------------------------------------------
    let sample = ['A', 'g', '@', '€'];
    println!("\n=== Glyph Inspection ===");
    println!("  {:<6} {:<8} {:<14} {:<14} {:<30}", "Char", "Glyph ID", "Advance (fu)", "Origin (fu)", "Outline commands");
    println!("  {}", "-".repeat(78));

    for &ch in &sample {
        match font.glyph_for_char(ch) {
            None => println!("  '{}'  — no glyph in this font", ch),
            Some(glyph_id) => {
                let advance = font.advance(glyph_id).unwrap_or(Vector2F::zero());
                let origin  = font.origin(glyph_id).unwrap_or(Vector2F::zero());

                let mut sink = OutlineCounter::new();
                let _ = font.outline(glyph_id, HintingOptions::None, &mut sink);

                println!(
                    "  {:<6} {:<8} ({:>6.1},{:>5.1})   ({:>5.1},{:>5.1})   \
                     {} moves, {} lines, {} quads, {} cubics ({} total)",
                    format!("'{}'", ch),
                    glyph_id,
                    advance.x(), advance.y(),
                    origin.x(),  origin.y(),
                    sink.moves, sink.lines, sink.quads, sink.cubics, sink.total(),
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 5. Demonstrate CSS font-matching: request bold italic.
    // -----------------------------------------------------------------------
    println!("\n=== CSS Match: Bold Italic Sans-Serif ===");
    let mut bold_italic = Properties::new();
    bold_italic.style(Style::Italic).weight(Weight::BOLD);

    match source.select_best_match(&[FamilyName::SansSerif], &bold_italic) {
        Ok(h) => {
            let f = h.load()?;
            println!("  Matched          : {}", f.full_name());
            println!("  PostScript name  : {}", f.postscript_name().unwrap_or_else(|| "(none)".into()));
            println!("  Weight           : {:.0}", f.properties().weight.0);
            println!("  Style            : {:?}", f.properties().style);
        }
        Err(e) => println!("  No match found: {:?}", e),
    }

    // -----------------------------------------------------------------------
    // 6. Raster bounds for the letter 'H' at 24 pt.
    // -----------------------------------------------------------------------
    println!("\n=== Raster Bounds: 'H' at 24pt ===");
    if let Some(glyph_id) = font.glyph_for_char('H') {
        use font_kit::canvas::RasterizationOptions;
        let bounds = font.raster_bounds(
            glyph_id,
            24.0,
            Transform2F::default(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )?;
        println!(
            "  Origin : ({}, {})",
            bounds.origin().x(),
            bounds.origin().y(),
        );
        println!(
            "  Size   : {}w × {}h pixels",
            bounds.size().x(),
            bounds.size().y(),
        );
    }

    Ok(())
}
