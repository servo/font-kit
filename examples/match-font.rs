extern crate font_kit;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage:\n\tmatch-font \"Times New Roman, Arial, serif\"");
        std::process::exit(1);
    }

    let mut families = Vec::new();
    for family in args[1].split(',') {
        let family = family.replace('\'', "");
        let family = family.trim();
        families.push(match family {
            "serif" => font_kit::family_name::FamilyName::Serif,
            "sans-serif" => font_kit::family_name::FamilyName::SansSerif,
            "monospace" => font_kit::family_name::FamilyName::Monospace,
            "cursive" => font_kit::family_name::FamilyName::Cursive,
            "fantasy" => font_kit::family_name::FamilyName::Fantasy,
            _ => font_kit::family_name::FamilyName::Title(family.to_string()),
        });
    }

    let properties = font_kit::properties::Properties::default();
    let handle = font_kit::source::SystemSource::new().select_best_match(&families, &properties)?;

    if let font_kit::handle::Handle::Path {
        ref path,
        font_index,
    } = handle
    {
        println!("Path: {}", path.display());
        println!("Index: {}", font_index);
    }

    let font = handle.load()?;

    println!("Family name: {}", font.family_name());
    println!(
        "PostScript name: {}",
        font.postscript_name().unwrap_or("?".to_string())
    );
    println!("Style: {:?}", font.properties().style);
    println!("Weight: {:?}", font.properties().weight);
    println!("Stretch: {:?}", font.properties().stretch);

    Ok(())
}
