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

use rnl_runtime::geom::{fixed_text_alignment, Orient, Pt};
use rnl_runtime::interp::obj::{Lit, PrimParams, Shape};
use rnl_runtime::markup::{self, Line};
use rnl_runtime::model::{Layer, LevelView, Style, SymbolView, TextView};

pub use svg::{svg_document, with_net_names};
pub use text::{lay_out, lay_out_reflected_baseline, Face};

/// Lay out 0.4 text once for drawing and solving. Coordinates are scene x-right/y-up.
/// The local layout is transformed as a whole, including bars and markup. The separate
/// alignment in the final glyph axes keeps the frame on its transformed side.
#[allow(clippy::too_many_arguments)]
pub fn modern_text(
    anchor: Pt,
    lines: Vec<Line>,
    halign: &str,
    valign: &str,
    rotate: f64,
    style: &Style,
    owner: Orient,
    fix_rotate: bool,
    upright: bool,
    scale: f64,
) -> (TextPrim, [f64; 4]) {
    let (a, b, c, d) = owner.matrix();
    modern_text_affine(
        anchor,
        lines,
        halign,
        valign,
        rotate,
        style,
        [f64::from(a), f64::from(b), f64::from(c), f64::from(d)],
        fix_rotate,
        upright,
        scale,
    )
}

/// Lay out a modern text under any orthogonal owner transform, including an annotation's
/// arbitrary rotation and mirror. The matrix is [xx, xy, yx, yy], in scene coordinates;
/// its translation is already reflected in `anchor`. This shares the glyph layout and
/// upright/fixed-direction policy with ordinary symbol texts.
#[allow(clippy::too_many_arguments)]
pub fn modern_text_affine(
    anchor: Pt,
    lines: Vec<Line>,
    halign: &str,
    valign: &str,
    rotate: f64,
    style: &Style,
    owner: [f64; 4],
    fix_rotate: bool,
    upright: bool,
    scale: f64,
) -> (TextPrim, [f64; 4]) {
    modern_text_affine_inner(anchor, lines, halign, valign, rotate, style, owner, None, fix_rotate, upright, scale)
}

// Keep the low part of the sum for strict 90/270 comparisons, without an epsilon.
fn annotation_upright_flip(owner_angle: f64, text_angle: f64, mirror: bool) -> bool {
    fn two_sum(a: f64, b: f64) -> (f64, f64) {
        let sum = a + b;
        let bv = sum - a;
        (sum, (a - (sum - bv)) + (b - bv))
    }
    let owner = owner_angle.rem_euclid(360.0);
    let text = text_angle.rem_euclid(360.0);
    let (first, e1) = two_sum(owner, if mirror { -text } else { text });
    let (sum, e2) = two_sum(first, if mirror { 180.0 } else { 0.0 });
    let (high, e3) = if sum < 0.0 {
        two_sum(sum, 360.0)
    } else if sum >= 360.0 {
        two_sum(sum, -360.0)
    } else {
        (sum, 0.0)
    };
    let low = e1 + e2 + e3;
    let after_90 = high > 90.0 || (high == 90.0 && low > 0.0);
    let through_270 = high < 270.0 || (high == 270.0 && low <= 0.0);
    after_90 && through_270
}

#[allow(clippy::too_many_arguments)]
fn modern_text_affine_inner(
    anchor: Pt,
    lines: Vec<Line>,
    halign: &str,
    valign: &str,
    rotate: f64,
    style: &Style,
    owner: [f64; 4],
    annotation_owner: Option<(f64, bool)>,
    fix_rotate: bool,
    upright: bool,
    scale: f64,
) -> (TextPrim, [f64; 4]) {
    let e = Eff::from(style);
    let mut t = TextPrim {
        anchor,
        lines,
        size: if e.size > 0.0 { e.size } else { 1.0 },
        color: e.textcolor.clone().unwrap_or_else(|| e.linecolor.clone()),
        font: e.font.clone(),
        bold: e.bold,
        italic: e.italic,
        halign: halign.into(),
        valign: valign.into(),
        vertical: false,
        transform: None,
        runs: Vec::new(),
        bars: Vec::new(),
    };
    let angle = rotate.rem_euclid(360.0);
    let (sn, cs) = if angle == 0.0 {
        (0.0, 1.0)
    } else if angle == 90.0 {
        (1.0, 0.0)
    } else if angle == 180.0 {
        (0.0, -1.0)
    } else if angle == 270.0 {
        (-1.0, 0.0)
    } else {
        let rad = angle.to_radians();
        (libm::sin(rad), libm::cos(rad))
    };
    let [a, b, c, d] = owner;
    let apply = |(x, y): Pt| (a * x + b * y, c * x + d * y);
    let (box_u, box_v) = (apply((cs, sn)), apply((-sn, cs)));
    let (mut u, mut v) = if fix_rotate { ((cs, sn), (-sn, cs)) } else { (box_u, box_v) };
    if upright {
        // Remove glyph reflection while leaving the box-side transform independent.
        if u.0 * v.1 - u.1 * v.0 < 0.0 {
            v = (-v.0, -v.1);
        }
        // The annotation carries its written angle: reconstructing it from atan2 can
        // move an exact boundary. Legacy orientation callers keep their existing path.
        let flip = if let Some((owner_angle, mirror)) = annotation_owner.filter(|_| !fix_rotate) {
            annotation_upright_flip(owner_angle, rotate, mirror)
        } else {
            let base = if a == 1.0 && c == 0.0 {
                0.0
            } else if a == 0.0 && c == 1.0 {
                90.0
            } else if a == -1.0 && c == 0.0 {
                180.0
            } else if a == 0.0 && c == -1.0 {
                270.0
            } else {
                libm::atan2(c, a).to_degrees().rem_euclid(360.0)
            };
            let rotation = rotate.rem_euclid(360.0);
            let angle =
                if fix_rotate { rotation } else { (base + (a * d - b * c).signum() * rotation).rem_euclid(360.0) };
            angle > 90.0 && angle <= 270.0
        };
        if flip {
            u = (-u.0, -u.1);
            v = (-v.0, -v.1);
        }
    }
    // Alignment names describe the side of the anchor, not a text-box centre.
    // Transform that side through the owner and express it in the final glyph axes;
    // then lay out from the resulting alignment. This matters for fixed rotation:
    // an R90 owner changes middle/bottom to end/middle, regardless of text width.
    let hx = match halign {
        "start" => 1.0,
        "end" => -1.0,
        _ => 0.0,
    };
    let vy = match valign {
        "top" => -1.0,
        "bottom" | "baseline" => 1.0,
        _ => 0.0,
    };
    let side = (box_u.0 * hx + box_v.0 * vy, box_u.1 * hx + box_v.1 * vy);
    let (x, y) = (side.0 * u.0 + side.1 * u.1, side.0 * v.0 + side.1 * v.1);
    let sign = |n: f64| {
        if n > 1e-9 {
            1
        } else if n < -1e-9 {
            -1
        } else {
            0
        }
    };
    // For fixed rotation, 13.3 changes a baseline to top only when the
    // owner's orientation puts that side below the original glyph basis.
    // A later 180-degree upright correction is a separate 9.2.5 operation.
    let owner_side_y = side.0 * -sn + side.1 * cs;
    let keep_baseline = valign == "baseline" && !(fix_rotate && sign(owner_side_y) < 0);
    t.halign = match sign(x) {
        1 => "start",
        -1 => "end",
        _ => "middle",
    }
    .into();
    t.valign = match sign(y) {
        1 if keep_baseline => "baseline",
        1 => "bottom",
        -1 if keep_baseline => "baseline",
        -1 => "top",
        _ => "middle",
    }
    .into();
    let local =
        if keep_baseline && sign(y) < 0 { lay_out_reflected_baseline(&mut t, scale) } else { lay_out(&mut t, scale) };
    let tx = anchor.0 - u.0 * anchor.0 - v.0 * anchor.1;
    let ty = anchor.1 - u.1 * anchor.0 - v.1 * anchor.1;
    let affine = [u.0, v.0, u.1, v.1, tx, ty];
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for (x, y) in [(local[0], local[1]), (local[0], local[3]), (local[2], local[1]), (local[2], local[3])] {
        let p = (affine[0] * x + affine[1] * y + tx, affine[2] * x + affine[3] * y + ty);
        b = [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)];
    }
    t.transform = Some(affine);
    (t, b)
}

/// Options of the drawing.
#[derive(Clone, Debug)]
pub struct RenderOptions {
    /// Pixels per scene unit in the SVG; the primitives give no dash shorter than a pixel at it.
    pub scale: f64,
    /// Draw the level's flylines (13.3, 13.7), those of every net, to check what is not wired:
    /// each a primitive of its own and, in the SVG, a group of class "flyline", which a viewer can
    /// hide; false by default.
    pub flylines: bool,
    /// Write the name of each connected part of every net (`LevelView::net_labels`) over the
    /// drawing, as a viewer's View > Show Net Names does: in the SVG only, a group of class
    /// "net-labels" after everything else; false by default.
    pub net_names: bool,
}

/// The `data-id` of the SVG group that draws a level's flyline `k`, and the owner of its
/// primitive: "flyline-0" for the first of `LevelView::flylines`.
pub fn flyline_id(k: usize) -> String {
    format!("flyline-{k}")
}

impl Default for RenderOptions {
    fn default() -> RenderOptions {
        RenderOptions { scale: 14.0, flylines: false, net_names: false }
    }
}

/// Draw one level as an SVG document.
pub fn svg(lv: &LevelView, o: &RenderOptions) -> String {
    let s = svg_document(&primitives(lv, o), o);
    if o.net_names {
        svg::with_net_names(&s, lv, o.scale)
    } else {
        s
    }
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
    /// The style's font hint (9.3), written by SVG; monospace hints use a fixed ASCII advance.
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    /// "start", "middle" or "end", after turning (9.2.5).
    pub halign: String,
    /// "baseline", "top", "middle" or "bottom", after turning.
    pub valign: String,
    /// Reads from bottom to top.
    pub vertical: bool,
    /// Optional scene-space affine [a,b,c,d,tx,ty] applied to the laid-out runs.
    pub transform: Option<[f64; 6]>,
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

fn annotation_angle(point: Pt, center: Pt) -> f64 {
    let (x, y) = (point.0 - center.0, point.1 - center.1);
    if x.is_finite() && y.is_finite() {
        libm::atan2(y, x)
    } else {
        libm::atan2(point.1 / 2.0 - center.1 / 2.0, point.0 / 2.0 - center.0 / 2.0)
    }
}

/// Stable circle through annotation arc points, calculated from scaled differences.
/// None means collinear; Err means the circle cannot be represented finitely.
fn annotation_circle3(a: Pt, b: Pt, c: Pt) -> Result<Option<(Pt, f64, bool)>, AnnotationBoundsError> {
    let raw = [b.0 - a.0, b.1 - a.1, c.0 - a.0, c.1 - a.1];
    let half = raw.iter().any(|x| !x.is_finite());
    let d = if half {
        [b.0 / 2.0 - a.0 / 2.0, b.1 / 2.0 - a.1 / 2.0, c.0 / 2.0 - a.0 / 2.0, c.1 / 2.0 - a.1 / 2.0]
    } else {
        raw
    };
    let scale = d.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Ok(None);
    }
    let (bx, by, cx, cy) = (d[0] / scale, d[1] / scale, d[2] / scale, d[3] / scale);
    let cross = bx * cy - by * cx;
    if cross == 0.0 {
        return Ok(None);
    }
    let (bb, cc) = (bx * bx + by * by, cx * cx + cy * cy);
    let ux = (bb * cy - cc * by) / (2.0 * cross);
    let uy = (cc * bx - bb * cx) / (2.0 * cross);
    let (ox, oy) = (ux * scale, uy * scale);
    let radius = libm::hypot(ux, uy) * scale;
    let center = if half { ((a.0 / 2.0 + ox) * 2.0, (a.1 / 2.0 + oy) * 2.0) } else { (a.0 + ox, a.1 + oy) };
    let radius = if half { radius * 2.0 } else { radius };
    if !center.0.is_finite() || !center.1.is_finite() || !radius.is_finite() {
        return Err(AnnotationBoundsError::NonFinite);
    }
    Ok(Some((center, radius, cross > 0.0)))
}

/// Transform a local graphic as one rigid, optionally mirrored annotation.
fn annotation_primitive(primitive: &mut Primitive, matrix: [f64; 4], anchor: Pt, mirrored: bool) {
    let point = |p: Pt| (anchor.0 + matrix[0] * p.0 + matrix[1] * p.1, anchor.1 + matrix[2] * p.0 + matrix[3] * p.1);
    let old = primitive.bbox;
    let corners = [point((old[0], old[1])), point((old[0], old[3])), point((old[2], old[1])), point((old[2], old[3]))];
    primitive.bbox = [
        corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
        corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
        corners.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
        corners.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
    ];
    match &mut primitive.kind {
        Kind::Path { segs, .. } => {
            for seg in segs {
                match seg {
                    PathSeg::Move(p) | PathSeg::Line(p) => *p = point(*p),
                    PathSeg::Arc { ccw, to, .. } => {
                        *to = point(*to);
                        if mirrored {
                            *ccw = !*ccw;
                        }
                    }
                    PathSeg::Cubic(a, b, c) => {
                        *a = point(*a);
                        *b = point(*b);
                        *c = point(*c);
                    }
                    PathSeg::Close => (),
                }
            }
        }
        Kind::Rect { min, max, strokes, fill } => {
            let (a, b) = (*min, *max);
            let segs = vec![
                PathSeg::Move(point(a)),
                PathSeg::Line(point((b.0, a.1))),
                PathSeg::Line(point(b)),
                PathSeg::Line(point((a.0, b.1))),
                PathSeg::Close,
            ];
            primitive.kind = Kind::Path { segs, strokes: strokes.clone(), fill: fill.clone() };
        }
        Kind::Circle { c, r, .. } => {
            *c = point(*c);
            primitive.bbox = [c.0 - *r, c.1 - *r, c.0 + *r, c.1 + *r];
        }
        Kind::Ellipse { c, rx, ry, strokes, fill } => {
            let (x, y) = *c;
            let (rx, ry) = (*rx, *ry);
            let k = 0.5522847498307936;
            let segs = vec![
                PathSeg::Move(point((x + rx, y))),
                PathSeg::Cubic(point((x + rx, y + k * ry)), point((x + k * rx, y + ry)), point((x, y + ry))),
                PathSeg::Cubic(point((x - k * rx, y + ry)), point((x - rx, y + k * ry)), point((x - rx, y))),
                PathSeg::Cubic(point((x - rx, y - k * ry)), point((x - k * rx, y - ry)), point((x, y - ry))),
                PathSeg::Cubic(point((x + k * rx, y - ry)), point((x + rx, y - k * ry)), point((x + rx, y))),
                PathSeg::Close,
            ];
            primitive.kind = Kind::Path { segs, strokes: strokes.clone(), fill: fill.clone() };
        }
        Kind::Flyline { a, b, .. } => {
            *a = point(*a);
            *b = point(*b);
        }
        Kind::Text(_) => (),
    }
}

/// Failure to form a finite annotation ink box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationBoundsError {
    NonFinite,
    InvalidGeometry,
}

/// Compute the same visible annotation ink as drawing, relative to its anchor.
/// `scale` is pixels per scene unit and must be positive and finite.
pub fn annotation_bounds(
    annotation: &rnl_runtime::model::AnnotationView,
    scale: f64,
) -> Result<Option<[f64; 4]>, AnnotationBoundsError> {
    if !scale.is_finite()
        || scale <= 0.0
        || !annotation.rotate.is_finite()
        || !annotation.anchor.0.is_finite()
        || !annotation.anchor.1.is_finite()
    {
        return Err(AnnotationBoundsError::NonFinite);
    }
    for graphic in &annotation.graphics {
        let p = &graphic.primitive;
        if p.shape == Shape::Text
            && merged(&annotation.style, &p.style).get("textvisible").and_then(Lit::boolean) == Some(false)
        {
            continue;
        }
        if p.points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            return Err(AnnotationBoundsError::NonFinite);
        }
        let n = p.points.len();
        let valid = match p.shape {
            Shape::Text => n == 1,
            Shape::Line => n >= 2,
            Shape::Rectangle | Shape::Circle | Shape::Ellipse => n == 1 || n == 2,
            Shape::Arc => n == 3,
            Shape::Bezier => n >= 4 && (n - 1) % 3 == 0,
        };
        if !valid {
            return Err(AnnotationBoundsError::InvalidGeometry);
        }
    }
    let mut relative = annotation.clone();
    for graphic in &mut relative.graphics {
        if graphic.primitive.shape == Shape::Text
            && merged(&relative.style, &graphic.primitive.style).get("textvisible").and_then(Lit::boolean)
                == Some(false)
        {
            continue;
        }
        if let Some(text) = &graphic.primitive.text {
            if markup::attr_refs(text).len() != graphic.text_refs.len() {
                return Err(AnnotationBoundsError::InvalidGeometry);
            }
            let mut index = 0;
            let expanded = markup::expand_math(text, |_| {
                use rnl_runtime::model::AnnotationTextRef;
                let result = match graphic.text_refs[index] {
                    AnnotationTextRef::X => rnl_runtime::geom::fmt_num(annotation.anchor.0),
                    AnnotationTextRef::Y => rnl_runtime::geom::fmt_num(annotation.anchor.1),
                    AnnotationTextRef::Empty => String::new(),
                };
                index += 1;
                result
            });
            graphic.lines = markup::parse_with_name_formats(&expanded, graphic.primitive.text_name_formats);
        } else if !graphic.text_refs.is_empty() {
            return Err(AnnotationBoundsError::InvalidGeometry);
        }
    }
    relative.anchor = (0.0, 0.0);
    let mut out = Out { prims: Vec::new(), scale };
    out.annotation(&relative);
    let mut bounds = None::<[f64; 4]>;
    for primitive in out.prims {
        let finite = |p: Pt| p.0.is_finite() && p.1.is_finite();
        let valid = match &primitive.kind {
            Kind::Path { segs, .. } => segs.iter().all(|s| match s {
                PathSeg::Move(p) | PathSeg::Line(p) => finite(*p),
                PathSeg::Arc { r, to, .. } => r.is_finite() && finite(*to),
                PathSeg::Cubic(a, b, c) => finite(*a) && finite(*b) && finite(*c),
                PathSeg::Close => true,
            }),
            Kind::Rect { min, max, .. } => finite(*min) && finite(*max),
            Kind::Circle { c, r, .. } => finite(*c) && r.is_finite(),
            Kind::Ellipse { c, rx, ry, .. } => finite(*c) && rx.is_finite() && ry.is_finite(),
            Kind::Text(t) => {
                finite(t.anchor)
                    && t.size.is_finite()
                    && t.transform.is_none_or(|m| m.iter().all(|x| x.is_finite()))
                    && t.runs.iter().all(|r| finite(r.at) && r.size.is_finite() && r.width.is_finite())
                    && t.bars.iter().all(|b| finite(b.a) && finite(b.b) && b.stroke.width.is_finite())
            }
            Kind::Flyline { a, b, .. } => finite(*a) && finite(*b),
        };
        if !valid {
            return Err(AnnotationBoundsError::NonFinite);
        }
        let strokes: &[Stroke] = match &primitive.kind {
            Kind::Path { strokes, .. }
            | Kind::Rect { strokes, .. }
            | Kind::Circle { strokes, .. }
            | Kind::Ellipse { strokes, .. }
            | Kind::Flyline { strokes, .. } => strokes,
            Kind::Text(_) => &[],
        };
        if strokes.iter().any(|s| !s.width.is_finite() || s.width < 0.0) {
            return Err(AnnotationBoundsError::NonFinite);
        }
        let radius = strokes
            .iter()
            .map(|s| {
                let width = if s.pixels { s.width / scale } else { s.width };
                // SVG's default miter limit is four times the half-width.
                width * if s.join == "miter" { 2.0 } else { 0.5 }
            })
            .fold(0.0_f64, f64::max);
        let b = [
            primitive.bbox[0] - radius,
            primitive.bbox[1] - radius,
            primitive.bbox[2] + radius,
            primitive.bbox[3] + radius,
        ];
        if b.iter().any(|v| !v.is_finite()) {
            return Err(AnnotationBoundsError::NonFinite);
        }
        bounds = Some(match bounds {
            None => b,
            Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
        });
    }
    Ok(bounds)
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

    fn annotation(&mut self, annotation: &rnl_runtime::model::AnnotationView) {
        let theta = annotation.rotate.rem_euclid(360.0).to_radians();
        let (sin, cos) = (libm::sin(theta), libm::cos(theta));
        let flip = if annotation.mirror { -1.0 } else { 1.0 };
        let matrix = [cos * flip, -sin, sin * flip, cos];
        let transform = |p: Pt| {
            (
                annotation.anchor.0 + matrix[0] * p.0 + matrix[1] * p.1,
                annotation.anchor.1 + matrix[2] * p.0 + matrix[3] * p.1,
            )
        };
        for graphic in &annotation.graphics {
            let p = &graphic.primitive;
            let style = merged(&annotation.style, &p.style);
            let order = sub(&annotation.order, p.order as f64);
            if p.shape == Shape::Text {
                if style.get("textvisible").and_then(|v| v.boolean()) == Some(false)
                    || p.points.is_empty()
                    || graphic.lines.iter().all(|line| line.is_empty())
                {
                    continue;
                }
                let (text, bbox) = modern_text_affine_inner(
                    transform(p.points[0]),
                    graphic.lines.clone(),
                    p.halign.as_deref().unwrap_or("start"),
                    p.valign.as_deref().unwrap_or("baseline"),
                    p.rotate.unwrap_or(0.0),
                    &style,
                    matrix,
                    Some((annotation.rotate, annotation.mirror)),
                    p.fix_rotate,
                    p.upright.unwrap_or(true),
                    self.scale,
                );
                self.prims.push(Primitive {
                    owner: annotation.object_id.clone(),
                    layer: annotation.layer,
                    order,
                    kind: Kind::Text(text),
                    bbox,
                });
            } else if !p.points.is_empty() {
                let first = self.prims.len();
                if p.shape == Shape::Arc && p.points.len() == 3 {
                    let (a, b, c) = (p.points[0], p.points[1], p.points[2]);
                    let eff = Eff::from(&style);
                    let fill = match p.params.fill.as_deref() {
                        Some("linecolor") => Some(eff.linecolor.clone()),
                        Some("fillcolor") => Some(eff.fillcolor.clone()),
                        _ => None,
                    };
                    let segs = match annotation_circle3(a, b, c) {
                        Ok(Some((center, r, ccw))) => {
                            let angle = |q: Pt| annotation_angle(q, center);
                            let sweep = (angle(c) - angle(a)).rem_euclid(std::f64::consts::TAU);
                            let span = if ccw { sweep } else { std::f64::consts::TAU - sweep };
                            vec![PathSeg::Move(a), PathSeg::Arc { r, large: span > std::f64::consts::PI, ccw, to: c }]
                        }
                        Ok(None) => vec![PathSeg::Move(a), PathSeg::Line(c)],
                        Err(_) => vec![PathSeg::Move(a), PathSeg::Arc { r: f64::NAN, large: false, ccw: false, to: c }],
                    };
                    self.path(
                        &annotation.object_id,
                        segs,
                        eff.strokes(self.scale),
                        fill,
                        annotation.layer,
                        order.clone(),
                        &p.points,
                    );
                    self.arrows(&annotation.object_id, &p.points, &p.params, &eff, annotation.layer, order, p.shape);
                } else {
                    self.shape(&annotation.object_id, p.shape, &p.points, &p.params, &style, annotation.layer, order);
                }
                for prim in &mut self.prims[first..] {
                    annotation_primitive(prim, matrix, annotation.anchor, annotation.mirror);
                }
                if p.shape == Shape::Arc && p.points.len() == 3 && first < self.prims.len() {
                    let (a, b, c) = (p.points[0], p.points[1], p.points[2]);
                    let points = [transform(a), transform(b), transform(c)];
                    self.prims[first].bbox = [
                        points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
                        points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                        points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                        points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
                    ];
                    if let Ok(Some((center, radius, local_ccw))) = annotation_circle3(a, b, c) {
                        let center = transform(center);
                        let (a, c) = (transform(a), transform(c));
                        let ccw = local_ccw != annotation.mirror;
                        let start = annotation_angle(a, center);
                        let end = annotation_angle(c, center);
                        let tau = std::f64::consts::TAU;
                        let span = if ccw { (end - start).rem_euclid(tau) } else { (start - end).rem_euclid(tau) };
                        let bbox = &mut self.prims[first].bbox;
                        if !radius.is_finite() || !center.0.is_finite() || !center.1.is_finite() || !span.is_finite() {
                            *bbox = [f64::NAN; 4];
                        } else {
                            for (angle, dx, dy) in [
                                (0.0, 1.0, 0.0),
                                (std::f64::consts::FRAC_PI_2, 0.0, 1.0),
                                (std::f64::consts::PI, -1.0, 0.0),
                                (3.0 * std::f64::consts::FRAC_PI_2, 0.0, -1.0),
                            ] {
                                let delta =
                                    if ccw { (angle - start).rem_euclid(tau) } else { (start - angle).rem_euclid(tau) };
                                if delta <= span + 1e-12 {
                                    let q = (center.0 + radius * dx, center.1 + radius * dy);
                                    bbox[0] = bbox[0].min(q.0);
                                    bbox[1] = bbox[1].min(q.1);
                                    bbox[2] = bbox[2].max(q.0);
                                    bbox[3] = bbox[3].max(q.1);
                                }
                            }
                        }
                    } else if annotation_circle3(a, b, c).is_err() {
                        self.prims[first].bbox = [f64::NAN; 4];
                    }
                }
            }
        }
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
                self.arrows(owner, pts, params, &e, layer, order, shape);
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
                self.arrows(owner, pts, params, &e, layer, order, shape);
            }
            Shape::Bezier => {
                let mut segs = vec![PathSeg::Move(pts[0])];
                let mut k = 1;
                while k + 2 < pts.len() {
                    segs.push(PathSeg::Cubic(pts[k], pts[k + 1], pts[k + 2]));
                    k += 3;
                }

                self.path(owner, segs, strokes, fill, layer, order.clone(), pts);
                self.arrows(owner, pts, params, &e, layer, order, shape);
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
        shape: Shape,
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
            if shape == Shape::Bezier && prev == tip {
                // A coincident control point gives no tangent; use the next distinct point (9.2.5).
                let next = if *end == "end" {
                    pts.iter().rev().copied().skip(1).find(|p| *p != tip)
                } else {
                    pts.iter().copied().skip(1).find(|p| *p != tip)
                };
                if let Some(next) = next {
                    prev = next;
                }
            }
            if shape == Shape::Arc {
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

    /// A text of the level, drawn as `primitives` draws it.
    fn view_text(&mut self, t: &TextView) {
        self.text(
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
            t.baseline_mirrors,
            t.fix_rotate.unwrap_or(false),
            t.upright.unwrap_or(true),
        );
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
        baseline_mirrors: bool,
        fix_rotate: bool,
        upright: bool,
    ) {
        if st.get("textvisible").and_then(|v| v.boolean()) == Some(false) {
            return;
        }
        if lines.iter().all(|l| l.is_empty()) {
            return;
        }
        let e = Eff::from(st);
        let size = if e.size > 0.0 { e.size } else { 1.0 };
        let color = e.textcolor.clone().unwrap_or_else(|| e.linecolor.clone());
        if baseline_mirrors {
            let (t, bbox) =
                modern_text(anchor, lines, halign, valign, rotate, st, code, fix_rotate, upright, self.scale);
            self.push(owner, layer, sub(&order, 0.9), Kind::Text(t), &[(bbox[0], bbox[1]), (bbox[2], bbox[3])]);
            return;
        }
        let (halign, valign) = if baseline_mirrors && fix_rotate {
            fixed_text_alignment(halign, valign, rotate, code)
        } else {
            (halign, valign)
        };
        let code = if fix_rotate { Orient::R0 } else { code };
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
            transform: None,
            runs: Vec::new(),
            bars: Vec::new(),
        };
        let [x0, y0, x1, y1] = if baseline_mirrors && flip_v && t.valign == "baseline" {
            lay_out_reflected_baseline(&mut t, self.scale)
        } else {
            lay_out(&mut t, self.scale)
        };
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
            let key = sub(order, p.order as f64);
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
                    p.modern_text,
                    p.fix_rotate,
                    p.upright.unwrap_or(true),
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
        let labels_after =
            sym.prims.iter().map(|p| p.order).chain(sym.nested.iter().map(|n| n.order)).max().map_or(0, |n| n + 1);
        for (k, pin) in sym.pins.iter().enumerate() {
            let (Some(label), Some(dir)) = (&pin.label, pin.dir) else {
                continue;
            };
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
                markup::parse_with_name_formats(label, pin.label_name_formats),
                halign,
                valign,
                0.0,
                &st,
                layer,
                sub(order, (labels_after + k) as f64),
                code,
                pin.label_name_formats,
                false,
                true,
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
                &sub(order, n.order as f64),
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

/// The bounding box of each text of the level as it is drawn, `x0, y0, x1, y1` in scene units, in
/// the order of `lv.texts`; None for a text that draws nothing. The primitives of texts that have
/// the same id are not told apart by their owner; this is how a viewer finds where one text is.
pub fn text_boxes(lv: &LevelView, o: &RenderOptions) -> Vec<Option<[f64; 4]>> {
    let mut out = Out { prims: Vec::new(), scale: o.scale };
    lv.texts
        .iter()
        .map(|t| {
            let from = out.prims.len();
            out.view_text(t);
            out.prims[from..]
                .iter()
                .map(|p| p.bbox)
                .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
        })
        .collect()
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
    for (k, f) in lv.flylines.iter().enumerate().filter(|_| o.flylines) {
        let kind = Kind::Flyline { a: f.a, b: f.b, strokes: vec![line::flyline()] };
        out.push(&flyline_id(k), Layer::Middle, vec![2.0, 0.0], kind, &[f.a, f.b]);
    }
    for annotation in &lv.annotations {
        out.annotation(annotation);
    }
    for s in &lv.shapes {
        out.shape("", s.shape, &s.points, &s.params, &s.style, s.layer, s.order.clone());
    }
    for t in &lv.texts {
        out.view_text(t);
    }
    let mut prims = out.prims;
    prims.sort_by(|a, b| {
        a.layer.cmp(&b.layer).then_with(|| a.order.partial_cmp(&b.order).unwrap_or(std::cmp::Ordering::Equal))
    });
    prims
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_text_angles_use_the_same_period_for_glyphs_and_upright() {
        for rotate in [1e20_f64, -1e20_f64, 1e100_f64, -1e100_f64] {
            for owner in Orient::ALL {
                for fix in [false, true] {
                    for upright in [false, true] {
                        let draw = |angle| {
                            modern_text(
                                (3.0, -2.0),
                                markup::parse("ABC"),
                                "start",
                                "baseline",
                                angle,
                                &Style::new(),
                                owner,
                                fix,
                                upright,
                                14.0,
                            )
                        };
                        let (large, bounds) = draw(rotate);
                        let (reduced, expected) = draw(rotate.rem_euclid(360.0));
                        assert_eq!(large.transform, reduced.transform);
                        assert_eq!((large.halign, large.valign), (reduced.halign, reduced.valign));
                        assert_eq!(bounds, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn fixed_rotation_changes_box_alignment_when_owner_turns() {
        let (text, b) = modern_text(
            (0.0, 0.0),
            markup::parse("asymmetric text"),
            "middle",
            "bottom",
            0.0,
            &Style::new(),
            Orient::R90,
            true,
            true,
            14.0,
        );
        assert_eq!((text.halign.as_str(), text.valign.as_str()), ("end", "middle"));
        assert!(b[0] < -1.0 && b[2].abs() < 1e-9, "{b:?}");
        assert!((b[1] + b[3]).abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn fixed_baseline_flipped_by_owner_uses_top_alignment() {
        let (text, b) = modern_text(
            (0.0, 0.0),
            markup::parse("ABC"),
            "start",
            "baseline",
            0.0,
            &Style::new(),
            Orient::MX,
            true,
            true,
            14.0,
        );
        assert_eq!(text.valign, "top");
        assert!(b[1] < 0.0 && b[3].abs() < 1e-9, "{b:?}");
        let (normal, _) = modern_text(
            (0.0, 0.0),
            markup::parse("ABC"),
            "start",
            "baseline",
            0.0,
            &Style::new(),
            Orient::MX,
            false,
            true,
            14.0,
        );
        assert_eq!(normal.valign, "baseline");
    }

    #[test]
    fn fixed_baseline_upright_turn_does_not_change_owner_r0_box() {
        for rotate in [0.0, 90.0, 180.0, 270.0] {
            let (fixed, a) = modern_text(
                (0.0, 0.0),
                markup::parse("ABC"),
                "start",
                "baseline",
                rotate,
                &Style::new(),
                Orient::R0,
                true,
                true,
                14.0,
            );
            let (normal, b) = modern_text(
                (0.0, 0.0),
                markup::parse("ABC"),
                "start",
                "baseline",
                rotate,
                &Style::new(),
                Orient::R0,
                false,
                true,
                14.0,
            );
            assert_eq!(fixed.valign, normal.valign, "rotate={rotate}");
            for (x, y) in a.into_iter().zip(b) {
                assert!((x - y).abs() < 1e-9, "rotate={rotate}: {a:?} != {b:?}");
            }
        }
    }

    #[test]
    fn quarter_turn_glyph_matrices_are_exact() {
        for (rotate, expected) in [
            (0.0, [1.0, 0.0, 0.0, 1.0]),
            (90.0, [0.0, -1.0, 1.0, 0.0]),
            (180.0, [-1.0, 0.0, 0.0, -1.0]),
            (270.0, [0.0, 1.0, -1.0, 0.0]),
            (-90.0, [0.0, 1.0, -1.0, 0.0]),
            (450.0, [0.0, -1.0, 1.0, 0.0]),
        ] {
            let (text, _) = modern_text(
                (0.0, 0.0),
                markup::parse("ABC"),
                "start",
                "baseline",
                rotate,
                &Style::new(),
                Orient::R0,
                false,
                false,
                14.0,
            );
            let m = text.transform.unwrap();
            assert_eq!([m[0], m[1], m[2], m[3]], expected, "rotate={rotate}");
        }
    }

    #[test]
    fn exact_270_reads_as_90() {
        let (text, _) = modern_text(
            (0.0, 0.0),
            markup::parse("ABC"),
            "start",
            "baseline",
            270.0,
            &Style::new(),
            Orient::R0,
            false,
            true,
            14.0,
        );
        let m = text.transform.unwrap();
        assert!(m[0].abs() < 1e-14 && (m[2] - 1.0).abs() < 1e-14, "{m:?}");
    }

    #[test]
    fn upright_boundaries_use_the_exact_angle_for_all_owner_orientations() {
        for owner in Orient::ALL {
            let (a, b, c, d) = owner.matrix();
            let base = match (a, c) {
                (1, 0) => 0.0,
                (0, 1) => 90.0,
                (-1, 0) => 180.0,
                (0, -1) => 270.0,
                _ => unreachable!(),
            };
            let det = f64::from(a * d - b * c);
            for fix in [false, true] {
                for target in [90.0 - 1e-8, 90.0, 90.0 + 1e-8, 270.0 - 1e-8, 270.0, 270.0 + 1e-8] {
                    let rotate = if fix { target } else { det * (target - base) };
                    let rad = rotate.to_radians();
                    let raw = (libm::cos(rad), libm::sin(rad));
                    let raw = if fix { raw } else { owner.apply(raw) };
                    let (text, _) = modern_text(
                        (0.0, 0.0),
                        markup::parse("ABC"),
                        "start",
                        "baseline",
                        rotate,
                        &Style::new(),
                        owner,
                        fix,
                        true,
                        14.0,
                    );
                    let m = text.transform.unwrap();
                    let dot = m[0] * raw.0 + m[2] * raw.1;
                    let flip = target > 90.0 && target <= 270.0;
                    assert!(
                        (dot - if flip { -1.0 } else { 1.0 }).abs() < 1e-12,
                        "owner={owner:?} fix={fix} target={target} rotate={rotate} dot={dot}"
                    );
                }
            }
        }
    }

    #[test]
    fn modern_text_affine_has_required_handedness_and_contains_all_corners() {
        let style = Style::new();
        for angle in [0.0, 30.0, 90.0, 180.0, 270.0, -45.0] {
            for owner in Orient::ALL {
                for fix in [false, true] {
                    for upright in [false, true] {
                        let lines = markup::parse("ABC");
                        let (t, bounds) = modern_text(
                            (1.0, 2.0),
                            lines,
                            "start",
                            "baseline",
                            angle,
                            &style,
                            owner,
                            fix,
                            upright,
                            14.0,
                        );
                        let m = t.transform.unwrap();
                        let det = m[0] * m[3] - m[1] * m[2];
                        assert!((det.abs() - 1.0).abs() < 1e-9);
                        if upright || fix {
                            assert!(det > 0.0);
                        }
                        if upright {
                            let direction = libm::atan2(m[2], m[0]).to_degrees().rem_euclid(360.0);
                            assert!(
                                direction <= 90.0 + 1e-9 || direction > 270.0 - 1e-9,
                                "{angle} {owner:?} {direction}"
                            );
                        }
                        assert!(bounds[0] < bounds[2] && bounds[1] < bounds[3]);
                    }
                }
            }
        }
    }

    #[test]
    fn bezier_arrows_skip_coincident_control_points() {
        for points in [
            [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)],
            [(0.0, 0.0), (0.0, 0.0), (3.0, 0.0), (3.0, 0.0)],
            [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (3.0, 0.0)],
            [(0.0, 0.0), (3.0, 0.0), (3.0, 0.0), (3.0, 0.0)],
        ] {
            let mut out = Out { prims: Vec::new(), scale: 14.0 };
            let params = PrimParams { arrow: Some("both".into()), ..PrimParams::default() };
            out.shape("curve", Shape::Bezier, &points, &params, &Style::default(), Layer::Middle, vec![]);
            assert_eq!(out.prims.len(), 3, "{points:?}");
            // Both endpoint triangles retain their full length and width and point away from the curve.
            for (arrow, tip, bounds) in [
                (&out.prims[1], (0.0, 0.0), [0.0, -0.21, 0.6, 0.21]),
                (&out.prims[2], (3.0, 0.0), [2.4, -0.21, 3.0, 0.21]),
            ] {
                let Kind::Path { segs, strokes, fill } = &arrow.kind else { panic!("arrow must be a path") };
                assert_eq!(segs[0], PathSeg::Move(tip));
                assert!(strokes.is_empty());
                assert!(fill.is_some());
                for (actual, expected) in arrow.bbox.into_iter().zip(bounds) {
                    assert!((actual - expected).abs() < 1e-12, "{points:?}: {:?}", arrow.bbox);
                }
            }
        }
    }
}

#[cfg(test)]
mod annotation_affine_tests {
    use super::*;

    #[test]
    fn orthogonal_wrapper_and_general_owner_share_one_text_layout() {
        for orientation in
            [Orient::R0, Orient::R90, Orient::R180, Orient::R270, Orient::MX, Orient::MY, Orient::MXR90, Orient::MYR90]
        {
            let (a, b, c, d) = orientation.matrix();
            for fixed in [false, true] {
                for upright in [false, true] {
                    for rotate in [0.0, 30.0, 90.0, 270.0] {
                        let lines = markup::parse("\u{03b1}_{x}");
                        let direct = modern_text(
                            (4.0, 5.0),
                            lines.clone(),
                            "start",
                            "baseline",
                            rotate,
                            &Style::new(),
                            orientation,
                            fixed,
                            upright,
                            14.0,
                        );
                        let affine = modern_text_affine(
                            (4.0, 5.0),
                            lines,
                            "start",
                            "baseline",
                            rotate,
                            &Style::new(),
                            [f64::from(a), f64::from(b), f64::from(c), f64::from(d)],
                            fixed,
                            upright,
                            14.0,
                        );
                        assert_eq!(direct, affine, "{orientation:?} {fixed} {upright} {rotate}");
                    }
                }
            }
        }
    }

    #[test]
    fn arbitrary_mirrored_owner_keeps_a_finite_text_bounds() {
        let theta = 30.0_f64.to_radians();
        let c = libm::cos(theta);
        let s = libm::sin(theta);
        // Mirror x first, then rotate by thirty degrees.
        let (t, b) = modern_text_affine(
            (4.0, 5.0),
            markup::parse("hello"),
            "start",
            "baseline",
            15.0,
            &Style::new(),
            [-c, -s, -s, c],
            false,
            false,
            14.0,
        );
        assert!(t.transform.is_some());
        assert!(b.into_iter().all(f64::is_finite));
        assert!(b[0] < b[2] && b[1] < b[3]);
    }
}

#[cfg(test)]
mod annotation_angle_periodicity {
    use super::*;
    #[test]
    fn enormous_angle_matches_its_normalized_rotation() {
        let angle = 1.0e20_f64;
        let args = |a| {
            modern_text_affine(
                (1.0, 2.0),
                markup::parse("abc"),
                "start",
                "baseline",
                a,
                &Style::new(),
                [1.0, 0.0, 0.0, 1.0],
                false,
                true,
                14.0,
            )
        };
        assert_eq!(args(angle), args(angle.rem_euclid(360.0)));
    }
}

#[cfg(test)]
mod annotation_bounds_regressions {
    use super::*;
    use rnl_runtime::model::{AnnotationGraphic, AnnotationView, SymbolPrim};
    fn arc() -> AnnotationView {
        AnnotationView {
            object_id: "%a".into(),
            source_kind: "symbol",
            source: "$S".into(),
            anchor_kind: "independent",
            anchor_object: None,
            anchor: (0.0, 0.0),
            rotate: 90.0,
            mirror: true,
            style: Style::new(),
            own_style: Style::new(),
            layer: Layer::Middle,
            order: vec![],
            graphics: vec![AnnotationGraphic {
                primitive: SymbolPrim {
                    order: 0,
                    text: None,
                    text_name_formats: true,
                    modern_text: true,
                    shape: Shape::Arc,
                    points: vec![(1.0, 0.0), (-1.0, 0.0), (0.0, -1.0)],
                    params: PrimParams::default(),
                    style: Style::new(),
                    halign: None,
                    valign: None,
                    rotate: None,
                    fix_rotate: false,
                    upright: None,
                },
                lines: vec![],
                text_refs: vec![],
            }],
        }
    }
    #[test]
    fn mirrored_rotated_three_quarter_arc_contains_cardinal_extrema() {
        let b = annotation_bounds(&arc(), 14.0).unwrap().unwrap();
        assert!(b[0] <= -1.0 && b[1] <= -1.0 && b[2] >= 1.0 && b[3] >= 1.0, "{b:?}");
    }
    #[test]
    fn coordinate_text_is_remeasured_at_the_candidate_anchor() {
        use rnl_runtime::model::AnnotationTextRef;
        let mut a = arc();
        let g = &mut a.graphics[0];
        g.primitive.shape = Shape::Text;
        g.primitive.points = vec![(0.0, 0.0)];
        g.primitive.text = Some("x=${x}".into());
        g.text_refs = vec![AnnotationTextRef::X];
        a.anchor = (1.0, 0.0);
        let first = annotation_bounds(&a, 14.0).unwrap().unwrap();
        a.anchor = (100000.0, 0.0);
        let second = annotation_bounds(&a, 14.0).unwrap().unwrap();
        assert!(second[2] - second[0] > first[2] - first[0], "{first:?} {second:?}");
    }
    #[test]
    fn hidden_text_has_no_ink_or_bounds() {
        let mut a = arc();
        let p = &mut a.graphics[0].primitive;
        p.shape = Shape::Text;
        p.points = vec![(f64::NAN, 0.0)];
        p.text = Some("hidden".into());
        p.style.insert("textvisible".into(), Lit::Bool(false));
        assert_eq!(annotation_bounds(&a, 14.0), Ok(None));
    }
    #[test]
    fn finite_extreme_circle_remains_finite_when_rotated() {
        let mut a = arc();
        a.rotate = 45.0;
        a.mirror = false;
        let p = &mut a.graphics[0].primitive;
        p.shape = Shape::Circle;
        p.points = vec![(0.0, 0.0)];
        p.params.radius = Some(f64::MAX);
        let b = annotation_bounds(&a, 14.0).unwrap().unwrap();
        assert_eq!(b, [-f64::MAX, -f64::MAX, f64::MAX, f64::MAX]);
    }
    #[test]
    fn finite_extreme_semicircle_does_not_overflow_center_calculation() {
        let mut a = arc();
        a.rotate = 0.0;
        a.mirror = false;
        a.graphics[0].primitive.points = vec![(f64::MAX, 0.0), (0.0, f64::MAX), (-f64::MAX, 0.0)];
        let b = annotation_bounds(&a, 14.0).unwrap().unwrap();
        assert_eq!(b[0], -f64::MAX);
        assert_eq!(b[2], f64::MAX);
        assert_eq!(b[3], f64::MAX);
        a.rotate = 45.0;
        let rotated = annotation_bounds(&a, 14.0).unwrap().unwrap();
        assert!(rotated.iter().all(|x| x.is_finite()));
    }
    #[test]
    fn finite_extreme_straight_line_is_not_lost_to_cost_overflow() {
        let mut a = arc();
        a.rotate = 0.0;
        a.mirror = false;
        let p = &mut a.graphics[0].primitive;
        p.shape = Shape::Line;
        p.points = vec![(-f64::MAX, 0.0), (f64::MAX, 0.0)];
        let b = annotation_bounds(&a, 14.0).unwrap().unwrap();
        assert_eq!(b[0], -f64::MAX);
        assert_eq!(b[2], f64::MAX);
    }
    #[test]
    fn nonfinite_geometry_is_an_error_and_empty_is_none() {
        let mut a = arc();
        a.graphics.clear();
        assert_eq!(annotation_bounds(&a, 14.0), Ok(None));
        a.anchor.0 = f64::INFINITY;
        assert_eq!(annotation_bounds(&a, 14.0), Err(AnnotationBoundsError::NonFinite));
    }
}

#[cfg(test)]
mod annotation_upright_boundary_regressions {
    use super::*;

    fn glyph_flipped(owner_angle: f64, mirror: bool, text_angle: f64) -> bool {
        let angle = owner_angle.rem_euclid(360.0).to_radians();
        let (s, c) = (libm::sin(angle), libm::cos(angle));
        let flip = if mirror { -1.0 } else { 1.0 };
        let matrix = [c * flip, -s, s * flip, c];
        let text_rad = text_angle.rem_euclid(360.0).to_radians();
        let (ts, tc) = (libm::sin(text_rad), libm::cos(text_rad));
        let raw = (matrix[0] * tc + matrix[1] * ts, matrix[2] * tc + matrix[3] * ts);
        let (text, _) = modern_text_affine_inner(
            (0.0, 0.0),
            markup::parse("A"),
            "start",
            "baseline",
            text_angle,
            &Style::new(),
            matrix,
            Some((owner_angle, mirror)),
            false,
            true,
            14.0,
        );
        let m = text.transform.unwrap();
        m[0] * raw.0 + m[2] * raw.1 < 0.0
    }

    #[test]
    fn annotation_render_path_passes_written_owner_metadata() {
        use rnl_runtime::model::{AnnotationGraphic, AnnotationView, SymbolPrim};
        let a = AnnotationView {
            object_id: "%a".into(),
            source_kind: "symbol",
            source: "$S".into(),
            anchor_kind: "independent",
            anchor_object: None,
            anchor: (0.0, 0.0),
            rotate: 271.3,
            mirror: true,
            style: Style::new(),
            own_style: Style::new(),
            layer: Layer::Middle,
            order: vec![],
            graphics: vec![AnnotationGraphic {
                primitive: SymbolPrim {
                    order: 0,
                    text: Some("A".into()),
                    text_name_formats: true,
                    modern_text: true,
                    shape: Shape::Text,
                    points: vec![(0.0, 0.0)],
                    params: PrimParams::default(),
                    style: Style::new(),
                    halign: None,
                    valign: None,
                    rotate: Some(361.3),
                    fix_rotate: false,
                    upright: Some(true),
                },
                lines: markup::parse("A"),
                text_refs: vec![],
            }],
        };
        let mut out = Out { prims: vec![], scale: 14.0 };
        out.annotation(&a);
        let Kind::Text(text) = &out.prims[0].kind else { panic!("expected text") };
        assert!(text.transform.unwrap()[2] > 0.0, "{:?}", text.transform);
        assert!(annotation_bounds(&a, 14.0).unwrap().is_some());
    }

    #[test]
    fn mirrored_decimal_exact_boundary_uses_written_angles() {
        assert!(!glyph_flipped(271.3, true, 361.3));
        // These normalized f64 values sum just above 270, despite a raw-sum roundoff to 270.
        assert!(!glyph_flipped(-181.1, true, -271.1));
    }

    #[test]
    fn decimal_and_negative_angles_keep_their_low_order_side() {
        assert!(glyph_flipped(0.1, false, 89.9));
        assert!(glyph_flipped(7.3, false, 82.7));
        assert!(!glyph_flipped(-30.1, false, 120.1));
        assert!(!glyph_flipped(13.7, true, 283.7));
        assert!(glyph_flipped(30.1, true, 120.1));
    }

    #[test]
    fn strict_boundary_and_adjacent_floats_are_distinct() {
        let next_up = |x: f64| f64::from_bits(x.to_bits() + 1);
        let next_down = |x: f64| f64::from_bits(x.to_bits() - 1);
        for (owner, text, mirror, expected) in [
            (12.5, 77.5, false, false),
            (12.5, next_up(77.5), false, true),
            (12.5, next_down(77.5), false, false),
            (12.5, 257.5, false, true),
            (12.5, next_up(257.5), false, false),
            (12.5, next_down(257.5), false, true),
            (12.5, 102.5, true, false),
            (12.5, next_down(102.5), true, true),
            (12.5, next_up(102.5), true, false),
            (12.5, 282.5, true, true),
            (12.5, next_down(282.5), true, false),
            (12.5, next_up(282.5), true, true),
            (0.0, 270.0, true, true),
            (0.0, next_up(270.0), true, true),
            (0.0, next_down(270.0), true, false),
            (300.0, 150.0, false, false),
            (300.0, next_up(150.0), false, true),
            (300.0, next_down(150.0), false, false),
        ] {
            assert_eq!(glyph_flipped(owner, mirror, text), expected, "owner={owner} text={text:?} mirror={mirror}");
        }
    }

    #[test]
    fn large_owner_is_normalized_before_small_text_angle() {
        assert!(glyph_flipped(1e20, false, 180.0));
        assert!(!glyph_flipped(1e20, true, 180.0));
        assert!(glyph_flipped(1e20, true, 1.0));
        for mirror in [false, true] {
            for text in [1.0, 180.0, 270.0] {
                assert_eq!(glyph_flipped(1e20, mirror, text), glyph_flipped(1e20_f64.rem_euclid(360.0), mirror, text));
            }
        }
    }

    #[test]
    fn all_eight_orientations_keep_existing_text_behavior() {
        for orientation in
            [Orient::R0, Orient::R90, Orient::R180, Orient::R270, Orient::MX, Orient::MY, Orient::MXR90, Orient::MYR90]
        {
            let (a, b, c, d) = orientation.matrix();
            let matrix = [f64::from(a), f64::from(b), f64::from(c), f64::from(d)];
            let metadata = [
                (0.0, false, [1., 0., 0., 1.]),
                (90.0, false, [0., -1., 1., 0.]),
                (180.0, false, [-1., 0., 0., -1.]),
                (270.0, false, [0., 1., -1., 0.]),
                (0.0, true, [-1., 0., 0., 1.]),
                (90.0, true, [0., -1., -1., 0.]),
                (180.0, true, [1., 0., 0., -1.]),
                (270.0, true, [0., 1., 1., 0.]),
            ]
            .into_iter()
            .find(|(_, _, m)| *m == matrix)
            .unwrap();
            for text_angle in [0.0, 45.0, 90.0, 180.0, 270.0, 361.3] {
                for fixed in [false, true] {
                    let input = markup::parse("A");
                    let prior = modern_text(
                        (0., 0.),
                        input.clone(),
                        "start",
                        "baseline",
                        text_angle,
                        &Style::new(),
                        orientation,
                        fixed,
                        true,
                        14.,
                    );
                    let actual = modern_text_affine_inner(
                        (0., 0.),
                        input,
                        "start",
                        "baseline",
                        text_angle,
                        &Style::new(),
                        matrix,
                        Some((metadata.0, metadata.1)),
                        fixed,
                        true,
                        14.,
                    );
                    assert_eq!(actual, prior, "{orientation:?} {text_angle} {fixed}");
                }
            }
        }
    }
}
