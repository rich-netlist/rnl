//! Estimated text widths. Fonts belong to the viewer (architecture 7), so these are rough widths
//! of a sans-serif face in em, enough for bounding boxes and laying out fractions.

use rnl_runtime::markup::{Line, Run};

/// Numerator and denominator are this much of the text size (13.6 leaves it to the viewer).
pub const FRAC_SIZE: f64 = 0.8;

fn char_w(c: char) -> f64 {
    if c as u32 > 0x2E80 {
        return 1.0;
    }
    if "ijlI|!.,:;'".contains(c) {
        return 0.25;
    }
    if "ftr()[]/\\ ".contains(c) {
        return 0.3;
    }
    if "mwMW".contains(c) {
        return 0.85;
    }
    if "ABCDGHKNOQRUVXY&%".contains(c) {
        return 0.7;
    }
    if c.is_uppercase() {
        return 0.62;
    }
    if c.is_ascii_digit() || "=+−-<>~^_±×".contains(c) {
        return 0.56;
    }
    0.52
}

/// Width of a string in em.
pub fn text_width(s: &str) -> f64 {
    s.chars().map(char_w).sum()
}

/// Width of a run in em; a fraction takes its wider part.
pub fn run_width(r: &Run) -> f64 {
    match r {
        Run::Text { text, style } => text_width(text) * if style.level != 0 { 0.7 } else { 1.0 },
        Run::Frac { num, den, .. } => {
            let n: f64 = num.iter().map(run_width).sum();
            let d: f64 = den.iter().map(run_width).sum();
            n.max(d) * FRAC_SIZE + 0.15
        }
    }
}

/// Width of a line in em.
pub fn line_width(l: &Line) -> f64 {
    l.iter().map(run_width).sum()
}
