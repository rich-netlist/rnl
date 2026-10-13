//! A level's drawing as primitives (decision 0006): the shapes and texts a frontend draws, in
//! serializable types of the view's own. They are converted from the renderer's (architecture
//! 4.3), which stay free to change; the field names are the contract with the frontends.
//!
//! Geometry is in scene units, x to the right and y up, as in the model. The SVG and the hits are
//! in pixels with y down: a point (x, y) is at (x·scale, −y·scale) there (`Level::scale`).

use rnl_runtime::geom::Pt;
use rnl_runtime::markup;
use rnl_runtime::model;
use serde::Serialize;

/// One thing drawn on a level: a shape or a text, with the object it draws and its place in the
/// drawing order (11.6). A level lists its primitives in drawing order already.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Primitive {
    /// The object it draws, as `Hit::id` and the SVG's `data-id` name it, such as "item-3" or
    /// "wire-0"; empty for a shape written in the scene and for a flyline.
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
/// yet (13.7), not a wire, by its two ends. The SVG draws a flyline as a thin grey dashed line.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Path { segments: Vec<Segment>, stroke: Option<Stroke>, fill: Option<String> },
    Rect { min: [f64; 2], max: [f64; 2], stroke: Option<Stroke>, fill: Option<String> },
    Circle { center: [f64; 2], r: f64, stroke: Option<Stroke>, fill: Option<String> },
    Ellipse { center: [f64; 2], rx: f64, ry: f64, stroke: Option<Stroke>, fill: Option<String> },
    Text(Text),
    Flyline { points: [[f64; 2]; 2] },
}

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
    pub color: String,
    /// The width in scene units; 0 is the hairline, the thin line that keeps its width at every
    /// zoom (9.3).
    pub width: f64,
    pub style: LineStyle,
    pub cap: Cap,
    pub join: Join,
    /// For a "double" line, the colour between its two lines; null for the other styles.
    pub inner: Option<String>,
}

/// A line's style (9.3). Dashes, dots and the gaps between them grow with the width; "double" is
/// two lines in the colour with the inner colour between them, three times the width in all;
/// "bold" is twice the width.
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

/// The ends of an open line (9.3).
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
    pub color: String,
    /// The font family the style names, a hint (9.3).
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

/// Where a text's anchor is across its lines: on the first line's baseline, at the top, middle or
/// bottom.
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
/// `\frac{…}{…}`, the numerator over the denominator.
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
    /// 0 on the line, 1 raised, −1 lowered; nested levels add up.
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
    let shape = match &p.kind {
        Kind::Path { segs, stroke: s, fill } => {
            Shape::Path { segments: segs.iter().map(segment).collect(), stroke: stroke(s), fill: fill.clone() }
        }
        Kind::Rect { min, max, stroke: s, fill } => {
            Shape::Rect { min: pt(*min), max: pt(*max), stroke: stroke(s), fill: fill.clone() }
        }
        Kind::Circle { c, r, stroke: s, fill } => {
            Shape::Circle { center: pt(*c), r: *r, stroke: stroke(s), fill: fill.clone() }
        }
        Kind::Ellipse { c, rx, ry, stroke: s, fill } => {
            Shape::Ellipse { center: pt(*c), rx: *rx, ry: *ry, stroke: stroke(s), fill: fill.clone() }
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
    let inner = (style == LineStyle::Double).then(|| s.inner.clone());
    Stroke { color: s.color.clone(), width: s.width, style, cap, join, inner }
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
        color: t.color.clone(),
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
                color: s.color.clone(),
                width: s.width,
                style: name(s.style),
                cap: name(s.cap),
                join: name(s.join),
                inner: s.inner.clone().unwrap_or_default(),
            })
        };
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
                Kind::Path { segs, stroke: stroke(s), fill: fill.clone() }
            }
            Shape::Rect { min, max, stroke: s, fill } => {
                Kind::Rect { min: pt(*min), max: pt(*max), stroke: stroke(s), fill: fill.clone() }
            }
            Shape::Circle { center, r, stroke: s, fill } => {
                Kind::Circle { c: pt(*center), r: *r, stroke: stroke(s), fill: fill.clone() }
            }
            Shape::Ellipse { center, rx, ry, stroke: s, fill } => {
                Kind::Ellipse { c: pt(*center), rx: *rx, ry: *ry, stroke: stroke(s), fill: fill.clone() }
            }
            Shape::Text(t) => Kind::Text(rnl_render::TextPrim {
                anchor: pt(t.anchor),
                lines: t.lines.iter().map(|l| l.iter().map(markup_run).collect()).collect(),
                size: t.size,
                color: t.color.clone(),
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
        assert_eq!((stroke(&s).inner, double.inner), (None, Some("#ffffff".into())));
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
}
