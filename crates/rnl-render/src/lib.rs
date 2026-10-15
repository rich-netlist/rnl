//! The shared RNL renderer (architecture 4.3): the effective model of a drawing level becomes
//! basic primitives (paths, rectangles, circles, ellipses, texts), each with the object it draws
//! and a bounding box; an output backend then writes them, here as SVG.
//!
//! The renderer draws what the model says and nothing more. Its defaults (colours, line width,
//! the pin names) are this implementation's own choices (principle 2.7, 9.3), and so is how it
//! draws a line style: it resolves each into strokes, which every output draws as they are. It lays
//! out texts with the metrics of the fonts shipped with the core (decision 0009), in runs at their
//! places and bars as strokes, which every output draws as they are in those fonts; what the fonts
//! do not map, in runs of its own at estimated widths, which an output draws in a font of the
//! system's, centred on them.
//!
//! Sines, cosines and arc tangents come from `libm`, a math library in Rust, so that native and
//! WebAssembly builds compute the same bits (decision 0008).

mod line;
mod svg;
mod text;

use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::interp::obj::{Lit, PrimParams, Shape};
use rnl_runtime::markup::{self, Line};
use rnl_runtime::model::{Layer, LevelView, Style, SymbolView};

pub use svg::svg_document;
pub use text::{lay_out, Face};

/// Options of the drawing.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    /// Pixels per scene unit in the SVG; the primitives give no dash shorter than a pixel at it.
    pub scale: f64,
}

impl Default for RenderOptions {
    fn default() -> RenderOptions {
        RenderOptions { scale: 14.0 }
    }
}

/// Draw one level as an SVG document.
pub fn svg(lv: &LevelView, o: &RenderOptions) -> String {
    svg_document(&primitives(lv, o), o)
}

/// One stroke of a line, its style resolved (9.3): a shape's strokes are drawn along its outline,
/// first to last, over its fill.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub color: String,
    /// In scene units, or in pixels when `pixels`.
    pub width: f64,
    /// The width and the dashes are in pixels, the same at every zoom: the hairline (9.3) and
    /// flylines. Otherwise they are in scene units.
    pub pixels: bool,
    /// The lengths of the dashes and of the gaps between them, in turn, in the width's unit,
    /// starting with a dash; empty for an unbroken line.
    pub dashes: Vec<f64>,
    /// How far into the dashes the outline starts, in the width's unit.
    pub dash_offset: f64,
    /// "butt", "round" or "square".
    pub cap: String,
    /// "miter", "round" or "bevel".
    pub join: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PathSeg {
    Move(Pt),
    Line(Pt),
    /// A circular arc to `to` with radius `r`; `ccw` turns counter-clockwise (y up).
    Arc {
        r: f64,
        large: bool,
        ccw: bool,
        to: Pt,
    },
    Cubic(Pt, Pt, Pt),
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Path {
        segs: Vec<PathSeg>,
        strokes: Vec<Stroke>,
        fill: Option<String>,
    },
    Rect {
        min: Pt,
        max: Pt,
        strokes: Vec<Stroke>,
        fill: Option<String>,
    },
    Circle {
        c: Pt,
        r: f64,
        strokes: Vec<Stroke>,
        fill: Option<String>,
    },
    Ellipse {
        c: Pt,
        rx: f64,
        ry: f64,
        strokes: Vec<Stroke>,
        fill: Option<String>,
    },
    Text(TextPrim),
    /// A line a viewer may show for a connection not drawn yet (13.7); not a wire.
    Flyline {
        a: Pt,
        b: Pt,
        strokes: Vec<Stroke>,
    },
}

/// A text at an anchor, and how `lay_out` laid it out: its runs and bars.
#[derive(Clone, Debug, PartialEq)]
pub struct TextPrim {
    pub anchor: Pt,
    pub lines: Vec<Line>,
    /// Text height, the em of the font, in scene units.
    pub size: f64,
    pub color: String,
    /// The font the style names, a hint (9.3): the text is laid out in the shipped family.
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    /// "start", "middle" or "end", after turning (9.2.5).
    pub halign: String,
    /// "baseline", "top", "middle" or "bottom", after turning.
    pub valign: String,
    /// Reads from bottom to top.
    pub vertical: bool,
    pub runs: Vec<TextRun>,
    pub bars: Vec<TextBar>,
}

/// A run of a laid-out text: `text` in one face and size from `at`, the start of its baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub at: Pt,
    /// The em in scene units.
    pub size: f64,
    /// The sum of its characters' advances, in scene units; when `fallback`, an estimate.
    pub width: f64,
    /// In the bold face.
    pub bold: bool,
    /// Leaning by `SLANT`, with the upright advances.
    pub italic: bool,
    /// Grapheme clusters the shipped fonts do not map, drawn in a font of the system's, which the
    /// layout does not know: `width` is estimated by the clusters' classes, and the run is drawn
    /// centred on it as wide as the system's font makes it.
    pub fallback: bool,
}

/// A straight line of a laid-out text, a fraction's bar or an overline, from `a` to `b`.
#[derive(Clone, Debug, PartialEq)]
pub struct TextBar {
    pub a: Pt,
    pub b: Pt,
    pub stroke: Stroke,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Primitive {
    /// The object it draws, as the model names it.
    pub owner: String,
    pub layer: Layer,
    pub order: Vec<f64>,
    pub kind: Kind,
    /// x0, y0, x1, y1 in scene units.
    pub bbox: [f64; 4],
}

/// The renderer's own default style (9.3: what no style gives is the renderer's).
fn style_get<'a>(st: &'a Style, k: &str) -> Option<&'a Lit> {
    st.get(k)
}

#[derive(Clone, Debug)]
struct Eff {
    linecolor: String,
    fillcolor: String,
    linewidth: f64,
    linestyle: String,
    cap: String,
    join: String,
    size: f64,
    textcolor: Option<String>,
    font: String,
    bold: bool,
    italic: bool,
    arrowsize: f64,
}

impl Eff {
    fn from(st: &Style) -> Eff {
        let s =
            |k: &str, d: &str| style_get(st, k).and_then(Lit::str).map(String::from).unwrap_or_else(|| d.to_string());
        let n = |k: &str, d: f64| style_get(st, k).and_then(Lit::num).unwrap_or(d);
        let b = |k: &str| style_get(st, k).and_then(Lit::boolean).unwrap_or(false);
        Eff {
            linecolor: s("linecolor", "#1d1d1f"),
            fillcolor: s("fillcolor", "#ffffff"),
            linewidth: n("linewidth", 0.0),
            linestyle: s("linestyle", "solid"),
            cap: s("cap", "round"),
            join: s("join", "round"),
            size: n("size", 1.0),
            textcolor: style_get(st, "textcolor").and_then(Lit::str).map(String::from),
            font: s("font", "sans-serif"),
            bold: b("bold"),
            italic: b("italic"),
            arrowsize: n("arrowsize", 0.6),
        }
    }

    /// The strokes of a line in this style, at `scale` pixels per scene unit; the fill colour is
    /// the one between the two lines of a "double" line.
    fn strokes(&self, scale: f64) -> Vec<Stroke> {
        line::Line {
            style: &self.linestyle,
            width: self.linewidth,
            color: &self.linecolor,
            inner: &self.fillcolor,
            cap: &self.cap,
            join: &self.join,
        }
        .strokes(scale)
    }
}

fn merged(base: &Style, over: &Style) -> Style {
    let mut s = base.clone();
    for (k, v) in over {
        s.insert(k.clone(), v.clone());
    }
    s
}

fn place(base: Pt, code: Orient, p: Pt) -> Pt {
    let d = code.apply(p);
    (base.0 + d.0, base.1 + d.1)
}

fn sub(order: &[f64], k: f64) -> Vec<f64> {
    let mut v = order.to_vec();
    v.push(k);
    v
}

struct Out {
    prims: Vec<Primitive>,
    /// Pixels per scene unit, at which the strokes are resolved.
    scale: f64,
}

impl Out {
    fn push(&mut self, owner: &str, layer: Layer, order: Vec<f64>, kind: Kind, pts: &[Pt]) {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for p in pts {
            b[0] = b[0].min(p.0);
            b[1] = b[1].min(p.1);
            b[2] = b[2].max(p.0);
            b[3] = b[3].max(p.1);
        }
        if pts.is_empty() {
            b = [0.0; 4];
        }
        self.prims.push(Primitive { owner: owner.to_string(), layer, order, kind, bbox: b });
    }

    /// A graphic (9.2.5) at scene points.
    #[allow(clippy::too_many_arguments)]
    fn shape(
        &mut self,
        owner: &str,
        shape: Shape,
        pts: &[Pt],
        params: &PrimParams,
        st: &Style,
        layer: Layer,
        order: Vec<f64>,
    ) {
        let e = Eff::from(st);
        let fill = match params.fill.as_deref() {
            Some("linecolor") => Some(e.linecolor.clone()),
            Some("fillcolor") => Some(e.fillcolor.clone()),
            _ => None,
        };
        let strokes = e.strokes(self.scale);
        match shape {
            Shape::Line => {
                let closed = pts.len() > 2
                    && (pts[0].0 - pts[pts.len() - 1].0).abs() < 1e-9
                    && (pts[0].1 - pts[pts.len() - 1].1).abs() < 1e-9;
                let mut segs = vec![PathSeg::Move(pts[0])];
                segs.extend(pts[1..].iter().map(|p| PathSeg::Line(*p)));
                if closed {
                    segs.push(PathSeg::Close);
                }
                self.path(owner, segs, strokes, fill, layer, order.clone(), pts);
                self.arrows(owner, pts, params, &e, layer, order, false);
            }
            Shape::Rectangle => {
                let (a, b) = if pts.len() == 1 {
                    let (w, h) = (params.width.unwrap_or(0.0), params.height.unwrap_or(0.0));
                    ((pts[0].0 - w / 2.0, pts[0].1 - h / 2.0), (pts[0].0 + w / 2.0, pts[0].1 + h / 2.0))
                } else {
                    (pts[0], pts[1])
                };
                let min = (a.0.min(b.0), a.1.min(b.1));
                let max = (a.0.max(b.0), a.1.max(b.1));
                self.push(owner, layer, order, Kind::Rect { min, max, strokes, fill }, &[min, max]);
            }
            Shape::Circle => {
                let (c, r) = if pts.len() == 1 {
                    (pts[0], params.radius.unwrap_or(0.0))
                } else {
                    (pts[0], dist(pts[0], pts[1]))
                };
                self.push(
                    owner,
                    layer,
                    order,
                    Kind::Circle { c, r, strokes, fill },
                    &[(c.0 - r, c.1 - r), (c.0 + r, c.1 + r)],
                );
            }
            Shape::Ellipse => {
                let (c, rx, ry) = if pts.len() == 1 {
                    (pts[0], params.width.unwrap_or(0.0) / 2.0, params.height.unwrap_or(0.0) / 2.0)
                } else {
                    (
                        ((pts[0].0 + pts[1].0) / 2.0, (pts[0].1 + pts[1].1) / 2.0),
                        (pts[0].0 - pts[1].0).abs() / 2.0,
                        (pts[0].1 - pts[1].1).abs() / 2.0,
                    )
                };
                self.push(
                    owner,
                    layer,
                    order,
                    Kind::Ellipse { c, rx, ry, strokes, fill },
                    &[(c.0 - rx, c.1 - ry), (c.0 + rx, c.1 + ry)],
                );
            }
            Shape::Arc => {
                let segs = arc_segs(pts[0], pts[1], pts[2]);
                self.path(owner, segs, strokes, fill, layer, order.clone(), pts);
                self.arrows(owner, pts, params, &e, layer, order, true);
            }
            Shape::Bezier => {
                let mut segs = vec![PathSeg::Move(pts[0])];
                let mut k = 1;
                while k + 2 < pts.len() {
                    segs.push(PathSeg::Cubic(pts[k], pts[k + 1], pts[k + 2]));
                    k += 3;
                }

                self.path(owner, segs, strokes, fill, layer, order.clone(), pts);
                self.arrows(owner, pts, params, &e, layer, order, false);
            }
            Shape::Text => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn path(
        &mut self,
        owner: &str,
        segs: Vec<PathSeg>,
        strokes: Vec<Stroke>,
        fill: Option<String>,
        layer: Layer,
        order: Vec<f64>,
        pts: &[Pt],
    ) {
        if strokes.is_empty() && fill.is_none() {
            return;
        }
        self.push(owner, layer, order, Kind::Path { segs, strokes, fill }, pts);
    }

    /// Arrow heads at the ends of a line (9.2.5): filled triangles in the line colour.
    #[allow(clippy::too_many_arguments)]
    fn arrows(
        &mut self,
        owner: &str,
        pts: &[Pt],
        params: &PrimParams,
        e: &Eff,
        layer: Layer,
        order: Vec<f64>,
        arc: bool,
    ) {
        if e.linestyle == "none" {
            return; // the heads belong to the line (9.3)
        }
        let ends: &[&str] = match params.arrow.as_deref() {
            Some("start") => &["start"],
            Some("end") => &["end"],
            Some("both") => &["start", "end"],
            _ => &[],
        };
        let size = if e.arrowsize > 0.0 { e.arrowsize } else { 0.6 };
        for end in ends {
            let (tip, mut prev) =
                if *end == "end" { (pts[pts.len() - 1], pts[pts.len() - 2]) } else { (pts[0], pts[1]) };
            if arc {
                if let Some((c, _r)) = circle3(pts[0], pts[1], pts[2]) {
                    let (rx, ry) = (tip.0 - c.0, tip.1 - c.1);
                    let mut t1 = (-ry, rx);
                    let other = pts[1];
                    if (other.0 - tip.0) * t1.0 + (other.1 - tip.1) * t1.1 > 0.0 {
                        t1 = (ry, -rx);
                    }
                    prev = (tip.0 - t1.0, tip.1 - t1.1);
                }
            }
            let (dx, dy) = (tip.0 - prev.0, tip.1 - prev.1);
            let l = (dx * dx + dy * dy).sqrt().max(1e-12);
            let (ux, uy) = (dx / l, dy / l);
            let b = (tip.0 - ux * size, tip.1 - uy * size);
            let w = size * 0.35;
            let p1 = (b.0 - uy * w, b.1 + ux * w);
            let p2 = (b.0 + uy * w, b.1 - ux * w);
            let segs = vec![PathSeg::Move(tip), PathSeg::Line(p1), PathSeg::Line(p2), PathSeg::Close];
            self.push(
                owner,
                layer,
                sub(&order, 0.5),
                Kind::Path { segs, strokes: Vec::new(), fill: Some(e.linecolor.clone()) },
                &[tip, p1, p2],
            );
        }
    }

    /// A text at an anchor (9.2.5, 13.6), laid out with the shipped fonts' metrics: the box sits by
    /// the alignment and turns with `rotate` and the instance's orientation, but the glyphs always
    /// read upright.
    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        owner: &str,
        anchor: Pt,
        lines: Vec<Line>,
        halign: &str,
        valign: &str,
        rotate: f64,
        st: &Style,
        layer: Layer,
        order: Vec<f64>,
        code: Orient,
    ) {
        if lines.iter().all(|l| l.is_empty()) {
            return;
        }
        let e = Eff::from(st);
        let size = if e.size > 0.0 { e.size } else { 1.0 };
        let color = e.textcolor.clone().unwrap_or_else(|| e.linecolor.clone());
        let rot = rotate as i64;
        let u = code.apply(rot_vec((1.0, 0.0), rot));
        let v = code.apply(rot_vec((0.0, 1.0), rot));
        let vertical = u.0.abs() < 0.5;
        let flip_a = if vertical { u.1 < -0.5 } else { u.0 < -0.5 };
        let normal = if vertical { (-1.0, 0.0) } else { (0.0, 1.0) };
        let flip_v = v.0 * normal.0 + v.1 * normal.1 < 0.0;
        let mut halign = halign.to_string();
        let mut valign = valign.to_string();
        if flip_a {
            halign = match halign.as_str() {
                "start" => "end".into(),
                "end" => "start".into(),
                h => h.into(),
            };
        }
        if flip_v {
            valign = match valign.as_str() {
                "top" => "bottom".into(),
                "bottom" => "top".into(),
                h => h.into(),
            };
        }
        let mut t = TextPrim {
            anchor,
            lines,
            size,
            color,
            font: e.font.clone(),
            bold: e.bold,
            italic: e.italic,
            halign,
            valign,
            vertical,
            runs: Vec::new(),
            bars: Vec::new(),
        };
        let [x0, y0, x1, y1] = lay_out(&mut t, self.scale);
        self.push(owner, layer, sub(&order, 0.9), Kind::Text(t), &[(x0, y0), (x1, y1)]);
    }

    #[allow(clippy::too_many_arguments)]
    fn symbol(
        &mut self,
        lv: &LevelView,
        owner: &str,
        sym: &SymbolView,
        base: Pt,
        code: Orient,
        own: &Style,
        layer: Layer,
        order: &[f64],
        texts: &std::collections::BTreeMap<String, Vec<Line>>,
        path: &str,
    ) {
        for (k, p) in sym.prims.iter().enumerate() {
            let st = merged(own, &p.style);
            let gpts: Vec<Pt> = p.points.iter().map(|q| place(base, code, *q)).collect();
            let key = sub(order, k as f64);
            if p.shape == Shape::Text {
                let lines = texts.get(&format!("{path}{k}")).cloned().unwrap_or_default();
                self.text(
                    owner,
                    gpts[0],
                    lines,
                    p.halign.as_deref().unwrap_or("start"),
                    p.valign.as_deref().unwrap_or("baseline"),
                    p.rotate.unwrap_or(0.0),
                    &st,
                    layer,
                    key,
                    code,
                );
                continue;
            }
            let mut params = p.params.clone();
            if matches!(p.shape, Shape::Rectangle | Shape::Ellipse) && p.points.len() == 1 && code.swaps_axes() {
                // the centred form turns with the symbol
                std::mem::swap(&mut params.width, &mut params.height);
            }
            self.shape(owner, p.shape, &gpts, &params, &st, layer, key);
        }
        // pin names: shown by this viewer just inside the pin, past the pin's lead when a line of
        // the symbol runs from the pin inwards, so that a block's names are inside its body (9.2.4
        // leaves it to the viewer)
        for (k, pin) in sym.pins.iter().enumerate() {
            let (Some(label), Some(dir)) = (&pin.label, pin.dir) else { continue };
            let inward = match dir {
                0 => (-1.0, 0.0),
                90 => (0.0, -1.0),
                180 => (1.0, 0.0),
                _ => (0.0, 1.0),
            };
            let lead = sym
                .prims
                .iter()
                .filter(|p| p.shape == Shape::Line && p.points.len() == 2)
                .filter_map(|p| {
                    let near = |q: Pt| (q.0 - pin.pos.0).abs() < 1e-9 && (q.1 - pin.pos.1).abs() < 1e-9;
                    let other = if near(p.points[0]) {
                        p.points[1]
                    } else if near(p.points[1]) {
                        p.points[0]
                    } else {
                        return None;
                    };
                    let d = (other.0 - pin.pos.0, other.1 - pin.pos.1);
                    let along = d.0 * inward.0 + d.1 * inward.1;
                    let across = d.0 * inward.1 - d.1 * inward.0;
                    (along > 1e-9 && across.abs() < 1e-9).then_some(along)
                })
                .fold(0.0, f64::max);
            let at = (pin.pos.0 + (lead + 0.45) * inward.0, pin.pos.1 + (lead + 0.45) * inward.1);
            let halign = if inward == (-1.0, 0.0) {
                "end"
            } else if inward == (1.0, 0.0) {
                "start"
            } else {
                "middle"
            };
            let valign = if inward == (0.0, -1.0) {
                "top"
            } else if inward == (0.0, 1.0) {
                "bottom"
            } else {
                "middle"
            };
            let mut st = own.clone();
            st.insert("size".into(), Lit::Num(0.55));
            st.insert("textcolor".into(), Lit::Str(rnl_runtime::interp::obj::WStr::new("#6b6b70", None)));
            self.text(
                owner,
                place(base, code, at),
                markup::parse(label),
                halign,
                valign,
                0.0,
                &st,
                layer,
                sub(order, 200.0 + k as f64),
                code,
            );
        }
        for (k, n) in sym.nested.iter().enumerate() {
            let inner = &lv.symbols[n.symbol];
            let p = place(base, code, n.at);
            self.symbol(
                lv,
                owner,
                inner,
                p,
                n.orient.then(code),
                own,
                layer,
                &sub(order, 100.0 + k as f64),
                texts,
                &format!("{path}{k}/"),
            );
        }
    }
}

fn rot_vec(v: Pt, deg: i64) -> Pt {
    let r = (deg as f64).to_radians();
    let (sin, cos) = (libm::sin(r), libm::cos(r));
    ((v.0 * cos - v.1 * sin).round(), (v.0 * sin + v.1 * cos).round())
}

fn dist(a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    (dx * dx + dy * dy).sqrt()
}

/// The circle through three points.
pub fn circle3(a: Pt, b: Pt, c: Pt) -> Option<(Pt, f64)> {
    let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
    if d.abs() < 1e-12 {
        return None;
    }
    let (aa, bb, cc) = (a.0 * a.0 + a.1 * a.1, b.0 * b.0 + b.1 * b.1, c.0 * c.0 + c.1 * c.1);
    let ux = (aa * (b.1 - c.1) + bb * (c.1 - a.1) + cc * (a.1 - b.1)) / d;
    let uy = (aa * (c.0 - b.0) + bb * (a.0 - c.0) + cc * (b.0 - a.0)) / d;
    Some(((ux, uy), dist(a, (ux, uy))))
}

/// An arc from a through b to c.
fn arc_segs(a: Pt, b: Pt, c: Pt) -> Vec<PathSeg> {
    let Some(((ux, uy), r)) = circle3(a, b, c) else {
        return vec![PathSeg::Move(a), PathSeg::Line(c)];
    };
    let cross = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let ccw = cross > 0.0; // in y-up coordinates a -> b -> c turns counter-clockwise
    let ang = |p: Pt| libm::atan2(p.1 - uy, p.0 - ux);
    let two_pi = std::f64::consts::TAU;
    let sweep_ccw = (ang(c) - ang(a)).rem_euclid(two_pi);
    let span = if ccw { sweep_ccw } else { two_pi - sweep_ccw };
    vec![PathSeg::Move(a), PathSeg::Arc { r, large: span > std::f64::consts::PI, ccw, to: c }]
}

/// The primitives of one level, in drawing order: by layer, then by position (11.6), with their
/// strokes resolved at the scale of `o`.
pub fn primitives(lv: &LevelView, o: &RenderOptions) -> Vec<Primitive> {
    let mut out = Out { prims: Vec::new(), scale: o.scale };
    for it in &lv.items {
        let sym = &lv.symbols[it.symbol];
        out.symbol(lv, &it.id, sym, it.at, it.orient, &it.style, it.layer, &it.order, &it.texts, "");
    }
    for w in &lv.wires {
        let e = Eff::from(&w.style);
        if w.points.len() < 2 {
            continue;
        }
        let mut segs = vec![PathSeg::Move(w.points[0])];
        segs.extend(w.points[1..].iter().map(|p| PathSeg::Line(*p)));
        out.path(&w.id, segs, e.strokes(o.scale), None, w.layer, w.order.clone(), &w.points);
    }
    for (a, b) in &lv.flylines {
        let kind = Kind::Flyline { a: *a, b: *b, strokes: vec![line::flyline()] };
        out.push("", Layer::Middle, vec![2.0, 0.0], kind, &[*a, *b]);
    }
    for s in &lv.shapes {
        out.shape("", s.shape, &s.points, &s.params, &s.style, s.layer, s.order.clone());
    }
    for t in &lv.texts {
        out.text(
            &t.id,
            t.anchor,
            t.lines.clone(),
            &t.halign,
            &t.valign,
            t.rotate,
            &t.style,
            t.layer,
            t.order.clone(),
            t.orient,
        );
    }
    let mut prims = out.prims;
    prims.sort_by(|a, b| {
        a.layer.cmp(&b.layer).then_with(|| a.order.partial_cmp(&b.order).unwrap_or(std::cmp::Ordering::Equal))
    });
    prims
}
