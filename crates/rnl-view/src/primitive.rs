//! A level's drawing as primitives (decision 0006): the shapes and texts a frontend draws, in
//! serializable types of the view's own. They are converted from the renderer's (architecture
//! 4.3), which stay free to change; the field names are the contract with the frontends.
//!
//! Geometry is in scene units, x to the right and y up, as in the model. The SVG and the hits are
//! in pixels with y down: a point (x, y) is at (x·scale, −y·scale) there (`Level::scale`).
//!
//! The renderer resolves each line style into strokes, which the SVG writes as they are: a
//! frontend that strokes with a colour, a width, dashes, a cap and a join draws the lines as the
//! SVG does. It lays out each text with the metrics of the fonts shipped with the core (decision
//! 0007), and gives its runs at their places and its bars as strokes, which the SVG writes as they
//! are in those fonts: a frontend that draws them in the same fonts draws the texts as the SVG
//! does. A test holds the SVG writer to what the types say.

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
    /// x0, y0, x1, y1: what it covers, in scene units; for a text, each run from its face's
    /// ascender to its descender, leaning with it when italic, and the bars.
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
/// yet (13.7), not a wire, by its two ends.
///
/// A shape is filled first and then stroked along its outline with each of its `strokes`, first to
/// last; with no strokes it has no outline. The outline, and so its dashes, starts where a path or
/// a flyline starts; at a rectangle's corner of lowest x and highest y, along the side of highest
/// y first; and at the point of a circle or an ellipse of highest x. It goes round a rectangle,
/// circle or ellipse clockwise, as SVG and Canvas draw them on a screen with y down.
///
/// A shape of no size is drawn as SVG draws it, which Canvas 2D does not always do: a rectangle
/// with no width or no height, and a circle or an ellipse with a radius of 0, draw nothing, filled
/// or stroked; a part of a path that stays at one point, such as a wire from a point to itself,
/// draws with each stroke only its caps, a dot as wide as the stroke with round caps, a square with
/// square caps and nothing with butt caps.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Path { segments: Vec<Segment>, strokes: Vec<Stroke>, fill: Option<Color> },
    Rect { min: [f64; 2], max: [f64; 2], strokes: Vec<Stroke>, fill: Option<Color> },
    Circle { center: [f64; 2], r: f64, strokes: Vec<Stroke>, fill: Option<Color> },
    Ellipse { center: [f64; 2], rx: f64, ry: f64, strokes: Vec<Stroke>, fill: Option<Color> },
    Text(Text),
    Flyline { points: [[f64; 2]; 2], strokes: Vec<Stroke> },
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

/// One stroke along a shape's outline, a line style (9.3) as the renderer resolved it: a frontend
/// draws it with these and nothing else. A "solid", "dashed", "dotted" or "dash-dot" line is one
/// stroke, the last three with their dashes; a "bold" line, one stroke twice as wide; a "double"
/// line, two strokes with butt caps and miter joins, one in the line's colour three times as wide
/// as each of its two lines and then one in the colour between them as wide as the gap; "none",
/// no stroke.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Stroke {
    pub color: Color,
    /// The width in the stroke's `unit`, centred on the outline.
    pub width: f64,
    pub unit: Unit,
    /// The lengths of the dashes and of the gaps between them, in turn, in the stroke's `unit`,
    /// repeated along the outline; empty for an unbroken line.
    pub dashes: Vec<f64>,
    /// How far into the dashes the outline starts, in the stroke's `unit`: with 0, the first dash
    /// starts where the outline starts.
    pub dash_offset: f64,
    pub cap: Cap,
    pub join: Join,
}

/// The unit of a stroke's width and dashes: "scene", scene units, which zoom with the drawing;
/// "pixel", pixels of the screen, the same at every zoom. The hairline, a line 0 wide in the style
/// (9.3), is 1.2 pixels wide and a flyline 1 pixel. The SVG is the drawing at the level's `scale`,
/// where a pixel is one of the SVG's own; no dash or gap is shorter than a pixel there.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Scene,
    Pixel,
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

/// The corners of a line (9.3). A miter that would be more than 4 times as long as the line is wide
/// is drawn as a bevel, as SVG does; Canvas's limit is 10 unless it is set.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Join {
    Miter,
    Round,
    Bevel,
}

/// A text (9.2.5, 13.6) as the core laid it out, with the metrics of the fonts shipped with it,
/// Noto Sans SC Regular and Bold (decision 0007). A frontend draws its `runs` and its `bars` and
/// nothing else, so it needs no alignment, baseline or measuring of its own; the other fields say
/// what the text is and how it was placed. Its glyphs read upright whatever turns it, or from
/// bottom to top when it is vertical.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Text {
    pub anchor: [f64; 2],
    /// The lines, first to last, each a list of runs of the markup.
    pub lines: Vec<Vec<Run>>,
    /// The text height, the em of the font, in scene units.
    pub size: f64,
    pub color: Color,
    /// The font family the style names, a hint (9.3): texts are laid out and drawn in the shipped
    /// family.
    pub font: String,
    pub bold: bool,
    pub italic: bool,
    /// Where the anchor is along the lines, after turning (9.2.5).
    pub halign: HAlign,
    /// Where the anchor is across the lines, after turning.
    pub valign: VAlign,
    /// Turned a quarter counter-clockwise: the lines read from bottom to top, and each run is drawn
    /// turned a quarter counter-clockwise about its `at`.
    pub vertical: bool,
    /// The runs as laid out, drawn in the text's colour.
    pub runs: Vec<TextRun>,
    /// The bars of its fractions and the lines over its overlined runs.
    pub bars: Vec<Bar>,
}

/// A run of a laid-out text: `text` in one face and size, drawn from `at`, the start of its
/// baseline. Its glyphs follow each other by their advances in the face, with no kerning,
/// ligatures, spacing of punctuation or other shaping, so that the run is `width` long; in CSS,
/// `font-kerning: none`, `font-variant-ligatures: none`, `text-spacing-trim: space-all` and
/// `text-autospace: no-autospace`. Its spaces are drawn as they are, those at its ends too; in
/// SVG, with `xml:space="preserve"` on its own `text` element.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct TextRun {
    pub text: String,
    /// The start of its baseline, in scene units.
    pub at: [f64; 2],
    /// The font size, the em, in scene units.
    pub size: f64,
    /// The sum of its characters' advances in its face, in scene units. A character the fonts do
    /// not map is 1 em wide, as the box they draw for it.
    pub width: f64,
    /// In Noto Sans SC Bold, else Regular.
    pub bold: bool,
    /// Leaning, since the fonts have no italic face: a point of a glyph moves forward along the
    /// baseline by 0.25 times its height above it, about 14°. Its advances are the upright face's.
    pub italic: bool,
}

/// A straight line of a text, the bar of a fraction or the line over an overlined run (13.6),
/// between its two `points`, drawn with its stroke: in the text's colour, in scene units, with butt
/// caps.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Deserialize, ts_rs::TS))]
pub struct Bar {
    pub points: [[f64; 2]; 2],
    pub stroke: Stroke,
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

/// Where a text's anchor is across its lines: on the first line's baseline; at the top, the first
/// line's ascender, 1.16 times the size above its baseline; at the bottom, the last line's
/// descender, 0.288 times the size below its baseline; or in the middle, halfway between the two.
/// The lines are the fonts' line height apart, 1.448 times the size.
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
/// `\frac{…}{…}`, the numerator over the denominator. Laid out, the numerator and the denominator
/// are at 0.8 of the fraction's size and centred on it; the fraction is as wide as the wider of
/// them and 0.15 of the size. The middle of its bar is at the middle of the em box, 0.38 of the
/// size above the baseline (the em box, which the ideographs fill, reaches from 0.12 of a run's
/// size below its baseline to 0.88 above it), and the bar is 0.06 of the size short of each end.
/// The numerator sits as low, and the denominator as high, as keeps the em boxes of their runs,
/// and their overlines, 0.1 of the size from the middle of the bar: with no scripts, fractions or
/// overlines in them, their baselines are 0.576 of the size above the fraction's and 0.424 below
/// it. A fraction's bar and an overline are 0.06 of their size wide, but at least 0.8 pixels at
/// the level's `scale`; the middle of an overline is 0.94 of its run's size above the run's
/// baseline, 0.06 above the top of the em box.
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
    /// `x^{y^{z}}` is at 2. Laid out, each level is 0.7 of the size of the level next to it towards
    /// 0, with its baseline 0.35 of that size higher, or 0.15 of it lower; in a fraction, the levels
    /// count from the fraction's.
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
    let strokes = |s: &[rnl_render::Stroke]| s.iter().map(stroke).collect();
    let color = |c: &Option<String>| c.clone().map(Color);
    let shape = match &p.kind {
        Kind::Path { segs, strokes: s, fill } => {
            Shape::Path { segments: segs.iter().map(segment).collect(), strokes: strokes(s), fill: color(fill) }
        }
        Kind::Rect { min, max, strokes: s, fill } => {
            Shape::Rect { min: pt(*min), max: pt(*max), strokes: strokes(s), fill: color(fill) }
        }
        Kind::Circle { c, r, strokes: s, fill } => {
            Shape::Circle { center: pt(*c), r: *r, strokes: strokes(s), fill: color(fill) }
        }
        Kind::Ellipse { c, rx, ry, strokes: s, fill } => {
            Shape::Ellipse { center: pt(*c), rx: *rx, ry: *ry, strokes: strokes(s), fill: color(fill) }
        }
        Kind::Text(t) => Shape::Text(text(t)),
        Kind::Flyline { a, b, strokes: s } => Shape::Flyline { points: [pt(*a), pt(*b)], strokes: strokes(s) },
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

/// A stroke in the view's types. The language allows no other caps and joins than those of the
/// enums (9.3).
fn stroke(s: &rnl_render::Stroke) -> Stroke {
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
    Stroke {
        color: Color(s.color.clone()),
        width: s.width,
        unit: if s.pixels { Unit::Pixel } else { Unit::Scene },
        dashes: s.dashes.clone(),
        dash_offset: s.dash_offset,
        cap,
        join,
    }
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
        runs: t.runs.iter().map(text_run).collect(),
        bars: t.bars.iter().map(|b| Bar { points: [pt(b.a), pt(b.b)], stroke: stroke(&b.stroke) }).collect(),
    }
}

fn text_run(r: &rnl_render::TextRun) -> TextRun {
    TextRun { text: r.text.clone(), at: pt(r.at), size: r.size, width: r.width, bold: r.bold, italic: r.italic }
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

    /// A stroke back in the renderer's types.
    fn back_stroke(s: &Stroke) -> rnl_render::Stroke {
        rnl_render::Stroke {
            color: s.color.0.clone(),
            width: s.width,
            pixels: s.unit == Unit::Pixel,
            dashes: s.dashes.clone(),
            dash_offset: s.dash_offset,
            cap: name(s.cap),
            join: name(s.join),
        }
    }

    /// A primitive back in the renderer's types.
    fn back(p: &Primitive) -> rnl_render::Primitive {
        let pt = |p: [f64; 2]| (p[0], p[1]);
        let stroke = |s: &[Stroke]| s.iter().map(back_stroke).collect();
        let color = |c: &Option<Color>| c.clone().map(|c| c.0);
        let kind = match &p.shape {
            Shape::Path { segments, strokes: s, fill } => {
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
                Kind::Path { segs, strokes: stroke(s), fill: color(fill) }
            }
            Shape::Rect { min, max, strokes: s, fill } => {
                Kind::Rect { min: pt(*min), max: pt(*max), strokes: stroke(s), fill: color(fill) }
            }
            Shape::Circle { center, r, strokes: s, fill } => {
                Kind::Circle { c: pt(*center), r: *r, strokes: stroke(s), fill: color(fill) }
            }
            Shape::Ellipse { center, rx, ry, strokes: s, fill } => {
                Kind::Ellipse { c: pt(*center), rx: *rx, ry: *ry, strokes: stroke(s), fill: color(fill) }
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
                runs: t.runs.iter().map(back_run).collect(),
                bars: t
                    .bars
                    .iter()
                    .map(|b| rnl_render::TextBar {
                        a: pt(b.points[0]),
                        b: pt(b.points[1]),
                        stroke: back_stroke(&b.stroke),
                    })
                    .collect(),
            }),
            Shape::Flyline { points, strokes: s } => {
                Kind::Flyline { a: pt(points[0]), b: pt(points[1]), strokes: stroke(s) }
            }
        };
        let layer = model::Layer::parse(&name(p.layer));
        rnl_render::Primitive { owner: p.owner.clone(), layer, order: p.order.clone(), kind, bbox: p.bbox }
    }

    fn back_run(r: &TextRun) -> rnl_render::TextRun {
        let at = (r.at[0], r.at[1]);
        rnl_render::TextRun { text: r.text.clone(), at, size: r.size, width: r.width, bold: r.bold, italic: r.italic }
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
    fn every_cap_join_and_alignment_of_the_language_is_kept() {
        let w = words();
        let style = |k: &str| values(&w.style_prop(k).unwrap().ty);
        let text = |k: &str| values(&w.keyword("TEXT").unwrap().param_list().iter().find(|p| p.name == k).unwrap().ty);
        let s = rnl_render::Stroke {
            color: "#1d1d1f".into(),
            width: 1.2,
            pixels: true,
            dashes: vec![4.8, 3.6],
            dash_offset: 0.5,
            cap: "round".into(),
            join: "round".into(),
        };
        for v in style("cap") {
            assert_eq!(name(stroke(&rnl_render::Stroke { cap: v.clone(), ..s.clone() }).cap), v);
        }
        for v in style("join") {
            assert_eq!(name(stroke(&rnl_render::Stroke { join: v.clone(), ..s.clone() }).join), v);
        }
        // a stroke reads back as it was, in either unit
        for pixels in [true, false] {
            let s = rnl_render::Stroke { pixels, ..s.clone() };
            assert_eq!(back_stroke(&stroke(&s)), s);
        }
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
            runs: Vec::new(),
            bars: Vec::new(),
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
                let want = rnl_render::primitives(&with_page_ids(lv), &RenderOptions::default());
                let prims = level.primitives.as_ref().unwrap();
                // the same primitives, kind by kind
                let json = serde_json::to_value(prims).unwrap();
                let kinds: Vec<&str> = json.as_array().unwrap().iter().map(|p| p["kind"].as_str().unwrap()).collect();
                assert_eq!(kinds, want.iter().map(kind).collect::<Vec<_>>(), "{f}: {}", level.title);
                // the JSON reads back as it was, and is all the renderer needs
                let read: Vec<Primitive> = serde_json::from_str(&serde_json::to_string(prims).unwrap()).unwrap();
                assert!(read == *prims, "{f}: {}", level.title);
                let read: Vec<rnl_render::Primitive> = read.iter().map(back).collect();
                assert!(read == want, "{f}: {}", level.title);
                // the SVG written from them is the level's
                let svg = rnl_render::svg_document(&read, &RenderOptions::default());
                assert!(Some(svg) == level.svg, "{f}: {}", level.title);
                assert_eq!(level.scale, Some(RenderOptions::default().scale));
                count += prims.len();
            }
        }
        assert!(count > 1000, "{count}");
    }

    /// Corpus case 17 draws every line style on every shape kind. A "double" line is two strokes
    /// on each, in scene units with butt caps and miter joins: the colour three times as wide as
    /// the colour between the lines.
    #[test]
    fn a_double_line_is_two_strokes_on_every_shape() {
        let options = BuildOptions { primitives: true, ..BuildOptions::default() };
        let d = crate::build(&crate::tests::corpus("cases/17-line-styles.sp"), &Disk, &options).unwrap();
        let mut doubles = Vec::new();
        for p in d.levels[0].primitives.as_ref().unwrap() {
            let (kind, strokes) = match &p.shape {
                Shape::Path { strokes, .. } => ("path", strokes),
                Shape::Rect { strokes, .. } => ("rect", strokes),
                Shape::Circle { strokes, .. } => ("circle", strokes),
                Shape::Ellipse { strokes, .. } => ("ellipse", strokes),
                _ => continue,
            };
            if let [outer, inner] = &strokes[..] {
                assert_eq!(outer.width, 3.0 * inner.width, "{p:?}");
                for s in [outer, inner] {
                    let plain = s.unit == Unit::Scene && s.dashes.is_empty() && s.dash_offset == 0.0;
                    assert!(plain && s.cap == Cap::Butt && s.join == Join::Miter, "{p:?}");
                }
                doubles.push((kind, p.owner.as_str()));
            }
        }
        for kind in ["path", "rect", "circle", "ellipse"] {
            assert!(doubles.contains(&(kind, "")), "{kind}: {doubles:?}");
        }
        // in a symbol by its own style and by the instance's, and on a wire
        for drawn in [("rect", "item-0"), ("path", "item-1"), ("path", "wire-0")] {
            assert!(doubles.contains(&drawn), "{drawn:?}: {doubles:?}");
        }
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
                    let (strokes, fill, color) = match &p.shape {
                        Shape::Path { strokes, fill, .. }
                        | Shape::Rect { strokes, fill, .. }
                        | Shape::Circle { strokes, fill, .. }
                        | Shape::Ellipse { strokes, fill, .. } => (&strokes[..], fill.as_ref(), None),
                        Shape::Text(t) => (&[][..], None, Some(&t.color)),
                        Shape::Flyline { strokes, .. } => (&strokes[..], None, None),
                    };
                    for Color(c) in strokes.iter().map(|s| &s.color).chain(fill).chain(color) {
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

    /// The SVG draws lines and texts, and the renderer lays out texts, as the types' documentation
    /// says (`Shape`, `Stroke`, `Unit`, `Cap`, `Join`, `Text`, `TextRun`, `Bar`, `VAlign`, `Run`,
    /// `RunStyle`), so that a frontend can draw as the SVG does. When the SVG writer or the layout
    /// changes, so does the documentation.
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
        let stroke = |color: &str, width: f64, unit: Unit, dashes: &[f64], dash_offset: f64, cap: Cap, join: Join| {
            Stroke { color: Color(color.into()), width, unit, dashes: dashes.to_vec(), dash_offset, cap, join }
        };
        // filled, and stroked with each of its strokes in turn; SVG starts the outline of a rect at
        // its x and y, the corner of lowest x and highest y, along the top side, and those of a
        // circle and an ellipse at cx + r, all clockwise on the screen
        type Stroked = fn(Vec<rnl_render::Stroke>) -> Kind;
        let shapes: [(&str, Stroked); 4] = [
            ("<path d=\"M0.00,-0.00 L14.00,-0.00\"", |strokes| Kind::Path {
                segs: vec![PathSeg::Move((0.0, 0.0)), PathSeg::Line((1.0, 0.0))],
                strokes,
                fill: Some("#eeeeee".into()),
            }),
            ("<rect x=\"0.00\" y=\"-28.00\" width=\"14.00\" height=\"28.00\"", |strokes| Kind::Rect {
                min: (0.0, 0.0),
                max: (1.0, 2.0),
                strokes,
                fill: Some("#eeeeee".into()),
            }),
            ("<circle cx=\"0.00\" cy=\"-0.00\" r=\"14.00\"", |strokes| Kind::Circle {
                c: (0.0, 0.0),
                r: 1.0,
                strokes,
                fill: Some("#eeeeee".into()),
            }),
            ("<ellipse cx=\"0.00\" cy=\"-0.00\" rx=\"14.00\" ry=\"28.00\"", |strokes| Kind::Ellipse {
                c: (0.0, 0.0),
                rx: 1.0,
                ry: 2.0,
                strokes,
                fill: Some("#eeeeee".into()),
            }),
        ];
        // the lengths in pixels of the SVG, scene units times the scale and pixels as they are: the
        // width to 2 decimals, and the dashes and their offset as they are, to 6 at most, so that
        // a long line's dashes keep in step
        for (unit, want) in [
            (
                Unit::Scene,
                [
                    "stroke=\"#102030\" stroke-width=\"10.50\" stroke-linecap=\"square\" stroke-linejoin=\"bevel\" stroke-dasharray=\"4.2,3.15,1.05,3.15\" stroke-dashoffset=\"1.575\"/>",
                    "stroke=\"#405060\" stroke-width=\"3.50\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                    "stroke=\"#506070\" stroke-width=\"7.00\" stroke-dasharray=\"7.0,7.0\"/>",
                ],
            ),
            (
                Unit::Pixel,
                [
                    "stroke=\"#102030\" stroke-width=\"0.75\" stroke-linecap=\"square\" stroke-linejoin=\"bevel\" stroke-dasharray=\"0.3,0.225,0.075,0.225\" stroke-dashoffset=\"0.1125\"/>",
                    "stroke=\"#405060\" stroke-width=\"0.25\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
                    "stroke=\"#506070\" stroke-width=\"0.50\" stroke-dasharray=\"0.5,0.5\"/>",
                ],
            ),
        ] {
            let dashed = stroke("#102030", 0.75, unit, &[0.3, 0.225, 0.075, 0.225], 0.1125, Cap::Square, Join::Bevel);
            let round = stroke("#405060", 0.25, unit, &[], 0.0, Cap::Round, Join::Round);
            // butt caps, miter joins and no offset are what SVG draws when it is not told
            let plain = stroke("#506070", 0.5, unit, &[0.5, 0.5], 0.0, Cap::Butt, Join::Miter);
            let strokes: Vec<rnl_render::Stroke> = [dashed, round, plain].iter().map(back_stroke).collect();
            for (head, shape) in shapes {
                let drawn = svg(shape(strokes.clone()));
                let elements: Vec<&str> = drawn.lines().filter(|l| l.starts_with(head)).collect();
                assert_eq!(elements.len(), 1, "{drawn}");
                // the shape once for each stroke, the first with the fill
                let fills = ["#eeeeee", "none", "none"];
                let want: String = fills.iter().zip(&want).map(|(f, w)| format!("{head} fill=\"{f}\" {w}")).collect();
                assert_eq!(elements[0], want);
                // with no strokes, no outline
                let fill = svg(shape(Vec::new()));
                assert!(fill.contains(&format!("{head} fill=\"#eeeeee\" stroke=\"none\"/>")), "{fill}");
            }
        }
        // a flyline as a line, its numbers written short
        let flyline = stroke("#8a8a8a", 1.0, Unit::Pixel, &[4.0, 3.0], 0.0, Cap::Butt, Join::Miter);
        let flyline = svg(Kind::Flyline { a: (0.0, 0.0), b: (1.0, 0.0), strokes: vec![back_stroke(&flyline)] });
        let want = "<line x1=\"0.00\" y1=\"-0.00\" x2=\"14.00\" y2=\"-0.00\" stroke=\"#8a8a8a\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/>";
        assert!(flyline.contains(want), "{flyline}");
        // shapes of no size written as they are, so that SVG's rules for them hold: it draws no
        // rect with no width, no circle or ellipse with a radius of 0, and only the caps of a path
        // that stays at one point
        let round = vec![back_stroke(&stroke("#102030", 1.2, Unit::Pixel, &[], 0.0, Cap::Round, Join::Round))];
        for (kind, want) in [
            (
                Kind::Rect { min: (0.0, 0.0), max: (0.0, 2.0), strokes: round.clone(), fill: Some("#eeeeee".into()) },
                "<rect x=\"0.00\" y=\"-28.00\" width=\"0.00\" height=\"28.00\" fill=\"#eeeeee\" stroke=\"#102030\"",
            ),
            (
                Kind::Circle { c: (0.0, 0.0), r: 0.0, strokes: round.clone(), fill: Some("#eeeeee".into()) },
                "<circle cx=\"0.00\" cy=\"-0.00\" r=\"0.00\" fill=\"#eeeeee\" stroke=\"#102030\"",
            ),
            (
                Kind::Ellipse { c: (0.0, 0.0), rx: 1.0, ry: 0.0, strokes: round.clone(), fill: Some("#eeeeee".into()) },
                "<ellipse cx=\"0.00\" cy=\"-0.00\" rx=\"14.00\" ry=\"0.00\" fill=\"#eeeeee\" stroke=\"#102030\"",
            ),
            (
                Kind::Path { segs: vec![PathSeg::Move((1.0, 1.0)), PathSeg::Line((1.0, 1.0))], strokes: round, fill: None },
                "<path d=\"M14.00,-14.00 L14.00,-14.00\" fill=\"none\" stroke=\"#102030\" stroke-width=\"1.20\" stroke-linecap=\"round\"",
            ),
        ] {
            let drawn = svg(kind);
            assert!(drawn.contains(want), "{drawn}");
        }

        // the texts in the shipped family, with no kerning, ligatures or other change to the
        // advances
        let fonts = "font-family=\"Noto Sans SC, Noto Sans CJK SC, Source Han Sans SC, sans-serif\" style=\"font-kerning:none;font-variant-ligatures:none;text-spacing-trim:space-all;text-autospace:no-autospace\">";
        assert!(svg(Kind::Path { segs: Vec::new(), strokes: Vec::new(), fill: None }).contains(fonts));
        // each run at the start of its baseline, in the text's colour and its size, held to its
        // width, with its spaces, those at its ends too; bold in the bold face; an italic run
        // leaning by 0.25 of the height above its baseline, and a vertical text's runs turned a
        // quarter counter-clockwise about their start
        let run = |text: &str, bold: bool, italic: bool| TextRun {
            text: text.into(),
            at: [1.0, 2.0],
            size: 0.5,
            width: 0.3,
            bold,
            italic,
        };
        let bar = Bar {
            points: [[1.0, 2.0], [1.5, 2.0]],
            stroke: stroke("#102030", 0.06, Unit::Scene, &[], 0.0, Cap::Butt, Join::Miter),
        };
        let text = |runs: Vec<TextRun>, bars: Vec<Bar>, vertical: bool| {
            let t = Text {
                anchor: [0.0, 0.0],
                lines: Vec::new(),
                size: 1.0,
                color: Color("#102030".into()),
                font: "serif".into(),
                bold: false,
                italic: false,
                halign: HAlign::Start,
                valign: VAlign::Baseline,
                vertical,
                runs,
                bars,
            };
            let p = Primitive {
                owner: String::new(),
                layer: Layer::Middle,
                order: Vec::new(),
                bbox: [0.0; 4],
                shape: Shape::Text(t),
            };
            rnl_render::svg_document(&[back(&p)], &o)
        };
        let head = "<text x=\"14.00\" y=\"-28.00\" font-size=\"7.00\"";
        let tail = " textLength=\"4.20\" lengthAdjust=\"spacingAndGlyphs\" fill=\"#102030\" xml:space=\"preserve\">";
        let lean = "matrix(1 0 -0.25 1 -7.00 0)";
        let turn = "rotate(-90 14.00 -28.00)";
        for (runs, vertical, want) in [
            (vec![run(" a  b< ", false, false)], false, format!("{head}{tail} a  b&lt; </text>")),
            (vec![run("V", true, false)], false, format!("{head} font-weight=\"bold\"{tail}V</text>")),
            (vec![run("V", false, true)], false, format!("{head} transform=\"{lean}\"{tail}V</text>")),
            (vec![run("V", false, false)], true, format!("{head} transform=\"{turn}\"{tail}V</text>")),
            (
                vec![run("V", true, true)],
                true,
                format!("{head} font-weight=\"bold\" transform=\"{turn} {lean}\"{tail}V</text>"),
            ),
        ] {
            let drawn = text(runs, Vec::new(), vertical);
            assert!(drawn.contains(&format!("\n{want}\n")), "{want}: {drawn}");
        }
        // the bars as lines with their strokes, after the runs
        let drawn = text(vec![run("V", false, false)], vec![bar], false);
        let want = format!(
            "{head}{tail}V</text><line x1=\"14.00\" y1=\"-28.00\" x2=\"21.00\" y2=\"-28.00\" stroke=\"#102030\" stroke-width=\"0.84\"/>"
        );
        assert!(drawn.contains(&want), "{drawn}");

        // the layout the types describe: the advances of the shipped faces, sub- and superscripts,
        // fractions, overlines, and the anchor on the lines
        let laid = |lines: &str, valign: &str, size: f64| {
            let mut t = rnl_render::TextPrim {
                anchor: (0.0, 0.0),
                lines: markup::parse(lines),
                size,
                color: "#102030".into(),
                font: "serif".into(),
                bold: false,
                italic: false,
                halign: "start".into(),
                valign: valign.into(),
                vertical: false,
                runs: Vec::new(),
                bars: Vec::new(),
            };
            rnl_render::lay_out(&mut t, o.scale);
            super::text(&t)
        };
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        let regular = rnl_render::Face::REGULAR;
        // the lines, from the first ascender to the last descender
        for (valign, first) in [
            ("baseline", 0.0),
            ("top", -1.16),
            ("bottom", 0.288 + 2.0 * 1.448),
            ("middle", (2.0 * 1.448 - 1.16 + 0.288) / 2.0),
        ] {
            let t = laid("a\\\\b\\\\c", valign, 1.0);
            let ys: Vec<f64> = t.runs.iter().map(|r| r.at[1]).collect();
            assert!(
                close(ys[0], first) && close(ys[1], first - 1.448) && close(ys[2], first - 2.0 * 1.448),
                "{valign}: {ys:?}"
            );
        }
        // the levels, each 0.7 of the size of the next towards 0, 0.35 of it higher or 0.15 lower
        let t = laid("x^{y^{z}}_{i}", "baseline", 1.0);
        let levels: Vec<(f64, f64)> = t.runs.iter().map(|r| (r.size, r.at[1])).collect();
        let want = [(1.0, 0.0), (0.7, 0.35), (0.49, 0.35 + 0.35 * 0.7), (0.7, -0.15)];
        assert!(
            levels.len() == 4 && levels.iter().zip(want).all(|(a, b)| close(a.0, b.0) && close(a.1, b.1)),
            "{levels:?}"
        );
        assert!(close(t.runs[1].at[0], regular.width("x")) && close(t.runs[1].width, 0.7 * regular.width("y")));
        // a fraction and its bar, and an overline
        let t = laid("\\frac{ab}{c}\\overline{d}", "baseline", 1.0);
        let (wn, wd) = (0.8 * regular.width("ab"), 0.8 * regular.width("c"));
        let w = wn + 0.15;
        let places: Vec<[f64; 3]> = t.runs.iter().map(|r| [r.at[0], r.at[1], r.size]).collect();
        let want = [[(w - wn) / 2.0, 0.576, 0.8], [(w - wd) / 2.0, -0.424, 0.8], [w, 0.0, 1.0]];
        assert!(
            places.len() == 3 && places.iter().zip(want).all(|(a, b)| (0..3).all(|k| close(a[k], b[k]))),
            "{places:?}"
        );
        let bars: Vec<[f64; 5]> = t
            .bars
            .iter()
            .map(|b| [b.points[0][0], b.points[0][1], b.points[1][0], b.points[1][1], b.stroke.width])
            .collect();
        let d = w + regular.width("d");
        let want = [[0.06, 0.38, w - 0.06, 0.38, 0.06], [w, 0.94, d, 0.94, 0.06]];
        assert!(bars.len() == 2 && bars.iter().zip(want).all(|(a, b)| (0..5).all(|k| close(a[k], b[k]))), "{bars:?}");
        assert!(t.bars.iter().all(|b| b.stroke.cap == Cap::Butt && b.stroke.unit == Unit::Scene), "{t:?}");
        // at least 0.8 pixels wide
        let t = laid("\\frac{a}{b}", "baseline", 0.5);
        assert!(close(t.bars[0].stroke.width, 0.8 / o.scale), "{t:?}");
        // a character the fonts do not map is 1 em wide
        assert!(close(laid("\u{10FFFF}", "baseline", 2.0).runs[0].width, 2.0));
    }
}
