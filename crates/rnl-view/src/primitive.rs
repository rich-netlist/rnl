//! A level's drawing as primitives (decision 0006): the shapes and texts a frontend draws, in
//! serializable types of the view's own. They are converted from the renderer's (architecture
//! 4.3), which stay free to change; the field names are the contract with the frontends.
//!
//! Geometry is in scene units, x to the right and y up, as in the model. The SVG and the hits are
//! in pixels with y down: a point (x, y) is at (x·scale, −y·scale) there (`Level::scale`).
//!
//! The SVG writer still decides how a line style is drawn and where the pieces of a text go. The
//! types say how, in pixels of the SVG, so that a frontend can draw as the SVG does; a test holds
//! the SVG writer to what they say.

use rnl_runtime::geom::Pt;
use rnl_runtime::markup;
use rnl_runtime::model;
use serde::Serialize;

/// One thing drawn on a level: a shape or a text, with the object it draws and its place in the
/// drawing order (11.6). A level lists its primitives in drawing order already.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Primitive {
    /// The `data-id` of the SVG group that draws it; empty for a shape written in the scene and
    /// for a flyline. A symbol's drawing, its pin names included, has its item, such as "item-3",
    /// and a wire's drawing its wire, such as "wire-0", as `Hit::id` names them. Any other text
    /// has the name the diagnostics give it, such as "#R1::name" for an instance's name or
    /// "a text" for a text written in the scene: several texts can share it, and no hit has it.
    pub owner: String,
    pub layer: Layer,
    /// Its place within the layer, compared element by element (11.6).
    pub order: Vec<f64>,
    /// x0, y0, x1, y1: what it covers, in scene units; for a text, an estimate.
    pub bbox: [f64; 4],
    #[serde(flatten)]
    pub shape: Shape,
}

/// The layers, drawn bottom first (11.6).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Bottom,
    Middle,
    Top,
}

/// What a primitive draws, by its `kind`, with points as x and y in scene units, y up: "path",
/// lines, arcs and curves (9.2.5), and arrow heads, which are filled triangles; "rect", with its
/// sides along the axes, by its lowest and highest corners; "circle"; "ellipse", with its axes
/// along x and y; "text"; "flyline", a line a viewer may show for a connection that is not drawn
/// yet (13.7), not a wire, by its two ends. The SVG draws a flyline 1 pixel wide in #8a8a8a, with
/// dashes 4 pixels long and gaps of 3.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Path { segments: Vec<Segment>, stroke: Option<Stroke>, fill: Option<Color> },
    Rect { min: [f64; 2], max: [f64; 2], stroke: Option<Stroke>, fill: Option<Color> },
    Circle { center: [f64; 2], r: f64, stroke: Option<Stroke>, fill: Option<Color> },
    Ellipse { center: [f64; 2], rx: f64, ry: f64, stroke: Option<Stroke>, fill: Option<Color> },
    Text(Text),
    Flyline { points: [[f64; 2]; 2] },
}

/// A colour as the language writes it (9.3), which is as CSS does: `#` and 3, 4, 6 or 8
/// hexadecimal digits of either case, for red, green and blue and then, with 4 or 8, the opacity.
/// So "#1f4fd81a" is a faint blue; a reader that takes the opacity first, as Qt's and Android's
/// do, has to move it.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Color(pub String);

/// A piece of a path, by its `kind`: "move" starts a new part of the path at `to`, as every path
/// starts; "line" goes straight to `to`; "arc" is a circular arc of radius `r` to `to`, the longer
/// way round when `large`, counter-clockwise (y up) when `ccw`; "cubic" is a cubic Bézier curve
/// to `to` with the control points `c1` and `c2`; "close" goes back to where the part started.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Segment {
    Move { to: [f64; 2] },
    Line { to: [f64; 2] },
    Arc { r: f64, large: bool, ccw: bool, to: [f64; 2] },
    Cubic { c1: [f64; 2], c2: [f64; 2], to: [f64; 2] },
    Close,
}

/// How a line is drawn, after the style queue and the renderer's defaults (9.3).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Stroke {
    pub color: Color,
    /// The width in scene units; 0 is the hairline, the thin line that keeps its width at every
    /// zoom (9.3), which the SVG draws 1.2 pixels wide.
    pub width: f64,
    pub style: LineStyle,
    pub cap: Cap,
    pub join: Join,
    /// For a "double" line, the colour between its two lines; null for the other styles.
    pub inner: Option<Color>,
}

/// A line's style (9.3). The SVG draws it with w, the width in its pixels (1.2 for the hairline),
/// and m, the larger of w and 1: "solid" is a line w wide; "dashed" has dashes 4m long and gaps of
/// 3m; "dotted", dots m long and gaps of 2.5m; "dash-dot", a dash of 5m, a gap of 2m, a dot of m
/// and a gap of 2m; "bold" is a solid line 2w wide. On a path, "double" is a line in the colour 3d
/// wide under a line in the inner colour d wide, where d is the width in scene units, at least
/// 0.12, times the scale, with butt caps and miter joins, whatever the stroke's cap and join; a
/// rectangle, circle or ellipse draws "double" as "solid".
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
    DashDot,
    Double,
    Bold,
}

/// The ends of an open line and of its dashes (9.3).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Cap {
    Butt,
    Round,
    Square,
}

/// The corners of a line (9.3).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Join {
    Miter,
    Round,
    Bevel,
}

/// A text laid out at an anchor (9.2.5, 13.6). Its glyphs read upright whatever turns it, or from
/// bottom to top when it is vertical.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Text {
    pub anchor: [f64; 2],
    /// The lines, first to last, each a list of runs.
    pub lines: Vec<Vec<Run>>,
    /// The text height in scene units; the lines are 1.15 times it apart.
    pub size: f64,
    pub color: Color,
    /// The font family the style names, a hint (9.3); the SVG falls back on sans-serif.
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    /// Where the anchor is along the lines, after turning (9.2.5).
    pub halign: HAlign,
    /// Where the anchor is across the lines, after turning.
    pub valign: VAlign,
    /// Turned a quarter counter-clockwise: the lines read from bottom to top.
    pub vertical: bool,
}

/// Where a text's anchor is along its lines: at their start, middle or end.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum HAlign {
    Start,
    Middle,
    End,
}

/// Where a text's anchor is across its lines: on the first line's baseline, or at the top, middle
/// or bottom of the lines. Until texts are laid out with the fonts' metrics, the SVG places them
/// by the size: at the top, the first baseline is 0.8 times the size below the anchor; at the
/// bottom, the last baseline is 0.2 times the size above it; in the middle, the point halfway
/// between the first and last baselines is 0.35 times the size below it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum VAlign {
    Baseline,
    Top,
    Middle,
    Bottom,
}

/// A piece of a line of text in one style (13.6), by its `kind`: "text", or "frac" for
/// `\frac{…}{…}`, the numerator over the denominator. The SVG places the pieces of a line that
/// holds a fraction by estimated widths, with the numerator and denominator at 0.8 of the size.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Run {
    Text { text: String, style: RunStyle },
    Frac { num: Vec<Run>, den: Vec<Run>, style: RunStyle },
}

/// The style the markup gives a run (13.6).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct RunStyle {
    /// How far the run is raised: each `^` it is in adds 1 and each `_` takes 1 away, so the z of
    /// `x^{y^{z}}` is at 2. The SVG draws a run at any level but 0 at 0.7 of the size, raised once
    /// when it is above 0 and lowered once when below.
    pub level: i8,
    pub bold: bool,
    pub italic: bool,
    pub overline: bool,
}

/// The renderer's primitives of a level in the view's types.
pub(crate) fn primitives(prims: &[rnl_render::Primitive]) -> Vec<Primitive> {
    prims.iter().map(primitive).collect()
}

fn primitive(p: &rnl_render::Primitive) -> Primitive {
    use rnl_render::Kind;
    let stroke = |s: &Option<rnl_render::Stroke>| s.as_ref().map(self::stroke);
    let color = |c: &Option<String>| c.clone().map(Color);
    let shape = match &p.kind {
        Kind::Path { segs, stroke: s, fill } => {
            Shape::Path { segments: segs.iter().map(segment).collect(), stroke: stroke(s), fill: color(fill) }
        }
        Kind::Rect { min, max, stroke: s, fill } => {
            Shape::Rect { min: pt(*min), max: pt(*max), stroke: stroke(s), fill: color(fill) }
        }
        Kind::Circle { c, r, stroke: s, fill } => {
            Shape::Circle { center: pt(*c), r: *r, stroke: stroke(s), fill: color(fill) }
        }
        Kind::Ellipse { c, rx, ry, stroke: s, fill } => {
            Shape::Ellipse { center: pt(*c), rx: *rx, ry: *ry, stroke: stroke(s), fill: color(fill) }
        }
        Kind::Text(t) => Shape::Text(text(t)),
        Kind::Flyline { a, b } => Shape::Flyline { points: [pt(*a), pt(*b)] },
    };
    Primitive { owner: p.owner.clone(), layer: layer(p.layer), order: p.order.clone(), bbox: p.bbox, shape }
}

fn pt(p: Pt) -> [f64; 2] {
    [p.0, p.1]
}

fn layer(l: model::Layer) -> Layer {
    match l {
        model::Layer::Bottom => Layer::Bottom,
        model::Layer::Middle => Layer::Middle,
        model::Layer::Top => Layer::Top,
    }
}

fn segment(s: &rnl_render::PathSeg) -> Segment {
    use rnl_render::PathSeg;
    match s {
        PathSeg::Move(p) => Segment::Move { to: pt(*p) },
        PathSeg::Line(p) => Segment::Line { to: pt(*p) },
        PathSeg::Arc { r, large, ccw, to } => Segment::Arc { r: *r, large: *large, ccw: *ccw, to: pt(*to) },
        PathSeg::Cubic(a, b, c) => Segment::Cubic { c1: pt(*a), c2: pt(*b), to: pt(*c) },
        PathSeg::Close => Segment::Close,
    }
}

/// A stroke in the view's types. The language allows no other styles, caps and joins than those
/// of the enums (9.3); the SVG draws an unknown style as solid.
fn stroke(s: &rnl_render::Stroke) -> Stroke {
    let style = match s.style.as_str() {
        "dashed" => LineStyle::Dashed,
        "dotted" => LineStyle::Dotted,
        "dash-dot" => LineStyle::DashDot,
        "double" => LineStyle::Double,
        "bold" => LineStyle::Bold,
        _ => LineStyle::Solid,
    };
    let cap = match s.cap.as_str() {
        "butt" => Cap::Butt,
        "square" => Cap::Square,
        _ => Cap::Round,
    };
    let join = match s.join.as_str() {
        "miter" => Join::Miter,
        "bevel" => Join::Bevel,
        _ => Join::Round,
    };
    let inner = (style == LineStyle::Double).then(|| Color(s.inner.clone()));
    Stroke { color: Color(s.color.clone()), width: s.width, style, cap, join, inner }
}

/// A text in the view's types. The SVG anchors an unknown alignment at the start along the lines
/// and in the middle across them.
fn text(t: &rnl_render::TextPrim) -> Text {
    let halign = match t.halign.as_str() {
        "middle" => HAlign::Middle,
        "end" => HAlign::End,
        _ => HAlign::Start,
    };
    let valign = match t.valign.as_str() {
        "baseline" => VAlign::Baseline,
        "top" => VAlign::Top,
        "bottom" => VAlign::Bottom,
        _ => VAlign::Middle,
    };
    Text {
        anchor: pt(t.anchor),
        lines: t.lines.iter().map(|l| l.iter().map(run).collect()).collect(),
        size: t.size,
        color: Color(t.color.clone()),
        font: t.font.clone(),
        bold: t.bold,
        italic: t.italic,
        halign,
        valign,
        vertical: t.vertical,
    }
}

fn run(r: &markup::Run) -> Run {
    match r {
        markup::Run::Text { text, style } => Run::Text { text: text.clone(), style: run_style(style) },
        markup::Run::Frac { num, den, style } => Run::Frac {
            num: num.iter().map(run).collect(),
            den: den.iter().map(run).collect(),
            style: run_style(style),
        },
    }
}

fn run_style(s: &markup::RunStyle) -> RunStyle {
    RunStyle { level: s.level, bold: s.bold, italic: s.italic, overline: s.overline }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::with_page_ids;
    use crate::tests::Disk;
    use crate::BuildOptions;
    use rnl_render::{Kind, PathSeg, RenderOptions};
    use rnl_runtime::words::{words, Literal, ParamType};
    use rnl_runtime::Options;
    use serde::Serialize;

    /// The netlists and package files of the conformance corpus.
    fn corpus() -> Vec<String> {
        let mut out = Vec::new();
        for dir in ["cases", "equivalence"] {
            let dir = crate::tests::corpus(dir);
            let mut files: Vec<String> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                .filter(|f| [".sp", ".scs", ".inc", ".rnl"].iter().any(|e| f.ends_with(e)))
                .map(|f| format!("{dir}/{f}"))
                .collect();
            files.sort();
            out.extend(files);
        }
        out
    }

    /// The name serde gives a value of the view's enums: the language's word for it.
    fn name(value: impl Serialize) -> String {
        serde_json::to_value(value).unwrap().as_str().unwrap().to_string()
    }

    /// A primitive back in the renderer's types; a stroke that is not "double" has no inner colour.
    fn back(p: &Primitive) -> rnl_render::Primitive {
        let pt = |p: [f64; 2]| (p[0], p[1]);
        let stroke = |s: &Option<Stroke>| {
            s.as_ref().map(|s| rnl_render::Stroke {
                color: s.color.0.clone(),
                width: s.width,
                style: name(s.style),
                cap: name(s.cap),
                join: name(s.join),
                inner: s.inner.clone().map(|c| c.0).unwrap_or_default(),
            })
        };
        let color = |c: &Option<Color>| c.clone().map(|c| c.0);
        let kind = match &p.shape {
            Shape::Path { segments, stroke: s, fill } => {
                let segs = segments
                    .iter()
                    .map(|s| match s {
                        Segment::Move { to } => PathSeg::Move(pt(*to)),
                        Segment::Line { to } => PathSeg::Line(pt(*to)),
                        Segment::Arc { r, large, ccw, to } => {
                            PathSeg::Arc { r: *r, large: *large, ccw: *ccw, to: pt(*to) }
                        }
                        Segment::Cubic { c1, c2, to } => PathSeg::Cubic(pt(*c1), pt(*c2), pt(*to)),
                        Segment::Close => PathSeg::Close,
                    })
                    .collect();
                Kind::Path { segs, stroke: stroke(s), fill: color(fill) }
            }
            Shape::Rect { min, max, stroke: s, fill } => {
                Kind::Rect { min: pt(*min), max: pt(*max), stroke: stroke(s), fill: color(fill) }
            }
            Shape::Circle { center, r, stroke: s, fill } => {
                Kind::Circle { c: pt(*center), r: *r, stroke: stroke(s), fill: color(fill) }
            }
            Shape::Ellipse { center, rx, ry, stroke: s, fill } => {
                Kind::Ellipse { c: pt(*center), rx: *rx, ry: *ry, stroke: stroke(s), fill: color(fill) }
            }
            Shape::Text(t) => Kind::Text(rnl_render::TextPrim {
                anchor: pt(t.anchor),
                lines: t.lines.iter().map(|l| l.iter().map(markup_run).collect()).collect(),
                size: t.size,
                color: t.color.0.clone(),
                font: t.font.clone(),
                bold: t.bold,
                italic: t.italic,
                halign: name(t.halign),
                valign: name(t.valign),
                vertical: t.vertical,
            }),
            Shape::Flyline { points } => Kind::Flyline { a: pt(points[0]), b: pt(points[1]) },
        };
        let layer = model::Layer::parse(&name(p.layer));
        rnl_render::Primitive { owner: p.owner.clone(), layer, order: p.order.clone(), kind, bbox: p.bbox }
    }

    fn markup_run(r: &Run) -> markup::Run {
        let style =
            |s: &RunStyle| markup::RunStyle { level: s.level, bold: s.bold, italic: s.italic, overline: s.overline };
        match r {
            Run::Text { text, style: s } => markup::Run::Text { text: text.clone(), style: style(s) },
            Run::Frac { num, den, style: s } => markup::Run::Frac {
                num: num.iter().map(markup_run).collect(),
                den: den.iter().map(markup_run).collect(),
                style: style(s),
            },
        }
    }

    /// A renderer's primitive without the inner colour of a stroke that is not "double", which
    /// nothing draws.
    fn drawn(p: &rnl_render::Primitive) -> rnl_render::Primitive {
        let mut p = p.clone();
        if let Kind::Path { stroke: Some(s), .. }
        | Kind::Rect { stroke: Some(s), .. }
        | Kind::Circle { stroke: Some(s), .. }
        | Kind::Ellipse { stroke: Some(s), .. } = &mut p.kind
        {
            if s.style != "double" {
                s.inner.clear();
            }
        }
        p
    }

    fn kind(p: &rnl_render::Primitive) -> &'static str {
        match p.kind {
            Kind::Path { .. } => "path",
            Kind::Rect { .. } => "rect",
            Kind::Circle { .. } => "circle",
            Kind::Ellipse { .. } => "ellipse",
            Kind::Text(_) => "text",
            Kind::Flyline { .. } => "flyline",
        }
    }

    /// The strings a style property or a TEXT parameter can be (9.3, 13.3).
    fn values(ty: &ParamType) -> Vec<String> {
        let ParamType::Enum(values) = ty else { panic!("{ty:?}") };
        values.iter().filter_map(|v| if let Literal::Str(s) = v { Some(s.clone()) } else { None }).collect()
    }

    #[test]
    fn every_line_style_and_alignment_of_the_language_is_kept() {
        let w = words();
        let style = |k: &str| values(&w.style_prop(k).unwrap().ty);
        let text = |k: &str| values(&w.keyword("TEXT").unwrap().param_list().iter().find(|p| p.name == k).unwrap().ty);
        let s = rnl_render::Stroke {
            color: "#1d1d1f".into(),
            width: 0.0,
            style: "solid".into(),
            cap: "round".into(),
            join: "round".into(),
            inner: "#ffffff".into(),
        };
        // "none" draws no line at all (9.3)
        for v in style("linestyle").into_iter().filter(|v| v != "none") {
            assert_eq!(name(stroke(&rnl_render::Stroke { style: v.clone(), ..s.clone() }).style), v);
        }
        for v in style("cap") {
            assert_eq!(name(stroke(&rnl_render::Stroke { cap: v.clone(), ..s.clone() }).cap), v);
        }
        for v in style("join") {
            assert_eq!(name(stroke(&rnl_render::Stroke { join: v.clone(), ..s.clone() }).join), v);
        }
        let double = stroke(&rnl_render::Stroke { style: "double".into(), ..s.clone() });
        assert_eq!((stroke(&s).inner, double.inner), (None, Some(Color("#ffffff".into()))));
        let t = rnl_render::TextPrim {
            anchor: (0.0, 0.0),
            lines: Vec::new(),
            size: 1.0,
            color: "#1d1d1f".into(),
            font: "sans-serif".into(),
            bold: false,
            italic: false,
            halign: "start".into(),
            valign: "baseline".into(),
            vertical: false,
        };
        for v in text("halign") {
            assert_eq!(name(super::text(&rnl_render::TextPrim { halign: v.clone(), ..t.clone() }).halign), v);
        }
        for v in text("valign") {
            assert_eq!(name(super::text(&rnl_render::TextPrim { valign: v.clone(), ..t.clone() }).valign), v);
        }
        for l in [model::Layer::Bottom, model::Layer::Middle, model::Layer::Top] {
            assert_eq!(name(layer(l)), l.as_str());
        }
    }

    #[test]
    fn every_primitive_of_the_corpus_reads_back_and_writes_the_svg() {
        let files = corpus();
        assert!(files.len() > 20);
        let options = BuildOptions { primitives: true, ..BuildOptions::default() };
        let mut count = 0;
        for f in files {
            let d = crate::build(&f, &Disk, &options).unwrap();
            let r = crate::interpret(&f, None, &Disk, &Options::default()).unwrap();
            assert_eq!(d.levels.len(), r.model.levels.len(), "{f}");
            for (level, lv) in d.levels.iter().zip(&r.model.levels) {
                let want = rnl_render::primitives(&with_page_ids(lv));
                let prims = level.primitives.as_ref().unwrap();
                // the same primitives, kind by kind
                let json = serde_json::to_value(prims).unwrap();
                let kinds: Vec<&str> = json.as_array().unwrap().iter().map(|p| p["kind"].as_str().unwrap()).collect();
                assert_eq!(kinds, want.iter().map(kind).collect::<Vec<_>>(), "{f}: {}", level.title);
                // the JSON reads back as it was, and is all the renderer needs
                let read: Vec<Primitive> = serde_json::from_str(&serde_json::to_string(prims).unwrap()).unwrap();
                assert!(read == *prims, "{f}: {}", level.title);
                let read: Vec<rnl_render::Primitive> = read.iter().map(back).collect();
                assert!(read == want.iter().map(drawn).collect::<Vec<_>>(), "{f}: {}", level.title);
                // the SVG written from them is the level's
                let svg = rnl_render::svg_document(&read, &RenderOptions::default());
                assert!(Some(svg) == level.svg, "{f}: {}", level.title);
                assert_eq!(level.scale, Some(RenderOptions::default().scale));
                count += prims.len();
            }
        }
        assert!(count > 1000, "{count}");
    }

    #[test]
    fn the_owners_and_the_colours_are_as_the_types_say() {
        let options = BuildOptions { primitives: true, ..BuildOptions::default() };
        let (mut named, mut shared, mut opacities) = (0, 0, 0);
        for f in corpus() {
            let d = crate::build(&f, &Disk, &options).unwrap();
            for level in &d.levels {
                let ids: Vec<&str> = level.hits.iter().filter_map(|h| h.id.as_deref()).collect();
                let prims = level.primitives.as_ref().unwrap();
                for p in prims {
                    let owner = p.owner.as_str();
                    let numbered =
                        |prefix: &str| owner.strip_prefix(prefix).is_some_and(|k| k.parse::<usize>().is_ok());
                    let text = matches!(p.shape, Shape::Text(_));
                    if owner.is_empty() || matches!(p.shape, Shape::Flyline { .. }) {
                        // a shape written in the scene, or a flyline
                        assert!(owner.is_empty() && !text, "{f}: {p:?}");
                    } else if numbered("item-") || numbered("wire-") {
                        assert!(ids.contains(&owner), "{f}: {owner}");
                    } else {
                        // any other text has a name of the diagnostics, which no hit has
                        assert!(text && !ids.contains(&owner), "{f}: {p:?}");
                        named += 1;
                        shared += usize::from(prims.iter().filter(|q| q.owner == owner).count() > 1);
                    }
                    let (stroke, fill, color) = match &p.shape {
                        Shape::Path { stroke, fill, .. }
                        | Shape::Rect { stroke, fill, .. }
                        | Shape::Circle { stroke, fill, .. }
                        | Shape::Ellipse { stroke, fill, .. } => (stroke.as_ref(), fill.as_ref(), None),
                        Shape::Text(t) => (None, None, Some(&t.color)),
                        Shape::Flyline { .. } => (None, None, None),
                    };
                    let strokes = stroke.into_iter().flat_map(|s| std::iter::once(&s.color).chain(&s.inner));
                    for Color(c) in strokes.chain(fill).chain(color) {
                        let digits = c.strip_prefix('#').unwrap_or_default();
                        let ok = [3, 4, 6, 8].contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_hexdigit());
                        assert!(ok, "{f}: {c}");
                        opacities += usize::from(digits.len() == 8);
                    }
                }
            }
        }
        assert!(named > 100 && shared > 0 && opacities > 0, "{named} {shared} {opacities}");
    }

    /// The SVG draws lines and places texts as the types' documentation says (`Stroke`,
    /// `LineStyle`, `Shape`, `Text`, `VAlign`, `Run`, `RunStyle`), so that a frontend can draw as
    /// it does. When the SVG writer changes, so does the documentation.
    #[test]
    fn the_svg_draws_as_the_types_say() {
        let o = RenderOptions::default();
        let svg = |kind: Kind| {
            let p = rnl_render::Primitive {
                owner: String::new(),
                layer: model::Layer::Middle,
                order: Vec::new(),
                kind,
                bbox: [0.0, 0.0, 1.0, 1.0],
            };
            rnl_render::svg_document(&[p], &o)
        };
        let stroke = |width: f64, style: &str| rnl_render::Stroke {
            color: "#102030".into(),
            width,
            style: style.into(),
            cap: "square".into(),
            join: "bevel".into(),
            inner: "#405060".into(),
        };
        let path: fn(rnl_render::Stroke) -> Kind = |s| Kind::Path {
            segs: vec![PathSeg::Move((0.0, 0.0)), PathSeg::Line((1.0, 0.0))],
            stroke: Some(s),
            fill: None,
        };
        let others: [fn(rnl_render::Stroke) -> Kind; 3] = [
            |s| Kind::Rect { min: (0.0, 0.0), max: (1.0, 1.0), stroke: Some(s), fill: None },
            |s| Kind::Circle { c: (0.0, 0.0), r: 1.0, stroke: Some(s), fill: None },
            |s| Kind::Ellipse { c: (0.0, 0.0), rx: 1.0, ry: 2.0, stroke: Some(s), fill: None },
        ];
        for width in [0.0, 0.05, 0.2] {
            // w, the width in pixels, and m, the larger of w and 1
            let w = if width > 0.0 { width * o.scale } else { 1.2 };
            let m = w.max(1.0);
            let line = |w: f64, dashes: &[f64]| {
                let mut a = format!(
                    "stroke=\"#102030\" stroke-width=\"{w:.2}\" stroke-linecap=\"square\" stroke-linejoin=\"bevel\""
                );
                if !dashes.is_empty() {
                    let d: Vec<String> = dashes.iter().map(|d| format!("{d:.1}")).collect();
                    a += &format!(" stroke-dasharray=\"{}\"", d.join(","));
                }
                a + "/>"
            };
            for (style, want) in [
                ("solid", line(w, &[])),
                ("bold", line(2.0 * w, &[])),
                ("dashed", line(w, &[4.0 * m, 3.0 * m])),
                ("dotted", line(w, &[m, 2.5 * m])),
                ("dash-dot", line(w, &[5.0 * m, 2.0 * m, m, 2.0 * m])),
            ] {
                for shape in std::iter::once(path).chain(others) {
                    let svg = svg(shape(stroke(width, style)));
                    assert!(svg.contains(&want), "{style} {width}: {svg}");
                }
            }
            // "double" on a path: the colour 3d wide under the inner colour d wide, with SVG's own
            // caps and joins, butt and miter
            let d = width.max(0.12) * o.scale;
            let double = svg(path(stroke(width, "double")));
            assert!(double.contains(&format!("stroke=\"#102030\" stroke-width=\"{:.2}\"/>", 3.0 * d)), "{double}");
            assert!(double.contains(&format!("stroke=\"#405060\" stroke-width=\"{d:.2}\"/>")), "{double}");
            assert!(!double.contains("stroke-linecap") && !double.contains("stroke-linejoin"), "{double}");
            // and as "solid" on the other shapes
            for shape in others {
                assert_eq!(svg(shape(stroke(width, "double"))), svg(shape(stroke(width, "solid"))));
            }
        }
        let flyline = svg(Kind::Flyline { a: (0.0, 0.0), b: (1.0, 0.0) });
        assert!(flyline.contains("stroke=\"#8a8a8a\" stroke-width=\"1\" stroke-dasharray=\"4,3\""), "{flyline}");

        // texts of size 1, with a font that the SVG falls back from
        let (fs, lh) = (o.scale, 1.15 * o.scale);
        let run = |text: &str, level: i8| markup::Run::Text {
            text: text.into(),
            style: markup::RunStyle { level, ..markup::RunStyle::default() },
        };
        let text = |lines: Vec<markup::Line>, valign: &str| {
            svg(Kind::Text(rnl_render::TextPrim {
                anchor: (0.0, 0.0),
                lines,
                size: 1.0,
                color: "#102030".into(),
                font: "Serif".into(),
                bold: false,
                italic: false,
                halign: "start".into(),
                valign: valign.into(),
                vertical: false,
            }))
        };
        // the first baseline of three lines, below the anchor
        let n = 3.0;
        for (valign, first) in [
            ("baseline", 0.0),
            ("top", 0.8 * fs),
            // the last baseline above it
            ("bottom", -0.2 * fs - (n - 1.0) * lh),
            // the point halfway between the first and the last baseline below it
            ("middle", 0.35 * fs - (n - 1.0) * lh / 2.0),
        ] {
            let svg = text(vec![vec![run("a", 0)]; 3], valign);
            let want = format!("<tspan x=\"0\" dy=\"{first:.2}\">a</tspan><tspan x=\"0\" dy=\"{lh:.2}\">a</tspan>");
            assert!(svg.contains(&want) && svg.contains("font-family=\"Serif, sans-serif\""), "{valign}: {svg}");
        }
        // raised and lowered once, whatever the level
        let svg = text(vec![vec![run("x", 0), run("y", 1), run("z", 2), run("i", -2)]], "baseline");
        let small = format!("font-size=\"{:.2}\"", 0.7 * fs);
        let want = format!(
            "x<tspan {small} baseline-shift=\"super\">y</tspan><tspan {small} baseline-shift=\"super\">z</tspan><tspan {small} baseline-shift=\"sub\">i</tspan>"
        );
        assert!(svg.contains(&want), "{svg}");
        // the numerator and the denominator of a fraction at 0.8 of the size
        let frac =
            markup::Run::Frac { num: vec![run("a", 0)], den: vec![run("b", 0)], style: markup::RunStyle::default() };
        let svg = text(vec![vec![frac]], "baseline");
        assert_eq!(svg.matches(&format!("font-size=\"{:.2}\"", 0.8 * fs)).count(), 2, "{svg}");
    }
}
