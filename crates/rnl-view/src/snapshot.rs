//! The snapshot of the editor interface (`notes/rnl-editor-interface/spec-draft.md` in rnl-dev,
//! section 3): what a converter or an editor reads of a document besides its drawing, by ids it
//! can write back into transactions. This first part gives what Analog Canvas's import needs:
//!
//! - `host`, the host circuit of each level, to check against its own netlist reader;
//! - `model`, the objects, named points, wires and texts of each level as the RNL draws them.
//!
//! Every value comes from the same interpretation as the Document. `sources` and `revision` come
//! later; until then each slot's `source` is null.

use std::collections::HashMap;

use rnl_runtime::geom::Pt;
use rnl_runtime::host::{HostDocument, LevelKind};
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::model::{self, quoted, Attach, LevelView, Style, SymbolInfo, SymbolOrigin};
use rnl_runtime::Interpretation;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::document::{Diagnostic, Document};
use crate::primitive::{self, Run};

/// The version of the snapshot's shape.
pub const SCHEMA: &str = "rnl-snapshot/1";

/// The parts of a snapshot to give; what is not asked for is not computed.
#[derive(Clone, Copy, Debug)]
pub struct Parts {
    /// The Document, in `drawing`.
    pub drawing: bool,
    /// Each level's host circuit.
    pub host: bool,
    /// Each level's model.
    pub model: bool,
}

impl Default for Parts {
    fn default() -> Parts {
        Parts { drawing: true, host: false, model: false }
    }
}

/// A document as the editor interface gives it: its files, packages and diagnostics, and each
/// drawing level's host circuit and model, as [`Parts`] ask.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Snapshot {
    /// "rnl-snapshot/1".
    pub schema: String,
    /// The version of the runtime that made it.
    pub runtime: String,
    /// The language version the main file declares, or the one a file without it is read as (16.1).
    pub language: String,
    /// "spice" or "spectre".
    pub dialect: String,
    /// The packages in effect, by name and version, the standard package first.
    pub packages: Vec<PackageUse>,
    /// The host files, as in the Document: the opened file first.
    pub files: Vec<FileRef>,
    /// The drawing levels, as in the Document.
    pub levels: Vec<LevelSnapshot>,
    pub diagnostics: Vec<Diagnostic>,
    /// Why the document could not be interpreted at all.
    pub failed: Option<String>,
    /// The Document, when `parts.drawing` asks for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub drawing: Option<Document>,
}

#[derive(Serialize, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PackageUse {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FileRef {
    /// As the Document's `SourceFile::path`.
    pub path: String,
    /// As shown.
    pub name: String,
}

/// A drawing level.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LevelSnapshot {
    /// `subckt:NAME` for a subcircuit, `file:PATH` for the file level, PATH as in `files`.
    pub id: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    /// The subcircuit's name, or the file's.
    pub name: String,
    /// The file it is defined in, an index into `files`.
    pub file: Option<usize>,
    /// The line of a subcircuit's header; 0 for a file level.
    pub line: u32,
    /// What the level lacks without a solver, as the Document's `Level::status`.
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub host: Option<HostLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub model: Option<LevelModel>,
}

/// The host circuit of a level, read only. Names are written as on the host's lines; a net
/// written several ways goes by its first.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostLevel {
    /// The subcircuit's ports in order, as net ids.
    pub ports: Vec<String>,
    /// The instances in netlist order.
    pub instances: Vec<HostInstance>,
    /// The nets: the ports first, then as the instances name them.
    pub nets: Vec<HostNet>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostInstance {
    /// `#M1`.
    pub id: String,
    /// An instance of a subcircuit, defined in the netlist or not.
    pub subckt: bool,
    /// The name in the model position: a model, a subcircuit or a Spectre master.
    pub master: Option<String>,
    pub terminals: Vec<HostTerminal>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostTerminal {
    /// From 1, in the order of the line's nodes.
    pub n: u32,
    /// From DEVICE, the subcircuit's ports or the adapter's defaults (6.4, 6.12).
    pub name: Option<String>,
    /// `&vout`.
    pub net: String,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct HostNet {
    /// `&vout`, `&"vdd!"`.
    pub id: String,
    pub global: bool,
    pub port: bool,
}

/// What the RNL makes of a level. Coordinates are scene units, y up.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct LevelModel {
    /// The symbols the objects are drawn with.
    pub symbols: Vec<SymbolRef>,
    /// The instances, bundles and usages, drawn or not.
    pub objects: Vec<ObjectView>,
    /// The named points.
    pub points: Vec<PointView>,
    /// The wires that have their vertices.
    pub wires: Vec<WireView>,
    /// The texts with an anchor and something to show.
    pub texts: Vec<TextView>,
}

/// A symbol, by an id unique in the level's `symbols`: its name, or its name and where it is
/// defined after `@` when two of the level's symbols share the name.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SymbolRef {
    pub id: String,
    /// `$NMOS`, or `%ota5t` for a symbol defined in a level.
    pub name: String,
    /// The package that defines it.
    pub package: Option<String>,
    /// The level that defines it, by its id.
    pub level: Option<String>,
    /// The viewer's own stand-in box (9.2.6).
    pub default: bool,
}

/// A value and the statement it comes from (`source`, a key of `sources`; null for now).
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct Slot<T> {
    pub value: Option<T>,
    pub source: Option<String>,
}

impl<T> Slot<T> {
    fn of(value: Option<T>) -> Slot<T> {
        Slot { value, source: None }
    }
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ObjectView {
    /// Unique in the level: `#M1`, `%gnd1`, or `@file:line:col` where the statement that made an
    /// object without a name of its own is, with `#n` after it when it made several.
    pub id: String,
    /// "instance", "bundle" or "usage".
    pub kind: String,
    /// The host instances it draws: one for an instance, the members of a bundle, none for a usage.
    pub host: Vec<String>,
    /// A usage's definition, such as `$GROUND`.
    pub definition: Option<String>,
    /// A usage's roles, each with the id of what it is attached to, or a point.
    #[cfg_attr(test, ts(type = "Record<string, string | [number, number]> | null"))]
    pub attach: Option<Map<String, Value>>,
    /// The id of its symbol in `symbols`.
    pub symbol: Slot<String>,
    /// Its registration point; null when it has none yet.
    pub at: Slot<[f64; 2]>,
    /// "R0", "R90", "R180", "R270", "MX", "MY", "MXR90" or "MYR90".
    pub orient: Slot<String>,
    /// The symbol's pins where they are drawn, when it has a symbol and a position.
    pub pins: Vec<PinView>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PinView {
    /// An instance's terminals by number, `#M1:1`; a usage's roles, `p`.
    pub terminals: Vec<String>,
    pub at: [f64; 2],
    /// The exit direction after the orientation, in degrees: 0 right, 90 up, 180 left, 270 down.
    pub dir: Option<u16>,
    /// The nets at the pin, as ids.
    pub nets: Vec<String>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PointView {
    /// `%j1`.
    pub id: String,
    pub at: Slot<[f64; 2]>,
    /// A junction dot is drawn at it.
    pub junction: bool,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct WireView {
    pub id: String,
    pub nets: Vec<String>,
    /// The vertices as the statement writes them.
    pub via: Vec<Vertex>,
    /// The vertices after the routing mode.
    pub points: Vec<[f64; 2]>,
    pub mode: String,
    /// The style properties the document's records give it; a package's, such as the standard
    /// package's defaults, are left out.
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
}

/// A vertex of a wire.
#[derive(Serialize, Debug, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(untagged)]
pub enum Vertex {
    Terminal { terminal: String },
    Point { point: String },
    Usage { usage: String, role: String },
    At { at: [f64; 2] },
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct TextView {
    /// `#M1::name` for a label with a key; else as an object's.
    pub id: String,
    /// The id of the object it is attached to; null for a text at a point.
    pub attach: Option<String>,
    /// The key of a label, such as "name".
    pub key: Option<String>,
    /// The text as written, before `${…}` is read (13.6).
    pub markup: Slot<String>,
    /// The document's records write it, or a part of it; false for a label its symbol alone gives,
    /// such as the formula of a block.
    pub written: bool,
    /// What it shows, as the Document's texts give it.
    pub lines: Vec<Vec<Run>>,
    pub anchor: [f64; 2],
    pub halign: String,
    pub valign: String,
    pub rotate: f64,
    /// Its size, from the document or a package, when one gives it.
    pub size: Option<f64>,
    /// The style properties the document's records give it, such as `textcolor`, `bold` and
    /// `italic`; a package's are left out.
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
}

fn lit(v: &Lit) -> Value {
    match v {
        Lit::Num(n) => serde_json::json!(n),
        Lit::Str(s) => Value::String(s.as_str().to_string()),
        Lit::Bool(b) => Value::Bool(*b),
        Lit::Arr(items) => Value::Array(items.iter().map(lit).collect()),
    }
}

fn style(s: &Style) -> Map<String, Value> {
    s.iter().map(|(k, v)| (k.clone(), lit(v))).collect()
}

fn xy(p: Pt) -> [f64; 2] {
    [p.0, p.1]
}

/// The id of a level.
fn level_id(doc: &HostDocument, lv: usize) -> String {
    let level = &doc.levels[lv];
    match level.kind {
        LevelKind::Subckt => format!("subckt:{}", level.name),
        _ => format!("file:{}", level.file),
    }
}

/// The snapshot of an interpretation, with its Document.
pub(crate) fn snapshot(r: &Interpretation, document: Document, parts: Parts) -> Snapshot {
    let files: Vec<FileRef> =
        document.files.iter().map(|f| FileRef { path: f.path.clone(), name: f.name.clone() }).collect();
    let file_index: HashMap<&str, usize> =
        document.files.iter().enumerate().map(|(k, f)| (f.path.as_str(), k)).collect();
    let levels = r
        .model
        .levels
        .iter()
        .zip(&document.levels)
        .map(|(lv, dl)| {
            let host_level = &r.doc.levels[lv.level];
            LevelSnapshot {
                id: level_id(&r.doc, lv.level),
                kind: lv.kind,
                name: host_level.name.clone(),
                file: file_index.get(host_level.file.as_str()).copied(),
                line: if host_level.kind == LevelKind::Subckt { host_level.line } else { 0 },
                status: dl.status.clone(),
                host: parts.host.then(|| host(&r.doc, lv)),
                model: parts.model.then(|| level_model(&r.doc, lv)),
            }
        })
        .collect();
    let language = if r.model.language.is_empty() { "0.2".to_string() } else { r.model.language.clone() };
    Snapshot {
        schema: SCHEMA.into(),
        runtime: env!("CARGO_PKG_VERSION").into(),
        language,
        dialect: r.doc.dialect.name.clone(),
        packages: r
            .model
            .packages
            .iter()
            .map(|(name, version)| PackageUse { name: name.clone(), version: version.clone() })
            .collect(),
        files,
        levels,
        diagnostics: document.diagnostics.clone(),
        failed: r.model.failed.clone(),
        drawing: parts.drawing.then_some(document),
    }
}

/// The nets of a host level by key, each with the first way it is written: the ports, then the
/// instances' nodes in order.
fn host_nets(doc: &HostDocument, lv: usize) -> Vec<(String, String)> {
    let level = &doc.levels[lv];
    let mut out: Vec<(String, String)> = Vec::new();
    for written in level.ports.iter().chain(level.instances.iter().flat_map(|i| i.nodes.iter())) {
        let key = doc.net_key(written);
        if !out.iter().any(|(k, _)| *k == key) {
            let shown = if key == "0" { "0".to_string() } else { written.clone() };
            out.push((key, shown));
        }
    }
    out
}

fn host(doc: &HostDocument, lv: &LevelView) -> HostLevel {
    let level = &doc.levels[lv.level];
    let nets = host_nets(doc, lv.level);
    let net_id = |written: &str| {
        let key = doc.net_key(written);
        let shown = nets.iter().find(|(k, _)| *k == key).map(|(_, s)| s.as_str()).unwrap_or(written);
        format!("&{}", quoted(shown))
    };
    let port_keys: Vec<String> = level.ports.iter().map(|p| doc.net_key(p)).collect();
    HostLevel {
        ports: level.ports.iter().map(|p| net_id(p)).collect(),
        instances: level
            .instances
            .iter()
            .enumerate()
            .map(|(i, inst)| HostInstance {
                id: format!("#{}", quoted(&inst.name)),
                subckt: inst.is_subckt,
                master: inst.model.clone(),
                terminals: inst
                    .nodes
                    .iter()
                    .enumerate()
                    .map(|(k, node)| HostTerminal {
                        n: k as u32 + 1,
                        name: lv.terminal_names.get(i).and_then(|names| names.get(k).cloned().flatten()),
                        net: net_id(node),
                    })
                    .collect(),
            })
            .collect(),
        nets: nets
            .iter()
            .map(|(key, shown)| HostNet {
                id: format!("&{}", quoted(shown)),
                global: key == "0" || doc.globals.contains(key),
                port: port_keys.contains(key),
            })
            .collect(),
    }
}

/// The symbols of a level's objects with ids unique among them.
fn symbol_refs(doc: &HostDocument, lv: &LevelView) -> Vec<(SymbolInfo, SymbolRef)> {
    let mut seen: Vec<SymbolInfo> = Vec::new();
    for o in &lv.objects {
        if let Some(s) = &o.symbol {
            if !seen.contains(s) {
                seen.push(s.clone());
            }
        }
    }
    seen.iter()
        .map(|s| {
            let (package, level) = match &s.origin {
                SymbolOrigin::Package(p) => (Some(p.clone()), None),
                SymbolOrigin::Level(l) => (None, Some(level_id(doc, *l))),
                SymbolOrigin::None => (None, None),
            };
            let shared = seen.iter().filter(|t| t.name == s.name).count() > 1;
            let id = match (&package, &level, shared) {
                (_, _, false) => s.name.clone(),
                (Some(p), _, true) => format!("{}@{p}", s.name),
                (_, Some(l), true) => format!("{}@{l}", s.name),
                _ => format!("{}@default", s.name),
            };
            (s.clone(), SymbolRef { id, name: s.name.clone(), package, level, default: s.default })
        })
        .collect()
}

fn level_model(doc: &HostDocument, lv: &LevelView) -> LevelModel {
    let symbols = symbol_refs(doc, lv);
    let symbol_id = |s: &SymbolInfo| symbols.iter().find(|(info, _)| info == s).map(|(_, r)| r.id.clone());
    let net_id = |shown: &str| format!("&{}", quoted(shown));
    let objects = lv
        .objects
        .iter()
        .map(|o| ObjectView {
            id: o.id.clone(),
            kind: o.kind.to_string(),
            host: o.host.clone(),
            definition: o.definition.clone(),
            attach: o.definition.as_ref().map(|_| {
                o.attach
                    .iter()
                    .map(|(role, a)| {
                        let v = match a {
                            Attach::Object(id) => Value::String(id.clone()),
                            Attach::At(p) => serde_json::json!([p.0, p.1]),
                        };
                        (role.clone(), v)
                    })
                    .collect()
            }),
            symbol: Slot::of(o.symbol.as_ref().and_then(&symbol_id)),
            at: Slot::of(o.at.map(xy)),
            orient: Slot::of(Some(o.orient.as_str().to_string())),
            pins: o
                .pins
                .iter()
                .map(|p| PinView { terminals: p.terminals.clone(), at: xy(p.at), dir: p.dir, nets: p.nets.clone() })
                .collect(),
        })
        .collect();
    let points = lv
        .points
        .iter()
        .map(|p| PointView { id: p.id.clone(), at: Slot::of(p.at.map(xy)), junction: p.junction })
        .collect();
    let wires = lv
        .wires
        .iter()
        .map(|w| WireView {
            id: w.object_id.clone(),
            nets: w.nets.iter().map(|n| net_id(n)).collect(),
            via: w
                .via
                .iter()
                .map(|v| match v {
                    model::Vertex::Terminal(t) => Vertex::Terminal { terminal: t.clone() },
                    model::Vertex::Point(p) => Vertex::Point { point: p.clone() },
                    model::Vertex::Usage { usage, role } => Vertex::Usage { usage: usage.clone(), role: role.clone() },
                    model::Vertex::At(p) => Vertex::At { at: xy(*p) },
                })
                .collect(),
            points: w.points.iter().map(|p| xy(*p)).collect(),
            mode: w.mode.clone(),
            style: style(&w.own_style),
        })
        .collect();
    let shows = |lines: &[rnl_runtime::markup::Line]| {
        lines.iter().flatten().any(|r| match r {
            rnl_runtime::markup::Run::Text { text, .. } => !text.trim().is_empty(),
            rnl_runtime::markup::Run::Frac { .. } => true,
        })
    };
    let texts = lv
        .texts
        .iter()
        .filter(|t| shows(&t.lines))
        .map(|t| TextView {
            id: t.object_id.clone(),
            attach: t.attach.clone(),
            key: t.key.clone(),
            markup: Slot::of(Some(t.markup.clone())),
            written: t.written,
            lines: t.lines.iter().map(|l| l.iter().map(primitive::run).collect()).collect(),
            anchor: xy(t.anchor),
            halign: t.halign.clone(),
            valign: t.valign.clone(),
            rotate: t.rotate,
            size: match t.style.get("size") {
                Some(Lit::Num(n)) => Some(*n),
                _ => None,
            },
            style: style(&t.own_style),
        })
        .collect();
    LevelModel { symbols: symbols.into_iter().map(|(_, r)| r).collect(), objects, points, wires, texts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildOptions, MemoryProvider};
    use serde_json::json;

    const NETLIST: &str = r##"* snapshot
* @RNL(type="header") { @VERSION("0.2")}
.subckt amp in out VSS
* @RNL {
*   %amp = @DEFINE-SYMBOL(box=[-2, -2, 2, 2]) {
*     (-2, 0) @PIN(terminal="in", label="in", direction="left");
*     (2, 0) @PIN(terminal="out", direction="right");
*     (0, -2) @PIN(terminal="VSS", direction="down");
*     @RECTANGLE {(-1.5, -1.5) (1.5, 1.5)};
*     ? @TEXT("${name}", key="name") => @PLACE(0, 2.5)
*   };
*   # @SET-SYMBOL %amp
* }
R1 in out 1k
.ends amp
X1 a b 0 amp
R2 a vdd! 2k
* @RNL {
*   %amp = @DEFINE-SYMBOL(box=[-1, -2, 1, 2]) {
*     (0, 2) @PIN(terminal=1, direction="up");
*     (0, -2) @PIN(terminal=2, direction="down");
*     @LINE {(0, 2) (0, -2)}
*   };
*   #X1 @PLACE(10, 0);
*   #R2 @SET-SYMBOL %amp => @PLACE(0, 0) => @ORIENT("R90");
*   #R2 @TEXT("R_{2}", key="name", textcolor="#ff0000");
*   #R2 @TEXT("", key="value");
*   %j1 = (5, 0);
*   $JUNCTION @ATTACH %j1;
*   &a @WIRE(mode="straight") {#R2:1 %j1 #X1:in};
*   (0, 6) @TEXT("note")
* }
.end
"##;

    fn snap(parts: Parts) -> Value {
        let files = MemoryProvider::new().with("/w/t.sp", NETLIST);
        let s = crate::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap();
        serde_json::to_value(s).unwrap()
    }

    #[test]
    fn the_host_circuit_and_the_model_by_ids() {
        let s = snap(Parts { drawing: false, host: true, model: true });
        assert_eq!((&s["schema"], &s["language"], &s["dialect"]), (&json!(SCHEMA), &json!("0.2"), &json!("spice")));
        assert!(s.get("drawing").is_none());
        assert_eq!(s["packages"][0]["name"], "rnl-std");
        let file = s["files"][0]["path"].as_str().unwrap().to_string();
        let ids: Vec<&str> = s["levels"].as_array().unwrap().iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids, [format!("file:{file}").as_str(), "subckt:amp"]);

        let top = &s["levels"][0];
        let host = &top["host"];
        assert_eq!(host["instances"][0]["id"], "#X1");
        assert_eq!(host["instances"][0]["subckt"], true);
        assert_eq!(
            host["instances"][0]["terminals"],
            json!([{"n": 1, "name": "in", "net": "&a"}, {"n": 2, "name": "out", "net": "&b"}, {"n": 3, "name": "VSS", "net": "&0"}])
        );
        // a name that needs quotes is quoted; 0 is global
        assert_eq!(host["instances"][1]["terminals"][1]["net"], "&\"vdd!\"");
        let nets = host["nets"].as_array().unwrap();
        assert!(nets.contains(&json!({"id": "&\"vdd!\"", "global": false, "port": false})), "{nets:?}");
        assert!(nets.contains(&json!({"id": "&0", "global": true, "port": false})), "{nets:?}");
        assert_eq!(s["levels"][1]["host"]["ports"], json!(["&in", "&out", "&VSS"]));

        let model = &top["model"];
        // two symbols of one name: the subcircuit's and the file's, told apart by where they are defined
        let symbols = model["symbols"].as_array().unwrap();
        let symbol = |id: &str| symbols.iter().find(|x| x["id"] == id).cloned();
        assert_eq!(symbol("%amp@subckt:amp").unwrap()["level"], "subckt:amp");
        assert_eq!(symbol(&format!("%amp@file:{file}")).unwrap()["package"], Value::Null);
        let objects = model["objects"].as_array().unwrap();
        let object = |id: &str| objects.iter().find(|o| o["id"] == id).cloned().unwrap_or(Value::Null);
        assert_eq!(object("#X1")["symbol"]["value"], "%amp@subckt:amp");
        assert_eq!(object("#X1")["at"]["value"], json!([10.0, 0.0]));
        let r2 = object("#R2");
        assert_eq!((&r2["orient"]["value"], &r2["host"]), (&json!("R90"), &json!(["#R2"])));
        // the pins where they are drawn, the terminals by number
        assert_eq!(r2["pins"][0], json!({"terminals": ["#R2:1"], "at": [-2.0, 0.0], "dir": 180, "nets": ["&a"]}));
        // every id once
        let mut all: Vec<&str> = objects.iter().map(|o| o["id"].as_str().unwrap()).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "{objects:?}");
        let junction = objects.iter().find(|o| o["definition"] == "$JUNCTION").unwrap();
        assert_eq!(junction["attach"], json!({"p": "%j1"}));
        assert!(junction["id"].as_str().unwrap().starts_with(&format!("@{file}:")), "{junction}");

        assert_eq!(
            model["points"],
            json!([{"id": "%j1", "at": {"value": [5.0, 0.0], "source": null}, "junction": true}])
        );
        // a wire's vertices as written, a terminal by number even when written by name
        assert_eq!(model["wires"][0]["via"], json!([{"terminal": "#R2:1"}, {"point": "%j1"}, {"terminal": "#X1:1"}]));
        assert_eq!(model["wires"][0]["nets"], json!(["&a"]));
        assert_eq!(model["wires"][0]["style"], json!({}));

        let texts = model["texts"].as_array().unwrap();
        let name = texts.iter().find(|t| t["id"] == "#R2::name").unwrap();
        assert_eq!(
            (&name["attach"], &name["key"], &name["markup"]["value"]),
            (&json!("#R2"), &json!("name"), &json!("R_{2}"))
        );
        // only what the document gives, not the standard package's defaults
        assert_eq!(name["style"], json!({"textcolor": "#ff0000"}));
        assert_eq!(name["written"], true);
        // a label the symbol alone gives is there, but not written by the document
        let x1 = texts.iter().find(|t| t["id"] == "#X1::name").unwrap();
        assert_eq!((&x1["written"], &x1["lines"][0][0]["text"]), (&json!(false), &json!("X1")));
        // a label emptied is not there; an annotation goes by where it is written
        assert!(texts.iter().all(|t| t["id"] != "#R2::value"), "{texts:?}");
        let note = texts.iter().find(|t| t["lines"][0][0]["text"] == "note").unwrap();
        assert_eq!(note["attach"], Value::Null);
        assert!(note["id"].as_str().unwrap().starts_with(&format!("@{file}:")), "{note}");
    }

    #[test]
    fn only_the_parts_asked_for() {
        let s = snap(Parts::default());
        assert!(s["drawing"]["levels"].is_array());
        assert!(s["levels"][0].get("host").is_none() && s["levels"][0].get("model").is_none());
        let s = snap(Parts { drawing: false, host: true, model: false });
        assert!(s.get("drawing").is_none() && s["levels"][0].get("model").is_none());
        assert!(s["levels"][0]["host"].is_object());
    }
}
