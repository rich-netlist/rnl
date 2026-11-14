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

use crate::document::{Diagnostic, Document, RoleOf, RuleAt};
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
    /// The format the file was read with, as the Document's `SourceFile::format`: only when the
    /// netlist's files are of more than one format, or the request set a format for some.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub format: Option<String>,
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
    /// The usages it is a member of in the level ([`UsageOf`]); only for a net that is in some.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<UsageOf>>", optional))]
    pub usages: Vec<UsageOf>,
}

/// A usage an object is a member of, in the object's level, by the usage's id: what the Document
/// gives as `Object::usages` (`MemberOf`). The usage's own object view lists the usages it is a
/// member of in turn: the chain upward.
#[derive(Serialize, Debug, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct UsageOf {
    /// "usage", or "group", a set marked with GROUP (11.5), by its id in `groups`.
    pub kind: String,
    /// The usage's id, as its object view has it: `%dp1`, `@file:line:col`. A failed application,
    /// which made no usage, has the name it binds, or where it is written.
    pub id: String,
    /// Its definition, such as `$DIFFERENTIAL_PAIR`, or in a 0.4 level, `%dot` for one a level defines, whose `%` a level before 0.4 leaves out;
    /// empty for a group, and when a failed statement names none.
    pub definition: String,
    /// The roles the object fills in it: for a group, `members`; none for a group within this one,
    /// as `MemberOf::roles`.
    pub roles: Vec<RoleOf>,
    /// The statement that applies the definition: its file, by index in `files` when it is one of
    /// them, and its first line.
    pub file: Option<usize>,
    pub line: u32,
    /// "written", "solver", "rule" or "body", as `MemberOf::source`.
    pub source: String,
    /// For a usage a rule made: the rule's package and where the rule is written.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub rule: Option<RuleAt>,
    /// For a usage the body of a definition made: the usage whose definition it is, by id.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub made_in: Option<String>,
    /// "in-effect", "failed", "overridden" or "disabled", as `MemberOf::state`.
    pub state: String,
    /// Why it failed or was overridden.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub reason: Option<String>,
}

/// A set marked with GROUP (11.5), which a viewer lists with the usages.
#[derive(Serialize, Debug, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct GroupView {
    /// Whole grid-unit addition to the solver's default spacing (11.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub spacing_x: Option<f64>,

    /// Whole grid-unit addition to the solver's default spacing (11.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub spacing_y: Option<f64>,

    /// `%core` when a record of the level names it; otherwise where GROUP is written,
    /// `@file:line:col`, with `#n` after it when several groups have that place.
    pub id: String,
    /// Its members in order, each an id or a point, nested sets followed down.
    #[cfg_attr(test, ts(type = "(string | [number, number])[]"))]
    pub members: Vec<Value>,
    /// The statement that gives GROUP: its file, by index in `files` when it is one of them, and
    /// its first line.
    pub file: Option<usize>,
    pub line: u32,
    /// "written", "solver", "rule" or "body", as `UsageOf::source`.
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub rule: Option<RuleAt>,
    /// For a group the body of a definition made: the usage whose definition it is, by id.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub made_in: Option<String>,
    /// "in-effect", or "disabled" when PRIORITY turns it off (11.5, 14.8).
    pub state: String,
    /// The groups it is within, the smallest ones, and the usages it fills a role of
    /// ([`UsageOf`]); only for a group that has some.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<UsageOf>>", optional))]
    pub usages: Vec<UsageOf>,
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
    /// Compound annotations retain their identity and local graphics.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<AnnotationView>>", optional))]
    pub annotations: Vec<AnnotationView>,
    /// The sets marked with GROUP (11.5), in the order of writing; only for a level that has some.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<GroupView>>", optional))]
    pub groups: Vec<GroupView>,
    /// For a 0.4 level, the definition of its junction dots, which SOLVER-CONFIG `junction` selects
    /// (14.10), readable before any solve; absent before 0.4, and when none is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub junction: Option<JunctionView>,
}

/// The junction dots of a 0.4 level ([`LevelModel::junction`]).
#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct JunctionView {
    /// The definition, as a usage names it: `$JUNCTION`, or `%dot` for one a level defines.
    pub definition: String,
    /// The symbol its dots in the level are drawn with, by its id in `symbols`, whose `definition`
    /// gives its graphics, such as the radius of its circle; null while the level has no dot yet,
    /// such as before a solve, as the symbol a rule gives a usage is only known for one.
    pub symbol: Option<String>,
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
    /// Local names of a 0.4 pin, in binding order; empty for an unnamed pin.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub names: Option<Vec<String>>,
    /// Whether the binding is a bus, including a one-element array.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub bus: Option<bool>,
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
    /// The owner id of a 0.4 usage's pin attachment; absent without a unique structural host.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub pin_of: Option<String>,
    /// The enclosing 0.4 usage's id. Follow this chain for inherited pin attachment.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub parent: Option<String>,
    /// Unique in the level: `#M1`, `%gnd1`, or `@file:line:col` where the statement that made an
    /// object without a name of its own is, with `#n` after it when it made several.
    pub id: String,
    /// "instance", "bundle" or "usage".
    pub kind: String,
    /// The host instances it draws: one for an instance, the members of a bundle, none for a usage.
    pub host: Vec<String>,
    /// A usage's definition, such as `$GROUND`, or in a 0.4 level, `%dot` for one a level defines, whose `%` a level before 0.4 leaves out.
    pub definition: Option<String>,
    /// A usage's roles, each with the id of what it is attached to, or a point; a set role (9.4)
    /// with the list of its members, each an id or a point, nested sets followed down, empty when the
    /// role is left empty.
    #[cfg_attr(test, ts(type = "Record<string, string | [number, number] | (string | [number, number])[]> | null"))]
    pub attach: Option<Map<String, Value>>,
    /// A usage's parameters as the statement that applies it writes them, each value of the type it
    /// is written in (a string, a number, a truth value, or a list of them); the definition's
    /// defaults are not among them. Only for a usage that writes some.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional, type = "Record<string, string | number | boolean | unknown[]>"))]
    pub params: Option<Map<String, Value>>,
    /// The style properties the document's records give it (9.3), each as it wins in the style
    /// queue (8.3), such as `linecolor`; a package's, such as the standard package's defaults, are
    /// left out. Only for an object that the document gives some.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional, type = "Record<string, string | number | boolean>"))]
    pub style: Option<Map<String, Value>>,
    /// The id of its symbol in `symbols`.
    pub symbol: Slot<String>,
    /// Its registration point; null when it has none yet.
    pub at: Slot<[f64; 2]>,
    /// "R0", "R90", "R180", "R270", "MX", "MY", "MXR90" or "MYR90".
    pub orient: Slot<String>,
    /// The assigned symbol's pins. In 0.4 these include unplaced pins with a null scene position.
    pub pins: Vec<PinView>,
    /// The usages it is a member of in the level, with the roles it fills, those of its terminals
    /// too for an instance ([`UsageOf`]); only for an object that is in some.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(test, ts(as = "Option<Vec<UsageOf>>", optional))]
    pub usages: Vec<UsageOf>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct PinView {
    /// The independent pin object kind, present in 0.4.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional, type = "\"pin\""))]
    pub kind: Option<&'static str>,
    /// Opaque editor id. Use `reference`, when present, for writing RNL source.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub id: Option<String>,
    /// Zero-based physical index in the owner's assigned symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub bus: Option<bool>,
    /// A source expression that reaches this exact pin; absent when none is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub reference: Option<String>,
    /// Symbol-local position, before the owner's orientation and translation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub local_at: Option<[f64; 2]>,
    /// An instance's terminals by number, `#M1:1`; a usage's roles, `p`.
    pub terminals: Vec<String>,
    /// Scene position; null when the owner has no position yet.
    pub at: Option<[f64; 2]>,
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
pub struct AnnotationView {
    pub id: String,
    pub source_kind: String,
    pub source: String,
    pub anchor_kind: String,
    pub anchor_object: Option<String>,
    pub anchor: [f64; 2],
    pub rotate: f64,
    pub mirror: bool,
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
    pub layer: String,
    pub graphics: Vec<AnnotationGraphic>,
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct AnnotationGraphic {
    pub shape: String,
    pub points: Vec<[f64; 2]>,
    #[cfg_attr(test, ts(type = "Record<string, string | number>"))]
    pub params: Map<String, Value>,
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
    pub lines: Vec<Vec<Run>>,
    pub text: Option<String>,
    pub text_refs: Vec<String>,
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub rotate: Option<f64>,
    pub fix_rotate: bool,
    pub upright: Option<bool>,
    /// For a text, false when its style in effect, its own over the annotation's, hides it
    /// (`textvisible=false`, 9.3); absent when it is shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub visible: Option<bool>,
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
    Pin { owner: String, pin: usize, reference: Option<String> },
    Point { point: String },
    Usage { usage: String, role: String },
    At { at: [f64; 2] },
}

#[derive(Serialize, Debug)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct TextView {
    /// `#M1::name` for a label with a key; else as an object's.
    pub id: String,
    /// The id of the object it is attached to, or of the point, for a text at a point the level
    /// names, such as `%j1`; null for a text at a point written in place, such as `(10, 0)`, or at
    /// one that the level does not name alone.
    pub attach: Option<String>,
    /// For a text attached to a point, named or written in place, the point's position `[x, y]`:
    /// what it hangs on even where `attach` is null. Null for a text at no point, and while its
    /// point has no position yet, such as `(_, 7)`.
    pub attach_point: Option<[f64; 2]>,
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
    /// Effective fixed rotation, when supplied by a TEXT statement.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub fix_rotate: Option<bool>,
    /// Effective upright, when supplied by a TEXT statement.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub upright: Option<bool>,
    /// False when its style in effect hides it, `textvisible=false` (9.3), whether the document or a
    /// package gives it, as a package's rule hiding default labels does; absent when it is shown.
    /// A hidden text takes no room and is not drawn, but its content stays.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, ts(optional))]
    pub visible: Option<bool>,
    /// Its size, from the document or a package, when one gives it.
    pub size: Option<f64>,
    /// The style properties the document's records give it, such as `textcolor`, `bold` and
    /// `italic`; a package's are left out.
    #[cfg_attr(test, ts(type = "Record<string, string | number | boolean>"))]
    pub style: Map<String, Value>,
}

/// A literal (3.7) as JSON.
pub(crate) fn lit(v: &Lit) -> Value {
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

/// The snapshot of an interpretation, with its Document; `set`, the request set formats for files.
pub(crate) fn snapshot(r: &Interpretation, document: Document, parts: Parts, options: &Options, set: bool) -> Snapshot {
    // each file's format, only when they are not all one or some was set, so that a snapshot of a
    // netlist of one format is what it has been
    let formats = set || r.doc.mixed();
    let files: Vec<FileRef> = document
        .files
        .iter()
        .map(|f| FileRef { path: f.path.clone(), name: f.name.clone(), format: f.format.clone().filter(|_| formats) })
        .collect();
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
                host: parts.host.then(|| host(&r.doc, lv, &usages_of(&r.doc, &document, lv.level, &file_index))),
                model: parts.model.then(|| {
                    let usages = usages_of(&r.doc, &document, lv.level, &file_index);
                    let groups = groups_of(lv, dl, &file_index, &usages);
                    level_model(&r.doc, lv, &usages, groups)
                }),
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
/// what the texts give: the standard package at the root and the version asked for, if any, the
/// generated records and the tolerance. Each part has its length before it, so that no two inputs
/// give the same bytes. `draft=false` stays in the text, as the revisions made while there was a
/// switch for the draft 0.3 have it, so that the same texts keep their revisions.
fn revision(r: &Interpretation, options: &Options) -> String {
    let mut files: Vec<(&str, String)> =
        r.sources.iter().map(|(name, text)| (name.as_str(), text.replace("\r\n", "\n").replace('\r', "\n"))).collect();
    files.sort();
    files.dedup();
    let mut reading =
        format!("std={} generated={} draft=false tolerance={}", options.std, options.generated, options.tolerance);
    // only when asked for, so that the revisions made before the option keep their values
    if let Some(v) = &options.std_version {
        reading.push_str(&format!(" std_version={v}"));
    }
    if options.include_std {
        reading.push_str(" include_std=true");
    }
    let mut bytes = Vec::new();
    let mut field = |part: &[u8]| {
        bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
        bytes.extend_from_slice(part);
    };
    field(b"rnl-revision/1");
    field(r.doc.dialect.name.as_bytes());
    field(reading.as_bytes());
    // the format of each file, only when they are not all one, so that the revision of a netlist
    // of one format is what it has been
    if r.doc.mixed() {
        let mut formats: Vec<String> = r.doc.file_formats.iter().map(|f| format!("{}={}", f.file, f.format)).collect();
        formats.sort();
        field(format!("formats {}", formats.join("\n")).as_bytes());
    }
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
    let ports = level.ports.iter().map(|p| (p, &level.file));
    let nodes = level.instances.iter().flat_map(|i| i.nodes.iter().map(move |n| (n, &i.file)));
    for (written, file) in ports.chain(nodes) {
        let key = doc.net_key_in(lv, written, file);
        if !out.iter().any(|(k, _)| *k == key) {
            let shown = if key == "0" { "0".to_string() } else { written.clone() };
            out.push((key, shown));
        }
    }
    out
}

/// The usages each object of a level is a member of ([`UsageOf`]), by the object's id in the
/// snapshot: `#M1`, `%dp1`, and a net by its key in the level.
fn usages_of(
    doc: &HostDocument,
    document: &Document,
    level: usize,
    files: &HashMap<&str, usize>,
) -> HashMap<String, Vec<UsageOf>> {
    let level_file = &doc.levels[level].file;
    document
        .objects
        .iter()
        .filter(|o| o.level == level && !o.usages.is_empty())
        .map(|o| {
            let key = match o.kind {
                "instance" => format!("#{}", quoted(&o.name)),
                "net" => doc.net_key_in(level, &o.name, level_file),
                _ => o.name.clone(),
            };
            let list = o
                .usages
                .iter()
                .map(|m| UsageOf {
                    kind: m.kind.to_string(),
                    id: m.id.clone(),
                    definition: m.definition.clone(),
                    roles: m.roles.clone(),
                    file: m.file.as_deref().and_then(|f| files.get(f).copied()),
                    line: m.line,
                    source: m.source.to_string(),
                    rule: m.rule.clone(),
                    made_in: m.made_in.clone(),
                    state: m.state.to_string(),
                    reason: m.reason.clone(),
                })
                .collect();
            (key, list)
        })
        .collect()
}

fn host(doc: &HostDocument, lv: &LevelView, usages: &HashMap<String, Vec<UsageOf>>) -> HostLevel {
    let level = &doc.levels[lv.level];
    let nets = host_nets(doc, lv.level);
    let net_id = |written: &str, file: &str| {
        let key = doc.net_key_in(lv.level, written, file);
        let shown = nets.iter().find(|(k, _)| *k == key).map(|(_, s)| s.as_str()).unwrap_or(written);
        format!("&{}", quoted(shown))
    };
    let port_keys: Vec<String> = level.ports.iter().map(|p| doc.net_key_in(lv.level, p, &level.file)).collect();
    HostLevel {
        ports: level.ports.iter().map(|p| net_id(p, &level.file)).collect(),
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
                        net: net_id(node, &inst.file),
                    })
                    .collect(),
            })
            .collect(),
        nets: nets
            .iter()
            .map(|(key, shown)| HostNet {
                id: format!("&{}", quoted(shown)),
                global: doc.is_global(lv.level, key),
                port: port_keys.contains(key),
                usages: usages.get(key).cloned().unwrap_or_default(),
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
                _ if lv
                    .objects
                    .iter()
                    .any(|o| o.symbol.as_ref() == Some(s) && o.pins.iter().any(|p| p.identity.is_some())) =>
                {
                    lv.unplaced_symbols.iter().find(|v| v.name == s.name && v.origin == s.origin)
                }
                // in a 0.4 level, any symbol an object uses, drawn or not, such as that of a junction
                // dot, which has no place of its own before a solve
                _ if rnl_runtime::interp::unit::keeps_0_4(&lv.language)
                    && lv.objects.iter().any(|o| o.symbol.as_ref() == Some(s)) =>
                {
                    // a view names a level's symbol without its `%`, as the reading keeps it
                    let named = |v: &&rnl_runtime::model::SymbolView| {
                        v.origin == s.origin && (v.name == s.name || format!("%{}", quoted(&v.name)) == s.name)
                    };
                    lv.unplaced_symbols.iter().chain(&lv.symbols).find(named)
                }
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
                names: p.identity.as_ref().map(|i| i.names.clone()),
                bus: p.identity.as_ref().map(|i| i.bus),
                terminals: p.terminals.clone(),
                at: xy(p.pos),
                dir: p.dir,
                label: p.label.as_ref().map(|m| PinLabel {
                    markup: m.clone(),
                    lines: rnl_runtime::markup::parse_with_name_formats(m, p.label_name_formats)
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

/// What a role is attached to, as the snapshot writes it: an id, a point, or a set role's members.
fn attached(a: &Attach) -> Value {
    match a {
        Attach::Object(id) => Value::String(id.clone()),
        Attach::At(p) => serde_json::json!([p.0, p.1]),
        Attach::Set(members) => Value::Array(members.iter().map(attached).collect()),
    }
}

/// The groups of a level ([`GroupView`]), with where each is written as the Document's usage table
/// has it.
fn groups_of(
    lv: &LevelView,
    dl: &crate::document::Level,
    files: &HashMap<&str, usize>,
    usages: &HashMap<String, Vec<UsageOf>>,
) -> Vec<GroupView> {
    lv.groups
        .iter()
        .map(|g| {
            let row = dl.usages.iter().find(|u| u.kind == "group" && u.id == g.id);
            let (source, rule) = match &g.origin {
                model::UsageOrigin::Written => ("written", None),
                model::UsageOrigin::Solver => ("solver", None),
                model::UsageOrigin::Rule { package, at } => {
                    ("rule", Some(RuleAt { package: package.clone(), file: at.file.clone(), line: at.line }))
                }
                model::UsageOrigin::Body => ("body", None),
            };
            GroupView {
                spacing_x: g.spacing_x,
                spacing_y: g.spacing_y,
                id: g.id.clone(),
                members: g.members.iter().map(attached).collect(),
                file: row.and_then(|u| u.file.as_deref()).and_then(|f| files.get(f).copied()),
                line: row.map_or(0, |u| u.first_line),
                source: source.into(),
                rule,
                made_in: g.parent.clone(),
                state: if g.disabled { "disabled" } else { "in-effect" }.into(),
                usages: usages.get(&g.id).cloned().unwrap_or_default(),
            }
        })
        .collect()
}

/// `Some(false)` when a `textvisible` in effect hides a text, else none.
fn hidden(textvisible: Option<&Lit>) -> Option<bool> {
    (textvisible.and_then(Lit::boolean) == Some(false)).then_some(false)
}

fn level_model(
    doc: &HostDocument,
    lv: &LevelView,
    usages: &HashMap<String, Vec<UsageOf>>,
    groups: Vec<GroupView>,
) -> LevelModel {
    let symbols = symbol_refs(doc, lv);
    let symbol_id = |s: &SymbolInfo| symbols.iter().find(|(info, _)| info == s).map(|(_, r)| r.id.clone());
    let net_id = |shown: &str| format!("&{}", quoted(shown));
    let objects = lv
        .objects
        .iter()
        .map(|o| ObjectView {
            pin_of: o.pin_of.clone(),
            parent: o.own_point.then_some(o.parent.clone()).flatten(),
            id: o.id.clone(),
            kind: o.kind.to_string(),
            host: o.host.clone(),
            definition: o.definition.clone(),
            attach: o
                .definition
                .as_ref()
                .map(|_| o.attach.iter().map(|(role, a)| (role.clone(), attached(a))).collect()),
            params: (!o.params.is_empty()).then(|| o.params.iter().map(|(n, v)| (n.clone(), lit(v))).collect()),
            style: (!o.own_style.is_empty()).then(|| style(&o.own_style)),
            symbol: Slot::of(o.symbol.as_ref().and_then(symbol_id)),
            at: Slot::of(o.at.map(xy)),
            orient: Slot::of(Some(o.orient.as_str().to_string())),
            pins: o
                .pins
                .iter()
                .map(|p| PinView {
                    kind: p.identity.as_ref().map(|_| "pin"),
                    id: p.identity.as_ref().map(|i| i.id.clone()),
                    index: p.identity.as_ref().map(|i| i.index),
                    names: p.identity.as_ref().map(|i| i.names.clone()),
                    bus: p.identity.as_ref().map(|i| i.bus),
                    reference: p.identity.as_ref().and_then(|i| i.reference.clone()),
                    local_at: p.identity.as_ref().map(|i| xy(i.local_at)),
                    terminals: p.terminals.clone(),
                    at: p.at.map(xy),
                    dir: p.dir,
                    nets: p.nets.clone(),
                })
                .collect(),
            usages: usages.get(&o.id).cloned().unwrap_or_default(),
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
                    model::Vertex::Pin { owner, pin, reference } => {
                        Vertex::Pin { owner: owner.clone(), pin: *pin, reference: reference.clone() }
                    }
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
            rnl_runtime::markup::Run::Frac { .. } | rnl_runtime::markup::Run::Sqrt { .. } => true,
        })
    };
    let texts = lv
        .texts
        .iter()
        .filter(|t| shows(&t.lines))
        .map(|t| TextView {
            id: t.object_id.clone(),
            attach: t.attach.clone(),
            attach_point: t.attach_point.map(|p| [p.0, p.1]),
            key: t.key.clone(),
            markup: Slot::of(Some(t.markup.clone())),
            written: t.written,
            lines: t.lines.iter().map(|l| l.iter().map(primitive::run).collect()).collect(),
            anchor: xy(t.anchor),
            halign: t.halign.clone(),
            valign: t.valign.clone(),
            rotate: t.rotate,
            fix_rotate: t.fix_rotate,
            upright: t.upright,
            visible: hidden(t.style.get("textvisible")),
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
    let annotations = lv
        .annotations
        .iter()
        .map(|a| AnnotationView {
            id: a.object_id.clone(),
            source_kind: a.source_kind.to_string(),
            source: a.source.clone(),
            anchor_kind: a.anchor_kind.to_string(),
            anchor_object: a.anchor_object.clone(),
            anchor: xy(a.anchor),
            rotate: a.rotate,
            mirror: a.mirror,
            style: style(&a.own_style),
            layer: a.layer.as_str().to_string(),
            graphics: a
                .graphics
                .iter()
                .map(|g| AnnotationGraphic {
                    shape: g.primitive.shape.as_str().to_lowercase(),
                    points: g.primitive.points.iter().map(|p| xy(*p)).collect(),
                    params: params(&g.primitive.params),
                    style: style(&g.primitive.style),
                    lines: g.lines.iter().map(|l| l.iter().map(primitive::run).collect()).collect(),
                    text: g.primitive.text.clone(),
                    text_refs: g
                        .text_refs
                        .iter()
                        .map(|r| {
                            match r {
                                rnl_runtime::model::AnnotationTextRef::X => "x",
                                rnl_runtime::model::AnnotationTextRef::Y => "y",
                                rnl_runtime::model::AnnotationTextRef::Empty => "empty",
                            }
                            .to_string()
                        })
                        .collect(),
                    halign: g.primitive.halign.clone(),
                    valign: g.primitive.valign.clone(),
                    rotate: g.primitive.rotate,
                    fix_rotate: g.primitive.fix_rotate,
                    upright: g.primitive.upright,
                    visible: g
                        .primitive
                        .text
                        .as_ref()
                        .and_then(|_| hidden(g.primitive.style.get("textvisible").or(a.style.get("textvisible")))),
                })
                .collect(),
        })
        .collect();
    let symbol = Slot::of(lv.block_symbol.as_ref().and_then(|b| symbol_id(&b.info)));
    let junction = lv
        .junction
        .as_ref()
        .map(|j| JunctionView { definition: j.definition.clone(), symbol: j.symbol.as_ref().and_then(&symbol_id) });
    LevelModel {
        symbols: symbols.into_iter().map(|(_, r)| r).collect(),
        symbol,
        objects,
        points,
        wires,
        texts,
        shapes,
        annotations,
        groups,
        junction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildOptions, MemoryProvider};
    use serde_json::json;

    #[test]
    fn text_snapshot_preserves_explicit_fixed_rotation_and_omits_unwritten_values() {
        for (version, param, expected) in [
            ("0.4", ", fix_rotate=true", Some(true)),
            ("0.4", ", fix_rotate=false", Some(false)),
            ("0.4", "", None),
            ("0.3", "", None),
        ] {
            let source = format!("* text\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ (2,3) @TEXT(\"ink\"{param}) }}\n.end\n");
            let files = MemoryProvider::new().with("t.sp", &source);
            let options =
                BuildOptions { runtime: Options { std: false, ..Options::default() }, ..BuildOptions::default() };
            let snap =
                crate::snapshot("t.sp", &files, &options, Parts { drawing: false, host: false, model: true }).unwrap();
            let value = serde_json::to_value(snap).unwrap();
            let text = &value["levels"][0]["model"]["texts"][0];
            match expected {
                Some(expected) => assert_eq!(text["fix_rotate"], expected),
                None => assert!(text.get("fix_rotate").is_none()),
            }
        }
    }

    /// The model part of the snapshot of `source`, the file `t.sp`, read with the standard package
    /// when `std` says so.
    fn model_of(source: &str, std: bool) -> serde_json::Value {
        let files = MemoryProvider::new().with("t.sp", source);
        let options = BuildOptions { runtime: Options { std, ..Options::default() }, ..BuildOptions::default() };
        let snap =
            crate::snapshot("t.sp", &files, &options, Parts { drawing: false, host: false, model: true }).unwrap();
        serde_json::to_value(snap).unwrap()["levels"][0]["model"].clone()
    }

    #[test]
    fn a_text_its_style_hides_says_so_whoever_hides_it() {
        // the file's own textvisible, and a package's rule that hides the default labels
        let source = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"); \
                      @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.4\") }\nR1 a 0 1k\n\
                      * @RNL { #R1 @PLACE(0, 0); (2, 3) @TEXT(\"hidden\", textvisible=false); (2, 5) @TEXT(\"shown\") }\n.end\n";
        let model = model_of(source, true);
        let text = |what: &str, value: &str| {
            let found =
                model["texts"].as_array().unwrap().iter().find(|t| t[what] == value || t[what]["value"] == value);
            found.unwrap_or_else(|| panic!("{value}: {model}")).clone()
        };
        assert_eq!(text("markup", "hidden").get("visible"), Some(&json!(false)));
        assert_eq!(text("markup", "shown").get("visible"), None);
        // the package hides the label: the file's own style says nothing of it
        let name = text("id", "#R1::name");
        assert_eq!(name.get("visible"), Some(&json!(false)));
        assert_eq!(name["style"], json!({}));
        // before 0.4 there is no textvisible, and no field
        let old = model_of(
            "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n* @RNL { (2, 3) @TEXT(\"x\") }\n.end\n",
            false,
        );
        assert!(old["texts"].as_array().unwrap().iter().all(|t| t.get("visible").is_none()));
    }

    #[test]
    fn a_level_says_its_junction_definition_and_the_symbol_with_its_circle() {
        let local = "%dotsym = @DEFINE-SYMBOL(box=[-1, -1, 1, 1]) { (0, 0) @CIRCLE(radius=0.6, fill=\"linecolor\") }; \
                     %dot = @DEFINE(roles=[[\"p\", \"point\"]]) { {? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\") }; \
                     # @SELECT-USAGE %dot => @SET-SYMBOL %dotsym; @SOLVER-CONFIG(junction=\"%dot\")";
        let file = |dots: &str| {
            format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a 0 1k\n* @RNL {{ {local}; #R1 @PLACE(0, 0); %j = (2, 3){dots} }}\n.end\n")
        };
        // before a solve, no dot: the definition, but no symbol yet
        let model = model_of(&file(""), false);
        assert_eq!(model["junction"], json!({"definition": "%dot", "symbol": null}));
        // with a dot, its symbol, and the symbol's circle
        let model = model_of(&file("; %dot @ATTACH %j"), false);
        assert_eq!(model["junction"], json!({"definition": "%dot", "symbol": "%dotsym"}));
        let usage = model["objects"].as_array().unwrap().iter().find(|o| o["kind"] == "usage").unwrap();
        assert_eq!(usage["definition"], "%dot", "a level's own definition keeps its %");
        let symbol = model["symbols"].as_array().unwrap().iter().find(|s| s["id"] == "%dotsym").unwrap();
        assert_eq!(symbol["definition"]["prims"][0]["params"]["radius"], 0.6);
        // the standard package's, by default
        let standard = model_of(
            "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\nR1 a 0 1k\n.end\n",
            true,
        );
        assert_eq!(standard["junction"]["definition"], "$JUNCTION");
    }

    #[test]
    fn before_0_4_a_level_has_no_junction_and_its_definitions_no_percent() {
        let source = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n\
                      * @RNL { %w = @DEFINE(roles=[\"x\"]) { ?x @ORIENT(\"R90\") }; %v = #R1 @AS %w }\n.end\n";
        let model = model_of(source, false);
        assert!(model.get("junction").is_none());
        let usage = model["objects"].as_array().unwrap().iter().find(|o| o["kind"] == "usage").unwrap();
        assert_eq!(usage["definition"], "w");
    }

    #[test]
    fn text_snapshot_preserves_explicit_upright_and_arbitrary_rotation() {
        for (param, expected) in [("", None), (", upright=true", Some(true)), (", upright=false", Some(false))] {
            let source = format!("* text\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ (2,3) @TEXT(\"ink\", rotate=30.5{param}) }}\n.end\n");
            let files = MemoryProvider::new().with("t.sp", &source);
            let options =
                BuildOptions { runtime: Options { std: false, ..Options::default() }, ..BuildOptions::default() };
            let snap =
                crate::snapshot("t.sp", &files, &options, Parts { drawing: false, host: false, model: true }).unwrap();
            let value = serde_json::to_value(snap).unwrap();
            let text = &value["levels"][0]["model"]["texts"][0];
            assert_eq!(text["rotate"], 30.5);
            match expected {
                Some(expected) => assert_eq!(text["upright"], expected),
                None => assert!(text.get("upright").is_none()),
            }
        }
    }

    #[test]
    fn pin_label_name_formats_in_the_snapshot_follow_the_file_version() {
        for version in ["0.2", "0.3", "0.4"] {
            let source = format!(
                "* pins\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n\
                 .subckt leaf p n\n* @RNL {{\n\
                 * %s = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1, direction=\"left\", label=\"\\subdigits{{Vout1}}\") }};\n\
                 * # @SET-SYMBOL %s\n* }}\n.ends leaf\nX1 a b leaf\n* @RNL {{ #X1 @PLACE(0,0) }}\n"
            );
            let options = BuildOptions {
                runtime: rnl_runtime::Options { std: false, ..Default::default() },
                ..Default::default()
            };
            let files = MemoryProvider::new().with("t.sp", &source);
            let value = serde_json::to_value(
                crate::snapshot("t.sp", &files, &options, Parts { drawing: false, host: false, model: true }).unwrap(),
            )
            .unwrap();
            let label = value["levels"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|l| l["model"]["symbols"].as_array().unwrap())
                .find_map(|s| s["definition"]["pins"][0].get("label"))
                .unwrap();
            assert_eq!(label["markup"], r"\subdigits{Vout1}");
            let runs: Vec<_> = label["lines"][0]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| (r["text"].as_str().unwrap(), r["style"]["level"].as_i64().unwrap()))
                .collect();
            assert_eq!(
                runs,
                if version == "0.4" { vec![("Vout", 0), ("1", -1)] } else { vec![(r"\subdigits{Vout1}", 0)] }
            );
        }
    }

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
    fn a_set_role_gives_its_members() {
        // ref and out of $CURRENT_MIRROR and tail of $DIFFERENTIAL_PAIR are set roles (9.4): each
        // gives its members, none, one or several, and each member is an object the model lists
        let text = "* sets\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.model nch nmos level=1\n\
                    M1 a g 0 0 nch\nM2 b g 0 0 nch\nM3 a g 0 0 nch\nM4 b g 0 0 nch\nM5 t c 0 0 nch\nM6 t c 0 0 nch\n\
                    * @RNL {\n\
                    *   %m1 = #M1(ref) #M2(out) @AS $CURRENT_MIRROR;\n\
                    *   %m2 = {#M1 #M3}(ref) {#M2 #M4}(out) @AS $CURRENT_MIRROR;\n\
                    *   %m3 = #M2(out) @AS $CURRENT_MIRROR;\n\
                    *   %dp1 = #M1(in_p) #M2(in_n) @AS $DIFFERENTIAL_PAIR;\n\
                    *   %dp2 = #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;\n\
                    *   %dp3 = #M1(in_p) #M2(in_n) {#M5 #M6}(tail) @AS $DIFFERENTIAL_PAIR\n\
                    * }\n.end\n";
        let files = MemoryProvider::new().with("/w/t.sp", text);
        let parts = Parts { drawing: false, host: false, model: true };
        let s =
            serde_json::to_value(crate::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap()).unwrap();
        let objects = s["levels"][0]["model"]["objects"].as_array().unwrap();
        let attach = |id: &str| objects.iter().find(|o| o["id"] == id).unwrap()["attach"].clone();
        assert_eq!(attach("%m1"), json!({"ref": ["#M1"], "out": ["#M2"]}));
        assert_eq!(attach("%m2"), json!({"ref": ["#M1", "#M3"], "out": ["#M2", "#M4"]}));
        assert_eq!(attach("%m3"), json!({"ref": [], "out": ["#M2"]}));
        assert_eq!(attach("%dp1"), json!({"in_p": "#M1", "in_n": "#M2", "tail": []}));
        assert_eq!(attach("%dp2"), json!({"in_p": "#M1", "in_n": "#M2", "tail": ["#M5"]}));
        assert_eq!(attach("%dp3"), json!({"in_p": "#M1", "in_n": "#M2", "tail": ["#M5", "#M6"]}));
        // no id is left that the snapshot does not define
        let ids: Vec<&Value> = objects.iter().map(|o| &o["id"]).collect();
        for usage in ["%m1", "%m2", "%m3", "%dp1", "%dp2", "%dp3"] {
            for (_, given) in attach(usage).as_object().unwrap() {
                let members = given.as_array().cloned().unwrap_or_else(|| vec![given.clone()]);
                assert!(members.iter().all(|m| ids.contains(&m)), "{usage}: {given}");
            }
        }
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
        // each net without the usages it is in, which the ground of the netlist has
        let nets: Vec<Value> = host["nets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| json!({"id": n["id"], "global": n["global"], "port": n["port"]}))
            .collect();
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
        // a version of the standard package asked for changes it
        let v02 = Options { std_version: Some("0.2".into()), ..Options::default() };
        assert_ne!(rev(NETLIST, &BuildOptions { runtime: v02, ..BuildOptions::default() }), r);
        let include_std = Options { include_std: true, ..Options::default() };
        assert_ne!(rev(NETLIST, &BuildOptions { runtime: include_std, ..BuildOptions::default() }), r);
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
    fn the_revision_counts_included_files_of_one_name_in_two_directories() {
        // a/x and b/x, both included by the main file, in SPICE and in Spectre: the text of each
        // counts
        let spice = (
            "/w/t.sp",
            "* t\n.include \"a/x.inc\"\n.include \"b/x.inc\"\nX1 n1 0 ca\nX2 n2 0 cb\n.end\n",
            ["/w/a/x.inc", "/w/b/x.inc"],
            ["* a\n.subckt ca p n\nR1 p n 1k\n.ends\n", "* b\n.subckt cb p n\nR1 p n 2k\n.ends\n"],
        );
        let spectre = (
            "/w/t.scs",
            "// t\nsimulator lang=spectre\ninclude \"a/x.scs\"\ninclude \"b/x.scs\"\nX1 (n1 0) ca\nX2 (n2 0) cb\n",
            ["/w/a/x.scs", "/w/b/x.scs"],
            [
                "// a\nsubckt ca (p n)\nR1 (p n) resistor r=1k\nends ca\n",
                "// b\nsubckt cb (p n)\nR1 (p n) resistor r=2k\nends cb\n",
            ],
        );
        for (entry, main, paths, texts) in [spice, spectre] {
            let revision = |first: &str, second: &str| {
                let files = MemoryProvider::new().with(entry, main).with(paths[0], first).with(paths[1], second);
                crate::snapshot(entry, &files, &BuildOptions::default(), Parts::default()).unwrap().revision
            };
            let r = revision(texts[0], texts[1]);
            assert_ne!(revision(texts[0], &texts[1].replace("2k", "3k")), r, "{entry}");
            assert_ne!(revision(&texts[0].replace("1k", "3k"), texts[1]), r, "{entry}");
        }
    }

    /// A text at a point the level names is attached to it, as one on an instance is; one at a point
    /// written in place is attached to nothing it can name, though both say where their point is.
    /// Neither is a label to click.
    #[test]
    fn a_text_at_a_named_point_says_which() {
        let text = "* points\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a b 1k\n\
                    * @RNL {\n\
                    *   #R1 @PLACE(0, 0);\n\
                    *   %j1 = (10, 0); &b @WIRE {#R1:2 %j1}; %j1 @TEXT(\"VDD\", dx=-31, dy=1.5) => %t1;\n\
                    *   (20, 0) @TEXT(\"here\") => %t2;\n\
                    *   #R1 @TEXT(\"on R1\") => %t3\n\
                    * }\n.end\n";
        let files = MemoryProvider::new().with("/w/t.sp", text);
        let parts = Parts { drawing: true, host: false, model: true };
        let s =
            serde_json::to_value(crate::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap()).unwrap();
        let texts = s["levels"][0]["model"]["texts"].as_array().unwrap();
        let attach = |id: &str| {
            texts.iter().find(|t| t["id"] == id).unwrap_or_else(|| panic!("{id}: {texts:?}"))["attach"].clone()
        };
        assert_eq!(attach("%t1"), json!("%j1"));
        assert_eq!(attach("%t2"), Value::Null);
        assert_eq!(attach("%t3"), json!("#R1"));
        // the point each text at a point hangs on, named or written in place; none for one on R1
        let point = |id: &str| texts.iter().find(|t| t["id"] == id).unwrap()["attach_point"].clone();
        assert_eq!(point("%t1"), json!([10.0, 0.0]));
        assert_eq!(point("%t2"), json!([20.0, 0.0]));
        assert_eq!(point("%t3"), Value::Null);
        // the point is one the snapshot lists, and the text is where it is drawn: its point and the offset
        let points = s["levels"][0]["model"]["points"].as_array().unwrap();
        assert!(points.iter().any(|p| p["id"] == "%j1"), "{points:?}");
        let d = crate::build("/w/t.sp", &files, &BuildOptions::default()).unwrap();
        let level = &d.levels[0];
        // no hit for the texts at points: only the one on R1 among them is a label
        let labels = level.hits.iter().filter(|h| h.kind == "label").count();
        // R1's name and value, and %t3
        assert_eq!(labels, 3, "{:?}", level.hits);
    }

    /// The same in a document of 0.4, and a text at a point with no position yet, such as one with
    /// an unknown coordinate, hangs on no point it can say.
    #[test]
    fn a_text_at_a_point_says_where_in_0_4() {
        let text = "* points\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\nR1 a b 1k\n\
                    * @RNL {\n\
                    *   #R1 @PLACE(0, 0);\n\
                    *   %j1 = (10, 0); &b @WIRE {#R1:2 %j1}; %j1 @TEXT(\"VDD\", dx=-31, dy=1.5) => %t1;\n\
                    *   (20, 0) @TEXT(\"here\") => %t2;\n\
                    *   #R1 @TEXT(\"on R1\") => %t3;\n\
                    *   (_, 7) @TEXT(\"x\") => @PLACE(6, 6) => %t4\n\
                    * }\n.end\n";
        let files = MemoryProvider::new().with("/w/t.sp", text);
        let parts = Parts { drawing: false, host: false, model: true };
        let s =
            serde_json::to_value(crate::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap()).unwrap();
        let texts = s["levels"][0]["model"]["texts"].as_array().unwrap();
        let text = |id: &str| texts.iter().find(|t| t["id"] == id).unwrap_or_else(|| panic!("{id}: {texts:?}"));
        assert_eq!(text("%t1")["attach"], json!("%j1"));
        assert_eq!(text("%t1")["attach_point"], json!([10.0, 0.0]));
        assert_eq!(text("%t2")["attach"], Value::Null);
        assert_eq!(text("%t2")["attach_point"], json!([20.0, 0.0]));
        assert_eq!(text("%t3")["attach_point"], Value::Null);
        // placed where it is drawn, but its point has no position: none to say
        assert_eq!(text("%t4")["attach_point"], Value::Null);
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
