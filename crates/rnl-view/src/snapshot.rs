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
use rnl_runtime::interp::obj::{Lit, PrimParams, Shape};
use rnl_runtime::interp::unit::DEFAULT_VERSION;
use rnl_runtime::model::{self, quoted, Attach, LevelView, Style, SymbolInfo, SymbolOrigin};
use rnl_runtime::numbering::{hex, sha256};
use rnl_runtime::{Interpretation, Options};
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
    /// The fingerprint of what it is made from (3.7), 32 hexadecimal digits: the texts of the host
    /// files and of the packages read, and the build's options. The same for the same texts and
    /// options, whatever their line endings; new after any change to a text, a space included.
    pub revision: String,
    /// "spice" or "spectre".
    pub dialect: String,
    /// The packages in effect, by name and version, in the order they are brought in: those of
    /// the standard package first, the packages it references before it (16.2, 16.3).
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
    /// The symbols the objects are drawn with, and the level's own.
    pub symbols: Vec<SymbolRef>,
    /// A subcircuit's own symbol, `#` (9.2.6), by its id in `symbols`: what its instances are drawn
    /// with when they are given no other. Null for a file level, and for a subcircuit without one.
    pub symbol: Slot<String>,
    /// The instances, bundles and usages, drawn or not.
    pub objects: Vec<ObjectView>,
    /// The named points.
    pub points: Vec<PointView>,
    /// The wires that have their vertices.
    pub wires: Vec<WireView>,
    /// The texts with an anchor and something to show.
    pub texts: Vec<TextView>,
    /// The graphics of the scene (13.4) that have their points, in the order written.
    pub shapes: Vec<ShapeView>,
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
    /// For a symbol the document defines, such as a block's (9.2.6), what it is made of, in its
    /// own coordinates; for a package's symbol, null: its name says it.
    pub definition: Option<SymbolDefinition>,
}

/// A symbol's box, pins and graphics, in its own coordinates, y up.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SymbolDefinition {
    /// x1, y1, x2, y2: the box the symbol writes, or the extent of its graphics.
    #[serde(rename = "box")]
    pub bbox: [f64; 4],
    /// The symbol writes its box.
    pub box_written: bool,
    pub pins: Vec<SymbolPin>,
    /// Its graphics, texts left out, in the order written.
    pub prims: Vec<SymbolPrim>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SymbolPin {
    /// The terminals at the pin, by number or name as the symbol writes them.
    pub terminals: Vec<String>,
    pub at: [f64; 2],
    /// The exit direction in degrees: 0 right, 90 up, 180 left, 270 down.
    pub dir: Option<u16>,
    /// The pin's name for display (9.2.4): as written, and as lines of runs.
    pub label: Option<PinLabel>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PinLabel {
    pub markup: String,
    pub lines: Vec<Vec<Run>>,
}

/// A graphic of a symbol (9.2.5).
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct SymbolPrim {
    /// "line", "rectangle", "circle", "ellipse", "arc" or "bezier".
    pub shape: String,
    /// The centre of a rectangle, circle or ellipse with its size, else its points.
    pub points: Vec<[f64; 2]>,
    /// Its parameters: width, height, radius, fill, arrow.
    #[cfg_attr(test, ts(type = "Record<string, string | number>"))]
    pub params: Map<String, Value>,
    /// The style properties it writes.
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
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

/// A graphic of the scene (13.4), as an `add-shape` op writes one.
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct ShapeView {
    /// `%box1` when the level names it; else where the statement that made it is, as an object's.
    pub id: String,
    /// "line", "rectangle", "circle", "ellipse", "arc" or "bezier".
    pub shape: String,
    /// The centre of a rectangle, circle or ellipse with its size, else its points; a point given
    /// by a reference, such as `%j1`, where it is.
    pub points: Vec<[f64; 2]>,
    /// Its parameters: width, height, radius, fill, arrow.
    #[cfg_attr(test, ts(type = "Record<string, string | number>"))]
    pub params: Map<String, Value>,
    /// The style properties the document's records give it; a package's are left out.
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
    /// "bottom", "middle" or "top".
    pub layer: String,
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
pub(crate) fn snapshot(r: &Interpretation, document: Document, parts: Parts, options: &Options) -> Snapshot {
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
    let language = if r.model.language.is_empty() { DEFAULT_VERSION.to_string() } else { r.model.language.clone() };
    Snapshot {
        schema: SCHEMA.into(),
        runtime: env!("CARGO_PKG_VERSION").into(),
        language,
        revision: revision(r, options),
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

/// The fingerprint of what a snapshot is made from (3.7): every file read, the host files and the
/// packages, by name and text with its line endings made "\n"; the dialect; the options that change
/// what the texts give: the standard package at the root, the generated records and the tolerance.
/// Each part has its length before it, so that no two inputs give the same bytes. The draft switch,
/// which has no effect, is written as off, as the revisions made before it lost its effect have
/// it, so that the same texts keep their revisions.
fn revision(r: &Interpretation, options: &Options) -> String {
    let mut files: Vec<(&str, String)> =
        r.sources.iter().map(|(name, text)| (name.as_str(), text.replace("\r\n", "\n").replace('\r', "\n"))).collect();
    files.sort();
    files.dedup();
    let reading =
        format!("std={} generated={} draft=false tolerance={}", options.std, options.generated, options.tolerance);
    let mut bytes = Vec::new();
    let mut field = |part: &[u8]| {
        bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
        bytes.extend_from_slice(part);
    };
    field(b"rnl-revision/1");
    field(r.doc.dialect.name.as_bytes());
    field(reading.as_bytes());
    for (name, text) in &files {
        field(name.as_bytes());
        field(text.as_bytes());
    }
    hex(&sha256(&bytes)[..16])
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

/// The symbols of a level's objects, and the level's own, with ids unique among them.
fn symbol_refs(doc: &HostDocument, lv: &LevelView) -> Vec<(SymbolInfo, SymbolRef)> {
    let mut seen: Vec<SymbolInfo> = Vec::new();
    for s in lv.objects.iter().filter_map(|o| o.symbol.as_ref()).chain(lv.block_symbol.as_ref().map(|b| &b.info)) {
        if !seen.contains(s) {
            seen.push(s.clone());
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
            // what a symbol the document defines is made of, from an object drawn with it, or from
            // the level that gives it itself
            let drawn = lv.objects.iter().find(|o| o.symbol.as_ref() == Some(s)).and_then(|o| o.symbol_view);
            let view = match (drawn, &lv.block_symbol) {
                (Some(k), _) => Some(&lv.symbols[k]),
                (None, Some(b)) if &b.info == s => Some(&b.view),
                _ => None,
            };
            let definition = view.filter(|_| package.is_none() && !s.default).map(definition);
            (s.clone(), SymbolRef { id, name: s.name.clone(), package, level, default: s.default, definition })
        })
        .collect()
}

fn definition(sv: &rnl_runtime::model::SymbolView) -> SymbolDefinition {
    SymbolDefinition {
        bbox: sv.bbox,
        box_written: sv.box_written,
        pins: sv
            .pins
            .iter()
            .map(|p| SymbolPin {
                terminals: p.terminals.clone(),
                at: xy(p.pos),
                dir: p.dir,
                label: p.label.as_ref().map(|m| PinLabel {
                    markup: m.clone(),
                    lines: rnl_runtime::markup::parse(m)
                        .iter()
                        .map(|l| l.iter().map(primitive::run).collect())
                        .collect(),
                }),
            })
            .collect(),
        prims: sv
            .prims
            .iter()
            .filter(|p| p.shape != Shape::Text)
            .map(|p| SymbolPrim {
                shape: p.shape.as_str().to_lowercase(),
                points: p.points.iter().map(|q| xy(*q)).collect(),
                params: params(&p.params),
                style: style(&p.style),
            })
            .collect(),
    }
}

/// The parameters a graphic gives (9.2.5).
fn params(pp: &PrimParams) -> Map<String, Value> {
    let mut params = Map::new();
    for (k, v) in [("width", pp.width), ("height", pp.height), ("radius", pp.radius)] {
        if let Some(v) = v {
            params.insert(k.into(), serde_json::json!(v));
        }
    }
    for (k, v) in [("fill", &pp.fill), ("arrow", &pp.arrow)] {
        if let Some(v) = v {
            params.insert(k.into(), Value::String(v.clone()));
        }
    }
    params
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
            symbol: Slot::of(o.symbol.as_ref().and_then(symbol_id)),
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
    // a wire some of whose points have no position is drawn in runs; it has not all its vertices
    let wires = lv
        .wires
        .iter()
        .filter(|w| w.whole)
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
    let shapes = lv
        .shapes
        .iter()
        .filter(|s| s.shape != Shape::Text)
        .map(|s| ShapeView {
            id: s.object_id.clone(),
            shape: s.shape.as_str().to_lowercase(),
            points: s.points.iter().map(|p| xy(*p)).collect(),
            params: params(&s.params),
            style: style(&s.own_style),
            layer: s.layer.as_str().to_string(),
        })
        .collect();
    let symbol = Slot::of(lv.block_symbol.as_ref().and_then(|b| symbol_id(&b.info)));
    LevelModel { symbols: symbols.into_iter().map(|(_, r)| r).collect(), symbol, objects, points, wires, texts, shapes }
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
*   # @SET-SYMBOL %amp;
*   %p1 = (1, 1);
*   (0, 3) @CIRCLE(radius=0.5, fill="fillcolor", fillcolor="#eef4ff") => @LAYER("bottom");
*   @LINE(arrow="end", linecolor="#00ff00") {(0, 0) %p1} => %arrow1
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
        // as they are brought in: the standard package after the sub-packages it references (16.3)
        let packages = s["packages"].as_array().unwrap();
        let names: Vec<&str> = packages.iter().map(|p| p["name"].as_str().unwrap()).collect();
        let std = ["style", "symbols", "virtual", "core", "fill", "semantics", "layout"];
        assert_eq!(names.len(), std.len() + 1);
        assert!(std.iter().zip(&names).all(|(p, n)| n.strip_prefix("rnl-std-") == Some(*p)), "{names:?}");
        assert_eq!(names[std.len()], "rnl-std");
        assert!(packages.iter().all(|p| p["version"] == "0.2"), "{packages:?}");
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
        // a symbol the document defines comes with what it is made of: box, pins with their
        // names, graphics; a package's does not
        let block = symbol("%amp@subckt:amp").unwrap();
        let def = &block["definition"];
        assert_eq!((&def["box"], &def["box_written"]), (&json!([-2.0, -2.0, 2.0, 2.0]), &json!(true)));
        assert_eq!(
            def["pins"][0],
            json!({"terminals": ["in"], "at": [-2.0, 0.0], "dir": 180,
                   "label": {"markup": "in", "lines": [[{"kind": "text", "text": "in", "style": {"level": 0, "bold": false, "italic": false, "overline": false}}]]}})
        );
        assert_eq!(def["pins"][1]["label"], Value::Null);
        assert_eq!(
            def["prims"],
            json!([{"shape": "rectangle", "points": [[-1.5, -1.5], [1.5, 1.5]], "params": {}, "style": {}}])
        );
        assert!(symbols.iter().filter(|x| x["package"] != Value::Null).all(|x| x["definition"].is_null()));
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
    fn a_subcircuits_own_symbol_and_the_graphics_of_the_scene() {
        let s = snap(Parts { drawing: false, host: false, model: true });
        let file = s["files"][0]["path"].as_str().unwrap().to_string();
        let (top, amp) = (&s["levels"][0]["model"], &s["levels"][1]["model"]);
        // the subcircuit's own symbol with what it is made of, though nothing in the level is drawn
        // with it
        assert_eq!(amp["symbol"], json!({"value": "%amp", "source": null}));
        let own = amp["symbols"].as_array().unwrap().iter().find(|x| x["id"] == "%amp").cloned().unwrap();
        assert_eq!((&own["level"], &own["definition"]["box"]), (&json!("subckt:amp"), &json!([-2.0, -2.0, 2.0, 2.0])));
        assert_eq!(own["definition"]["pins"].as_array().unwrap().len(), 3);
        assert_eq!(top["symbol"]["value"], Value::Null);
        // the graphics in the order written: their params, the style the document gives, the layer;
        // a point given by a reference where it is
        let shapes = amp["shapes"].as_array().unwrap();
        assert_eq!(shapes.len(), 2, "{shapes:?}");
        assert!(shapes[0]["id"].as_str().unwrap().starts_with(&format!("@{file}:")), "{shapes:?}");
        assert_eq!(
            (&shapes[0]["shape"], &shapes[0]["points"], &shapes[0]["params"], &shapes[0]["style"], &shapes[0]["layer"]),
            (
                &json!("circle"),
                &json!([[0.0, 3.0]]),
                &json!({"radius": 0.5, "fill": "fillcolor"}),
                &json!({"fillcolor": "#eef4ff"}),
                &json!("bottom")
            )
        );
        assert_eq!(
            shapes[1],
            json!({"id": "%arrow1", "shape": "line", "points": [[0.0, 0.0], [1.0, 1.0]], "params": {"arrow": "end"},
                   "style": {"linecolor": "#00ff00"}, "layer": "middle"})
        );
        assert_eq!(top["shapes"], json!([]));
    }

    #[test]
    fn the_revision_follows_the_texts_and_the_options() {
        let rev = |text: &str, options: &BuildOptions| {
            let files = MemoryProvider::new().with("/w/t.sp", text);
            crate::snapshot("/w/t.sp", &files, options, Parts::default()).unwrap().revision
        };
        let options = BuildOptions::default();
        let r = rev(NETLIST, &options);
        assert_eq!(r.len(), 32, "{r}");
        assert!(r.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{r}");
        assert_eq!(rev(NETLIST, &options), r);
        // the line endings do not count; a space does, and so do the options
        assert_eq!(rev(&NETLIST.replace('\n', "\r\n"), &options), r);
        assert_ne!(rev(&NETLIST.replace("R1 in out 1k", "R1 in out  1k"), &options), r);
        let no_std = BuildOptions { runtime: Options { std: false, ..Options::default() }, ..BuildOptions::default() };
        assert_ne!(rev(NETLIST, &no_std), r);
        // the draft switch has no effect, and so does not change the revision
        let draft = BuildOptions { runtime: Options { draft: true, ..Options::default() }, ..BuildOptions::default() };
        assert_eq!(rev(NETLIST, &draft), r);
        let spectre = BuildOptions { dialect: Some("spectre".into()), ..BuildOptions::default() };
        assert_ne!(rev(NETLIST, &spectre), r);
    }

    #[test]
    fn the_revision_counts_files_of_one_name_in_two_package_directories() {
        // two packages whose files are both pk.rnl, in two package directories, as the files of two
        // standard series are: the text of each counts
        let netlist = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); \
                       @USE-PACKAGE(name=\"pa\", version=\"1.0\"); @USE-PACKAGE(name=\"pb\", version=\"1.0\")}\n\
                       R1 a 0 1k\n.end\n";
        let package = |name: &str, comment: &str| {
            format!("/* {comment} */\n@RNL(type=\"header\") {{ @VERSION(\"0.3\"); @PACKAGE(name=\"{name}\", version=\"1.0\")}}\n")
        };
        let snapshot = |second: &str| {
            let mut files = MemoryProvider::new()
                .with("/w/t.sp", netlist)
                .with("/p1/pk.rnl", &package("pa", "the first"))
                .with("/p2/pk.rnl", &package("pb", second));
            files.package_dirs = vec!["/p1".into(), "/p2".into()];
            crate::snapshot("/w/t.sp", &files, &BuildOptions::default(), Parts::default()).unwrap()
        };
        let s = snapshot("the second");
        let names: Vec<&str> = s.packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"pa") && names.contains(&"pb"), "{names:?}");
        assert_ne!(snapshot("the second, changed").revision, s.revision);
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

    #[test]
    fn a_wire_through_a_point_without_a_position_is_drawn_in_runs_but_not_given() {
        // the free point has no position: the drawing has the run from R1:2 to (4, 0) and a
        // flyline from there to R2:1, the snapshot only the wire that has all its vertices
        let netlist = "* runs
R1 a b 1k
R2 b c 1k
* @RNL {
*   #R1 @PLACE(0, 0); #R2 @PLACE(10, 0);
*   &b @WIRE {#R1:2 (4, 0) (_, _) #R2:1};
*   &a @WIRE {#R1:1 (-4, 0)}
* }
.end
";
        let files = MemoryProvider::new().with("/w/r.sp", netlist);
        let r = crate::interpret("/w/r.sp", Some("spice"), &files, &rnl_runtime::Options::default()).unwrap();
        let runs: Vec<&str> = r.model.levels[0].wires.iter().map(|w| w.nets[0].as_str()).collect();
        assert_eq!(runs, ["b", "a"], "the run of &b is drawn");
        let parts = Parts { drawing: false, host: false, model: true };
        let s = crate::snapshot("/w/r.sp", &files, &BuildOptions::default(), parts).unwrap();
        let s = serde_json::to_value(s).unwrap();
        let given = s["levels"][0]["model"]["wires"].as_array().unwrap();
        assert_eq!(given.len(), 1, "{given:?}");
        assert_eq!(given[0]["nets"], json!(["&a"]));
    }
}
