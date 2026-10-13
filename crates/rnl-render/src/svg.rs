//! The SVG backend: primitives in scene units (y up) to an SVG document (y down). It writes the
//! strokes as the renderer resolved them and decides nothing about how a line looks.

use std::fmt::Write;

use rnl_runtime::geom::Pt;
use rnl_runtime::markup::Run;

use crate::text::{run_width, FRAC_SIZE};
use crate::{Kind, PathSeg, Primitive, RenderOptions, Stroke, TextPrim};

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

struct W {
    s: f64,
}

impl W {
    fn px(&self, p: Pt) -> (f64, f64) {
        (p.0 * self.s, -p.1 * self.s)
    }

    fn d(&self, segs: &[PathSeg]) -> String {
        let mut d = String::new();
        for seg in segs {
            match seg {
                PathSeg::Move(p) => {
                    let (x, y) = self.px(*p);
                    let _ = write!(d, "{}M{x:.2},{y:.2}", if d.is_empty() { "" } else { " " });
                }
                PathSeg::Line(p) => {
                    let (x, y) = self.px(*p);
                    let _ = write!(d, " L{x:.2},{y:.2}");
                }
                PathSeg::Arc { r, large, ccw, to } => {
                    let (x, y) = self.px(*to);
                    // SVG's y points down: counter-clockwise in y-up is sweep-flag 0
                    let _ = write!(
                        d,
                        " A{:.2},{:.2} 0 {} {} {x:.2},{y:.2}",
                        r * self.s,
                        r * self.s,
                        u8::from(*large),
                        u8::from(!*ccw)
                    );
                }
                PathSeg::Cubic(a, b, c) => {
                    let (a, b, c) = (self.px(*a), self.px(*b), self.px(*c));
                    let _ = write!(d, " C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}", a.0, a.1, b.0, b.1, c.0, c.1);
                }
                PathSeg::Close => d.push_str(" Z"),
            }
        }
        d
    }

    /// A length of a stroke in pixels: as it is when the stroke is in pixels, else times the scale.
    fn len(&self, st: &Stroke, x: f64) -> f64 {
        if st.pixels {
            x
        } else {
            x * self.s
        }
    }

    /// The attributes that draw a stroke. The width is written to 2 decimals, and the dashes and
    /// their offset to as many as they need, at least 1 and at most 6, so that a long dashed line
    /// keeps in step with the primitives' dashes (rounded to a tenth of a pixel, the error of each
    /// period adds up along the line). When `short`, each number has no more decimals than it
    /// needs. What SVG draws when it is not said, butt caps, miter joins and no dash offset, is
    /// left out.
    fn stroke_attrs(&self, st: &Stroke, short: bool) -> String {
        // x to `decimals` decimals, less the zeros at the end after the first `keep` decimals, or
        // all of them when `short`
        let num = |x: f64, decimals: usize, keep: usize| {
            let s = format!("{x:.decimals$}");
            let Some(point) = s.find('.') else { return s };
            let digits = s[point + 1..].trim_end_matches('0').len().max(if short { 0 } else { keep });
            s[..if digits == 0 { point } else { point + 1 + digits }].to_string()
        };
        let dash = |x: f64| num(self.len(st, x), 6, 1);
        let mut a = format!("stroke=\"{}\" stroke-width=\"{}\"", esc(&st.color), num(self.len(st, st.width), 2, 2));
        if st.cap != "butt" {
            let _ = write!(a, " stroke-linecap=\"{}\"", esc(&st.cap));
        }
        if st.join != "miter" {
            let _ = write!(a, " stroke-linejoin=\"{}\"", esc(&st.join));
        }
        if !st.dashes.is_empty() {
            let d: Vec<String> = st.dashes.iter().map(|d| dash(*d)).collect();
            let _ = write!(a, " stroke-dasharray=\"{}\"", d.join(","));
            if st.dash_offset != 0.0 {
                let _ = write!(a, " stroke-dashoffset=\"{}\"", dash(st.dash_offset));
            }
        }
        a
    }

    /// The element once for each stroke, first to last, the first with the fill and the others
    /// without; `head` is the element up to its fill. With no stroke, the fill alone.
    fn stroked(&self, head: &str, fill: &Option<String>, strokes: &[Stroke]) -> String {
        if strokes.is_empty() {
            return format!("{head} fill=\"{}\" stroke=\"none\"/>", Self::fill(fill));
        }
        let mut out = String::new();
        for (k, st) in strokes.iter().enumerate() {
            let fill = if k == 0 { Self::fill(fill) } else { "none".into() };
            let _ = write!(out, "{head} fill=\"{fill}\" {}/>", self.stroke_attrs(st, false));
        }
        out
    }

    fn fill(f: &Option<String>) -> String {
        match f {
            Some(c) => esc(c),
            None => "none".into(),
        }
    }

    fn prim(&self, p: &Primitive) -> String {
        match &p.kind {
            Kind::Path { segs, strokes, fill } => self.stroked(&format!("<path d=\"{}\"", self.d(segs)), fill, strokes),
            Kind::Rect { min, max, strokes, fill } => {
                let (x, y) = self.px((min.0, max.1));
                let head = format!(
                    "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{:.2}\" height=\"{:.2}\"",
                    (max.0 - min.0) * self.s,
                    (max.1 - min.1) * self.s
                );
                self.stroked(&head, fill, strokes)
            }
            Kind::Circle { c, r, strokes, fill } => {
                let (x, y) = self.px(*c);
                self.stroked(&format!("<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{:.2}\"", r * self.s), fill, strokes)
            }
            Kind::Ellipse { c, rx, ry, strokes, fill } => {
                let (x, y) = self.px(*c);
                let head =
                    format!("<ellipse cx=\"{x:.2}\" cy=\"{y:.2}\" rx=\"{:.2}\" ry=\"{:.2}\"", rx * self.s, ry * self.s);
                self.stroked(&head, fill, strokes)
            }
            Kind::Flyline { a, b, strokes } => {
                // a line, with no fill and its numbers written short
                let (a, b) = (self.px(*a), self.px(*b));
                let head = format!("<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\"", a.0, a.1, b.0, b.1);
                strokes.iter().map(|st| format!("{head} {}/>", self.stroke_attrs(st, true))).collect()
            }
            Kind::Text(t) => self.text(t),
        }
    }

    fn tspan(r: &Run, fs: f64) -> String {
        let Run::Text { text, style } = r else { return String::new() };
        let mut attrs = Vec::new();
        if style.level != 0 {
            attrs.push(format!("font-size=\"{:.2}\"", fs * 0.7));
            attrs.push(format!("baseline-shift=\"{}\"", if style.level > 0 { "super" } else { "sub" }));
        }
        if style.bold {
            attrs.push("font-weight=\"bold\"".into());
        }
        if style.italic {
            attrs.push("font-style=\"italic\"".into());
        }
        if style.overline {
            attrs.push("text-decoration=\"overline\"".into());
        }
        if attrs.is_empty() {
            esc(text)
        } else {
            format!("<tspan {}>{}</tspan>", attrs.join(" "), esc(text))
        }
    }

    fn text(&self, t: &TextPrim) -> String {
        let (x, y) = self.px(t.anchor);
        let fs = t.size * self.s;
        let n = t.lines.len() as f64;
        let lh = fs * 1.15;
        let first = match t.valign.as_str() {
            "baseline" => 0.0,
            "top" => fs * 0.8,
            "bottom" => -(n - 1.0) * lh - fs * 0.2,
            _ => fs * 0.35 - (n - 1.0) * lh / 2.0,
        };
        let weight = if t.bold { " font-weight=\"bold\"" } else { "" };
        let style = if t.italic { " font-style=\"italic\"" } else { "" };
        let family = format!("{}, sans-serif", esc(&t.font)); // the font is a hint (9.3)
        let rot = if t.vertical { " rotate(-90)" } else { "" };
        if t.lines.iter().flatten().any(|r| matches!(r, Run::Frac { .. })) {
            return self.text_with_fractions(t, x, y, fs, first, lh, &family, &format!("{weight}{style}"), rot);
        }
        let mut body = String::new();
        for (k, runs) in t.lines.iter().enumerate() {
            let spans: String = runs.iter().map(|r| Self::tspan(r, fs)).collect();
            let _ = write!(body, "<tspan x=\"0\" dy=\"{:.2}\">{spans}</tspan>", if k == 0 { first } else { lh });
        }
        format!(
            "<text transform=\"translate({x:.2},{y:.2}){rot}\" font-size=\"{fs:.2}\" font-family=\"{family}\" fill=\"{}\" text-anchor=\"{}\"{weight}{style}>{body}</text>",
            esc(&t.color),
            esc(&t.halign)
        )
    }

    /// A line holding `\frac` (13.6). SVG has no text layout, so the pieces are placed by estimated
    /// widths and each piece is held to its width with textLength; numerator and denominator are
    /// centred over a bar as wide as the wider of them.
    #[allow(clippy::too_many_arguments)]
    fn text_with_fractions(
        &self,
        t: &TextPrim,
        x: f64,
        y: f64,
        fs: f64,
        first: f64,
        lh: f64,
        family: &str,
        font_attrs: &str,
        rot: &str,
    ) -> String {
        let mut out = format!(
            "<g transform=\"translate({x:.2},{y:.2}){rot}\" fill=\"{}\" font-family=\"{family}\"{font_attrs}>",
            esc(&t.color)
        );
        let small = fs * FRAC_SIZE;
        let held = |runs: &[Run], x: f64, y: f64, size: f64, anchor: &str| -> String {
            let w: f64 = runs.iter().map(run_width).sum::<f64>() * size;
            if w <= 0.0 {
                return String::new();
            }
            let spans: String = runs.iter().map(|r| Self::tspan(r, size)).collect();
            format!(
                "<text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{size:.2}\" text-anchor=\"{anchor}\" textLength=\"{w:.2}\" lengthAdjust=\"spacingAndGlyphs\">{spans}</text>"
            )
        };
        for (k, runs) in t.lines.iter().enumerate() {
            let yb = first + k as f64 * lh;
            let total: f64 = runs.iter().map(run_width).sum::<f64>() * fs;
            let mut cx = match t.halign.as_str() {
                "middle" => -total / 2.0,
                "end" => -total,
                _ => 0.0,
            };
            for r in runs {
                let w = run_width(r) * fs;
                match r {
                    Run::Frac { num, den, .. } => {
                        let mid = cx + w / 2.0;
                        out.push_str(&held(num, mid, yb - fs * 0.72, small, "middle"));
                        let _ = write!(
                            out,
                            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" stroke-width=\"{:.2}\"/>",
                            cx + fs * 0.06,
                            yb - fs * 0.3,
                            cx + w - fs * 0.06,
                            yb - fs * 0.3,
                            esc(&t.color),
                            (fs * 0.06).max(0.8)
                        );
                        out.push_str(&held(den, mid, yb + fs * 0.42, small, "middle"));
                    }
                    _ => out.push_str(&held(std::slice::from_ref(r), cx, yb, fs, "start")),
                }
                cx += w;
            }
        }
        out.push_str("</g>");
        out
    }
}

/// Write primitives as an SVG document framing everything drawn, with a margin.
pub fn svg_document(prims: &[Primitive], o: &RenderOptions) -> String {
    let w = W { s: o.scale };
    if prims.is_empty() {
        return "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"60\"><text x=\"10\" y=\"30\" font-size=\"12\" fill=\"#888\">(nothing is drawn)</text></svg>".into();
    }
    let pad = 2.5;
    let x0 = prims.iter().map(|p| p.bbox[0]).fold(f64::MAX, f64::min) - pad;
    let y0 = prims.iter().map(|p| p.bbox[1]).fold(f64::MAX, f64::min) - pad;
    let x1 = prims.iter().map(|p| p.bbox[2]).fold(f64::MIN, f64::max) + pad;
    let y1 = prims.iter().map(|p| p.bbox[3]).fold(f64::MIN, f64::max) + pad;
    let (vw, vh) = ((x1 - x0) * o.scale, (y1 - y0) * o.scale);
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{vw:.0}\" height=\"{vh:.0}\" viewBox=\"{:.2} {:.2} {vw:.2} {vh:.2}\">",
        x0 * o.scale,
        -y1 * o.scale
    );
    let _ = write!(
        s,
        "\n<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{vw:.2}\" height=\"{vh:.2}\" fill=\"#ffffff\"/>",
        x0 * o.scale,
        -y1 * o.scale
    );
    for p in prims {
        s.push('\n');
        if p.owner.is_empty() {
            s.push_str(&w.prim(p));
        } else {
            let _ = write!(s, "<g data-id=\"{}\">{}</g>", esc(&p.owner), w.prim(p));
        }
    }
    s.push_str("\n</svg>\n");
    s
}
