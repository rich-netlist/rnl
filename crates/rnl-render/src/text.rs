//! Text laid out with the metrics of the fonts shipped with the core (decision 0007): Noto Sans SC,
//! Regular and Bold. `fonts/noto-sans-sc.metrics`, which `scripts/fonts.py metrics` writes from the
//! fonts, holds what the layout needs of them: the units per em, the ascender, descender and line
//! gap, the top of the em box, and the advance of every character they map.
//!
//! A run is as wide as the sum of its characters' advances, with no kerning, ligatures or other
//! shaping, so a frontend that draws each run from its start in the same face draws the text as it
//! is laid out. The font the style names (9.3) is a hint that the layout does not follow.

use rnl_runtime::geom::Pt;
use rnl_runtime::markup::Run;

use crate::{Stroke, TextBar, TextPrim, TextRun};

/// The metrics file, laid out as `scripts/fonts.py` says: a header, the metrics of each face, and
/// the runs of code points with their advances in each face.
const METRICS: &[u8] = include_bytes!("../fonts/noto-sans-sc.metrics");
/// Where the faces' metrics start, and how many bytes each face's take.
const FACES_AT: usize = 8;
const FACE_LEN: usize = 12;

/// The family of the shipped fonts.
pub const FAMILY: &str = "Noto Sans SC";

/// Each level of sub- or superscript (13.6) is this much of the size of the level next to it
/// towards 0, with its baseline `RAISE` times that size above it or `DROP` times it below.
const SCRIPT: f64 = 0.7;
const RAISE: f64 = 0.35;
const DROP: f64 = 0.15;
/// A fraction (13.6): the numerator and the denominator are `FRAC_SIZE` of its size, with their
/// baselines `FRAC_NUM` of it above the fraction's and `FRAC_DEN` below it, centred on the
/// fraction, which is as wide as the wider of them and `FRAC_SPACE` of its size. The bar is
/// `FRAC_BAR` of the size above the baseline and `BAR_INSET` short of each end.
const FRAC_SIZE: f64 = 0.8;
const FRAC_NUM: f64 = 0.72;
const FRAC_DEN: f64 = 0.42;
const FRAC_SPACE: f64 = 0.15;
const FRAC_BAR: f64 = 0.3;
const BAR_INSET: f64 = 0.06;
/// A fraction's bar and an overline are this much of their size wide, but at least `LEAST_BAR`
/// pixels.
const BAR: f64 = 0.06;
const LEAST_BAR: f64 = 0.8;
/// An italic run, which has no face of its own, leans: a point of a glyph moves forward along the
/// baseline by this much of its height above it (about 14°, as browsers slant a face).
pub const SLANT: f64 = 0.25;

fn u16_at(k: usize) -> u16 {
    u16::from_le_bytes([METRICS[k], METRICS[k + 1]])
}

fn u32_at(k: usize) -> u32 {
    u32::from_le_bytes([METRICS[k], METRICS[k + 1], METRICS[k + 2], METRICS[k + 3]])
}

fn faces() -> usize {
    usize::from(u16_at(6))
}

/// Where the runs start, and how many bytes each takes.
fn runs_at() -> usize {
    FACES_AT + FACE_LEN * faces() + 4
}

fn run_len() -> usize {
    6 + 2 * faces()
}

/// A face of the shipped family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face(usize);

impl Face {
    pub const REGULAR: Face = Face(0);
    pub const BOLD: Face = Face(1);

    pub fn of(bold: bool) -> Face {
        if bold {
            Face::BOLD
        } else {
            Face::REGULAR
        }
    }

    /// The face's `k`th metric: the units per em, the ascender, the descender, the line gap, the
    /// top of the em box and the advance of .notdef.
    fn metric(self, k: usize) -> i64 {
        let at = FACES_AT + FACE_LEN * self.0 + 2 * k;
        let x = u16_at(at);
        if k == 0 || k == 5 {
            i64::from(x)
        } else {
            i64::from(x as i16)
        }
    }

    fn em(self, units: i64) -> f64 {
        units as f64 / self.metric(0) as f64
    }

    /// How far the face's lines reach above the baseline, in em (hhea).
    pub fn ascender(self) -> f64 {
        self.em(self.metric(1))
    }

    /// How far they reach below it, in em, as a negative number.
    pub fn descender(self) -> f64 {
        self.em(self.metric(2))
    }

    /// How far apart the baselines of lines are, in em: the ascender less the descender, and the
    /// line gap.
    pub fn line_height(self) -> f64 {
        self.em(self.metric(1) - self.metric(2) + self.metric(3))
    }

    /// The top of the em box (OS/2 sTypoAscender), in em, where an overline goes.
    pub fn em_top(self) -> f64 {
        self.em(self.metric(4))
    }

    /// The advance of a character, in units. A character the fonts do not map has that of their
    /// .notdef glyph, the box they draw for it, 1 em.
    fn units(self, c: char) -> i64 {
        let (at, len) = (runs_at(), run_len());
        let first = |k: usize| u32_at(at + len * k);
        let cp = u32::from(c);
        // the runs that start at or before the character
        let (mut lo, mut hi) = (0, u32_at(at - 4) as usize);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if first(mid) <= cp {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo > 0 {
            let k = lo - 1;
            if cp - first(k) < u32::from(u16_at(at + len * k + 4)) {
                return i64::from(u16_at(at + len * k + 6 + 2 * self.0));
            }
        }
        self.metric(5)
    }

    /// The advance of a character, in em.
    pub fn advance(self, c: char) -> f64 {
        self.em(self.units(c))
    }

    /// The width of a string in em: the sum of its characters' advances.
    pub fn width(self, s: &str) -> f64 {
        self.em(s.chars().map(|c| self.units(c)).sum())
    }
}

/// The size and the baseline's shift of a run `level` levels away from where its size is `s`.
fn level(level: i32, s: f64) -> (f64, f64) {
    let (mut size, mut shift) = (s, 0.0);
    for _ in 0..level.unsigned_abs() {
        shift += if level > 0 { RAISE * size } else { -DROP * size };
        size *= SCRIPT;
    }
    (size, shift)
}

/// A run in the text's own frame: `u` along the lines from the anchor, `v` across them, up.
struct Piece {
    text: String,
    u: f64,
    v: f64,
    size: f64,
    width: f64,
    bold: bool,
    italic: bool,
}

/// A bar in the text's frame, from `u0` to `u1` at `v`, `width` wide.
struct Bar {
    u0: f64,
    u1: f64,
    v: f64,
    width: f64,
}

struct Lay {
    bold: bool,
    italic: bool,
    /// The least width of a bar, in scene units.
    least: f64,
    pieces: Vec<Piece>,
    bars: Vec<Bar>,
}

impl Lay {
    fn face(&self, bold: bool) -> Face {
        Face::of(self.bold || bold)
    }

    fn bar(&self, size: f64) -> f64 {
        (BAR * size).max(self.least)
    }

    /// The width of runs whose levels count from `base`, where the size is `s`.
    fn width(&self, runs: &[Run], base: i32, s: f64) -> f64 {
        runs.iter()
            .map(|r| match r {
                Run::Text { text, style } => {
                    let (size, _) = level(i32::from(style.level) - base, s);
                    self.face(style.bold).width(text) * size
                }
                Run::Frac { num, den, style } => {
                    let (size, _) = level(i32::from(style.level) - base, s);
                    let small = size * FRAC_SIZE;
                    let base = i32::from(style.level);
                    self.width(num, base, small).max(self.width(den, base, small)) + FRAC_SPACE * size
                }
            })
            .sum()
    }

    /// Place runs whose levels count from `base`, where the size is `s`, from `u` on the baseline
    /// `v`; gives where the next run starts.
    fn place(&mut self, runs: &[Run], base: i32, mut u: f64, v: f64, s: f64) -> f64 {
        for r in runs {
            match r {
                Run::Text { text, style } => {
                    let (size, shift) = level(i32::from(style.level) - base, s);
                    let face = self.face(style.bold);
                    let width = face.width(text) * size;
                    if style.overline {
                        let at = v + shift + face.em_top() * size;
                        self.bars.push(Bar { u0: u, u1: u + width, v: at, width: self.bar(size) });
                    }
                    let (bold, italic) = (self.bold || style.bold, self.italic || style.italic);
                    self.pieces.push(Piece { text: text.clone(), u, v: v + shift, size, width, bold, italic });
                    u += width;
                }
                Run::Frac { num, den, style } => {
                    let (size, shift) = level(i32::from(style.level) - base, s);
                    let (v, small, base) = (v + shift, size * FRAC_SIZE, i32::from(style.level));
                    let (wn, wd) = (self.width(num, base, small), self.width(den, base, small));
                    let w = wn.max(wd) + FRAC_SPACE * size;
                    self.place(num, base, u + (w - wn) / 2.0, v + FRAC_NUM * size, small);
                    let bar = Bar {
                        u0: u + BAR_INSET * size,
                        u1: u + w - BAR_INSET * size,
                        v: v + FRAC_BAR * size,
                        width: self.bar(size),
                    };
                    self.bars.push(bar);
                    self.place(den, base, u + (w - wd) / 2.0, v - FRAC_DEN * size, small);
                    u += w;
                }
            }
        }
        u
    }
}

/// Lay out a text (9.2.5, 13.6): its runs and bars, from its lines, size, faces, alignment and
/// direction, with the bars at least `LEAST_BAR` pixels wide at `scale` pixels per scene unit.
/// Gives the box they cover, each run from the ascender to the descender of its face.
///
/// The lines are the face's line height apart. The anchor is on the first line's baseline, or at
/// the first line's ascender ("top"), at the last line's descender ("bottom"), or halfway between
/// them ("middle", and any other alignment); along the lines, at their start, middle or end.
pub fn lay_out(t: &mut TextPrim, scale: f64) -> [f64; 4] {
    let face = Face::of(t.bold);
    let s = t.size;
    let mut lay =
        Lay { bold: t.bold, italic: t.italic, least: LEAST_BAR / scale, pieces: Vec::new(), bars: Vec::new() };
    let n = t.lines.len() as f64;
    let pitch = face.line_height() * s;
    let first = match t.valign.as_str() {
        "baseline" => 0.0,
        "top" => -face.ascender() * s,
        "bottom" => (n - 1.0) * pitch - face.descender() * s,
        _ => ((n - 1.0) * pitch - (face.ascender() + face.descender()) * s) / 2.0,
    };
    for (k, line) in t.lines.iter().enumerate() {
        let (pieces, bars) = (lay.pieces.len(), lay.bars.len());
        let w = lay.place(line, 0, 0.0, first - k as f64 * pitch, s);
        let du = match t.halign.as_str() {
            "middle" => -w / 2.0,
            "end" => -w,
            _ => 0.0,
        };
        for p in &mut lay.pieces[pieces..] {
            p.u += du;
        }
        for b in &mut lay.bars[bars..] {
            b.u0 += du;
            b.u1 += du;
        }
    }
    // the text's frame in the scene: a vertical text reads up, its glyphs' tops to the left
    let (a, vertical) = (t.anchor, t.vertical);
    let at = |u: f64, v: f64| -> Pt {
        if vertical {
            (a.0 - v, a.1 + u)
        } else {
            (a.0 + u, a.1 + v)
        }
    };
    let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let mut cover = |u0: f64, v0: f64, u1: f64, v1: f64| {
        for p in [at(u0, v0), at(u1, v1)] {
            bbox = [bbox[0].min(p.0), bbox[1].min(p.1), bbox[2].max(p.0), bbox[3].max(p.1)];
        }
    };
    for p in &lay.pieces {
        let f = Face::of(p.bold);
        let (lo, hi) = (f.descender() * p.size, f.ascender() * p.size);
        let lean = if p.italic { SLANT } else { 0.0 };
        cover(p.u + lean * lo, p.v + lo, p.u + p.width + lean * hi, p.v + hi);
    }
    for b in &lay.bars {
        cover(b.u0, b.v - b.width / 2.0, b.u1, b.v + b.width / 2.0);
    }
    if lay.pieces.is_empty() && lay.bars.is_empty() {
        bbox = [a.0, a.1, a.0, a.1];
    }
    t.runs = lay
        .pieces
        .into_iter()
        .map(|p| TextRun {
            at: at(p.u, p.v),
            text: p.text,
            size: p.size,
            width: p.width,
            bold: p.bold,
            italic: p.italic,
        })
        .collect();
    t.bars = lay
        .bars
        .iter()
        .map(|b| TextBar {
            a: at(b.u0, b.v),
            b: at(b.u1, b.v),
            stroke: Stroke {
                color: t.color.clone(),
                width: b.width,
                pixels: false,
                dashes: Vec::new(),
                dash_offset: 0.0,
                cap: "butt".into(),
                join: "miter".into(),
            },
        })
        .collect();
    bbox
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::markup;

    #[track_caller]
    fn close(got: &[f64], want: &[f64]) {
        assert!(
            got.len() == want.len() && got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12),
            "{got:?} {want:?}"
        );
    }

    #[test]
    fn the_metrics_are_those_of_noto_sans_sc() {
        assert_eq!(&METRICS[..4], b"RNLM");
        assert_eq!((u16_at(4), faces()), (1, 2));
        assert_eq!(METRICS.len(), runs_at() + run_len() * u32_at(runs_at() - 4) as usize);
        for face in [Face::REGULAR, Face::BOLD] {
            assert_eq!((face.metric(0), face.ascender(), face.descender()), (1000, 1.16, -0.288));
            assert_eq!((face.line_height(), face.em_top()), (1.448, 0.88));
            // an ideograph, the last character the fonts map, and characters they do not map
            let wide = ['中', '\u{3106C}', '\u{1F}', '\t', '\u{10FFFF}'];
            assert_eq!(wide.map(|c| face.advance(c)), [1.0; 5]);
            // a combining mark
            assert_eq!(face.advance('\u{301}'), 0.0);
        }
        let (r, b) = (Face::REGULAR, Face::BOLD);
        // the first character the fonts map, letters and the minus sign
        assert_eq!([' ', 'a', 'M', '−'].map(|c| r.advance(c)), [0.224, 0.563, 0.812, 0.555]);
        assert_eq!([' ', 'a', 'M', '−'].map(|c| b.advance(c)), [0.227, 0.591, 0.853, 0.59]);
        assert_eq!((r.width("VDD"), b.width("VDD"), r.width("")), (1.951, 2.047, 0.0));
    }

    fn text(lines: &str, halign: &str, valign: &str, vertical: bool) -> (TextPrim, [f64; 4]) {
        let mut t = TextPrim {
            anchor: (10.0, 20.0),
            lines: markup::parse(lines),
            size: 2.0,
            color: "#102030".into(),
            font: "serif".into(),
            bold: false,
            italic: false,
            halign: halign.into(),
            valign: valign.into(),
            vertical,
            runs: Vec::new(),
            bars: Vec::new(),
        };
        let bbox = lay_out(&mut t, 14.0);
        (t, bbox)
    }

    #[test]
    fn runs_follow_each_other_and_lines_are_a_line_height_apart() {
        let r = Face::REGULAR;
        let (t, bbox) = text("ab\\\\c", "start", "baseline", false);
        assert_eq!(t.runs.len(), 2);
        assert_eq!((t.runs[0].text.as_str(), t.runs[0].at, t.runs[0].size), ("ab", (10.0, 20.0), 2.0));
        close(&[t.runs[0].width, t.runs[1].at.0, t.runs[1].at.1], &[r.width("ab") * 2.0, 10.0, 20.0 - 2.896]);
        close(&bbox, &[10.0, 20.0 - 2.896 - 0.576, 10.0 + r.width("ab") * 2.0, 20.0 + 2.32]);
        // sub- and superscripts at 0.7 of the size of the level next to them, raised 0.35 of it or
        // lowered 0.15
        let (t, _) = text("V_{DD}x^{2^{n}}", "start", "baseline", false);
        let sizes: Vec<f64> = t.runs.iter().flat_map(|r| [r.size, r.at.1 - 20.0]).collect();
        close(&sizes, &[2.0, 0.0, 1.4, -0.3, 2.0, 0.0, 1.4, 0.7, 0.98, 0.7 + 0.49]);
        let v = r.width("V") * 2.0;
        close(&[t.runs[1].at.0, t.runs[2].at.0], &[10.0 + v, 10.0 + v + r.width("DD") * 1.4]);
    }

    #[test]
    fn the_anchor_is_where_the_alignments_say() {
        let w = Face::REGULAR.width("ab") * 2.0;
        for (halign, x) in [("start", 10.0), ("middle", 10.0 - w / 2.0), ("end", 10.0 - w)] {
            let (t, _) = text("ab", halign, "baseline", false);
            close(&[t.runs[0].at.0], &[x]);
        }
        // two lines, from the first one's ascender to the last one's descender
        let h = 2.896;
        for (valign, first) in [
            ("baseline", 20.0),
            ("top", 20.0 - 2.32),
            ("bottom", 20.0 + 0.576 + h),
            ("middle", 20.0 + (h - 2.32 + 0.576) / 2.0),
        ] {
            let (t, bbox) = text("a\\\\b", "start", valign, false);
            close(&[t.runs[0].at.1, t.runs[1].at.1], &[first, first - h]);
            close(&[bbox[1], bbox[3]], &[first - h - 0.576, first + 2.32]);
        }
        // a vertical text reads up from the anchor, the tops of its glyphs to the left
        let (t, bbox) = text("ab", "start", "baseline", true);
        assert_eq!(t.runs[0].at, (10.0, 20.0));
        close(&bbox, &[10.0 - 2.32, 20.0, 10.0 + 0.576, 20.0 + w]);
    }

    #[test]
    fn fractions_bold_italic_and_overlines() {
        let r = Face::REGULAR;
        let (t, _) = text("x\\frac{ab}{c}", "start", "baseline", false);
        let x = 10.0 + r.width("x") * 2.0;
        let (wn, wd) = (r.width("ab") * 1.6, r.width("c") * 1.6);
        let w = wn + 0.3;
        let at: Vec<f64> = t.runs[1..].iter().flat_map(|r| [r.at.0, r.at.1, r.size]).collect();
        close(&at, &[x + (w - wn) / 2.0, 21.44, 1.6, x + (w - wd) / 2.0, 19.16, 1.6]);
        let bar = &t.bars[0];
        close(&[bar.a.0, bar.a.1, bar.b.0, bar.b.1, bar.stroke.width], &[x + 0.12, 20.6, x + w - 0.12, 20.6, 0.12]);
        let plain =
            (bar.stroke.pixels, bar.stroke.dashes.is_empty(), bar.stroke.cap.as_str(), bar.stroke.join.as_str());
        assert_eq!((bar.stroke.color.as_str(), plain), ("#102030", (false, true, "butt", "miter")));
        // a bar at least 0.8 pixels wide
        let mut small = TextPrim { size: 0.5, ..t.clone() };
        lay_out(&mut small, 14.0);
        assert_eq!(small.bars[0].stroke.width, 0.8 / 14.0);
        // bold in the bold face, italic leaning with the upright advances, an overline at the top
        // of the em box
        let (t, bbox) = text("\\textbf{ab}\\textit{\\overline{ab}}", "start", "baseline", false);
        let (b, i) = (Face::BOLD.width("ab") * 2.0, r.width("ab") * 2.0);
        let flags: Vec<(bool, bool)> = t.runs.iter().map(|r| (r.bold, r.italic)).collect();
        assert_eq!(flags, [(true, false), (false, true)]);
        close(&[t.runs[0].width, t.runs[1].width], &[b, i]);
        close(&[t.bars[0].a.0, t.bars[0].a.1, t.bars[0].b.0, t.bars[0].b.1], &[10.0 + b, 21.76, 10.0 + b + i, 21.76]);
        close(&[bbox[0], bbox[2]], &[10.0, 10.0 + b + i + 0.25 * 2.32]);
    }
}
