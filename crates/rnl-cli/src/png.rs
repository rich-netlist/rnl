//! PNG images of the drawings (`rnl render --png`): a level's SVG rasterized by resvg, in Rust, with
//! the fonts the command holds and none of the computer's, so that an image is drawn alike offline
//! and on a computer with no fonts.
//!
//! The fonts are the faces the renderer lays out text with, Noto Sans SC Regular and Bold (decision
//! 0009), subset to Latin, Greek and the signs of schematics by `scripts/fonts.py png` and held in
//! `fonts/` under the SIL Open Font License 1.1. They keep the outlines and advances of the full
//! faces and have no layout tables, so a text is as wide in an image as the renderer lays it out.
//! Every family a drawing names, and every generic one, is drawn in them; a character they do not
//! map, such as a CJK one, is drawn as an empty box.

use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};

/// The family of the faces, which the SVG names first.
const FAMILY: &str = "Noto Sans SC";
const REGULAR: &[u8] = include_bytes!("../fonts/noto-sans-sc-latin-400.otf");
const BOLD: &[u8] = include_bytes!("../fonts/noto-sans-sc-latin-700.otf");

/// The fonts an image is drawn with, loaded once: the two faces, and each generic family set to them.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(REGULAR.to_vec());
        db.load_font_data(BOLD.to_vec());
        db.set_serif_family(FAMILY);
        db.set_sans_serif_family(FAMILY);
        db.set_monospace_family(FAMILY);
        db.set_cursive_family(FAMILY);
        db.set_fantasy_family(FAMILY);
        Arc::new(db)
    })
    .clone()
}

/// The PNG image of an SVG, `scale` pixels for a unit of it, on white.
pub fn png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    let options = usvg::Options { font_family: FAMILY.to_string(), fontdb: fonts(), ..usvg::Options::default() };
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())?;
    let size = tree.size();
    let (w, h) = ((size.width() * scale).ceil(), (size.height() * scale).ceil());
    let too_big = || format!("an image of {w} by {h} pixels is too big; give a smaller --scale");
    // a side of at most 32768 pixels, and at most 256 million pixels in all
    if w > 32768.0 || h > 32768.0 || w * h > 2.56e8 {
        return Err(too_big());
    }
    let mut pixmap = tiny_skia::Pixmap::new((w as u32).max(1), (h as u32).max(1)).ok_or_else(too_big)?;
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pixels of an image that are not white.
    fn inked(image: &[u8]) -> usize {
        let pixmap = tiny_skia::Pixmap::decode_png(image).unwrap();
        pixmap.pixels().iter().filter(|p| (p.red(), p.green(), p.blue()) != (255, 255, 255)).count()
    }

    fn text(family: &str, text: &str) -> String {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"60\" height=\"20\" font-family=\"{family}\">\
             <text x=\"2\" y=\"15\" font-size=\"12\">{text}</text></svg>"
        )
    }

    #[test]
    fn text_is_drawn_with_the_fonts_held_whatever_family_is_named() {
        for family in ["Noto Sans SC, Noto Sans CJK SC, sans-serif", "Courier New", "monospace", "serif"] {
            let image = png(&text(family, "R1 10k"), 2.0).unwrap();
            assert!(inked(&image) > 50, "{family}");
        }
        // the size is the SVG's, times the scale
        let pixmap = tiny_skia::Pixmap::decode_png(&png(&text("serif", "x"), 3.0).unwrap()).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (180, 60));
        // a character the fonts do not map is drawn as the box of .notdef, as wide as the renderer
        // lays it out
        assert!(inked(&png(&text("serif", "电阻"), 2.0).unwrap()) > 50);
    }

    #[test]
    fn an_image_too_big_is_refused() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20000\" height=\"20000\"/>";
        assert!(png(svg, 2.0).unwrap_err().contains("too big"));
        assert!(png("<svg", 1.0).is_err());
    }
}
