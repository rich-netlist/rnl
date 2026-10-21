//! The actual symbols of subcircuits, without reinterpreting a package or resolving a name again.
use std::collections::{BTreeMap, HashMap};

use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::markup;
use rnl_runtime::model::{ItemView, Layer, LevelView, Style, SymbolOrigin, SymbolView, TextView};
use rnl_runtime::Interpretation;
use serde::Serialize;

use super::Level;

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InspectedSymbol {
    pub name: String,
    pub origin: String,
    pub default: bool,
    pub file: Option<String>,
    pub first_line: u32,
    pub source: String,
    pub bbox: [f64; 4],
    pub pins: Vec<InspectedPin>,
    pub labels: Vec<InspectedLabel>,
    /// Preview in the renderer's pixel coordinates; labels have data-id="symbol-label".
    pub svg: String,
    pub scale: f64,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InspectedPin {
    pub at: Pt,
    pub terminals: Vec<String>,
    pub name: Option<String>,
    pub direction: Option<u16>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct InspectedLabel {
    pub key: Option<String>,
    pub text: String,
    pub at: Option<Pt>,
}

pub(super) fn inspect(r: &Interpretation, levels: &mut [Level]) -> Vec<InspectedSymbol> {
    let mut out = Vec::new();
    let mut seen = HashMap::new();
    // default_symbol() is determined only by terminal order and names. Its runtime object is
    // per instance; thousands of identical defaults need only one inspector entry.
    let mut defaults = HashMap::new();
    // Split only files whose definition is actually displayed, once per file.
    let sources: HashMap<_, _> = r.sources.iter().map(|(p, s)| (p.as_str(), s.as_str())).collect();
    let mut lines: HashMap<String, Vec<&str>> = HashMap::new();
    let mut add = |symbols: &[SymbolView], root: usize| {
        let s = &symbols[root];
        if let Some(&k) = seen.get(&s.id) {
            return k;
        }
        let default_key =
            s.default.then(|| s.pins.iter().map(|p| (p.terminals.clone(), p.label.clone())).collect::<Vec<_>>());
        if let Some(k) = default_key.as_ref().and_then(|key| defaults.get(key)).copied() {
            seen.insert(s.id, k);
            return k;
        }
        let mut detail = preview(symbols, root);
        if let Some((a, b)) = &s.source {
            detail.file = Some(a.file.clone());
            detail.first_line = a.line;
            if let Some(text) = sources.get(a.file.as_str()) {
                let text = lines.entry(a.file.clone()).or_insert_with(|| text.lines().collect());
                detail.source = text
                    .iter()
                    .enumerate()
                    .skip(a.line.saturating_sub(1) as usize)
                    .take((b.line.saturating_sub(a.line) + 1) as usize)
                    .map(|(i, line)| {
                        let from = if i + 1 == a.line as usize { a.col.saturating_sub(1) as usize } else { 0 };
                        let to = if i + 1 == b.line as usize { b.col as usize } else { line.chars().count() };
                        line.chars().skip(from).take(to.saturating_sub(from)).collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        detail.origin = match &s.origin {
            SymbolOrigin::Package(p) => {
                let version = s.package_version.as_deref().unwrap_or("");
                format!("Package {p} {version}").trim_end().to_string()
            }
            SymbolOrigin::Level(lv) => {
                r.doc.levels.get(*lv).map(|l| format!("Defined in {}", l.name)).unwrap_or_default()
            }
            SymbolOrigin::None if s.default => "Viewer default symbol".into(),
            SymbolOrigin::None => "Inline symbol".into(),
        };
        let k = out.len();
        seen.insert(s.id, k);
        if let Some(key) = default_key {
            defaults.insert(key, k);
        }
        out.push(detail);
        k
    };
    for (lv, target) in r.model.levels.iter().zip(levels) {
        if let Some(block) = &lv.block_symbol {
            target.symbol = Some(add(&block.symbols, 0));
        }
        let host = &r.doc.levels[lv.level];
        // Host index is stable, including bundles and not-yet-placed instances.
        let names: HashMap<_, _> = host.instances.iter().enumerate().map(|(k, i)| (i.name.as_str(), k)).collect();
        let items: HashMap<_, _> = lv
            .items
            .iter()
            .map(|it| (it, &lv.symbols))
            .chain(lv.unplaced_items.iter().map(|it| (it, &lv.unplaced_symbols)))
            .flat_map(|(it, table)| it.host.iter().map(move |&h| (h, (it.symbol, table))))
            .collect();
        for child in &mut target.children {
            if let Some((root, table)) = names.get(child.instance.as_str()).and_then(|h| items.get(h)) {
                child.symbol = Some(add(table, *root));
            }
        }
    }
    out
}

// Show template keys, as in the package gallery, without pretending to have an instance value.
fn template(mut text: &str) -> String {
    let mut out = String::new();
    while let Some((before, rest)) = text.split_once("${") {
        let Some((key, after)) = rest.split_once('}') else { break };
        out.push_str(before);
        out.push_str(key);
        text = after;
    }
    out.push_str(text);
    out
}

fn preview(symbols: &[SymbolView], root: usize) -> InspectedSymbol {
    // A level can contain a distinct default symbol for every instance. Copy only this
    // symbol's reachable graph, never the entire level's table for each preview (quadratic).
    let mut table = Vec::new();
    copy_symbol(symbols, root, &mut HashMap::new(), &mut table);
    let symbols = table.as_slice();
    let root = 0;
    let s = &symbols[root];
    let render = rnl_render::RenderOptions::default();
    let mut alone = LevelView { symbols: symbols.to_vec(), ..LevelView::default() };
    let mut texts = BTreeMap::new();
    let mut pins = Vec::new();
    let mut labels = Vec::new();
    let mut visiting = Vec::new();
    collect(
        symbols,
        root,
        (0.0, 0.0),
        Orient::R0,
        "",
        &mut visiting,
        &mut texts,
        &mut alone.texts,
        &mut pins,
        &mut labels,
    );
    alone.items.push(ItemView {
        id: "symbol-body".into(),
        object_id: String::new(),
        kind: "instance",
        symbol: root,
        at: (0.0, 0.0),
        orient: Orient::R0,
        style: Style::new(),
        layer: Layer::Middle,
        order: vec![],
        texts,
        host: vec![],
        pin_nets: vec![],
    });
    InspectedSymbol {
        name: s.name.clone(),
        origin: String::new(),
        default: s.default,
        file: None,
        first_line: 0,
        source: String::new(),
        bbox: s.bbox,
        pins,
        labels,
        svg: rnl_render::svg(&alone, &render),
        scale: render.scale,
    }
}

fn copy_symbol(
    source: &[SymbolView],
    root: usize,
    seen: &mut HashMap<usize, usize>,
    out: &mut Vec<SymbolView>,
) -> usize {
    if let Some(&k) = seen.get(&root) {
        return k;
    }
    let k = out.len();
    seen.insert(root, k);
    out.push(source[root].clone());
    for (i, nested) in source[root].nested.iter().enumerate() {
        out[k].nested[i].symbol = copy_symbol(source, nested.symbol, seen, out);
    }
    k
}

#[allow(clippy::too_many_arguments)]
fn collect(
    symbols: &[SymbolView],
    root: usize,
    at: Pt,
    orient: Orient,
    path: &str,
    visiting: &mut Vec<usize>,
    texts: &mut BTreeMap<String, Vec<markup::Line>>,
    drawn_labels: &mut Vec<TextView>,
    pins: &mut Vec<InspectedPin>,
    labels: &mut Vec<InspectedLabel>,
) {
    if visiting.contains(&root) {
        return;
    }
    visiting.push(root);
    let s = &symbols[root];
    let place = |p: Pt| {
        let q = orient.apply(p);
        (at.0 + q.0, at.1 + q.1)
    };
    for (k, p) in s.prims.iter().enumerate() {
        if let Some(t) = &p.text {
            texts.insert(format!("{path}{k}"), markup::parse(&template(t)));
        }
    }
    for p in s.pins.iter().filter(|_| path.is_empty()) {
        pins.push(InspectedPin {
            at: place(p.pos),
            terminals: p.terminals.clone(),
            name: p.label.clone(),
            direction: p.dir.map(|d| {
                let v = orient.apply(match d {
                    90 => (0.0, 1.0),
                    180 => (-1.0, 0.0),
                    270 => (0.0, -1.0),
                    _ => (1.0, 0.0),
                });
                if v.0 > 0.5 {
                    0
                } else if v.1 > 0.5 {
                    90
                } else if v.0 < -0.5 {
                    180
                } else {
                    270
                }
            }),
        });
    }
    // A nested usage draws graphics; its own terminals and keyed labels do not become the
    // outer symbol's interface. Direct symbol inclusion is already flattened by the runtime.
    for label in s.labels.iter().filter(|_| path.is_empty()) {
        labels.push(InspectedLabel { key: label.key.clone(), text: label.text.clone(), at: label.at.map(place) });
        if let Some(anchor) = label.at {
            drawn_labels.push(TextView {
                id: "symbol-label".into(),
                object_id: String::new(),
                attach: None,
                target: None,
                key: label.key.clone(),
                markup: label.text.clone(),
                written: false,
                anchor: place(anchor),
                lines: markup::parse(&template(&label.text)),
                halign: label.halign.clone().unwrap_or_else(|| "start".into()),
                valign: label.valign.clone().unwrap_or_else(|| "baseline".into()),
                rotate: label.rotate,
                orient,
                style: label.style.clone(),
                own_style: Style::new(),
                layer: Layer::Top,
                order: vec![],
            });
        }
    }
    for (k, n) in s.nested.iter().enumerate() {
        collect(
            symbols,
            n.symbol,
            place(n.at),
            n.orient.then(orient),
            &format!("{path}{k}/"),
            visiting,
            texts,
            drawn_labels,
            pins,
            labels,
        );
    }
    visiting.pop();
}

#[cfg(test)]
mod tests {
    use crate::{build, BuildOptions, MemoryProvider};

    #[test]
    fn symbols_follow_actual_assignments_and_keep_their_original_definitions() {
        let text = include_str!("../../../../tests/viewer/inputs/symbol-inspector.sp");
        let d = build("a.sp", &MemoryProvider::new().with("a.sp", text), &BuildOptions::default()).unwrap();
        assert!(d.diagnostics.iter().all(|d| d.severity != "error"), "{:?}", d.diagnostics);
        assert_eq!(d.files[0].semantics, [12, 31]);
        let root = &d.levels[0];
        let leaf = &d.levels[1];
        let child = |name| root.children.iter().find(|c| c.instance == name).unwrap().symbol.unwrap();
        assert_eq!(child("Xown"), leaf.symbol.unwrap());
        assert_eq!(child("Xparent"), child("Xunplaced"));
        assert_ne!(child("Xown"), child("Xparent"));
        assert_eq!(d.symbols.len(), 3, "shared symbols are carried once");
        let own = &d.symbols[child("Xown")];
        assert_eq!(own.bbox, [-2.0, -1.0, 2.0, 1.0]);
        assert_eq!(own.first_line, 5);
        assert!(own.source.starts_with("%own = @DEFINE-SYMBOL"), "{}", own.source);
        assert!(own.source.ends_with('}'));
        assert_eq!(own.pins[0].name.as_deref(), Some("p"));
        assert_eq!(own.labels[0].text, "${name}");
        assert!(own.svg.contains("data-id=\"symbol-label\""));
        let parent = &d.symbols[child("Xparent")];
        assert_eq!(parent.bbox, [-4.0, -2.0, 4.0, 2.0]);
        assert_eq!(parent.first_line, 21);
        assert!(parent.source.contains("width=8"));
        let package = &d.symbols[child("Xpackage")];
        assert_eq!(package.origin, "Package rnl-std-basic 0.3");
        assert!(package.source.starts_with("$RESISTOR = @DEFINE-SYMBOL"));
        assert_eq!(package.first_line, 15);
        assert!(!package.source.contains("$VARIABLE_RESISTOR"));
    }

    #[test]
    fn same_line_definitions_do_not_include_their_neighbours() {
        let text = "* title\n.subckt leaf p n\n* @RNL { %a = @DEFINE-SYMBOL { @LINE {(0, 0) (1, 1)} }; %b = @DEFINE-SYMBOL { @LINE {(0, 0) (2, 2)} }; # @SET-SYMBOL %b }\n.ends\n";
        let d = build("a.sp", &MemoryProvider::new().with("a.sp", text), &BuildOptions::default()).unwrap();
        let b = &d.symbols[d.levels.iter().find(|l| l.title == "leaf").unwrap().symbol.unwrap()];
        assert_eq!(b.source, "%b = @DEFINE-SYMBOL { @LINE {(0, 0) (2, 2)} }");
    }

    #[test]
    fn a_local_alias_keeps_the_package_origin_and_version() {
        let text =
            "* title\n.subckt leaf p n\n.ends\nX1 a 0 leaf\n* @RNL { %alias = $RESISTOR; #X1 @SET-SYMBOL %alias }\n";
        let d = build("a.sp", &MemoryProvider::new().with("a.sp", text), &BuildOptions::default()).unwrap();
        assert_eq!(d.symbols.len(), 1);
        assert_eq!(d.symbols[0].origin, "Package rnl-std-basic 0.3");
        assert_eq!(d.symbols[0].first_line, 15);
    }

    #[test]
    fn thousands_of_default_instances_share_one_preview() {
        let mut text = "* title\n.subckt leaf p n\n.ends\n".to_string();
        for i in 0..2000 {
            text.push_str(&format!("X{i} a 0 leaf\n"));
        }
        let d = build("a.sp", &MemoryProvider::new().with("a.sp", &text), &BuildOptions::default()).unwrap();
        assert_eq!(d.levels[0].children.len(), 2000);
        assert_eq!(d.symbols.len(), 1);
        assert!(d.symbols[0].default);
        assert!(d.levels[0].children.iter().all(|c| c.symbol == Some(0)));
    }

    #[test]
    fn nested_usage_keeps_graphics_but_does_not_add_an_outer_interface() {
        let text = r#"* title
.subckt leaf p n
* @RNL {
* %inner = @DEFINE-SYMBOL {
*   (0, 0) @PIN(terminal=1); (0, 0) @TEXT("nested ink");
*   ? @TEXT("inner label", key="inner") => @PLACE(1, 0)
* };
* %decoration = @DEFINE(roles=["pt"]) {};
* %outer = @DEFINE-SYMBOL {
*   (-3, 0) @PIN(terminal=1); (3, 0) @PIN(terminal=2);
*   (0, 0) @AS %decoration => @SET-SYMBOL %inner;
*   ? @TEXT("rotated label", key="outer", rotate=90) => @PLACE(4, 0)
* };
* # @SET-SYMBOL %outer
* }
.ends
"#;
        let d = build("a.sp", &MemoryProvider::new().with("a.sp", text), &BuildOptions::default()).unwrap();
        assert!(d.diagnostics.iter().all(|d| d.severity != "error"), "{:?}", d.diagnostics);
        let s = &d.symbols[d.levels[1].symbol.unwrap()];
        assert_eq!(s.pins.len(), 2);
        assert_eq!(s.labels.len(), 1);
        assert_eq!(s.labels[0].key.as_deref(), Some("outer"));
        assert!(s.svg.contains("nested ink"), "{}", s.svg);
        assert!(!s.svg.contains("inner label"));
        assert!(s.svg.contains("rotate(-90"), "{}", s.svg);
    }
}
