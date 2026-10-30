//! Optional inspection overlays. Geometry is shared by all frontends; style is a renderer choice.
use std::fmt::Write;

use rnl_runtime::model::LevelView;
use serde::{Deserialize, Serialize};

use crate::{Hit, Level, UsageRole};

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct UsageFrame {
    pub bounds: [f64; 4],
    pub title: [f64; 2],
    pub title_width: f64,
    pub role_widths: Vec<f64>,
    /// Role anchors in declaration order; absent for roles with no visible geometry.
    pub roles: Vec<Option<[f64; 2]>>,
    /// A virtual symbol's roles refer to its local connections.
    pub local: bool,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct CurrentArrow {
    pub terminal: String,
    pub direction: String,
    pub pin: [f64; 2],
    /// Tail, tip and the two arrowhead corners, in SVG pixels with y down.
    pub points: [[f64; 2]; 4],
}

fn union(bounds: impl Iterator<Item = [f64; 4]>) -> Option<[f64; 4]> {
    bounds.reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
}
fn hit_bounds(h: &Hit) -> Option<[f64; 4]> {
    union(h.points.iter().map(|p| [p[0], p[1], p[0], p[1]]))
}

/// The title of a usage's frame, `$PAIR: %dp1`; of a group's, which has no definition,
/// `@GROUP: %core` (11.5).
pub(crate) fn usage_title(kind: &str, definition: &str, id: &str) -> String {
    let definition = if kind == "group" { "@GROUP" } else { definition };
    format!("{definition}: {id}")
}

pub(crate) fn usage_frame(
    object: usize,
    local: bool,
    roles: &[UsageRole],
    hits: &[Hit],
    title: &str,
) -> Option<UsageFrame> {
    let b = if local {
        hits.iter().find(|h| h.kind == "box" && h.objects.first() == Some(&object)).and_then(hit_bounds)
    } else {
        union(roles.iter().filter_map(|r| r.bounds))
    }?;
    let mut anchors = Vec::new();
    let mut placed: Vec<[f64; 4]> = Vec::new();
    for r in roles {
        let rb = if local && roles.len() > 1 && r.bounds.is_some() {
            union(
                hits.iter()
                    .filter(|h| h.kind == "pin" && h.objects.iter().any(|o| r.objects.contains(o)))
                    .filter_map(hit_bounds)
                    .filter(|p| p[0] >= b[0] && p[2] <= b[2] && p[1] >= b[1] && p[3] <= b[3]),
            )
            .or(Some(b))
        } else if local {
            r.bounds.map(|_| b)
        } else {
            r.bounds
        };
        anchors.push(rb.map(|rbox| {
            let x = (rbox[0] + rbox[2]) / 2.0;
            let mut y = (rbox[1] + rbox[3]) / 2.0;
            let half = label_width(&r.name) / 2.0 + 4.0;
            if local {
                while placed.iter().any(|p| {
                    x - half < p[2] + 2.0 && x + half > p[0] - 2.0 && y - 9.0 < p[3] + 2.0 && y + 9.0 > p[1] - 2.0
                }) {
                    y += 20.0;
                }
            }
            placed.push([x - half, y - 9.0, x + half, y + 9.0]);
            [x, y]
        }));
    }
    Some(UsageFrame {
        bounds: [b[0] - 12.0, b[1] - 12.0, b[2] + 12.0, b[3] + 12.0],
        title: [b[0] - 4.0, b[1] - 12.0],
        title_width: label_width(title),
        role_widths: roles.iter().map(|r| label_width(&r.name)).collect(),
        roles: anchors,
        local,
    })
}

pub(crate) fn currents(lv: &LevelView, scale: f64) -> Vec<CurrentArrow> {
    let mut arrows = Vec::new();
    for c in &lv.currents {
        let mut places = Vec::new();
        if let (Some(at), Some(dir)) = (c.at, c.dir) {
            places.push((at, dir));
        }
        if let Some(net) = &c.port_net {
            // Declaration ports have no scene point. Use only explicit port usages on this net.
            for o in &lv.objects {
                if o.kind != "usage"
                    || !matches!(o.definition.as_deref(), Some("$IO_PORT" | "$INPUT_PORT" | "$OUTPUT_PORT"))
                {
                    continue;
                }
                for p in &o.pins {
                    if p.nets.contains(net) {
                        if let Some(dir) = p.dir {
                            places.push((p.at, dir));
                        }
                    }
                }
            }
        }
        for (at, dir) in places {
            let (dx, dy) = match dir {
                0 => (1.0, 0.0),
                90 => (0.0, -1.0),
                180 => (-1.0, 0.0),
                270 => (0.0, 1.0),
                _ => continue,
            };
            let pin = [at.0 * scale, -at.1 * scale];
            // A port's exit points into the circuit; a device's exit points out of the device.
            let inward = (c.direction == "in") != c.port_net.is_some();
            let (tail, tip, sign) = if inward { (20.0, 6.0, -1.0) } else { (6.0, 20.0, 1.0) };
            let tip = [pin[0] + dx * tip, pin[1] + dy * tip];
            arrows.push(CurrentArrow {
                terminal: c.terminal.clone(),
                direction: c.direction.clone(),
                pin,
                points: [
                    [pin[0] + dx * tail, pin[1] + dy * tail],
                    tip,
                    [tip[0] - sign * dx * 5.0 - dy * 3.5, tip[1] - sign * dy * 5.0 + dx * 3.5],
                    [tip[0] - sign * dx * 5.0 + dy * 3.5, tip[1] - sign * dy * 5.0 - dx * 3.5],
                ],
            });
        }
    }
    arrows
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UsageMode {
    #[default]
    None,
    Semantic,
    All,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayStyle {
    pub palette: Vec<String>,
    pub width: f64,
    /// Empty for solid lines; lengths and gaps in SVG pixels.
    pub dashes: Vec<f64>,
    pub current_color: String,
}
impl Default for OverlayStyle {
    fn default() -> Self {
        Self {
            palette: ["#e03131", "#e5b500", "#4263eb", "#2f9e44", "#9c36b5", "#009ccc"].map(String::from).to_vec(),
            width: 8.0 / 3.0,
            dashes: vec![1.0, 5.0],
            current_color: "#d9480f".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayOptions {
    pub usages: UsageMode,
    pub currents: bool,
    pub roles: bool,
    pub style: OverlayStyle,
}
impl OverlayOptions {
    pub fn validate(&self) -> Result<(), String> {
        let color = |c: &str| {
            c.starts_with('#') && matches!(c.len(), 4 | 7 | 9) && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
        };
        if self.style.palette.is_empty()
            || !self.style.palette.iter().all(|c| color(c))
            || !color(&self.style.current_color)
        {
            return Err("overlay colors must be hex colors; palette must not be empty".into());
        }
        if !self.style.width.is_finite()
            || self.style.width <= 0.0
            || self.style.dashes.iter().any(|d| !d.is_finite() || *d <= 0.0)
        {
            return Err("overlay width and dash lengths must be finite and positive".into());
        }
        Ok(())
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#x27;")
}
fn rect(b: [f64; 4]) -> String {
    format!("x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"", b[0], b[1], b[2] - b[0], b[3] - b[1])
}
fn label_width(s: &str) -> f64 {
    s.chars().map(|c| rnl_render::Face::REGULAR.advance(c).unwrap_or(1.0) * 13.0).sum()
}
fn attribute(svg: &mut String, name: &str, value: &str) {
    let header_end = svg.find('>').unwrap();
    if let Some(start) = svg[..header_end].find(&format!("{name}=\"")) {
        let from = start + name.len() + 2;
        let to = from + svg[from..].find('"').unwrap();
        svg.replace_range(from..to, value);
    } else {
        svg.insert_str(header_end, &format!(" {name}=\"{value}\""));
    }
}
fn badge(s: &str, at: [f64; 2], color: &str, class: &str, centered: bool) -> (String, [f64; 4]) {
    let width = label_width(s);
    let x = at[0] - if centered { width / 2.0 } else { 0.0 };
    let b = [x - 4.0, at[1] - 9.0, x + width + 4.0, at[1] + 9.0];
    (format!("<g class=\"{class}\"><rect {} rx=\"3\" fill=\"#ffffff\" fill-opacity=\"0.75\"/><text x=\"{}\" y=\"{}\" textLength=\"{width}\" lengthAdjust=\"spacingAndGlyphs\" text-anchor=\"{}\" fill=\"{color}\" font-size=\"13\" font-family=\"system-ui, sans-serif\" dominant-baseline=\"central\">{}</text></g>", rect(b), at[0], at[1], if centered { "middle" } else { "start" }, esc(s)), b)
}

/// Export a level with optional overlays. Disabled options return the original SVG bytes.
pub fn svg(level: &Level, options: &OverlayOptions) -> Result<String, String> {
    options.validate()?;
    let base = level.svg.as_ref().ok_or("overlay export needs a level built with svg=true")?;
    if options.usages == UsageMode::None && !options.currents {
        return Ok(base.clone());
    }
    let mut under = String::from("\n<g class=\"usage-underlays\" pointer-events=\"none\">");
    let mut over = String::from("\n<g class=\"usage-overlays\" pointer-events=\"none\">");
    let mut bounds = Vec::new();
    let dashes = options.style.dashes.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ");
    for (k, u) in level.usages.iter().enumerate() {
        if options.usages == UsageMode::None || (options.usages == UsageMode::Semantic && u.has_drawing) {
            continue;
        }
        let Some(f) = &u.frame else { continue };
        let color = &options.style.palette[k % options.style.palette.len()];
        bounds.push(f.bounds);
        let _ = write!(
            under,
            "<rect class=\"usage-fill\" data-usage=\"{}\" {} fill=\"{color}\" fill-opacity=\"0.2\" rx=\"10\"/>",
            u.object,
            rect(f.bounds)
        );
        let _ = write!(over, "<g class=\"usage-frame\" data-usage=\"{}\"><rect {} fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" stroke-dasharray=\"{dashes}\" stroke-linecap=\"round\" rx=\"10\"/>", u.object, rect(f.bounds), options.style.width);
        let title = usage_title(u.kind, &u.definition, &u.id);
        if f.title_width + 16.0 <= f.bounds[2] - f.bounds[0] {
            let (s, b) = badge(&title, f.title, color, "usage-title", false);
            over.push_str(&s);
            bounds.push(b);
        }
        if options.roles {
            for (role, at) in u.roles.iter().zip(&f.roles) {
                if let Some(at) = at {
                    let (s, b) = badge(&role.name, *at, color, "usage-role-label", true);
                    over.push_str(&s);
                    bounds.push(b);
                }
            }
        }
        over.push_str("</g>");
    }
    under.push_str("</g>");
    over.push_str("</g>");
    if options.currents {
        let _ = write!(over, "\n<g class=\"current-overlays\" pointer-events=\"none\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">", options.style.current_color, options.style.width);
        for c in &level.currents {
            let [a, b, h, j] = c.points;
            let _ = write!(over, "<path class=\"current-arrow\" data-terminal=\"{}\" data-direction=\"{}\" d=\"M{},{} L{},{} M{},{} L{},{} L{},{}\"/>", esc(&c.terminal), c.direction, a[0],a[1],b[0],b[1],h[0],h[1],b[0],b[1],j[0],j[1]);
            bounds.extend(c.points.map(|p| [p[0] - 3.0, p[1] - 3.0, p[0] + 3.0, p[1] + 3.0]));
        }
        over.push_str("</g>");
    }
    // Expand export canvas only; hits and the underlying drawing are never changed.
    let mut out = base.clone();
    if let Some(start) = out.find("viewBox=\"") {
        let from = start + 9;
        let end = from + out[from..].find('"').unwrap();
        let v: Vec<f64> = out[from..end].split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if v.len() == 4 {
            bounds.push([v[0], v[1], v[0] + v[2], v[1] + v[3]]);
        }
    }
    if let Some(b) = union(bounds.into_iter()) {
        let pad = options.style.width / 2.0 + 2.0;
        let b = [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad];
        attribute(&mut out, "viewBox", &format!("{} {} {} {}", b[0], b[1], b[2] - b[0], b[3] - b[1]));
        attribute(&mut out, "width", &format!("{:.0}", b[2] - b[0]));
        attribute(&mut out, "height", &format!("{:.0}", b[3] - b[1]));
        if let Some(k) = out.find("<rect class=\"backdrop\"") {
            let end = k + out[k..].find("/>").unwrap() + 2;
            out.replace_range(k..end, &format!("<rect class=\"backdrop\" {} fill=\"#ffffff\"/>", rect(b)));
        }
    }
    let insert = out
        .find("class=\"backdrop\"")
        .and_then(|k| out[k..].find("/>").map(|end| k + end + 2))
        .unwrap_or_else(|| out.find('>').unwrap() + 1);
    out.insert_str(insert, &under);
    let end = out.rfind("</svg>").ok_or("invalid SVG")?;
    out.insert_str(end, &over);
    Ok(out)
}

/// Apply export options to every SVG in a Document (also used by npm/WASM).
pub fn apply(document: &mut crate::Document, options: &OverlayOptions) -> Result<(), String> {
    options.validate()?;
    for level in &mut document.levels {
        if level.svg.is_some() {
            level.svg = Some(svg(level, options)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildOptions, MemoryProvider};

    fn document() -> crate::Document {
        let source = include_str!("../../../tests/viewer/inputs/usage-current.sp");
        let doc = crate::build(
            "usage-current.sp",
            &MemoryProvider::new().with("usage-current.sp", source),
            &BuildOptions::default(),
        )
        .unwrap();
        assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
        doc
    }

    #[test]
    fn disabled_overlays_keep_svg_bytes_and_geometry_is_style_free() {
        let d = document();
        for lv in &d.levels {
            assert_eq!(svg(lv, &OverlayOptions::default()).unwrap().as_bytes(), lv.svg.as_ref().unwrap().as_bytes());
            assert!(!lv.svg.as_ref().unwrap().contains("current-overlays"));
            for u in &lv.usages {
                let json = serde_json::to_string(&u.frame).unwrap();
                for style in ["color", "stroke", "opacity", "dashes"] {
                    assert!(!json.contains(style));
                }
            }
        }
        let lv = &d.levels[0];
        let frame = |id: &str| lv.usages.iter().find(|u| u.id == id).unwrap().frame.as_ref().unwrap().bounds;
        assert_eq!(frame("%pair"), frame("%nested"));
        assert!(frame("%spread")[2] > frame("%pair")[2]);
        let all = svg(lv, &OverlayOptions { usages: UsageMode::All, ..Default::default() }).unwrap();
        let semantic = svg(lv, &OverlayOptions { usages: UsageMode::Semantic, ..Default::default() }).unwrap();
        let ground = lv.usages.iter().find(|u| u.id == "%ground").unwrap();
        assert!(all.contains(&format!("data-usage=\"{}\"", ground.object)));
        assert!(!semantic.contains(&format!("data-usage=\"{}\"", ground.object)));
        let custom: OverlayOptions =
            serde_json::from_str(r##"{"usages":"all","style":{"palette":["#123456"],"width":4,"dashes":[]}}"##)
                .unwrap();
        let svg = svg(lv, &custom).unwrap();
        assert!(svg.contains("stroke=\"#123456\" stroke-width=\"4\" stroke-dasharray=\"\""));
    }

    #[test]
    fn currents_follow_rotated_mirrored_pins_and_declaration_port_convention() {
        let d = document();
        let top = &d.levels[0];
        let arrow = |id: &str| top.currents.iter().find(|c| c.terminal == id).unwrap();
        assert_eq!(arrow("#R1:1").points[1][1] - arrow("#R1:1").points[0][1], 14.0);
        assert_eq!(arrow("#R1:2").points[1][1] - arrow("#R1:2").points[0][1], 14.0);
        assert_eq!(arrow("#R2:1").points[1][0] - arrow("#R2:1").points[0][0], -14.0);
        assert_eq!(arrow("#R3:1").points[1][1] - arrow("#R3:1").points[0][1], -14.0);
        assert_eq!(arrow("#R4:1").points[1][0] - arrow("#R4:1").points[0][0], 14.0);
        assert!(!top.currents.iter().any(|c| c.terminal == "#R2:2"));
        assert!(top.currents.iter().any(|c| c.terminal == "#X1:1"));
        let cell = &d.levels[1];
        for id in ["#:input", "#:output"] {
            let c = cell.currents.iter().find(|c| c.terminal == id).unwrap();
            assert_eq!(c.points[1][0] - c.points[0][0], 14.0);
        }
        let source = "* ports with no symbols\n.subckt empty a b\n* @RNL { #:a @CURRENT(\"in\"); #:b @CURRENT(\"out\") }\n.ends\n.end\n";
        let empty = crate::build("empty.sp", &MemoryProvider::new().with("empty.sp", source), &BuildOptions::default())
            .unwrap();
        assert!(empty.levels.iter().all(|lv| lv.currents.is_empty()));
    }

    #[test]
    fn virtual_frames_ignore_other_symbols_on_the_attached_net() {
        let source = include_str!("../../../tests/viewer/inputs/virtual-selection.sp");
        let d = crate::build(
            "virtual-selection.sp",
            &MemoryProvider::new().with("virtual-selection.sp", source),
            &BuildOptions::default(),
        )
        .unwrap();
        let lv = &d.levels[0];
        for id in ["%ground", "%ground2", "%supply", "%port"] {
            let u = lv.usages.iter().find(|u| u.id == id).unwrap();
            let f = u.frame.as_ref().unwrap();
            assert!(f.local);
            let b = lv.hits.iter().find(|h| h.kind == "box" && h.objects[0] == u.object).and_then(hit_bounds).unwrap();
            assert_eq!(f.bounds, [b[0] - 12.0, b[1] - 12.0, b[2] + 12.0, b[3] + 12.0]);
        }
    }

    #[test]
    fn overlay_svg_goldens() {
        let d = document();
        let o = OverlayOptions { usages: UsageMode::All, currents: true, roles: true, ..Default::default() };
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/overlays");
        for (k, lv) in d.levels.iter().enumerate() {
            let actual = svg(lv, &o).unwrap();
            let file = dir.join(if k == 0 { "usage-current.svg" } else { "usage-current.cell.svg" });
            if std::env::var_os("RNL_BLESS_OVERLAYS").is_some() {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(&file, &actual).unwrap();
            }
            assert_eq!(std::fs::read_to_string(file).unwrap().replace("\r\n", "\n"), actual);
        }
    }

    #[test]
    fn a_group_frame_is_titled_with_group() {
        let source = "* a group
R1 a b 1k
R2 b c 1k
* @RNL {
*   #R1 @PLACE(0, 0);
*   #R2 @PLACE(30, 0);
*   %core = {#R1 #R2} @GROUP
* }
.end
";
        let d = crate::build("group.sp", &MemoryProvider::new().with("group.sp", source), &BuildOptions::default())
            .unwrap();
        let lv = &d.levels[0];
        let core = lv.usages.iter().find(|u| u.id == "%core").unwrap();
        assert_eq!(core.kind, "group");
        assert!(core.frame.is_some());
        let svg = svg(lv, &OverlayOptions { usages: UsageMode::All, ..Default::default() }).unwrap();
        assert!(svg.contains(">@GROUP: %core</text>"), "{svg}");
        assert!(!svg.contains(">: %core</text>"));
    }

    #[test]
    fn invalid_export_styles_are_rejected() {
        for style in [
            r#"{"palette":[]}"#,
            r#"{"width":0}"#,
            r#"{"width":-1}"#,
            r#"{"dashes":[-2]}"#,
            r#"{"current_color":"url(https://invalid)"}"#,
        ] {
            let options = OverlayOptions { style: serde_json::from_str(style).unwrap(), ..Default::default() };
            assert!(options.validate().is_err());
        }
    }
}
