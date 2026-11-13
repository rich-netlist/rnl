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
#[derive(Clone, Debug, PartialEq)]
pub struct SymbolPrim {
    /// Position among this symbol's primitives and nested usages (9.2.1).
    pub order: usize,
    /// Original fixed text, before substituting an instance's parameters.
    pub text: Option<String>,
    /// Read the name formats of 0.4 in a preview of this text (13.6).
    pub text_name_formats: bool,
    pub modern_text: bool,
    pub shape: Shape,
    pub points: Vec<Pt>,
    pub params: PrimParams,
    pub style: Style,
    /// For a fixed text: its alignment and rotation; the content is per item (`ItemView::texts`).
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub rotate: Option<f64>,
    /// Keep the text direction in scene coordinates while its anchor follows the symbol.
    pub fix_rotate: bool,
    pub upright: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PinView {
    /// Local identity information of a pin declared in a 0.4 symbol.
    pub identity: Option<SymbolPinIdentity>,
    pub pos: Pt,
    pub label: Option<String>,
    /// Read `\subrest` and `\subdigits` in the label (13.6); false for files before 0.4.
    pub label_name_formats: bool,
    /// Exit direction in degrees: 0 right, 90 up, 180 left, 270 down.
    pub dir: Option<u16>,
    pub terminals: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolPinIdentity {
    pub names: Vec<String>,
    pub bus: bool,
}

/// A symbol drawn inside another (9.2.7).
#[derive(Clone, Debug)]
pub struct NestedView {
    /// Position among this symbol's primitives and nested usages (9.2.1).
    pub order: usize,
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
    /// Identity within this interpretation (names alone are not unique).
    pub id: u32,
    pub source: Option<(crate::diag::Loc, crate::diag::Loc)>,
    pub labels: Vec<SymbolLabel>,
    pub package_version: Option<String>,
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

/// A symbol's label template, before it is attached to a particular instance.
#[derive(Clone, Debug)]
pub struct SymbolLabel {
    pub key: Option<String>,
    pub text: String,
    /// The template's source file reads the name formats of 0.4 (13.6).
    pub text_name_formats: bool,
    pub modern_text: bool,
    pub at: Option<Pt>,
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub rotate: f64,
    pub fix_rotate: Option<bool>,
    pub upright: Option<bool>,
    pub style: Style,
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
    /// Its id in the editor interface: [`ObjectInfo::id`], `#M1` for an instance, `%gnd1` for a usage
    /// the level names, else where the statement is. Empty for an item that is not drawn on a level.
    pub object_id: String,
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

/// A line a viewer may draw for a connection not drawn as a wire (`LevelView::flylines`).
#[derive(Clone, Debug, PartialEq)]
pub struct FlylineView {
    pub a: Pt,
    pub b: Pt,
    /// The nets it connects, as they are displayed: its net, or those of the wire over whose gap
    /// it is drawn.
    pub nets: Vec<String>,
}

/// Where a viewer may label one connected part of a net with its name (`LevelView::net_labels`):
/// of the middles of the part's segments as drawn and of its pins, the one nearest the centre of
/// its vertices and pins.
#[derive(Clone, Debug, PartialEq)]
pub struct NetLabelView {
    /// The net, as it is displayed.
    pub net: String,
    pub at: Pt,
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
    /// An independent pin with its exact symbol index and, when available, a source reference (6.5).
    Pin {
        owner: String,
        pin: usize,
        reference: Option<String>,
    },
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

/// What a text is attached to, as the level draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextTarget {
    /// The item at this index of `LevelView::items`: an instance, a bundle or a usage.
    Item(usize),
    /// The wire at this index of `LevelView::wires`; the first run of one drawn in several.
    Wire(usize),
    /// The net of this displayed name.
    Net(String),
    /// The pin at this index of the symbol of the item at this index of `LevelView::items`: for a
    /// text attached to a terminal, which stands for the nets of its pin, as the pin does.
    Pin(usize, usize),
}

#[derive(Clone, Debug)]
pub struct TextView {
    pub id: String,
    /// Its id in the editor interface: [`ObjectInfo::id`], or `#M1::name` for a key text.
    pub object_id: String,
    /// The id of the object it is attached to, or of the point, for a text at a point the level
    /// names, such as `%j1`, as [`PointInfo::id`]; None for a text at a point written in place,
    /// such as `(10, 0)`, or at one that the level does not name alone.
    pub attach: Option<String>,
    /// What a click on it stands for in the drawing: where the object it is attached to is drawn,
    /// its net, or for a terminal the pin it is drawn at. None for a text at a point, and for one
    /// attached to an object that is not drawn as an item, a wire or a pin of one.
    pub target: Option<TextTarget>,
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
    /// The effective fix_rotate item; None when no statement supplies it.
    pub fix_rotate: Option<bool>,
    pub upright: Option<bool>,
    /// Apply the source file's baseline reflection rule (9.2.5).
    pub baseline_mirrors: bool,
    /// Effective owner orientation; fixed-rotation and scene texts use R0.
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

/// A single selectable compound annotation with local graphics and an arbitrary transform.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationView {
    pub object_id: String,
    pub source_kind: &'static str,
    pub source: String,
    pub anchor_kind: &'static str,
    pub anchor_object: Option<String>,
    pub anchor: Pt,
    pub rotate: f64,
    pub mirror: bool,
    pub style: Style,
    pub own_style: Style,
    pub layer: Layer,
    pub order: Vec<f64>,
    pub graphics: Vec<AnnotationGraphic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationGraphic {
    pub primitive: SymbolPrim,
    pub lines: Vec<Line>,
    /// Each markup reference, resolved in its writing unit, in occurrence order.
    pub text_refs: Vec<AnnotationTextRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationTextRef {
    X,
    Y,
    Empty,
}

#[derive(Clone, Debug, Default)]
pub struct LevelView {
    /// The language version of the file that owns this drawing level.
    pub language: String,
    /// Effective terminal currents for inspection only; no solver or drawing semantics.
    pub currents: Vec<CurrentView>,
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
    pub annotations: Vec<AnnotationView>,
    pub texts: Vec<TextView>,
    /// Lines a viewer may draw for connections not drawn as wires, not wires themselves (13.3,
    /// 13.7), to check what is not wired: over the gaps of a wire some of whose points have no
    /// position yet, from the last point before a gap that has one to the first after it, those of
    /// the wires first; then between the connected parts of each net, as for `net_labels`, net by
    /// net in the order of the level.
    pub flylines: Vec<FlylineView>,
    /// Where a viewer may label each connected part of a net with the net's name: each wire of
    /// the net with those that meet it at an end or a bend of both as drawn, and the pins at those,
    /// is one part, and so is each pin on the net that no wire meets so; a wire that ends on the
    /// middle of another is apart from it. One label per part, of every net drawn on the level.
    pub net_labels: Vec<NetLabelView>,
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
    /// The applications of definitions written in the level's records that failed (7.2).
    pub failed_usages: Vec<FailedUsage>,
    /// The sets marked with GROUP (11.5), in the order of writing, those PRIORITY turns off too.
    pub groups: Vec<GroupInfo>,
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

#[derive(Clone, Debug)]
pub struct CurrentView {
    pub terminal: String,
    pub direction: String,
    pub at: Option<Pt>,
    pub dir: Option<u16>,
    /// A declaration port has no scene position; viewers may use an explicit port symbol.
    pub port_net: Option<String>,
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
    /// Complete table for drawing the symbol, including nested symbols; root is index zero.
    pub symbols: Vec<SymbolView>,
}

/// An instance, bundle or usage of a level, drawn or not: what the editor interface's snapshot
/// gives of it (`notes/rnl-editor-interface` in rnl-dev, 3.5).
#[derive(Clone, Debug)]
pub struct ObjectInfo {
    /// A scene usage with its own point (0.4), independently of its symbol or position.
    pub own_point: bool,
    /// The owner of all pin-valued roles, when no role names another positioned object.
    /// Nets do not affect this structural relation. Descendants follow `parent`.
    pub pin_of: Option<String>,
    /// A usage has its own symbol or creates drawing objects in its body/descendants.
    pub usage_has_drawing: bool,
    /// The complete statement that creates a usage, inclusive source coordinates.
    pub source: Option<(crate::diag::Loc, crate::diag::Loc)>,
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
    /// A usage's parameters as the statement that applies it writes them, in the order of the
    /// definition; the definition's defaults are not among them (9.3).
    pub params: Vec<(String, Lit)>,
    /// The style properties the document's own records give it, each as it wins in the style queue
    /// (8.3); those a package gives, such as the standard package's defaults, are left out.
    pub own_style: Style,
    /// The symbol it is drawn with.
    pub symbol: Option<SymbolInfo>,
    pub at: Option<Pt>,
    pub orient: Orient,
    /// The symbol's pins, including unplaced pins in 0.4.
    pub pins: Vec<PinPlace>,
    /// Its symbol, by index in `LevelView::symbols`, when it is drawn.
    pub symbol_view: Option<usize>,
    /// Where a usage comes from; [`UsageOrigin::Written`] for an instance or a bundle.
    pub origin: UsageOrigin,
    /// The usage whose expansion made a usage (14.8), by its id; none for one that a record of the
    /// level or a rule makes.
    pub parent: Option<String>,
    /// A usage whose priority, its own or that of the usage it is made in, is "disabled" (14.8):
    /// its facts are left out of the drawing and the solver's problem.
    pub disabled: bool,
}

/// Where a usage comes from.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum UsageOrigin {
    /// A statement of a record of the document.
    #[default]
    Written,
    /// A statement of a record that a solver wrote back, `type="generated"` (2.8).
    Solver,
    /// A rule (10.5): the package it is in, empty for the document's own, and where it is written.
    Rule { package: String, at: crate::diag::Loc },
    /// The body of the definition that the usage [`ObjectInfo::parent`] applies (9.4).
    Body,
}

/// A set marked with GROUP (11.5), for a tool to list with the usages. A reference to a set is
/// spread in a set (5.2), so a group is never itself a member of another: one group is within
/// another when its members are among the other's.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupInfo {
    /// Optional whole grid-unit additions to the solver's default spacing (11.5).
    pub spacing_x: Option<f64>,
    pub spacing_y: Option<f64>,
    /// `%core` when a record of the level names the set; otherwise where GROUP is written,
    /// `@file:line:col`, with `#n` after it when several groups have that place, as each expansion
    /// of a definition makes its own.
    pub id: String,
    /// Its members as a set role holds them ([`Attach`]): in order, each an object by its id or a
    /// point, nested sets followed down. In draft 0.4, usages without a symbol contribute
    /// their role members recursively; a usage with a symbol contributes itself (11.5).
    pub members: Vec<Attach>,
    /// The statement that gives GROUP: its first and last character.
    pub source: Option<(crate::diag::Loc, crate::diag::Loc)>,
    /// Where the statement comes from, as for a usage ([`ObjectInfo::origin`]).
    pub origin: UsageOrigin,
    /// The usage whose expansion wrote it, by id; none for a record of the level or a rule.
    pub parent: Option<String>,
    /// PRIORITY turns it off, on itself or on a usage it is made in (11.5, 14.8): the solver does
    /// not get it.
    pub disabled: bool,
    /// Where it stands in the tree of groups of a 0.4 level, by the rules of [`crate::group_tree`] on
    /// its members that have a place of their own, the objects the solver's problem places;
    /// [`GroupPlace::Off`] in a level of an earlier version, which has no tree of groups. The solver
    /// then also leaves out a set of fewer than two of the objects it lays out, and joins two sets
    /// that become one there, so its tree can have fewer groups.
    pub place: GroupPlace,
}

/// Where a group stands in the tree of groups ([`GroupInfo::place`]); groups by their index in
/// [`LevelView::groups`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroupPlace {
    /// In the tree, under this group, or at the top.
    Tree { parent: Option<usize> },
    /// Its members are those of this group, which stands for it.
    Same(usize),
    /// It shares members with this group without either holding the other, and is left out.
    Crosses(usize),
    /// Not in the tree: it has no members.
    Empty,
    /// Not in the tree: turned off, itself or by its first GROUP on the same members.
    #[default]
    Off,
}

/// An application of a definition, AS or ATTACH, written in a record of the level, that failed
/// and made no usage (7.2): what it names, so that a tool can list it with the objects it names.
#[derive(Clone, Debug, PartialEq)]
pub struct FailedUsage {
    /// The name the statement binds, such as `%dp1`, when it binds one.
    pub id: Option<String>,
    /// The definition, such as `$DIFFERENTIAL_PAIR`, when the statement names one.
    pub definition: Option<String>,
    /// The objects written for its roles, each with the role it is written for, labelled or by
    /// place, when that is known, and its id as [`ObjectInfo::attach`] writes ids.
    pub roles: Vec<(Option<String>, Attach)>,
    /// The statement: its first and last character.
    pub source: (crate::diag::Loc, crate::diag::Loc),
    /// Why it failed: the message of its error.
    pub reason: String,
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

/// A pin of an assigned symbol, in scene coordinates when the owner is positioned.
#[derive(Clone, Debug)]
pub struct PinPlace {
    /// Physical identity in 0.4, independent of names and placement.
    pub identity: Option<ScenePinIdentity>,
    /// The terminals at the pin: an instance's by number, `#M1:1`; a usage's by role, `p`.
    pub terminals: Vec<String>,
    pub at: Option<Pt>,
    /// The exit direction after the object's orientation, in degrees.
    pub dir: Option<u16>,
    /// The nets at the pin, as ids.
    pub nets: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ScenePinIdentity {
    /// Opaque editor id, not an RNL reference. Together with the owner, the index identifies the pin.
    pub id: String,
    /// Zero-based position in the assigned symbol's pin list and in the solver's symbol.
    pub index: usize,
    pub names: Vec<String>,
    pub bus: bool,
    /// A source expression for this exact physical pin, when one is available.
    pub reference: Option<String>,
    /// Position in the symbol's own coordinates.
    pub local_at: Pt,
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
