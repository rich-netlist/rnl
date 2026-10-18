//! The effective model: what each drawing level looks like after the RNL takes effect.
//!
//! Renderers, viewers and a future solver read this and nothing of the interpreter's internals
//! (architecture 4.5). Coordinates are scene units, x to the right and y up. Styles hold only the
//! properties some RNL gave; a renderer fills in its own defaults (9.3, principle 2.7).

use std::collections::BTreeMap;

use crate::geom::{Orient, Pt};
use crate::interp::obj::{Lit, PrimParams, Shape};
use crate::markup::Line;

pub type Style = BTreeMap<String, Lit>;

/// A name as a reference writes it (3.3): as it is when it is ASCII letters, digits and `_`,
/// else in double quotes, a quote doubled, such as `"vdd!"`.
pub fn quoted(name: &str) -> String {
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('"', "\"\""))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Layer {
    Bottom,
    #[default]
    Middle,
    Top,
}

impl Layer {
    pub fn parse(s: &str) -> Layer {
        match s {
            "bottom" => Layer::Bottom,
            "top" => Layer::Top,
            _ => Layer::Middle,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Layer::Bottom => "bottom",
            Layer::Middle => "middle",
            Layer::Top => "top",
        }
    }
}

/// A graphic of a symbol in local coordinates.
#[derive(Clone, Debug)]
pub struct SymbolPrim {
    pub shape: Shape,
    pub points: Vec<Pt>,
    pub params: PrimParams,
    pub style: Style,
    /// For a fixed text: its alignment and rotation; the content is per item (`ItemView::texts`).
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub rotate: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinView {
    pub pos: Pt,
    pub label: Option<String>,
    /// Exit direction in degrees: 0 right, 90 up, 180 left, 270 down.
    pub dir: Option<u16>,
    pub terminals: Vec<String>,
}

/// A symbol drawn inside another (9.2.7).
#[derive(Clone, Debug)]
pub struct NestedView {
    pub symbol: usize,
    pub at: Pt,
    pub orient: Orient,
}

/// A device that is not drawn since no symbol can be chosen for it, with the reason.
#[derive(Clone, Debug)]
pub struct UndrawnView {
    /// As `ItemView::id`.
    pub id: String,
    /// The host instances: the index of each in its level's instances.
    pub host: Vec<usize>,
    /// Why, such as "model card not found".
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct SymbolView {
    pub name: String,
    pub prims: Vec<SymbolPrim>,
    pub pins: Vec<PinView>,
    pub nested: Vec<NestedView>,
    /// The symbol's box (9.2.3): x1, y1, x2, y2 in local coordinates. When the symbol writes no
    /// `box=`, the extent of all its graphics, text excluded.
    pub bbox: [f64; 4],
    /// The symbol writes `box=`.
    pub box_written: bool,

    /// The viewer's own stand-in for an instance with no symbol (9.2.6).
    pub default: bool,
    /// Where the symbol is defined: [`SymbolOrigin`].
    pub origin: SymbolOrigin,
}

/// Where a symbol is defined: in a package, or in a level of the document, such as the symbol of
/// a subcircuit (9.2.6), which may share its name with one in a package or another level.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SymbolOrigin {
    Package(String),
    /// The host level whose records define it.
    Level(usize),
    /// The viewer's own stand-in, or not found.
    #[default]
    None,
}

/// An instance, bundle or usage drawn with a symbol.
#[derive(Clone, Debug)]
pub struct ItemView {
    pub id: String,
    pub kind: &'static str,
    pub symbol: usize,
    pub at: Pt,
    pub orient: Orient,
    pub style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
    /// The fixed texts of the symbol with their `${…}` values, by path: "3" is prim 3, "1/3" is
    /// prim 3 of nested symbol 1.
    pub texts: BTreeMap<String, Vec<Line>>,
    /// The host instances drawn: the index of each in its level's instances. One for an
    /// instance, its members for a bundle, none for a usage.
    pub host: Vec<usize>,
    /// The nets at each pin of the symbol, by pin index, as the nets are displayed.
    pub pin_nets: Vec<Vec<String>>,
}

/// A wire, or a part of one: a wire some of whose points have no position yet, such as a free
/// point before the level is solved, is drawn in its runs of consecutive points that have one,
/// each run of two points or more a WireView with the wire's id, nets, mode and style, and a
/// flyline over each gap (`LevelView::flylines`). A wire whose points all have a position is one
/// WireView.
#[derive(Clone, Debug)]
pub struct WireView {
    pub id: String,
    /// Its id in the editor interface: [`ObjectInfo::id`].
    pub object_id: String,
    pub nets: Vec<String>,
    /// The vertices after the routing mode (13.1).
    pub points: Vec<Pt>,
    /// The vertices as the statement gives them.
    pub via: Vec<Vertex>,
    pub mode: String,
    pub style: Style,
    /// The properties of `style` the document's own records give, not a package.
    pub own_style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
    /// The whole wire, drawn through all its points; false for a run of a wire some of whose
    /// points have no position, which the editor interface's snapshot leaves out.
    pub whole: bool,
}

/// A vertex of a wire as written: a pin, a named point, a role of a usage, or coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Vertex {
    /// `#M1:1`, `#:inp`.
    Terminal(String),
    /// `%j1`.
    Point(String),
    /// `%gnd1:p`: the usage's id and the role.
    Usage {
        usage: String,
        role: String,
    },
    At(Pt),
}

/// A graphic written in the scene: an annotation (13.4).
#[derive(Clone, Debug)]
pub struct ShapeView {
    /// Its id in the editor interface: `%box1` when the level names it, else where the statement
    /// that made it is, as [`ObjectInfo::id`].
    pub object_id: String,
    pub shape: Shape,
    pub points: Vec<Pt>,
    pub params: PrimParams,
    pub style: Style,
    /// The properties of `style` the document's own records give, not a package.
    pub own_style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct TextView {
    pub id: String,
    /// Its id in the editor interface: [`ObjectInfo::id`], or `#M1::name` for a key text.
    pub object_id: String,
    /// The id of the object it is attached to; None for a text at a point.
    pub attach: Option<String>,
    /// The key of a label, such as "name".
    pub key: Option<String>,
    /// The text as written, before `${…}` is read (13.6).
    pub markup: String,
    /// The document's records write it, or a part of it; false for a label its symbol alone gives.
    pub written: bool,
    pub anchor: Pt,
    pub lines: Vec<Line>,
    pub halign: String,
    pub valign: String,
    pub rotate: f64,
    /// A text written in a symbol turns with its instance (9.2.5); scene texts do not.
    pub orient: Orient,
    pub style: Style,
    /// The properties of `style` the document's own records give, not a package.
    pub own_style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ConstraintView {
    pub label: String,
    pub kind: String,
    /// ok, dropped, needs-solver, unsolved, disabled, failed.
    pub status: String,
    pub priority: String,
}

#[derive(Clone, Debug, Default)]
pub struct LevelView {
    /// The host level drawn: its index in the host document's levels.
    pub level: usize,
    pub title: String,
    pub kind: &'static str,
    pub name: String,
    pub file: String,
    pub line: u32,
    pub symbols: Vec<SymbolView>,
    pub items: Vec<ItemView>,
    pub wires: Vec<WireView>,
    pub shapes: Vec<ShapeView>,
    pub texts: Vec<TextView>,
    /// Lines a viewer may draw for connections not drawn as wires, not wires themselves: over the
    /// gaps of a wire some of whose points have no position yet, from the last point before a gap
    /// that has one to the first after it, those of the wires first; then for the nets that a ROUTE
    /// asks to wire (13.7).
    pub flylines: Vec<(Pt, Pt)>,
    pub constraints: Vec<ConstraintView>,
    /// Drawn objects without a position, left to a solver.
    pub unplaced: Vec<String>,
    /// The objects of `unplaced` that have a symbol, in the same order, each as an item at the
    /// origin with the nets at its pins, for a viewer to list what is not drawn. Their symbols
    /// are `unplaced_symbols`, not `symbols`: nothing here is drawn on the level.
    pub unplaced_items: Vec<ItemView>,
    pub unplaced_symbols: Vec<SymbolView>,
    /// The devices the level does not draw whatever their position, since no symbol can be chosen
    /// for them, such as a transistor whose model card is not found; a viewer lists them with the
    /// objects that are not drawn, each with its reason.
    pub undrawn: Vec<UndrawnView>,
    /// The instances, bundles and usages of the level, drawn or not, as the editor interface
    /// sees them.
    pub objects: Vec<ObjectInfo>,
    /// The named points of the level.
    pub points: Vec<PointInfo>,
    /// The terminal names of each host instance of the level, in the order of its nodes: from
    /// DEVICE, the subcircuit's ports or the adapter's defaults (6.4, 6.12); None where there is
    /// no name.
    pub terminal_names: Vec<Vec<Option<String>>>,
    /// A subcircuit's own symbol, `# @SET-SYMBOL` (9.2.6).
    pub block_symbol: Option<BlockSymbol>,
    /// The packages in effect at the level, one of each name, in the order of their places:
    /// those at the root first (16.2, 16.3).
    pub packages: Vec<LevelPackage>,
}

/// A package in effect at a drawing level (16.2, 16.3).
#[derive(Clone, Debug, PartialEq)]
pub struct LevelPackage {
    pub name: String,
    /// What its PACKAGE says, or the `version=` of the USE-PACKAGE of an inline package.
    pub version: String,
    /// Where its content is placed: at the root (None), or at a USE-PACKAGE written in a level,
    /// the host level's index.
    pub at: Option<usize>,
    /// The USE-PACKAGE written in a file that brings it in, itself or through the packages it
    /// references: the package it names, and where it is written. None for the standard package
    /// used implicitly and what it references.
    pub via: Option<(String, crate::diag::Loc)>,
}

/// The symbol a subcircuit gives itself (9.2.6): what its instances are drawn with when they are
/// given no other.
#[derive(Clone, Debug)]
pub struct BlockSymbol {
    pub info: SymbolInfo,
    /// What it is made of, without the symbols drawn in it (9.2.7). It is not among the level's
    /// `symbols`, which are those of the level's own items.
    pub view: SymbolView,
}

/// An instance, bundle or usage of a level, drawn or not: what the editor interface's snapshot
/// gives of it (`notes/rnl-editor-interface` in rnl-dev, 3.5).
#[derive(Clone, Debug)]
pub struct ObjectInfo {
    /// Its id, unique in the level: `#M1` for an instance, `%gnd1` for an object named in the level,
    /// or, for one without a name of its own, where the statement that made it is,
    /// `@file:line:col`, with `#n` after it when that statement made several.
    pub id: String,
    /// "instance", "bundle" or "usage".
    pub kind: &'static str,
    /// The ids of the host instances it draws: one for an instance, the members of a bundle, none
    /// for a usage.
    pub host: Vec<String>,
    /// A usage's definition, such as `$GROUND`.
    pub definition: Option<String>,
    /// A usage's roles in order, with what each is attached to.
    pub attach: Vec<(String, Attach)>,
    /// The symbol it is drawn with.
    pub symbol: Option<SymbolInfo>,
    pub at: Option<Pt>,
    pub orient: Orient,
    /// The symbol's pins where they are drawn, when it has a symbol and a position.
    pub pins: Vec<PinPlace>,
    /// Its symbol, by index in `LevelView::symbols`, when it is drawn.
    pub symbol_view: Option<usize>,
}

/// What a role of a usage is attached to: an object, by its id, or a point; for a set role (9.4),
/// its members in order, each an object or a point, nested sets followed down, none when the role
/// is left empty.
#[derive(Clone, Debug, PartialEq)]
pub enum Attach {
    Object(String),
    At(Pt),
    Set(Vec<Attach>),
}

/// A symbol an object is drawn with, as the snapshot names it.
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub origin: SymbolOrigin,
    pub default: bool,
}

/// A pin of a drawn object, in scene coordinates.
#[derive(Clone, Debug)]
pub struct PinPlace {
    /// The terminals at the pin: an instance's by number, `#M1:1`; a usage's by role, `p`.
    pub terminals: Vec<String>,
    pub at: Pt,
    /// The exit direction after the object's orientation, in degrees.
    pub dir: Option<u16>,
    /// The nets at the pin, as ids.
    pub nets: Vec<String>,
}

/// A named point of a level.
#[derive(Clone, Debug)]
pub struct PointInfo {
    pub id: String,
    pub at: Option<Pt>,
    /// A junction dot is drawn at it (a usage of `$JUNCTION`).
    pub junction: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub levels: Vec<LevelView>,
    /// Set when the whole interpretation failed (1.5 rule 3, 7.6). Draft 0.2 has no case of it
    /// (7.5), so it is never set.
    pub failed: Option<String>,
    /// The language version the main file declares (16.1); empty when it declares none.
    pub language: String,
    /// The packages in effect in some drawing level, by name and version, each once, in the
    /// order they are brought in: those of the standard package first, the packages a package
    /// references before it (16.2, 16.3).
    pub packages: Vec<(String, String)>,
}

/// A reference to a host object written in a record (6.3), found while interpreting: where the
/// name is written and what it reaches. A tool uses these to go from a name to its object and
/// back; the interpretation itself does not need them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostRef {
    /// Where the reference starts: its `#` or `&`.
    pub loc: crate::diag::Loc,
    /// How many characters the reference takes: the prefix and the name, without any path after
    /// them; for the net of a terminal such as `#M1:d`, up to the end of the terminal.
    pub len: u32,
    /// The host level the name is resolved in.
    pub level: usize,
    pub target: RefTarget,
}

/// What a reference reaches. `#M1:d` gives two: the instance M1 (`#M1`), and the net at its
/// terminal d (`#M1:d`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RefTarget {
    /// An instance, by its index in the level's instances.
    Instance(usize),
    /// A net, by its key (`HostDocument::net_key`).
    Net(String),
}

/// A statement, or a part of one, that does not take effect, found while interpreting: where it
/// is written, and whether it failed (an error, 7.2) or lost a conflict to another statement (8.5).
/// A tool uses these to mark dead text; the interpretation itself does not need them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FailedSpan {
    pub start: crate::diag::Loc,
    /// The last character, included.
    pub end: crate::diag::Loc,
    /// Dropped in a conflict (8.5), as opposed to failed.
    pub conflict: bool,
}
