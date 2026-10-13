//! Run-time objects (spec-draft 5.1, 5.2, 6.5): handles and what they carry.
//!
//! Every statement with a keyword and every `{…}` makes a new object; references only fetch
//! existing handles. Objects live in one arena and are referred to by [`ObjId`]. Host objects
//! (the level, instances, nets, terminals) are unique per drawing level.

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::diag::DiagId;
use crate::geom::Orient;
use crate::syntax::{Member, SetExpr};

use super::ctx::Ctx;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjId(pub u32);

pub type SceneId = usize;
pub type UnitId = usize;
pub type RecId = usize;

/// The bare `#` before the level it is evaluated in is known (6.3).
pub const LEVEL: ObjId = ObjId(0);
/// The bare `?` in a symbol body: the object the symbol draws (13.3).
pub const OWNER: ObjId = ObjId(1);
/// No object: what a statement without a result gives.
pub const NOTHING: ObjId = ObjId(2);

/// A string with the unit whose text it is written in: a string passed into a definition is
/// read with the aliases of the file that wrote it (16.5).
#[derive(Clone, Debug, PartialEq)]
pub struct WStr {
    pub s: Rc<str>,
    pub unit: Option<UnitId>,
}

impl WStr {
    pub fn new(s: &str, unit: Option<UnitId>) -> WStr {
        WStr { s: Rc::from(s), unit }
    }

    pub fn as_str(&self) -> &str {
        &self.s
    }
}

/// A literal value of a parameter (3.7).
#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Num(f64),
    Str(WStr),
    Bool(bool),
    Arr(Vec<Lit>),
}

impl Lit {
    pub fn num(&self) -> Option<f64> {
        match self {
            Lit::Num(v) => Some(*v),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Lit::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn boolean(&self) -> Option<bool> {
        match self {
            Lit::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn show(&self) -> String {
        match self {
            Lit::Num(v) => crate::geom::fmt_num(*v),
            Lit::Str(s) => format!("{:?}", s.as_str()),
            Lit::Bool(b) => b.to_string(),
            Lit::Arr(items) => format!("[{}]", items.iter().map(Lit::show).collect::<Vec<_>>().join(", ")),
        }
    }
}

/// Style properties and their values (9.3).
pub type StyleMap = BTreeMap<String, Lit>;

/// Ordering data: the position of a statement (8.3, 8.5, 11.6). Lexicographic.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PosElem {
    Int(i64),
    Tag(&'static str),
}

pub type Pos = Vec<PosElem>;

pub fn pos(items: &[i64]) -> Pos {
    items.iter().map(|&k| PosElem::Int(k)).collect()
}

pub fn pos_plus(p: &Pos, tag: &'static str, k: i64) -> Pos {
    let mut out = p.clone();
    out.push(PosElem::Tag(tag));
    out.push(PosElem::Int(k));
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Disabled,
    Low,
    Normal,
    High,
}

impl Priority {
    pub fn parse(s: &str) -> Option<Priority> {
        Some(match s {
            "disabled" => Priority::Disabled,
            "low" => Priority::Low,
            "normal" => Priority::Normal,
            "high" => Priority::High,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Disabled => "disabled",
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
        }
    }
}

/// The parts of a TEXT other than its style (13.3): each is an item that a later TEXT with the
/// same key replaces one by one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextItems {
    pub text: Option<WStr>,
    pub halign: Option<String>,
    pub valign: Option<String>,
    pub rotate: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
}

impl TextItems {
    pub fn update(&mut self, o: &TextItems) {
        if o.text.is_some() {
            self.text = o.text.clone();
        }
        if o.halign.is_some() {
            self.halign = o.halign.clone();
        }
        if o.valign.is_some() {
            self.valign = o.valign.clone();
        }
        if o.rotate.is_some() {
            self.rotate = o.rotate;
        }
        if o.dx.is_some() {
            self.dx = o.dx;
        }
        if o.dy.is_some() {
            self.dy = o.dy;
        }
    }
}

/// A pin of a symbol (9.2.4).
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    /// Terminals: numbers or names; several for a bus pin.
    pub terms: Vec<TermKey>,
    pub pos: (f64, f64),
    pub label: Option<String>,
    /// Exit direction in degrees: 0 right, 90 up, 180 left, 270 down.
    pub dir: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TermKey {
    Num(u32),
    Name(String),
}

impl TermKey {
    pub fn same(&self, o: &TermKey) -> bool {
        match (self, o) {
            (TermKey::Name(a), TermKey::Name(b)) => a.eq_ignore_ascii_case(b),
            (TermKey::Num(a), TermKey::Num(b)) => a == b,
            _ => false,
        }
    }
}

/// A graphic of a symbol, in local coordinates (9.2.5).
#[derive(Clone, Debug, PartialEq)]
pub struct SymPrim {
    pub shape: Shape,
    pub points: Vec<(f64, f64)>,
    pub params: PrimParams,
    pub style: StyleMap,
    /// For TEXT: what it says and how it is placed.
    pub text: Option<TextItems>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    Line,
    Rectangle,
    Circle,
    Ellipse,
    Arc,
    Bezier,
    Text,
}

impl Shape {
    pub fn parse(kw: &str) -> Option<Shape> {
        Some(match kw {
            "LINE" => Shape::Line,
            "RECTANGLE" => Shape::Rectangle,
            "CIRCLE" => Shape::Circle,
            "ELLIPSE" => Shape::Ellipse,
            "ARC" => Shape::Arc,
            "BEZIER" => Shape::Bezier,
            "TEXT" => Shape::Text,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Shape::Line => "LINE",
            Shape::Rectangle => "RECTANGLE",
            Shape::Circle => "CIRCLE",
            Shape::Ellipse => "ELLIPSE",
            Shape::Arc => "ARC",
            Shape::Bezier => "BEZIER",
            Shape::Text => "TEXT",
        }
    }
}

/// The non-style parameters of a graphic.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrimParams {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub radius: Option<f64>,
    pub fill: Option<String>,
    pub arrow: Option<String>,
}

/// A label written in a symbol body on the bare `?` (13.3).
#[derive(Clone, Debug)]
pub struct SymLabel {
    pub key: Option<String>,
    pub items: TextItems,
    /// Its local position (PLACE in the body), or the local offset (dx, dy).
    pub position: Option<(f64, f64)>,
    pub style: StyleMap,
    /// The unit the key is written in: its aliases decide what `::` finds first (16.5).
    pub unit: UnitId,
    pub site: Option<Rc<Ctx>>,
    pub off: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SymbolDef {
    pub pins: Vec<Pin>,
    pub prims: Vec<SymPrim>,
    pub labels: Vec<SymLabel>,
    /// Usages drawn in the symbol (9.2.7).
    pub usages: Vec<ObjId>,
    pub bbox: Option<[f64; 4]>,
    /// The renderer's own default for objects without a symbol (9.2.6).
    pub default: bool,
}

#[derive(Clone, Debug)]
pub struct Role {
    pub name: String,
    pub kinds: Option<Vec<String>>,
    pub is_set: bool,
}

#[derive(Clone, Debug)]
pub struct DefData {
    pub roles: Vec<Role>,
    pub params: Vec<(String, Option<Lit>)>,
    pub body: Option<Rc<SetExpr>>,
    pub def_unit: UnitId,
    pub header: Rc<super::unit::Header>,
    pub rec: Option<RecId>,
    pub off: usize,
    /// The names its body cannot see (6.9).
    pub defining: Rc<super::ctx::Defining>,
}

impl DefData {
    pub fn local_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(b) = &self.body {
            for m in &b.members {
                for bd in &m.binds {
                    if bd.prefix == '%' && !out.contains(&bd.name) {
                        out.push(bd.name.clone());
                    }
                }
            }
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct UsageData {
    pub definition: ObjId,
    /// Role name -> the object (single role) or the set (set role).
    pub roles: Vec<(String, ObjId)>,
    pub role_sets: Vec<String>,
    pub params: Vec<(String, Option<Lit>)>,
    pub locals: Vec<(String, ObjId)>,
    /// The usage whose expansion made this one (14.8).
    pub parent: Option<ObjId>,
    pub generated: bool,
    pub priority: Option<Priority>,
    pub in_symbol: bool,
    pub sym_symbol: Option<ObjId>,
    pub sym_orient: Orient,
    pub own_of: Option<ObjId>,
    pub site: Option<Rc<Ctx>>,
    pub off: usize,
    pub frame: Option<super::ctx::FrameRef>,
    pub ctx_origin: Rc<str>,
}

impl UsageData {
    pub fn role(&self, name: &str) -> Option<ObjId> {
        self.roles.iter().find(|(n, _)| n == name).map(|(_, o)| *o)
    }

    pub fn local(&self, name: &str) -> Option<ObjId> {
        self.locals.iter().find(|(n, _)| n == name).map(|(_, o)| *o)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintKind {
    Align,
    Order,
    Distribute,
    Distance,
    OrientRelation,
}

impl ConstraintKind {
    pub fn parse(kw: &str) -> Option<ConstraintKind> {
        Some(match kw {
            "ALIGN" => ConstraintKind::Align,
            "ORDER" => ConstraintKind::Order,
            "DISTRIBUTE" => ConstraintKind::Distribute,
            "DISTANCE" => ConstraintKind::Distance,
            "ORIENT-RELATION" => ConstraintKind::OrientRelation,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ConstraintKind::Align => "ALIGN",
            ConstraintKind::Order => "ORDER",
            ConstraintKind::Distribute => "DISTRIBUTE",
            ConstraintKind::Distance => "DISTANCE",
            ConstraintKind::OrientRelation => "ORIENT-RELATION",
        }
    }
}

/// How far a constraint is from its parameters: "x"/"y" and the sense along it (14.3, 14.5).
#[derive(Clone, Debug)]
pub struct ConstraintParams {
    pub axis: Option<char>,
    /// "increasing", "decreasing" or "either".
    pub sense: Option<&'static str>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub relation: Option<Orient>,
}

#[derive(Clone, Debug)]
pub struct ConstraintData {
    pub kind: ConstraintKind,
    pub args: Vec<ObjId>,
    pub params: ConstraintParams,
    pub priority_param: Option<Priority>,
    pub chain: Vec<ObjId>,
    pub status: Option<&'static str>,
    pub eff: Option<Priority>,
    pub failed: bool,
    pub site: Option<Rc<Ctx>>,
    pub off: usize,
}

/// One ADD-TO or REMOVE-FROM on a set (5.2.2).
#[derive(Clone, Debug)]
pub struct SetMod {
    pub pos: Pos,
    pub add: bool,
    pub source: ObjId,
    pub ctx: Rc<Ctx>,
}

#[derive(Clone, Debug)]
pub enum SetKind {
    Plain,
    /// A selection (10.1): evaluated where and when it is used.
    Select {
        keyword: String,
        scope: ObjId,
        params: BTreeMap<String, Lit>,
        writer: UnitId,
        definition: Option<ObjId>,
        site: Option<Rc<Ctx>>,
        off: usize,
    },
    /// A generated set (10.5): filled per drawing level in the generation phase.
    Gen {
        by_scene: HashMap<SceneId, Vec<ObjId>>,
    },
    Union {
        source: ObjId,
    },
}

#[derive(Clone, Debug)]
pub struct SetData {
    /// Members with "spread": a reference to a set is spread one level (5.2).
    pub items: Vec<(ObjId, bool)>,
    /// A written `{…}` stays a member inside another set.
    pub explicit: bool,
    pub mods: Vec<SetMod>,
    /// The indexes in `mods` of the edits made in each scene, in order; `add_mod` keeps it.
    pub mods_by_scene: HashMap<SceneId, Vec<usize>>,
    /// The sources in `mods` that are lazy or can become so (10.3), in order; `add_mod` keeps it.
    pub lazy_sources: Vec<ObjId>,
    /// Whether one of `mods` is a REMOVE-FROM; `add_mod` keeps it.
    pub removes: bool,
    /// The host level unit that wrote it.
    pub writer: Option<UnitId>,
    pub kind: SetKind,
}

impl SetData {
    pub fn plain(items: Vec<(ObjId, bool)>, explicit: bool) -> SetData {
        SetData {
            items,
            explicit,
            mods: Vec::new(),
            mods_by_scene: HashMap::new(),
            lazy_sources: Vec::new(),
            removes: false,
            writer: None,
            kind: SetKind::Plain,
        }
    }

    /// Add an ADD-TO or REMOVE-FROM (5.2.2); `lazy` tells whether its source is lazy or can become so.
    pub fn add_mod(&mut self, m: SetMod, lazy: bool) {
        if let Some(s) = m.ctx.scene {
            self.mods_by_scene.entry(s).or_default().push(self.mods.len());
        }
        if lazy {
            self.lazy_sources.push(m.source);
        }
        self.removes |= !m.add;
        self.mods.push(m);
    }
}

#[derive(Clone, Debug)]
pub enum ObjData {
    Failed(Option<DiagId>),
    Marker,
    LevelObj {
        scene: SceneId,
    },
    Inst {
        scene: SceneId,
        idx: usize,
        bundle: Option<ObjId>,
    },
    Net {
        scene: SceneId,
        key: String,
        display: String,
    },
    Term {
        owner: ObjId,
        index: u32,
        name: Option<String>,
    },
    Point {
        x: Option<f64>,
        y: Option<f64>,
        local: bool,
    },
    /// `X::position` (6.5).
    Pos {
        parent: ObjId,
    },
    Wire {
        nets: Vec<ObjId>,
        bus: bool,
        points: Vec<ObjId>,
        mode: String,
        style: StyleMap,
        generated: bool,
        failed: bool,
        site: Option<Rc<Ctx>>,
        off: usize,
    },
    Route {
        net: ObjId,
        mode: String,
        style: StyleMap,
    },
    Prim {
        shape: Shape,
        points: Vec<ObjId>,
        params: PrimParams,
        style: StyleMap,
    },
    Text {
        attach: ObjId,
        items: TextItems,
        style: StyleMap,
        from_symbol: bool,
        sym_pos: Option<(f64, f64)>,
        generated: bool,
        failed: bool,
        site: Option<Rc<Ctx>>,
        off: usize,
    },
    KeyText {
        attach: ObjId,
        key: String,
        layers: Vec<(Pos, TextItems, StyleMap)>,
        from_symbol: bool,
        sym_pos: Option<(f64, f64)>,
        generated: bool,
        failed: bool,
        site: Option<Rc<Ctx>>,
        off: usize,
    },
    Usage(Box<UsageData>),
    Bundle {
        members: Vec<ObjId>,
        empty: bool,
        site: Option<Rc<Ctx>>,
    },
    Constraint(Box<ConstraintData>),
    Set(Box<SetData>),
    Symbol(Box<SymbolDef>),
    Style(StyleMap),
    Definition(Box<DefData>),
    /// A label in a symbol body that a PLACE in the body positions.
    SymLabel {
        symbol: ObjId,
        index: usize,
    },
}

#[derive(Clone, Debug)]
pub struct Obj {
    pub data: ObjData,
    /// The name it was first bound to: "g0" for `%g0`, "$X" for `$X`; empty when anonymous.
    pub name: String,
}

/// A member key for memoisation: a top-level member of a unit's records, or of a scope record.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemberKey {
    None,
    M { unit: UnitId, rec: RecId, k: usize, embedded: bool },
    Scope { rec: RecId, k: usize },
}

/// A shared member of a record's syntax tree, with the record it is in.
#[derive(Clone, Debug)]
pub struct MemberRef {
    pub rec: Option<RecId>,
    pub m: Rc<Member>,
}

impl MemberRef {
    pub fn same(&self, o: &MemberRef) -> bool {
        self.rec == o.rec && Rc::ptr_eq(&self.m, &o.m)
    }
}
