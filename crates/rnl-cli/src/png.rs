//! PNG images of the drawings (`rnl render --png`): a level's SVG rasterized by resvg, in Rust, with
//! the fonts the command holds and those given with `--font`, and none of the computer's, so that an
//! image is drawn alike offline and on a computer with no fonts.
//!
//! The fonts held are the faces the renderer lays out text with, Noto Sans SC Regular and Bold
//! (decision 0009), subset by `scripts/fonts.py png` to Latin, Greek, the signs of schematics, CJK
//! punctuation and the 3755 common hanzi of GB 2312 level 1, and held in `fonts/` under the SIL Open
//! Font License 1.1. They keep the outlines and advances of the full faces and have no layout
//! tables, so a text is as wide in an image as the renderer lays it out. Every family a drawing
//! names, and every generic one, is drawn in them. A character they do not map is drawn in a font
//! given with `--font` that maps it, and otherwise as an empty box; [`Image::missing`] names those.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};

/// The family of the faces, which the SVG names first.
const FAMILY: &str = "Noto Sans SC";
const REGULAR: &[u8] = include_bytes!("../fonts/noto-sans-sc-common-400.otf");
const BOLD: &[u8] = include_bytes!("../fonts/noto-sans-sc-common-700.otf");

/// The fonts an image is drawn with: the two faces held, each generic family set to them, and the
/// fonts given after them.
#[derive(Clone)]
pub struct Fonts {
    db: Arc<usvg::fontdb::Database>,
}

fn held_db() -> usvg::fontdb::Database {
    let mut db = usvg::fontdb::Database::new();
    db.load_font_data(REGULAR.to_vec());
    db.load_font_data(BOLD.to_vec());
    db.set_serif_family(FAMILY);
    db.set_sans_serif_family(FAMILY);
    db.set_monospace_family(FAMILY);
    db.set_cursive_family(FAMILY);
    db.set_fantasy_family(FAMILY);
    db
}

impl Fonts {
    /// The faces the command holds, loaded once.
    pub fn held() -> Fonts {
        static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
        Fonts { db: DB.get_or_init(|| Arc::new(held_db())).clone() }
    }

    /// The faces held and, after them, every face of each file, for the characters the faces held
    /// do not map. A file that cannot be read, or that holds no font, is an error naming it.
    pub fn with_files(files: &[impl AsRef<Path>]) -> Result<Fonts, String> {
        if files.is_empty() {
            return Ok(Fonts::held());
        }
        let mut db = held_db();
        for file in files {
            let file = file.as_ref();
            let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
            let before = db.len();
            db.load_font_data(bytes);
            if db.len() == before {
                return Err(format!("{}: no font is read from it", file.display()));
            }
        }
        Ok(Fonts { db: Arc::new(db) })
    }

    /// Whether a face of these fonts maps `c`.
    fn map_all(&self, chars: &[char]) -> Vec<bool> {
        let mut mapped = vec![false; chars.len()];
        for face in self.db.faces() {
            self.db.with_face_data(face.id, |data, index| {
                let Ok(f) = ttf_parser::Face::parse(data, index) else { return };
                for (m, c) in mapped.iter_mut().zip(chars) {
                    *m = *m || f.glyph_index(*c).is_some();
                }
            });
        }
        mapped
    }
}

/// A PNG image, and the characters of its texts that no font of it maps, each once, in the order
/// they first come: those are drawn as empty boxes.
pub struct Image {
    pub png: Vec<u8>,
    pub missing: Vec<char>,
}

/// The PNG image of an SVG drawn with the fonts held, `scale` pixels for a unit of it, on white.
pub fn png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    draw(svg, scale, &Fonts::held()).map(|image| image.png)
}

/// The characters of the texts of a tree, each once, in order; white space and controls, which
/// draw nothing, left out.
fn characters(tree: &usvg::Tree) -> Vec<char> {
    fn walk(group: &usvg::Group, out: &mut Vec<char>, seen: &mut std::collections::HashSet<char>) {
        for node in group.children() {
            match node {
                usvg::Node::Group(g) => walk(g, out, seen),
                usvg::Node::Text(text) => {
                    for chunk in text.chunks() {
                        for c in chunk.text().chars() {
                            if !c.is_whitespace() && !c.is_control() && seen.insert(c) {
                                out.push(c);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(tree.root(), &mut out, &mut std::collections::HashSet::new());
    out
}

/// The PNG image of an SVG drawn with `fonts`, `scale` pixels for a unit of it, on white, and the
/// characters no font of them maps.
pub fn draw(svg: &str, scale: f32, fonts: &Fonts) -> Result<Image, String> {
    let options =
        usvg::Options { font_family: FAMILY.to_string(), fontdb: fonts.db.clone(), ..usvg::Options::default() };
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())?;
    let chars = characters(&tree);
    let missing = chars.iter().zip(fonts.map_all(&chars)).filter(|(_, m)| !m).map(|(c, _)| *c).collect();
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
    Ok(Image { png: pixmap.encode_png().map_err(|e| e.to_string())?, missing })
}

/// The characters no font maps, as a warning names them: the first eight, then how many more.
pub fn named(missing: &[char]) -> String {
    let shown: Vec<String> = missing.iter().take(8).map(|c| c.to_string()).collect();
    let mut out = shown.join(" ");
    if missing.len() > 8 {
        out.push_str(&format!(" and {} more", missing.len() - 8));
    }
    out
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

    /// A font of one face that maps 龘 (U+9F98), which is not in GB 2312, made from the pinned
    /// Noto Sans SC by `scripts/fonts.py png`.
    const EXTRA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fonts/noto-sans-sc-9f98.otf");

    #[test]
    fn text_is_drawn_with_the_fonts_held_whatever_family_is_named() {
        for family in ["Noto Sans SC, Noto Sans CJK SC, sans-serif", "Courier New", "monospace", "serif"] {
            let image = png(&text(family, "R1 10k"), 2.0).unwrap();
            assert!(inked(&image) > 50, "{family}");
        }
        // the size is the SVG's, times the scale
        let pixmap = tiny_skia::Pixmap::decode_png(&png(&text("serif", "x"), 3.0).unwrap()).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (180, 60));
    }

    #[test]
    fn the_common_hanzi_and_cjk_punctuation_are_held() {
        // a title of sample 15, and full-width punctuation
        let title = "五管 OTA：偏置由 R1、R2、R3 分压给出（电阻串）。";
        let image = draw(&text("Noto Sans SC", title), 2.0, &Fonts::held()).unwrap();
        assert_eq!(image.missing, Vec::<char>::new());
        assert!(inked(&image.png) > 50);
    }

    #[test]
    fn a_character_no_font_maps_is_named_and_a_font_given_draws_it() {
        // 龘 is not among the common hanzi; 한 is hangul
        let image = draw(&text("serif", "电阻 龘 한"), 2.0, &Fonts::held()).unwrap();
        assert_eq!(image.missing, vec!['龘', '한']);
        assert_eq!(named(&image.missing), "龘 한");
        let fonts = Fonts::with_files(&[EXTRA]).unwrap();
        let image = draw(&text("serif", "电阻 龘 한"), 2.0, &fonts).unwrap();
        assert_eq!(image.missing, vec!['한']);
        // the glyph of the font given is drawn, not the empty box: the images differ
        let boxed = draw(&text("serif", "龘"), 2.0, &Fonts::held()).unwrap().png;
        let drawn = draw(&text("serif", "龘"), 2.0, &fonts).unwrap().png;
        assert_ne!(boxed, drawn);
    }

    #[test]
    fn a_warning_names_eight_characters_and_how_many_more() {
        let missing: Vec<char> = "가나다라마바사아자차".chars().collect();
        assert_eq!(named(&missing), "가 나 다 라 마 바 사 아 and 2 more");
    }

    #[test]
    fn a_font_file_that_cannot_be_read_or_holds_no_font_is_an_error() {
        let missing = Fonts::with_files(&["no/such/font.otf"]).err().unwrap();
        assert!(missing.starts_with("no/such/font.otf: "), "{missing}");
        let not_font = Fonts::with_files(&[concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")]).err().unwrap();
        assert!(not_font.ends_with("Cargo.toml: no font is read from it"), "{not_font}");
    }

    #[test]
    fn an_image_too_big_is_refused() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20000\" height=\"20000\"/>";
        assert!(png(svg, 2.0).unwrap_err().contains("too big"));
        assert!(png("<svg", 1.0).is_err());
    }
}
