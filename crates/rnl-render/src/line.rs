//! Line styles resolved into strokes (9.3): what a frontend draws with a colour, a width, dashes,
//! a cap and a join and nothing else. The hairline's width, the dashes and the double line are
//! this renderer's own choices (9.3 leaves them to the renderer).

use crate::Stroke;

/// The width of the hairline, the line 0 wide that keeps its width at every zoom (9.3), in pixels.
const HAIRLINE: f64 = 1.2;

/// The narrowest each of the two lines of a "double" line and the gap between them can be, in
/// scene units.
const DOUBLE: f64 = 0.12;

/// A line as its style gives it, after the style queue and the renderer's defaults (9.3).
pub(crate) struct Line<'a> {
    pub style: &'a str,
    /// In scene units; 0, or anything not above it, is the hairline.
    pub width: f64,
    pub color: &'a str,
    /// The colour between the two lines of a "double" line.
    pub inner: &'a str,
    pub cap: &'a str,
    pub join: &'a str,
}

impl Line<'_> {
    /// The strokes that draw the line, first to last, at `scale` pixels per scene unit. A line is
    /// `width` scene units wide, or the hairline, 1.2 pixels wide at every zoom; the dashes are in
    /// the width's unit, in multiples of m, the width but at least 1 pixel at `scale`, and start
    /// where the outline starts:
    ///
    /// - "solid": one stroke as wide as the line;
    /// - "bold": one stroke twice as wide;
    /// - "dashed": dashes 4m long and gaps of 3m;
    /// - "dotted": dots m long and gaps of 2.5m;
    /// - "dash-dot": a dash of 5m, a gap of 2m, a dot of m and a gap of 2m;
    /// - "double": two lines d wide with a gap of d between them, where d is the width, but at
    ///   least 0.12, in scene units: a stroke 3d wide in the colour under a stroke d wide in the
    ///   inner colour, both with butt caps and miter joins whatever the style's;
    /// - "none": no stroke.
    ///
    /// The language allows no other styles (9.3); any other is drawn as "solid".
    pub(crate) fn strokes(&self, scale: f64) -> Vec<Stroke> {
        let stroke = |color: &str, width: f64, pixels: bool, dashes: Vec<f64>| Stroke {
            color: color.to_string(),
            width,
            pixels,
            dashes,
            dash_offset: 0.0,
            cap: self.cap.to_string(),
            join: self.join.to_string(),
        };
        let (width, pixels) = if self.width > 0.0 { (self.width, false) } else { (HAIRLINE, true) };
        // the unit of the dashes: the width, but at least a pixel
        let m = width.max(if pixels { 1.0 } else { 1.0 / scale });
        match self.style {
            "none" => Vec::new(),
            "bold" => vec![stroke(self.color, 2.0 * width, pixels, Vec::new())],
            "dashed" => vec![stroke(self.color, width, pixels, vec![4.0 * m, 3.0 * m])],
            "dotted" => vec![stroke(self.color, width, pixels, vec![m, 2.5 * m])],
            "dash-dot" => vec![stroke(self.color, width, pixels, vec![5.0 * m, 2.0 * m, m, 2.0 * m])],
            "double" => {
                let d = self.width.max(DOUBLE);
                let line = |color: &str, width: f64| Stroke {
                    cap: "butt".into(),
                    join: "miter".into(),
                    ..stroke(color, width, false, Vec::new())
                };
                vec![line(self.color, 3.0 * d), line(self.inner, d)]
            }
            _ => vec![stroke(self.color, width, pixels, Vec::new())],
        }
    }
}

/// The stroke of a flyline (13.7): 1 pixel wide in grey, with dashes 4 pixels long and gaps of 3,
/// and butt caps.
pub(crate) fn flyline() -> Stroke {
    Stroke {
        color: "#8a8a8a".into(),
        width: 1.0,
        pixels: true,
        dashes: vec![4.0, 3.0],
        dash_offset: 0.0,
        cap: "butt".into(),
        join: "miter".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{svg_document, Kind, PathSeg, Primitive, RenderOptions};
    use rnl_runtime::model::Layer;
    use rnl_runtime::words::{words, Literal, ParamType};

    fn line(style: &str, width: f64) -> Line<'_> {
        Line { style, width, color: "#102030", inner: "#405060", cap: "square", join: "bevel" }
    }

    /// The SVG of one shape drawn with `strokes`.
    fn svg(shape: fn(Vec<Stroke>) -> Kind, strokes: Vec<Stroke>) -> String {
        let p = Primitive {
            owner: String::new(),
            layer: Layer::Middle,
            order: Vec::new(),
            kind: shape(strokes),
            bbox: [0.0, 0.0, 1.0, 1.0],
        };
        svg_document(&[p], &RenderOptions::default())
    }

    const SHAPES: [fn(Vec<Stroke>) -> Kind; 4] = [
        |strokes| Kind::Path {
            segs: vec![PathSeg::Move((0.0, 0.0)), PathSeg::Line((1.0, 0.0)), PathSeg::Line((1.0, 1.0))],
            strokes,
            fill: None,
        },
        |strokes| Kind::Rect { min: (0.0, 0.0), max: (1.0, 1.0), strokes, fill: None },
        |strokes| Kind::Circle { c: (0.0, 0.0), r: 1.0, strokes, fill: None },
        |strokes| Kind::Ellipse { c: (0.0, 0.0), rx: 1.0, ry: 2.0, strokes, fill: None },
    ];

    #[test]
    fn every_line_style_of_the_language_is_resolved() {
        let w = words();
        let ParamType::Enum(styles) = &w.style_prop("linestyle").unwrap().ty else { panic!() };
        let styles: Vec<&str> =
            styles.iter().filter_map(|v| if let Literal::Str(s) = v { Some(s.as_str()) } else { None }).collect();
        let solid = line("solid", 0.2).strokes(14.0);
        for style in &styles {
            let strokes = line(style, 0.2).strokes(14.0);
            // only "solid" is drawn as "solid"
            assert_eq!(strokes == solid, *style == "solid", "{style}");
        }
        assert_eq!(styles.len(), 7, "{styles:?}");
        assert_eq!(line("wavy", 0.2).strokes(14.0), solid);
    }

    #[test]
    fn the_strokes_are_as_documented() {
        let scale = 14.0;
        let plain = |color: &str, width: f64, pixels: bool, dashes: &[f64]| Stroke {
            color: color.into(),
            width,
            pixels,
            dashes: dashes.to_vec(),
            dash_offset: 0.0,
            cap: "square".into(),
            join: "bevel".into(),
        };
        for width in [0.0, -1.0, f64::NAN, 0.05, 0.2, 1.0] {
            let (w, pixels) = if width > 0.0 { (width, false) } else { (1.2, true) };
            let m = if pixels { 1.2 } else { width.max(1.0 / scale) };
            for (style, want) in [
                ("solid", vec![plain("#102030", w, pixels, &[])]),
                ("bold", vec![plain("#102030", 2.0 * w, pixels, &[])]),
                ("dashed", vec![plain("#102030", w, pixels, &[4.0 * m, 3.0 * m])]),
                ("dotted", vec![plain("#102030", w, pixels, &[m, 2.5 * m])]),
                ("dash-dot", vec![plain("#102030", w, pixels, &[5.0 * m, 2.0 * m, m, 2.0 * m])]),
                ("none", Vec::new()),
            ] {
                assert_eq!(line(style, width).strokes(scale), want, "{style} {width}");
            }
            // two lines d wide and a gap of d, in scene units, with butt caps and miter joins
            let d = width.max(0.12);
            let butt = |color: &str, width: f64| Stroke {
                cap: "butt".into(),
                join: "miter".into(),
                ..plain(color, width, false, &[])
            };
            assert_eq!(line("double", width).strokes(scale), [butt("#102030", 3.0 * d), butt("#405060", d)]);
        }
        // a dash is at least a pixel long, whatever the scale
        assert_eq!(line("dotted", 0.05).strokes(10.0)[0].dashes, [0.1, 0.25]);
        assert_eq!(line("dotted", 0.05).strokes(40.0)[0].dashes, [0.05, 0.125]);
    }

    /// The value of an attribute of the one element of `svg` that strokes.
    fn stroke_attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
        let element = svg.lines().find(|l| l.contains(" stroke=\"#"))?;
        let at = element.find(&format!(" {name}=\""))? + name.len() + 3;
        Some(&element[at..at + element[at..].find('"')?])
    }

    /// The SVG of each line style on each shape draws what the SVG writer drew before the renderer
    /// resolved the styles: w is the width in pixels, 1.2 for the hairline, and m the larger of w
    /// and 1. Only "double" on a rectangle, circle or ellipse changed: it was drawn as "solid".
    /// The numbers are those written before, except a dash or a gap that is not a whole number of
    /// tenths of a pixel, such as a gap of 3.15 pixels: that was rounded to a tenth, which put a
    /// long line's dashes out of step with the primitives', and is now written to at most 6
    /// decimals.
    #[test]
    fn the_svg_draws_each_style_as_before() {
        let scale = RenderOptions::default().scale;
        // a number written with `decimals` decimals, as near as that allows to `want`
        let near = |written: &str, want: f64, decimals: usize| {
            let x: f64 = written.parse().unwrap();
            let half = 0.5 / 10f64.powi(decimals as i32);
            (x - want).abs() <= half + 1e-9 && written.split_once('.').is_some_and(|(_, d)| d.len() == decimals)
        };
        // a dash written as before when it is a whole number of tenths, and else to 6 decimals
        // with no trailing zero
        let dash = |written: &str, want: f64| {
            let tenths = (want * 10.0 - (want * 10.0).round()).abs() < 1e-9;
            let (_, decimals) = written.split_once('.').unwrap();
            let x: f64 = written.parse().unwrap();
            if tenths {
                written == format!("{want:.1}")
            } else {
                (x - want).abs() <= 0.5e-6 + 1e-12 && decimals.len() <= 6 && !decimals.ends_with('0')
            }
        };
        // the widths from 0 to 2 scene units in steps of 0.005
        for k in 0..=400 {
            let width = k as f64 * 0.005;
            let w = if width > 0.0 { width * scale } else { 1.2 };
            let m = w.max(1.0);
            for (style, w, dashes) in [
                ("solid", w, vec![]),
                ("bold", 2.0 * w, vec![]),
                ("dashed", w, vec![4.0 * m, 3.0 * m]),
                ("dotted", w, vec![m, 2.5 * m]),
                ("dash-dot", w, vec![5.0 * m, 2.0 * m, m, 2.0 * m]),
            ] {
                for shape in SHAPES {
                    let svg = svg(shape, line(style, width).strokes(scale));
                    assert_eq!(svg.matches("stroke=").count(), 1, "{style} {width}: {svg}");
                    assert!(near(stroke_attr(&svg, "stroke-width").unwrap(), w, 2), "{style} {width}: {svg}");
                    assert_eq!(stroke_attr(&svg, "stroke-linecap"), Some("square"), "{svg}");
                    assert_eq!(stroke_attr(&svg, "stroke-linejoin"), Some("bevel"), "{svg}");
                    let written: Vec<&str> =
                        stroke_attr(&svg, "stroke-dasharray").map_or(vec![], |d| d.split(',').collect());
                    assert_eq!(written.len(), dashes.len(), "{style} {width}: {svg}");
                    for (written, want) in written.iter().zip(&dashes) {
                        assert!(dash(written, *want), "{style} {width}: {svg}");
                    }
                }
            }
            // "double": the colour 3d wide under the inner colour d wide, with SVG's own caps and
            // joins, butt and miter; on a path as before, and now on the other shapes too
            let d = width.max(0.12) * scale;
            for shape in SHAPES {
                let svg = svg(shape, line("double", width).strokes(scale));
                let elements: Vec<&str> = svg.lines().filter(|l| l.contains(" stroke=")).collect();
                assert_eq!(elements.len(), 1, "{width}: {svg}");
                let strokes: Vec<&str> = elements[0].split("/>").filter(|e| !e.is_empty()).collect();
                assert_eq!(strokes.len(), 2, "{width}: {svg}");
                for (element, (color, w)) in strokes.iter().zip([("#102030", 3.0 * d), ("#405060", d)]) {
                    let end = format!("fill=\"none\" stroke=\"{color}\" stroke-width=\"");
                    let at = element.find(&end).unwrap_or_else(|| panic!("{width}: {svg}")) + end.len();
                    // the width and nothing after it
                    let (written, after) = element[at..].split_once('"').unwrap();
                    assert!(near(written, w, 2) && after.is_empty(), "{width}: {svg}");
                }
            }
        }
        let flyline = svg(|strokes| Kind::Flyline { a: (0.0, 0.0), b: (1.0, 0.0), strokes }, vec![flyline()]);
        assert!(flyline.contains("stroke=\"#8a8a8a\" stroke-width=\"1\" stroke-dasharray=\"4,3\"/>"), "{flyline}");
    }

    /// The SVG's dashes are the primitives', not rounded to a tenth of a pixel: rounded, the gaps
    /// of a dashed line 0.075 wide, 3.15 pixels, would be 0.05 pixels off, and its dashes a whole
    /// period out of step with the primitives' within 1400 pixels.
    #[test]
    fn a_long_dashed_line_keeps_in_step() {
        let scale = RenderOptions::default().scale;
        for (style, width, want) in [
            ("dashed", 0.075, "4.2,3.15"),
            ("dotted", 0.25, "3.5,8.75"),
            ("dash-dot", 0.125, "8.75,3.5,1.75,3.5"),
            ("dotted", 0.1234, "1.7276,4.319"),
        ] {
            let svg = svg(SHAPES[0], line(style, width).strokes(scale));
            assert_eq!(stroke_attr(&svg, "stroke-dasharray"), Some(want), "{style} {width}: {svg}");
        }
    }
}
