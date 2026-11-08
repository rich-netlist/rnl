//! Geometry hints: read-only diagnostics about how a level is drawn, for an author who checks a
//! drawing without looking at it, such as a language model. They are measured on the level as its
//! records draw it, written or solved, and never change it nor block it: each is a hint, or a
//! warning where two nets look joined. Each names what it is about, the instance and its pin, the
//! net, and the line of the record that places it. `rnl check` prints them, and a Document has
//! them when asked for (`BuildOptions::hints`, [`crate::Level::hints`]).
//!
//! The measures are simple and general:
//!
//! - a wire over a symbol: a segment that runs through the inside of a symbol's box, not one that
//!   ends on a pin of that symbol;
//! - a wire over a pin of another net, and the end of a wire on a wire of another net: they look
//!   joined (a warning); a wire that carries several nets, such as a bus, and a wire or pin of one
//!   of them are not two nets;
//! - a wire over a text, a text over a text: the boxes of the texts as the renderer lays them out;
//! - two nets' wires on one line over a stretch (a warning), or one grid step apart along two;
//! - the crossings: two segments of different wires crossing inside both, one summary line;
//! - a wire leaving a pin against the pin's direction: its first segment does not go the way the
//!   pin points;
//! - a T-join without a junction dot: at a point, the ends of wires, twice the wires that pass
//!   through it, and the pins, of one net, three or more, and no mark at the point, such as a dot;
//! - a silent open: a wire whose end lies on a wire or a pin of its net that it does not name, so
//!   that the two are not joined although they look it;
//! - a port that faces into the drawing (a symbol or another net's wire between it and the outside
//!   in the way it faces), or that is enclosed (so in every way);
//! - a net drawn in pieces joined only by its name, some of them wired, and a pin joined by its
//!   name alone, with neither a wire nor a label nor another symbol at it; a label is a text
//!   attached to the pin, such as a net-name text (`key="netname"`); nets with a supply or ground
//!   symbol are left out, as such symbols stand for their net, and so are the nets of an object
//!   not placed yet, and those a ROUTE asks to wire, which a solver joins;
//! - a wire of no length, or with a segment of none, and the wires of a net that close a loop.
//!
//! The objects are told apart by their structure, as the language gives it, never by the names of
//! a package's definitions or symbols ([`classify`]): an instance or a bundle is a device; a usage
//! attached to a net, or whose pins all lie at its own point and are on a net, stands for that net,
//! a port when a role is attached to a port of the level itself, and a rail otherwise, such as a
//! ground attached to a terminal; a usage whose pins all lie at its own point and are on no net, or that has none,
//! marks the point it is drawn at, such as a junction dot, a no-connect or an arrow head; any other
//! usage is a device.
//! So a dot is any such mark: whether a mark is a dot is up to a package, and whether a point with
//! two wires has one is up to the author (13.2), so a dot that joins fewer is not said.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rnl_runtime::diag::{Loc, Severity};
use rnl_runtime::geom::{fmt_num, Pt};
use rnl_runtime::markup;
use rnl_runtime::model::{Attach, ItemView, LevelView, ObjectInfo, SymbolView, TextTarget, Vertex};
use rnl_runtime::problem::{FactKind, LevelProblem, PointRef, Position, Slot};

const EPS: f64 = 1e-6;
/// One grid step of the standard symbols, in scene units.
const GRID: f64 = 1.0;
/// How many items of one kind a hint names before it says how many more.
const LIST: usize = 8;

/// What a geometry hint is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    WireOverSymbol,
    LooksJoined,
    WireOverText,
    TextOverText,
    Overlap,
    Close,
    Crossings,
    WrongExit,
    NoDot,
    SilentOpen,
    PortInside,
    PortEnclosed,
    PartlyNamed,
    NameOnly,
    ZeroLength,
    Loop,
}

impl Kind {
    pub const ALL: [Kind; 16] = [
        Kind::PortEnclosed,
        Kind::PortInside,
        Kind::NameOnly,
        Kind::PartlyNamed,
        Kind::LooksJoined,
        Kind::Overlap,
        Kind::WireOverSymbol,
        Kind::SilentOpen,
        Kind::NoDot,
        Kind::WrongExit,
        Kind::WireOverText,
        Kind::TextOverText,
        Kind::Close,
        Kind::ZeroLength,
        Kind::Loop,
        Kind::Crossings,
    ];

    /// Its name in a Document ([`crate::LevelHint::kind`]), such as "looks-joined".
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::WireOverSymbol => "wire-over-symbol",
            Kind::LooksJoined => "looks-joined",
            Kind::WireOverText => "wire-over-text",
            Kind::TextOverText => "text-over-text",
            Kind::Overlap => "overlap",
            Kind::Close => "close",
            Kind::Crossings => "crossings",
            Kind::WrongExit => "wrong-exit",
            Kind::NoDot => "no-dot",
            Kind::SilentOpen => "silent-open",
            Kind::PortInside => "port-inside",
            Kind::PortEnclosed => "port-enclosed",
            Kind::PartlyNamed => "partly-named",
            Kind::NameOnly => "name-only",
            Kind::ZeroLength => "zero-length",
            Kind::Loop => "loop",
        }
    }

    /// A count of hints of this kind, such as `3 wires over symbols` or `1 crossing`.
    pub fn count(self, n: usize) -> String {
        if n != 1 {
            return format!("{n} {}", self.plural());
        }
        let one = match self {
            Kind::WireOverSymbol => "wire over a symbol",
            Kind::LooksJoined => "place where two nets look joined",
            Kind::WireOverText => "wire over a text",
            Kind::TextOverText => "text over a text",
            Kind::Overlap => "pair of wires of two nets on one line",
            Kind::Close => "pair of wires of two nets one grid step apart",
            Kind::Crossings => "crossing",
            Kind::WrongExit => "wire leaving a pin against its direction",
            Kind::NoDot => "T-join without a junction dot",
            Kind::SilentOpen => "silent open",
            Kind::PortInside => "port facing into the drawing or buried",
            Kind::PortEnclosed => "port enclosed",
            Kind::PartlyNamed => "net partly wired and partly joined by name",
            Kind::NameOnly => "pin joined by name only",
            Kind::ZeroLength => "wire or segment of no length",
            Kind::Loop => "loop of one net",
        };
        format!("1 {one}")
    }

    /// A few words for a count, such as `3 wires over symbols`.
    pub fn plural(self) -> &'static str {
        match self {
            Kind::WireOverSymbol => "wires over symbols",
            Kind::LooksJoined => "places where two nets look joined",
            Kind::WireOverText => "wires over texts",
            Kind::TextOverText => "texts over texts",
            Kind::Overlap => "pairs of wires of two nets on one line",
            Kind::Close => "pairs of wires of two nets one grid step apart",
            Kind::Crossings => "crossings",
            Kind::WrongExit => "wires leaving a pin against its direction",
            Kind::NoDot => "T-joins without a junction dot",
            Kind::SilentOpen => "silent opens",
            Kind::PortInside => "ports facing into the drawing or buried",
            Kind::PortEnclosed => "ports enclosed",
            Kind::PartlyNamed => "nets partly wired and partly joined by name",
            Kind::NameOnly => "pins joined by name only",
            Kind::ZeroLength => "wires or segments of no length",
            Kind::Loop => "loops of one net",
        }
    }
}

/// One geometry hint.
#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    pub kind: Kind,
    pub severity: Severity,
    /// Where the record that draws or places the first thing it names is; the level's line when
    /// there is none.
    pub loc: Option<Loc>,
    pub message: String,
}

impl Hint {
    /// As `rnl check` prints a diagnostic: `hint file.sp:12:3: …`.
    pub fn render(&self) -> String {
        match &self.loc {
            Some(l) => format!("{} {l}: {}", self.severity.as_str(), self.message),
            None => format!("{}: {}", self.severity.as_str(), self.message),
        }
    }
}

/// The geometry hints of one level, in the order of [`Kind::ALL`], the crossings last.
#[derive(Clone, Debug, Default)]
pub struct LevelHints {
    pub title: String,
    pub hints: Vec<Hint>,
    /// How many crossings of wires the level has.
    pub crossings: usize,
}

impl LevelHints {
    /// How many hints of each kind, in the order of [`Kind::ALL`], the crossings by their count.
    pub fn counts(&self) -> Vec<(Kind, usize)> {
        Kind::ALL
            .iter()
            .map(|&k| {
                (
                    k,
                    if k == Kind::Crossings {
                        self.crossings
                    } else {
                        self.hints.iter().filter(|h| h.kind == k).count()
                    },
                )
            })
            .filter(|(_, n)| *n > 0)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Device,
    Port,
    Rail,
    /// A mark drawn on a wire or a pin: no connection, a current arrow, an arrow head.
    Mark,
}

struct Part {
    name: String,
    class: Class,
    /// For a usage, whether a role is attached to a port of the level, as a port tag's is: such a
    /// usage stands for the port and blocks no other port.
    on_port: bool,
    bbox: [f64; 4],
    /// Indexes into [`Drawing::pins`].
    pins: Vec<usize>,
    loc: Option<Loc>,
}

struct PinAt {
    part: usize,
    pos: Pt,
    dir: Option<Pt>,
    nets: Vec<String>,
    /// As a message names it: `#M3:s`, `%a_port:p`.
    name: String,
    /// What a wire names it by: `#M3:3`, `%a_port:p`.
    keys: Vec<String>,
}

struct WireAt {
    nets: Vec<String>,
    /// Its points, a point repeated taken once.
    points: Vec<Pt>,
    /// Whether each end is where a run of a wire drawn in parts stops at a gap: the wire goes on
    /// as a flyline from there.
    open: [bool; 2],
    /// Every vertex the wire names.
    keys: BTreeSet<String>,
    name: String,
    loc: Option<Loc>,
    /// For a wire drawn straight through its points, where a point is repeated, a segment of no
    /// length.
    repeated: Option<Pt>,
    /// The wire it is drawn of, by its id in the editor interface, when it is drawn in runs, one
    /// for each run of its points that have a position: the runs of one wire are one piece, joined
    /// by its flylines over the gaps.
    runs_of: Option<String>,
}

struct TextAt {
    name: String,
    /// The box of its glyphs: from the baseline to the height of a capital, not the line's box.
    bbox: [f64; 4],
    loc: Option<Loc>,
    /// The pin it is attached to, by its index in [`Drawing::pins`].
    pin: Option<usize>,
}

struct Seg {
    wire: usize,
    a: Pt,
    b: Pt,
}

/// One coordinate of a point.
type Axis = fn(Pt) -> f64;

/// The size of a cell of the grid the segments and texts are found by.
const CELL: f64 = 4.0;

fn cell(p: Pt) -> (i64, i64) {
    ((p.0 / CELL).floor() as i64, (p.1 / CELL).floor() as i64)
}

/// The cells a box covers, a little widened.
fn cells(b: [f64; 4]) -> impl Iterator<Item = (i64, i64)> {
    let (lo, hi) = (cell((b[0] - 1e-6, b[1] - 1e-6)), cell((b[2] + 1e-6, b[3] + 1e-6)));
    (lo.0..=hi.0).flat_map(move |x| (lo.1..=hi.1).map(move |y| (x, y)))
}

fn seg_box(s: &Seg) -> [f64; 4] {
    [s.a.0.min(s.b.0), s.a.1.min(s.b.1), s.a.0.max(s.b.0), s.a.1.max(s.b.1)]
}

/// What the hints read of a level, with what finds it by place.
struct Drawing<'a> {
    lv: &'a LevelView,
    parts: Vec<Part>,
    pins: Vec<PinAt>,
    wires: Vec<WireAt>,
    texts: Vec<TextAt>,
    segs: Vec<Seg>,
    /// Where the marks are drawn ([`Class::Mark`]), such as junction dots.
    marks: Vec<Pt>,
    level_loc: Loc,
    /// The pins by their point.
    pin_at: HashMap<(i64, i64), Vec<usize>>,
    /// The points of the wires by their point: the wire and the point's index in it.
    vertex_at: HashMap<(i64, i64), Vec<(usize, usize)>>,
    /// The segments by the cells their boxes cover.
    seg_cells: HashMap<(i64, i64), Vec<usize>>,
    /// The texts by the cells their boxes cover.
    text_cells: HashMap<(i64, i64), Vec<usize>>,
    /// The nets a ROUTE asks to wire (13.7), as the problem names them: a solver joins their pins.
    routed: BTreeSet<String>,
}

/// The geometry hints of one level as it is drawn; `problem` is what a solver reads of it, read
/// with the records of the solver, for the line of the record that places each object and for the
/// ports of the level. A level with nothing placed has none.
pub fn level_hints(lv: &LevelView, problem: Option<&LevelProblem>) -> LevelHints {
    let mut out = LevelHints { title: lv.title.clone(), ..LevelHints::default() };
    if lv.items.is_empty() && lv.wires.is_empty() {
        return out;
    }
    let d = Drawing::new(lv, problem);
    let mut hints = Vec::new();
    d.ports(&mut hints);
    d.name_joins(&mut hints);
    d.looks_joined(&mut hints);
    d.overlaps(&mut hints);
    d.over_symbols(&mut hints);
    d.silent_opens(&mut hints);
    d.dots(&mut hints);
    d.exits(&mut hints);
    d.texts(&mut hints);
    d.zero_and_loops(&mut hints);
    out.crossings = d.crossings(&mut hints);
    let order = |k: Kind| Kind::ALL.iter().position(|x| *x == k).unwrap_or(0);
    hints.sort_by_key(|h| order(h.kind));
    out.hints = hints;
    out
}

/// The geometry hints of every drawing level of an interpretation read with its problems
/// (`Options::problems`), in the order of the levels.
pub fn interpretation_hints(r: &rnl_runtime::Interpretation) -> Vec<LevelHints> {
    r.model
        .levels
        .iter()
        .map(|lv| level_hints(lv, r.problems.iter().find(|p| p.level == lv.level && p.title == lv.title)))
        .collect()
}

// ---------------------------------------------------------------- geometry

/// The nets a usage's roles are attached to, as `&name`.
fn role_nets(o: &ObjectInfo) -> Vec<&str> {
    fn nets<'b>(a: &'b Attach, out: &mut Vec<&'b str>) {
        match a {
            Attach::Object(id) if id.starts_with('&') => out.push(id),
            Attach::Set(items) => items.iter().for_each(|x| nets(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (_, a) in &o.attach {
        nets(a, &mut out);
    }
    out
}

/// Whether a usage stands for a net: one of its roles is attached to a net, or its symbol has pins,
/// all at the usage's own point, on a net, such as a ground attached to a terminal.
fn stands_for_net(o: &ObjectInfo, it: &ItemView, sym: &SymbolView) -> bool {
    o.kind == "usage"
        && (!role_nets(o).is_empty()
            || (pins_at_own_point(sym) && !sym.pins.is_empty() && it.pin_nets.iter().any(|n| !n.is_empty())))
}

/// The nets a usage that stands for a net stands for, as `&name`: those its roles are attached to,
/// and those of its pins.
fn standing_nets(o: &ObjectInfo, it: &ItemView) -> Vec<String> {
    let mut nets: Vec<String> = role_nets(o).iter().map(|n| n.to_string()).collect();
    nets.extend(it.pin_nets.iter().flatten().map(|n| format!("&{n}")));
    nets.sort();
    nets.dedup();
    nets
}

/// Whether every pin of a symbol lies at its own point, the point it is placed at; one with no pin
/// has none elsewhere.
fn pins_at_own_point(sym: &SymbolView) -> bool {
    sym.pins.iter().all(|p| near(p.pos, (0.0, 0.0)))
}

/// What a drawn object is, from its structure as the language gives it, not from the names of a
/// package's definitions or symbols:
/// - an instance or a bundle is a device;
/// - a usage with a role attached to a net, or whose pins all lie at its own point and are on a
///   net, such as a ground attached to a terminal, stands for that net: a port when a role is
///   attached to a port of the level itself (`ports`, as `&name`) and no other usage stands for
///   that net (`standing`, how many do), as a port tag does; a rail, such as a ground or a supply,
///   otherwise, the grounds drawn on a port `vss` included;
/// - a usage whose pins all lie at its own point and are on no net, or that has none, marks that
///   point, such as a junction dot, a no-connect or an arrow head;
/// - any other usage is a device.
fn classify(
    it: &ItemView,
    info: Option<&ObjectInfo>,
    sym: &SymbolView,
    ports: &BTreeSet<String>,
    standing: &HashMap<String, usize>,
) -> Class {
    let Some(o) = info.filter(|o| o.kind == "usage") else { return Class::Device };
    if stands_for_net(o, it, sym) {
        let port = role_nets(o).iter().any(|n| ports.contains(*n) && standing.get(*n) == Some(&1));
        return if port { Class::Port } else { Class::Rail };
    }
    if pins_at_own_point(sym) {
        Class::Mark
    } else {
        Class::Device
    }
}

fn near(a: Pt, b: Pt) -> bool {
    (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS
}

fn dist(a: Pt, b: Pt) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// `q` on the segment from `a` to `b`, its ends included.
fn on_segment(q: Pt, a: Pt, b: Pt) -> bool {
    a.0.min(b.0) - EPS <= q.0
        && q.0 <= a.0.max(b.0) + EPS
        && a.1.min(b.1) - EPS <= q.1
        && q.1 <= a.1.max(b.1) + EPS
        && ((b.0 - a.0) * (q.1 - a.1) - (b.1 - a.1) * (q.0 - a.0)).abs() < EPS * (1.0 + dist(a, b))
}

/// `q` on the segment and at neither end.
fn inside_segment(q: Pt, a: Pt, b: Pt) -> bool {
    on_segment(q, a, b) && !near(q, a) && !near(q, b)
}

/// Two segments that cross inside both: a shared end or a touch does not count.
fn cross(p: Pt, q: Pt, r: Pt, s: Pt) -> bool {
    let side = |a: Pt, b: Pt, c: Pt| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    let (d1, d2, d3, d4) = (side(p, q, r), side(p, q, s), side(r, s, p), side(r, s, q));
    d1 * d2 < -EPS && d3 * d4 < -EPS
}

/// Two segments that meet anywhere, ends and touches included.
fn meet(p: Pt, q: Pt, r: Pt, s: Pt) -> bool {
    cross(p, q, r, s) || on_segment(r, p, q) || on_segment(s, p, q) || on_segment(p, r, s) || on_segment(q, r, s)
}

/// A segment with a part of positive length strictly inside a box (Liang–Barsky).
fn through_box(a: Pt, b: Pt, bx: [f64; 4], eps: f64) -> bool {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    for (p, q) in
        [(-dx, a.0 - (bx[0] + eps)), (dx, (bx[2] - eps) - a.0), (-dy, a.1 - (bx[1] + eps)), (dy, (bx[3] - eps) - a.1)]
    {
        if p == 0.0 {
            // parallel to this side: outside it, or along it, is not inside
            if q <= 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            if t > t1 {
                return false;
            }
            t0 = t0.max(t);
        } else {
            if t < t0 {
                return false;
            }
            t1 = t1.min(t);
        }
    }
    t1 - t0 > 1e-9
}

fn boxes_overlap(a: [f64; 4], b: [f64; 4], eps: f64) -> bool {
    a[0] < b[2] - eps && b[0] < a[2] - eps && a[1] < b[3] - eps && b[1] < a[3] - eps
}

fn pt(p: Pt) -> String {
    format!("({}, {})", fmt_num(round(p.0)), fmt_num(round(p.1)))
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn way(d: Pt) -> &'static str {
    if d.0.abs() >= d.1.abs() {
        if d.0 > 0.0 {
            "right"
        } else {
            "left"
        }
    } else if d.1 > 0.0 {
        "up"
    } else {
        "down"
    }
}

fn unit(d: Pt) -> Pt {
    let l = (d.0 * d.0 + d.1 * d.1).sqrt();
    if l < EPS {
        (0.0, 0.0)
    } else {
        (d.0 / l, d.1 / l)
    }
}

fn dir_of(deg: u16) -> Option<Pt> {
    match deg {
        0 => Some((1.0, 0.0)),
        90 => Some((0.0, 1.0)),
        180 => Some((-1.0, 0.0)),
        270 => Some((0.0, -1.0)),
        _ => None,
    }
}

fn key(p: Pt) -> (i64, i64) {
    ((p.0 * 1000.0).round() as i64, (p.1 * 1000.0).round() as i64)
}

/// A list of names, at most [`LIST`] of them, and how many more.
fn list(names: &[String]) -> String {
    let mut s = names.iter().take(LIST).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > LIST {
        s.push_str(&format!(" and {} more", names.len() - LIST));
    }
    s
}

fn nets(ns: &[String]) -> String {
    ns.iter().map(|n| format!("&{}", rnl_runtime::model::quoted(n))).collect::<Vec<_>>().join(" ")
}

fn share(a: &[String], b: &[String]) -> bool {
    a.iter().any(|n| b.contains(n))
}

/// A place `@file:line:col` as the model's ids write it.
fn site(id: &str) -> Option<Loc> {
    let rest = id.strip_prefix('@')?;
    let rest = rest.split('#').next()?;
    let mut parts = rest.rsplitn(3, ':');
    let col = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    let file = parts.next()?;
    Some(Loc::new(file, line, col))
}

/// The words of a definition's name: `$GROUND_STUB` is `ground stub`.
fn words(def: &str) -> String {
    def.trim_start_matches('$').to_lowercase().replace('_', " ")
}

// ---------------------------------------------------------------- reading the level

impl<'a> Drawing<'a> {
    fn new(lv: &'a LevelView, problem: Option<&LevelProblem>) -> Drawing<'a> {
        let by_id: HashMap<&str, &ObjectInfo> = lv.objects.iter().map(|o| (o.id.as_str(), o)).collect();
        let level_loc = Loc::line(lv.file.clone(), lv.line.max(1));
        // where each object is placed: the first PLACE of its origin in the order of 8.5
        let mut placed: HashMap<String, Loc> = HashMap::new();
        if let Some(p) = problem {
            let mut first: BTreeMap<usize, Loc> = BTreeMap::new();
            for f in &p.facts {
                let (FactKind::Place { at: PointRef::Origin(k), .. }, Some(src)) = (&f.kind, &f.source) else {
                    continue;
                };
                first.entry(*k).or_insert_with(|| src.clone());
            }
            for (k, o) in p.objects.iter().enumerate() {
                if let Position::Own { x: Slot::Fixed { fact, .. }, .. } = &o.position {
                    if let Some(src) = p.facts.get(*fact).and_then(|f| f.source.clone()) {
                        first.entry(k).or_insert(src);
                    }
                }
            }
            for (k, loc) in first {
                if let Some(id) = p.ids.objects.get(k) {
                    placed.insert(id.clone(), loc);
                }
            }
        }
        let names = Names::new(lv, &by_id);
        let mut d = Drawing {
            lv,
            parts: Vec::new(),
            pins: Vec::new(),
            wires: Vec::new(),
            texts: Vec::new(),
            segs: Vec::new(),
            marks: Vec::new(),
            level_loc,
            pin_at: HashMap::new(),
            vertex_at: HashMap::new(),
            seg_cells: HashMap::new(),
            text_cells: HashMap::new(),
            routed: problem.map(|p| p.routes.iter().map(|r| r.net.clone()).collect()).unwrap_or_default(),
        };
        // the ports of the level, for what each object is (`classify`)
        let ports: BTreeSet<String> = problem
            .map(|p| p.nets.iter().filter(|n| n.port.is_some()).map(|n| format!("&{}", n.name)).collect())
            .unwrap_or_default();
        // how many drawn usages stand for each net
        let mut standing: HashMap<String, usize> = HashMap::new();
        for it in &lv.items {
            let sym = &lv.symbols[it.symbol];
            if let Some(o) = by_id.get(it.object_id.as_str()).filter(|o| stands_for_net(o, it, sym)) {
                for n in standing_nets(o, it) {
                    *standing.entry(n).or_default() += 1;
                }
            }
        }
        for it in &lv.items {
            let sym = &lv.symbols[it.symbol];
            let info = (!it.object_id.is_empty()).then(|| by_id.get(it.object_id.as_str()).copied()).flatten();
            let class = classify(it, info, sym, &ports, &standing);
            let on_port = info.is_some_and(|o| role_nets(o).iter().any(|n| ports.contains(*n)));
            let corners = [
                (sym.bbox[0], sym.bbox[1]),
                (sym.bbox[0], sym.bbox[3]),
                (sym.bbox[2], sym.bbox[1]),
                (sym.bbox[2], sym.bbox[3]),
            ]
            .map(|c| {
                let q = it.orient.apply(c);
                (it.at.0 + q.0, it.at.1 + q.1)
            });
            let bbox = [
                corners.iter().map(|c| c.0).fold(f64::MAX, f64::min),
                corners.iter().map(|c| c.1).fold(f64::MAX, f64::min),
                corners.iter().map(|c| c.0).fold(f64::MIN, f64::max),
                corners.iter().map(|c| c.1).fold(f64::MIN, f64::max),
            ];
            let id = if it.object_id.is_empty() { it.id.clone() } else { it.object_id.clone() };
            let name = names.object(&id);
            let loc = placed.get(&id).cloned().or_else(|| info.and_then(|o| o.source.as_ref().map(|(a, _)| a.clone())));
            let k = d.parts.len();
            let mut pins = Vec::new();
            for (j, pv) in sym.pins.iter().enumerate() {
                let q = it.orient.apply(pv.pos);
                let pos = (it.at.0 + q.0, it.at.1 + q.1);
                let dir = pv.dir.and_then(dir_of).map(|v| it.orient.apply(v));
                let terminals: Vec<String> =
                    info.and_then(|o| o.pins.get(j)).map(|p| p.terminals.clone()).unwrap_or_default();
                let mut keys = Vec::new();
                let mut shown = Vec::new();
                for t in &terminals {
                    if t.starts_with('#') {
                        keys.push(t.clone());
                        shown.push(names.terminal(t));
                    } else {
                        keys.push(format!("{id}:{t}"));
                        shown.push(if class == Class::Device { format!("{name}:{t}") } else { name.clone() });
                    }
                }
                let pin_name = shown.first().cloned().unwrap_or_else(|| format!("{name}:{}", j + 1));
                pins.push(d.pins.len());
                if class != Class::Mark {
                    d.pin_at.entry(key(pos)).or_default().push(d.pins.len());
                }
                d.pins.push(PinAt {
                    part: k,
                    pos,
                    dir,
                    nets: it.pin_nets.get(j).cloned().unwrap_or_default(),
                    name: pin_name,
                    keys,
                });
            }
            if class == Class::Mark {
                d.marks.push(it.at);
            }
            d.parts.push(Part { name, class, on_port, bbox, pins, loc });
        }
        // where each vertex a wire names is: a pin by its key, a named point
        let mut at_key: HashMap<String, Pt> = HashMap::new();
        for p in &d.pins {
            for k in &p.keys {
                at_key.entry(k.clone()).or_insert(p.pos);
            }
        }
        for p in &lv.points {
            if let Some(at) = p.at {
                at_key.entry(p.id.clone()).or_insert(at);
            }
        }
        let vkey = |v: &Vertex| -> Option<String> {
            match v {
                Vertex::Terminal(t) => Some(t.clone()),
                Vertex::Point(p) => Some(p.clone()),
                Vertex::Usage { usage, role } => Some(format!("{usage}:{role}")),
                Vertex::At(_) => None,
            }
        };
        let vpos = |v: &Vertex| -> Option<Pt> {
            match v {
                Vertex::At(p) => Some(*p),
                _ => vkey(v).and_then(|k| at_key.get(&k).copied()),
            }
        };
        for w in &lv.wires {
            if w.points.is_empty() {
                continue;
            }
            let keys: BTreeSet<String> = w.via.iter().filter_map(vkey).collect();
            // a point written twice in a row is one point; a segment of no length is said only in the
            // middle of a wire, not where an end repeats the place of the pin or point it names
            let last = w.points.len().saturating_sub(1);
            let repeated = if w.mode == "straight" {
                w.points
                    .windows(2)
                    .enumerate()
                    .find(|(i, s)| *i > 0 && i + 1 < last && near(s[0], s[1]))
                    .map(|(_, s)| s[0])
            } else {
                None
            };
            let mut points = w.points.clone();
            points.dedup_by(|a, b| near(*a, *b));
            let (a, b) = (points[0], points[points.len() - 1]);
            // the vertex at each end; a run of a wire drawn in parts ends at a gap, where the wire
            // goes on as a flyline, unless that end is the wire's own
            let at_end = |q: Pt, own: usize| -> (Option<usize>, bool) {
                if w.via.get(own).and_then(vpos).is_some_and(|p| near(p, q)) {
                    return (Some(own), false);
                }
                let v = w.via.iter().position(|v| vpos(v).is_some_and(|p| near(p, q)));
                (v, !w.whole)
            };
            let (ia, open_a) = at_end(a, 0);
            let (ib, open_b) = at_end(b, w.via.len().saturating_sub(1));
            let (va, vb) = (ia.and_then(|i| w.via.get(i)), ib.and_then(|i| w.via.get(i)));
            let shown = |v: Option<&Vertex>, p: Pt| -> String {
                match v {
                    Some(Vertex::Terminal(t)) => names.terminal(t),
                    Some(Vertex::Point(n)) => n.clone(),
                    Some(Vertex::Usage { usage, role }) => {
                        let o = names.object(usage);
                        if usage.starts_with('@') {
                            o
                        } else {
                            format!("{o}:{role}")
                        }
                    }
                    _ => pt(p),
                }
            };
            let name = format!("the wire of {} from {} to {}", nets(&w.nets), shown(va, a), shown(vb, b));
            // the place of a wire written in a package, such as a stub's, is not the author's
            let loc = site(&w.object_id).filter(|l| !(l.file.ends_with(".rnl") && !lv.file.ends_with(".rnl")));
            let k = d.wires.len();
            for (i, p) in points.iter().enumerate() {
                d.vertex_at.entry(key(*p)).or_default().push((k, i));
            }
            for s in points.windows(2) {
                let seg = Seg { wire: k, a: s[0], b: s[1] };
                for c in cells(seg_box(&seg)) {
                    d.seg_cells.entry(c).or_default().push(d.segs.len());
                }
                d.segs.push(seg);
            }
            d.wires.push(WireAt {
                nets: w.nets.clone(),
                points,
                open: [open_a, open_b],
                keys,
                name,
                loc,
                repeated,
                runs_of: (!w.whole && !w.object_id.is_empty()).then(|| w.object_id.clone()),
            });
        }
        let boxes = rnl_render::text_boxes(lv, &rnl_render::RenderOptions::default());
        for (t, b) in lv.texts.iter().zip(boxes) {
            let Some(line_box) = b else { continue };
            if line_box[2] - line_box[0] < EPS || line_box[3] - line_box[1] < EPS {
                continue;
            }
            let bbox = glyph_box(t, line_box);
            let content: String = t.lines.iter().map(|l| markup::plain(l)).collect::<Vec<_>>().join(" ");
            let owner = t.attach.as_deref().map(|a| names.object_or_terminal(a));
            let name = match (t.key.as_deref(), &owner) {
                (Some("name"), Some(o)) => format!("the name \"{content}\" of {o}"),
                (Some("value"), Some(o)) => format!("the value \"{content}\" of {o}"),
                (Some("netname"), Some(o)) => format!("the net label \"{content}\" at {o}"),
                (Some(k), Some(o)) => format!("the text {k} \"{content}\" of {o}"),
                (_, Some(o)) => format!("the text \"{content}\" at {o}"),
                _ => format!("the text \"{content}\""),
            };
            let pin = match t.target {
                Some(TextTarget::Pin(i, j)) => d.parts.get(i).and_then(|p| p.pins.get(j)).copied(),
                _ => None,
            };
            let owner_loc = match t.target {
                Some(TextTarget::Item(i)) | Some(TextTarget::Pin(i, _)) => d.parts.get(i).and_then(|p| p.loc.clone()),
                _ => None,
            };
            let loc =
                site(&t.object_id).filter(|l| !(l.file.ends_with(".rnl") && !lv.file.ends_with(".rnl"))).or(owner_loc);
            for c in cells(bbox) {
                d.text_cells.entry(c).or_default().push(d.texts.len());
            }
            d.texts.push(TextAt { name, bbox, loc, pin });
        }
        d
    }

    fn hint(&self, out: &mut Vec<Hint>, kind: Kind, severity: Severity, loc: Option<&Loc>, message: String) {
        let loc = loc.cloned().unwrap_or_else(|| self.level_loc.clone());
        out.push(Hint { kind, severity, loc: Some(loc), message: format!("{}: {message}", self.lv.title) });
    }

    /// `the symbol of #M1 (placed at line 12)`.
    fn placed(&self, part: usize) -> String {
        let p = &self.parts[part];
        match &p.loc {
            Some(l) => format!("{} (line {})", p.name, l.line),
            None => p.name.clone(),
        }
    }

    fn wire_named(&self, w: usize) -> String {
        let w = &self.wires[w];
        match &w.loc {
            Some(l) => format!("{} (line {})", w.name, l.line),
            None => w.name.clone(),
        }
    }

    /// The pins at a point, marks left out.
    fn pins_at(&self, q: Pt) -> impl Iterator<Item = (usize, &PinAt)> {
        self.pin_at
            .get(&key(q))
            .into_iter()
            .flatten()
            .map(move |&i| (i, &self.pins[i]))
            .filter(move |(_, p)| near(p.pos, q))
    }

    /// The segments that hold a point, their ends included.
    fn segs_at(&self, q: Pt) -> impl Iterator<Item = usize> + '_ {
        self.seg_cells
            .get(&cell(q))
            .into_iter()
            .flatten()
            .copied()
            .filter(move |&i| on_segment(q, self.segs[i].a, self.segs[i].b))
    }

    /// The wires whose drawing holds a point, a wire of one point included.
    fn wires_at(&self, q: Pt) -> BTreeSet<usize> {
        let mut out: BTreeSet<usize> = self.segs_at(q).map(|i| self.segs[i].wire).collect();
        out.extend(
            self.vertex_at
                .get(&key(q))
                .into_iter()
                .flatten()
                .filter(|(w, i)| near(self.wires[*w].points[*i], q))
                .map(|(w, _)| *w),
        );
        out
    }

    /// The segments whose cells a box covers, each once, in order.
    fn segs_near(&self, b: [f64; 4]) -> BTreeSet<usize> {
        cells(b).flat_map(|c| self.seg_cells.get(&c).into_iter().flatten().copied()).collect()
    }

    // ---------------------------------------------------------------- the hints

    /// A wire over a symbol: a segment through the inside of a symbol's box, not one that ends on a
    /// pin of that symbol.
    fn over_symbols(&self, out: &mut Vec<Hint>) {
        let mut seen = BTreeSet::new();
        for (k, p) in self.parts.iter().enumerate() {
            if p.class == Class::Mark {
                continue;
            }
            let shrunk = [p.bbox[0] + 0.05, p.bbox[1] + 0.05, p.bbox[2] - 0.05, p.bbox[3] - 0.05];
            for i in self.segs_near(p.bbox) {
                let s = &self.segs[i];
                if seen.contains(&(s.wire, k)) {
                    continue;
                }
                if p.pins.iter().any(|&j| near(self.pins[j].pos, s.a) || near(self.pins[j].pos, s.b)) {
                    continue;
                }
                if through_box(s.a, s.b, shrunk, 0.0) {
                    seen.insert((s.wire, k));
                }
            }
        }
        for (w, k) in seen {
            let p = &self.parts[k];
            self.hint(
                out,
                Kind::WireOverSymbol,
                Severity::Hint,
                self.wires[w].loc.as_ref().or(p.loc.as_ref()),
                format!("{} runs over the symbol of {}", self.wire_named(w), self.placed(k)),
            );
        }
    }

    /// A pin on a wire of another net, and the end of a wire on a wire of another net: they look
    /// joined.
    fn looks_joined(&self, out: &mut Vec<Hint>) {
        for p in &self.pins {
            if p.nets.is_empty() || self.parts[p.part].class == Class::Mark {
                continue;
            }
            for w in self.wires_at(p.pos) {
                let wire = &self.wires[w];
                if share(&p.nets, &wire.nets) {
                    continue;
                }
                self.hint(
                    out,
                    Kind::LooksJoined,
                    Severity::Warning,
                    wire.loc.as_ref().or(self.parts[p.part].loc.as_ref()),
                    format!(
                        "{} of {} lies on {}, a wire of another net, at {}: it looks joined",
                        p.name,
                        nets(&p.nets),
                        self.wire_named(w),
                        pt(p.pos)
                    ),
                );
            }
        }
        for (w, wire) in self.wires.iter().enumerate() {
            for q in [wire.points[0], wire.points[wire.points.len() - 1]] {
                for v in self.wires_at(q) {
                    let other = &self.wires[v];
                    // an end on a pin of the other net is said above
                    if v == w
                        || share(&wire.nets, &other.nets)
                        || self.pins_at(q).any(|(_, p)| share(&p.nets, &other.nets))
                    {
                        continue;
                    }
                    self.hint(
                        out,
                        Kind::LooksJoined,
                        Severity::Warning,
                        wire.loc.as_ref(),
                        format!(
                            "{} ends on {}, a wire of another net, at {}: it looks joined",
                            self.wire_named(w),
                            self.wire_named(v),
                            pt(q)
                        ),
                    );
                }
            }
        }
    }

    /// Two nets' wires on one line over a stretch, or one grid step apart along two or more.
    fn overlaps(&self, out: &mut Vec<Hint>) {
        let mut seen: BTreeSet<(usize, usize, bool)> = BTreeSet::new();
        let hv = |g: &Seg| {
            if (g.a.1 - g.b.1).abs() < EPS {
                Some(0)
            } else if (g.a.0 - g.b.0).abs() < EPS {
                Some(1)
            } else {
                None
            }
        };
        for (i, s) in self.segs.iter().enumerate() {
            let Some(ax) = hv(s) else { continue };
            let b = seg_box(s);
            for j in self.segs_near([b[0] - GRID, b[1] - GRID, b[2] + GRID, b[3] + GRID]) {
                if j <= i {
                    continue;
                }
                let t = &self.segs[j];
                if s.wire == t.wire || hv(t) != Some(ax) || share(&self.wires[s.wire].nets, &self.wires[t.wire].nets) {
                    continue;
                }
                // along: the axis the segments run on; across: the other
                let (along, across): (Axis, Axis) =
                    if ax == 0 { (|p: Pt| p.0, |p: Pt| p.1) } else { (|p: Pt| p.1, |p: Pt| p.0) };
                let gap = (across(s.a) - across(t.a)).abs();
                let lo = along(s.a).min(along(s.b)).max(along(t.a).min(along(t.b)));
                let hi = along(s.a).max(along(s.b)).min(along(t.a).max(along(t.b)));
                let shared = hi - lo;
                let (a, b) = (s.wire.min(t.wire), s.wire.max(t.wire));
                if gap < EPS && shared > EPS {
                    if seen.insert((a, b, true)) {
                        let at = if ax == 0 { ((lo + hi) / 2.0, across(s.a)) } else { (across(s.a), (lo + hi) / 2.0) };
                        self.hint(
                            out,
                            Kind::Overlap,
                            Severity::Warning,
                            self.wires[s.wire].loc.as_ref(),
                            format!(
                                "{} and {} run on one line for {} at {}: they look like one wire",
                                self.wire_named(s.wire),
                                self.wire_named(t.wire),
                                fmt_num(round(shared)),
                                pt(at)
                            ),
                        );
                    }
                } else if gap > EPS && gap <= GRID + EPS && shared >= 2.0 * GRID - EPS && seen.insert((a, b, false)) {
                    self.hint(
                        out,
                        Kind::Close,
                        Severity::Hint,
                        self.wires[s.wire].loc.as_ref(),
                        format!(
                            "{} and {} run {} apart for {}: crowded",
                            self.wire_named(s.wire),
                            self.wire_named(t.wire),
                            fmt_num(round(gap)),
                            fmt_num(round(shared))
                        ),
                    );
                }
            }
        }
    }

    /// The crossings, one summary line, and their count.
    fn crossings(&self, out: &mut Vec<Hint>) -> usize {
        let mut found: Vec<String> = Vec::new();
        for (i, s) in self.segs.iter().enumerate() {
            for j in self.segs_near(seg_box(s)) {
                let t = &self.segs[j];
                if j <= i || s.wire == t.wire || !cross(s.a, s.b, t.a, t.b) {
                    continue;
                }
                // where: the crossing point
                let (x1, y1, x2, y2) = (s.a.0, s.a.1, s.b.0, s.b.1);
                let (x3, y3, x4, y4) = (t.a.0, t.a.1, t.b.0, t.b.1);
                let den = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
                let u = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / den;
                let at = (x1 + u * (x2 - x1), y1 + u * (y2 - y1));
                found.push(format!(
                    "{} with {} at {}",
                    nets(&self.wires[s.wire].nets),
                    nets(&self.wires[t.wire].nets),
                    pt(at)
                ));
            }
        }
        if !found.is_empty() {
            let n = found.len();
            let what = if n == 1 { "1 crossing of wires".to_string() } else { format!("{n} crossings of wires") };
            self.hint(out, Kind::Crossings, Severity::Hint, None, format!("{what}: {}", list(&found)));
        }
        found.len()
    }

    /// A wire leaving a pin against the pin's direction: its first segment does not go the way the
    /// pin points, within 60 degrees.
    fn exits(&self, out: &mut Vec<Hint>) {
        for (w, wire) in self.wires.iter().enumerate() {
            let pts = &wire.points;
            if pts.len() < 2 {
                continue;
            }
            for (e, end, next) in [(0, pts[0], pts[1]), (1, pts[pts.len() - 1], pts[pts.len() - 2])] {
                if wire.open[e] {
                    continue;
                }
                let here: Vec<&PinAt> = self
                    .pins_at(end)
                    .map(|(_, p)| p)
                    .filter(|p| p.dir.is_some() && (p.nets.is_empty() || share(&p.nets, &wire.nets)))
                    .collect();
                if here.is_empty() {
                    continue;
                }
                let go = unit((next.0 - end.0, next.1 - end.1));
                if here.iter().any(|p| {
                    let d = p.dir.unwrap();
                    go.0 * d.0 + go.1 * d.1 > 0.5
                }) {
                    continue;
                }
                let p = here[0];
                self.hint(
                    out,
                    Kind::WrongExit,
                    Severity::Hint,
                    wire.loc.as_ref().or(self.parts[p.part].loc.as_ref()),
                    format!(
                        "{} leaves {} going {}, against the pin, which points {}",
                        self.wire_named(w),
                        p.name,
                        way(go),
                        way(p.dir.unwrap())
                    ),
                );
            }
        }
    }

    /// At point `q`, net by net: the ends of wires and the pins (`ends`), and the wires passing
    /// through, by a point of theirs or along a segment (`through`).
    fn incidence(&self, q: Pt) -> BTreeMap<&str, (usize, usize)> {
        let mut by: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for (w, i) in self.vertex_at.get(&key(q)).into_iter().flatten() {
            let wire = &self.wires[*w];
            if !near(wire.points[*i], q) {
                continue;
            }
            let last = wire.points.len() - 1;
            let end = (*i == 0 && !wire.open[0]) || (*i == last && !wire.open[1]) || last == 0;
            for n in &wire.nets {
                let e = by.entry(n.as_str()).or_default();
                if end {
                    e.0 += 1;
                } else {
                    e.1 += 1;
                }
            }
        }
        for s in self.segs_at(q) {
            let g = &self.segs[s];
            if !inside_segment(q, g.a, g.b) {
                continue;
            }
            for n in &self.wires[g.wire].nets {
                by.entry(n.as_str()).or_default().1 += 1;
            }
        }
        for (_, p) in self.pins_at(q) {
            for n in &p.nets {
                if let Some(e) = by.get_mut(n.as_str()) {
                    e.0 += 1;
                }
            }
        }
        by
    }

    /// T-joins without a mark at the point, such as a junction dot.
    fn dots(&self, out: &mut Vec<Hint>) {
        let mut points: BTreeMap<(i64, i64), Pt> = BTreeMap::new();
        for w in &self.wires {
            for p in &w.points {
                points.insert(key(*p), *p);
            }
        }
        for p in &self.pins {
            points.insert(key(p.pos), p.pos);
        }
        let dots: BTreeSet<(i64, i64)> = self.marks.iter().map(|j| key(*j)).collect();
        for q in points.values().filter(|q| !dots.contains(&key(**q))) {
            for (n, (ends, through)) in self.incidence(*q) {
                // two wires that pass through each other without an end there are a crossing
                if ends + 2 * through < 3 || ends + through < 2 || ends == 0 {
                    continue;
                }
                let wires: Vec<usize> =
                    self.wires_at(*q).into_iter().filter(|&w| self.wires[w].nets.iter().any(|x| x == n)).collect();
                let mut what: Vec<String> = self
                    .pins_at(*q)
                    .filter(|(_, p)| p.nets.iter().any(|x| x == n))
                    .map(|(_, p)| p.name.clone())
                    .collect();
                what.extend(wires.iter().map(|&w| self.wire_named(w)));
                self.hint(
                    out,
                    Kind::NoDot,
                    Severity::Hint,
                    wires.first().and_then(|&w| self.wires[w].loc.as_ref()),
                    format!(
                        "&{} joins at {} without a junction dot: {} meet there; draw a junction dot at the point",
                        rnl_runtime::model::quoted(n),
                        pt(*q),
                        list(&what)
                    ),
                );
            }
        }
    }

    /// Which wires and pins the language joins: those that name a vertex in common, a point or a
    /// terminal, through one another; nodes `0..wires` are the wires, the pins after them.
    fn named_joins(&self) -> Vec<usize> {
        let nw = self.wires.len();
        let mut up: Vec<usize> = (0..nw + self.pins.len()).collect();
        let mut first: HashMap<&str, usize> = HashMap::new();
        let names = self.wires.iter().enumerate().flat_map(|(w, wire)| wire.keys.iter().map(move |k| (w, k.as_str())));
        let pins = self.pins.iter().enumerate().flat_map(|(p, pin)| pin.keys.iter().map(move |k| (nw + p, k.as_str())));
        for (node, k) in names.chain(pins) {
            match first.get(k) {
                Some(&other) => union(&mut up, node, other),
                None => {
                    first.insert(k, node);
                }
            }
        }
        up
    }

    /// A wire end on a wire or a pin of its net that the language does not join it to: neither it
    /// nor a wire it shares a named point or terminal with names that pin or a vertex of that wire.
    fn silent_opens(&self, out: &mut Vec<Hint>) {
        let nw = self.wires.len();
        let mut up = self.named_joins();
        let mut said: BTreeSet<(usize, usize, (i64, i64))> = BTreeSet::new();
        for (w, wire) in self.wires.iter().enumerate() {
            for q in [wire.points[0], wire.points[wire.points.len() - 1]] {
                let pins: Vec<(usize, &PinAt)> = self.pins_at(q).filter(|(_, p)| share(&p.nets, &wire.nets)).collect();
                if !pins.is_empty() {
                    if !pins.iter().any(|(i, _)| find(&mut up, nw + *i) == find(&mut up, w)) {
                        let p = pins[0].1;
                        self.hint(
                            out,
                            Kind::SilentOpen,
                            Severity::Hint,
                            wire.loc.as_ref(),
                            format!(
                                "{} ends on {} at {} without naming it, so they are not joined; write {} in the wire",
                                self.wire_named(w),
                                p.name,
                                pt(q),
                                p.keys.first().map_or(p.name.as_str(), |k| k.as_str())
                            ),
                        );
                    }
                    continue;
                }
                for v in self.wires_at(q) {
                    if v == w || !share(&wire.nets, &self.wires[v].nets) || find(&mut up, v) == find(&mut up, w) {
                        continue;
                    }
                    if !said.insert((w.min(v), w.max(v), key(q))) {
                        continue;
                    }
                    self.hint(
                        out,
                        Kind::SilentOpen,
                        Severity::Hint,
                        wire.loc.as_ref(),
                        format!(
                            "{} ends on {} at {} but names no point of it, so they are not joined; name one point, such as %j = {}, in both",
                            self.wire_named(w),
                            self.wire_named(v),
                            pt(q),
                            pt(q)
                        ),
                    );
                }
            }
        }
    }

    /// What lies between a point and the outside of the drawing in a way: a symbol or a wire of
    /// another net. Another port does not: ports stand side by side at an edge.
    fn blocker(&self, from: Pt, d: Pt, own: usize, net: &[String]) -> Option<String> {
        let far = (from.0 + d.0 * 1e4, from.1 + d.1 * 1e4);
        let mut best: Option<(f64, String)> = None;
        let mut take = |at: f64, what: String| {
            if best.as_ref().is_none_or(|(b, _)| at < *b) {
                best = Some((at, what));
            }
        };
        for (k, p) in self.parts.iter().enumerate() {
            if k == own || matches!(p.class, Class::Mark | Class::Port) || p.on_port {
                continue;
            }
            // a symbol all of whose pins are on the port's net stands for it
            if !p.pins.is_empty() && p.pins.iter().all(|&i| share(&self.pins[i].nets, net)) {
                continue;
            }
            let shrunk = [p.bbox[0] + 0.05, p.bbox[1] + 0.05, p.bbox[2] - 0.05, p.bbox[3] - 0.05];
            if through_box(from, far, shrunk, 0.0) {
                let c = ((p.bbox[0] + p.bbox[2]) / 2.0, (p.bbox[1] + p.bbox[3]) / 2.0);
                take(dist(from, c), format!("the symbol of {}", self.placed(k)));
            }
        }
        for s in &self.segs {
            if share(&self.wires[s.wire].nets, net) {
                continue;
            }
            if meet(from, far, s.a, s.b) {
                let c = ((s.a.0 + s.b.0) / 2.0, (s.a.1 + s.b.1) / 2.0);
                take(dist(from, c), self.wire_named(s.wire));
            }
        }
        best.map(|(_, w)| w)
    }

    /// Ports that face into the drawing, are enclosed, or are buried: open only the way they face,
    /// a symbol on each other side, with the drawing going on beyond them that way.
    fn ports(&self, out: &mut Vec<Hint>) {
        for (k, p) in self.parts.iter().enumerate() {
            if p.class != Class::Port {
                continue;
            }
            let Some(&pi) = p.pins.first() else { continue };
            let pin = &self.pins[pi];
            let c = ((p.bbox[0] + p.bbox[2]) / 2.0, (p.bbox[1] + p.bbox[3]) / 2.0);
            let ways = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
            let start = |d: Pt| -> Pt {
                let x = if d.0 > 0.0 {
                    p.bbox[2]
                } else if d.0 < 0.0 {
                    p.bbox[0]
                } else {
                    c.0
                };
                let y = if d.1 > 0.0 {
                    p.bbox[3]
                } else if d.1 < 0.0 {
                    p.bbox[1]
                } else {
                    c.1
                };
                (x, y)
            };
            let blocked: Vec<(Pt, Option<String>)> =
                ways.iter().map(|&d| (d, self.blocker(start(d), d, k, &pin.nets))).collect();
            // the way it faces: away from its pin, the side its name is on
            let out_way = pin.dir.map(|d| (-d.0, -d.1));
            let net = nets(&pin.nets);
            if blocked.iter().all(|(_, b)| b.is_some()) {
                let said: Vec<String> =
                    blocked.iter().map(|(d, b)| format!("{} {}", way(*d), b.as_deref().unwrap_or(""))).collect();
                self.hint(
                    out,
                    Kind::PortEnclosed,
                    Severity::Hint,
                    p.loc.as_ref(),
                    format!(
                        "the port {} of {net} is enclosed: something lies between it and the outside in every way ({}); place it at the edge of the drawing",
                        self.placed(k),
                        said.join("; ")
                    ),
                );
                continue;
            }
            let Some(o) = out_way else { continue };
            let free: Vec<&str> = blocked.iter().filter(|(_, b)| b.is_none()).map(|(d, _)| way(*d)).collect();
            let facing = blocked.iter().find(|(d, _)| near(*d, o)).and_then(|(_, b)| b.as_ref());
            if let Some(b) = facing {
                self.hint(
                    out,
                    Kind::PortInside,
                    Severity::Hint,
                    p.loc.as_ref(),
                    format!(
                        "the port {} of {net} faces {} into the drawing: {b} lies between it and the edge; place it at the edge, facing out (the way {} is free)",
                        self.placed(k),
                        way(o),
                        free.join(", ")
                    ),
                );
                continue;
            }
            // open only the way it faces, which is free; buried when the drawing goes on beyond it
            // that way, so that it does not stand at the drawing's edge
            let beyond = |b: &[f64; 4]| {
                if o.0 > 0.5 {
                    b[2] > p.bbox[2] + 0.5
                } else if o.0 < -0.5 {
                    b[0] < p.bbox[0] - 0.5
                } else if o.1 > 0.5 {
                    b[3] > p.bbox[3] + 0.5
                } else {
                    b[1] < p.bbox[1] - 0.5
                }
            };
            let further =
                self.parts.iter().enumerate().any(|(j, q)| j != k && q.class != Class::Mark && beyond(&q.bbox));
            if free.len() == 1 && further {
                let said: Vec<String> =
                    blocked.iter().filter_map(|(d, b)| b.as_ref().map(|b| format!("{} {b}", way(*d)))).collect();
                self.hint(
                    out,
                    Kind::PortInside,
                    Severity::Hint,
                    p.loc.as_ref(),
                    format!(
                        "the port {} of {net} is buried in the drawing: it faces {}, but parts lie on its three other sides ({}) and the drawing goes on beyond it; place it at the edge of the drawing",
                        self.placed(k),
                        way(o),
                        said.join("; ")
                    ),
                );
            }
        }
    }

    /// Nets drawn in pieces joined by their name, and pins joined by their name alone. A wire drawn
    /// in parts is one piece, closed by its flylines over the gaps, and the pins of a net a ROUTE
    /// asks to wire are left to the solver: they are not joined by name.
    fn name_joins(&self, out: &mut Vec<Hint>) {
        let nw = self.wires.len();
        let mut up: Vec<usize> = (0..nw + self.pins.len()).collect();
        for (i, w) in self.wires.iter().enumerate() {
            for q in [w.points[0], w.points[w.points.len() - 1]] {
                for j in self.wires_at(q) {
                    if j != i && share(&w.nets, &self.wires[j].nets) {
                        union(&mut up, i, j);
                    }
                }
            }
        }
        for (k, p) in self.pins.iter().enumerate() {
            for i in self.wires_at(p.pos) {
                if share(&p.nets, &self.wires[i].nets) {
                    union(&mut up, nw + k, i);
                }
            }
            for (j, q) in self.pins_at(p.pos) {
                if j != k && share(&p.nets, &q.nets) {
                    union(&mut up, nw + k, nw + j);
                }
            }
        }
        // the runs of a wire drawn in parts are joined by its flylines over the gaps (the flylines
        // between the parts of a net join nothing: they show what is not wired)
        let mut first_run: HashMap<&str, usize> = HashMap::new();
        for (i, w) in self.wires.iter().enumerate() {
            if let Some(of) = w.runs_of.as_deref() {
                let first = *first_run.entry(of).or_insert(i);
                union(&mut up, first, i);
            }
        }
        // the nets a supply or ground symbol stands for, and those of an object not placed yet,
        // whose drawing is not done
        let mut rails: BTreeSet<&String> =
            self.pins.iter().filter(|p| self.parts[p.part].class == Class::Rail).flat_map(|p| p.nets.iter()).collect();
        rails.extend(self.lv.unplaced_items.iter().flat_map(|it| it.pin_nets.iter().flatten()));
        rails.extend(self.pins.iter().flat_map(|p| p.nets.iter()).filter(|n| self.routed.contains(*n)));
        let labelled: BTreeSet<usize> = self.texts.iter().filter_map(|t| t.pin).collect();
        let mut by_net: BTreeMap<&String, Vec<usize>> = BTreeMap::new();
        for (k, p) in self.pins.iter().enumerate() {
            if !matches!(self.parts[p.part].class, Class::Device | Class::Port) {
                continue;
            }
            for n in &p.nets {
                if !rails.contains(n) {
                    by_net.entry(n).or_default().push(k);
                }
            }
        }
        // which pieces hold a wire
        let mut wired: BTreeSet<usize> = BTreeSet::new();
        for i in 0..nw {
            let r = find(&mut up, i);
            wired.insert(r);
        }
        for (n, ks) in by_net {
            if ks.len() < 2 {
                continue;
            }
            let mut pieces: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
            for &k in &ks {
                let r = find(&mut up, nw + k);
                pieces.entry(r).or_default().push(k);
            }
            let any_wired = pieces.keys().any(|r| wired.contains(r));
            if pieces.len() >= 2 && any_wired {
                let said: Vec<String> = pieces
                    .iter()
                    .map(|(r, ks)| {
                        let names: Vec<String> = ks.iter().map(|&k| self.pins[k].name.clone()).collect();
                        let how = if wired.contains(r) {
                            "wired"
                        } else if ks.iter().any(|k| labelled.contains(k)) {
                            "a label"
                        } else if ks.iter().any(|&k| self.parts[self.pins[k].part].class == Class::Port) {
                            "a port"
                        } else {
                            "by name"
                        };
                        format!("{{{}}} {how}", list(&names))
                    })
                    .collect();
                let loc = ks.iter().find_map(|&k| self.parts[self.pins[k].part].loc.as_ref());
                self.hint(
                    out,
                    Kind::PartlyNamed,
                    Severity::Hint,
                    loc,
                    format!(
                        "&{} is drawn in {} pieces joined only by its name: {}; wire them, or draw the net by name alone",
                        rnl_runtime::model::quoted(n),
                        pieces.len(),
                        said.join(", ")
                    ),
                );
            }
            // a device's pin alone in its piece, with no label at it
            for (r, ks) in &pieces {
                if ks.len() != 1 || wired.contains(r) {
                    continue;
                }
                let k = ks[0];
                let p = &self.pins[k];
                if self.parts[p.part].class != Class::Device || labelled.contains(&k) {
                    continue;
                }
                self.hint(
                    out,
                    Kind::NameOnly,
                    Severity::Hint,
                    self.parts[p.part].loc.as_ref(),
                    format!(
                        "{} is joined to &{} by its name alone: no wire, port or label at it, so it looks unconnected; wire it, or give it a label at {}",
                        p.name,
                        rnl_runtime::model::quoted(n),
                        p.keys.first().map_or(p.name.as_str(), |k| k.as_str())
                    ),
                );
            }
        }
    }

    /// Wires of no length, segments of none, and the loops of a net.
    fn zero_and_loops(&self, out: &mut Vec<Hint>) {
        for (w, wire) in self.wires.iter().enumerate() {
            if wire.points.len() < 2 {
                self.hint(
                    out,
                    Kind::ZeroLength,
                    Severity::Hint,
                    wire.loc.as_ref(),
                    format!("{} has no length", self.wire_named(w)),
                );
            } else if let Some(at) = wire.repeated {
                self.hint(
                    out,
                    Kind::ZeroLength,
                    Severity::Hint,
                    wire.loc.as_ref(),
                    format!("{} has a segment of no length at {}", self.wire_named(w), pt(at)),
                );
            }
        }
        let mut by_net: BTreeMap<&String, Vec<usize>> = BTreeMap::new();
        for (w, wire) in self.wires.iter().enumerate() {
            for n in &wire.nets {
                by_net.entry(n).or_default().push(w);
            }
        }
        let mut pins_of: BTreeMap<&String, Vec<Pt>> = BTreeMap::new();
        for p in &self.pins {
            for n in &p.nets {
                pins_of.entry(n).or_default().push(p.pos);
            }
        }
        for (n, ws) in by_net {
            // the special points: the ends of the segments and the pins of the net, by their place
            let mut special: HashMap<(i64, i64), Vec<Pt>> = HashMap::new();
            for &w in &ws {
                for p in &self.wires[w].points {
                    special.entry(cell(*p)).or_default().push(*p);
                }
            }
            for p in pins_of.get(n).into_iter().flatten() {
                special.entry(cell(*p)).or_default().push(*p);
            }
            let mut nodes: HashMap<(i64, i64), usize> = HashMap::new();
            let mut up: Vec<usize> = Vec::new();
            let mut node = |p: Pt, up: &mut Vec<usize>| -> usize {
                *nodes.entry(key(p)).or_insert_with(|| {
                    up.push(up.len());
                    up.len() - 1
                })
            };
            let mut looped: Option<(usize, Pt)> = None;
            'net: for &w in &ws {
                for s in self.wires[w].points.windows(2) {
                    let b = [s[0].0.min(s[1].0), s[0].1.min(s[1].1), s[0].0.max(s[1].0), s[0].1.max(s[1].1)];
                    let mut cut: Vec<Pt> = cells(b)
                        .flat_map(|c| special.get(&c).into_iter().flatten().copied())
                        .filter(|q| inside_segment(*q, s[0], s[1]))
                        .collect();
                    cut.sort_by(|p, q| dist(s[0], *p).total_cmp(&dist(s[0], *q)));
                    cut.dedup_by(|p, q| near(*p, *q));
                    let mut prev = s[0];
                    for q in cut.into_iter().chain([s[1]]) {
                        let (a, b) = (node(prev, &mut up), node(q, &mut up));
                        if find(&mut up, a) == find(&mut up, b) {
                            looped = Some((w, prev));
                            break 'net;
                        }
                        union(&mut up, a, b);
                        prev = q;
                    }
                }
            }
            if let Some((w, at)) = looped {
                self.hint(
                    out,
                    Kind::Loop,
                    Severity::Hint,
                    self.wires[w].loc.as_ref(),
                    format!(
                        "the wires of &{} close a loop, or draw one stretch twice, near {}: {}",
                        rnl_runtime::model::quoted(n),
                        pt(at),
                        self.wire_named(w)
                    ),
                );
            }
        }
    }

    /// Wires over texts and texts over texts, by the boxes of their glyphs.
    fn texts(&self, out: &mut Vec<Hint>) {
        for t in &self.texts {
            let mut seen = BTreeSet::new();
            for i in self.segs_near(t.bbox) {
                let s = &self.segs[i];
                if seen.contains(&s.wire) || !through_box(s.a, s.b, t.bbox, 0.02) {
                    continue;
                }
                seen.insert(s.wire);
                self.hint(
                    out,
                    Kind::WireOverText,
                    Severity::Hint,
                    self.wires[s.wire].loc.as_ref().or(t.loc.as_ref()),
                    format!("{} runs over {}", self.wire_named(s.wire), t.name),
                );
            }
        }
        for (i, a) in self.texts.iter().enumerate() {
            let near_by: BTreeSet<usize> =
                cells(a.bbox).flat_map(|c| self.text_cells.get(&c).into_iter().flatten().copied()).collect();
            for j in near_by {
                let b = &self.texts[j];
                if j > i && boxes_overlap(a.bbox, b.bbox, 0.05) {
                    self.hint(
                        out,
                        Kind::TextOverText,
                        Severity::Hint,
                        a.loc.as_ref().or(b.loc.as_ref()),
                        format!("{} lies over {}", a.name, b.name),
                    );
                }
            }
        }
    }
}

fn find(up: &mut [usize], i: usize) -> usize {
    let mut r = i;
    while up[r] != r {
        r = up[r];
    }
    let mut j = i;
    while up[j] != r {
        let n = up[j];
        up[j] = r;
        j = n;
    }
    r
}

fn union(up: &mut [usize], a: usize, b: usize) {
    let (x, y) = (find(up, a), find(up, b));
    if x != y {
        up[x] = y;
    }
}

/// The height of a capital, as a share of the em of the shipped fonts.
const CAP: f64 = 0.72;

/// The box of a text's glyphs, from the box the renderer lays its lines out in, which runs from
/// the ascender of its first line to the descender of its last: from the baseline of the last line
/// to the height of a capital on the first, the way the text reads (9.2.5), so that a stack of
/// labels, a line apart, does not touch.
fn glyph_box(t: &rnl_runtime::model::TextView, b: [f64; 4]) -> [f64; 4] {
    let size = t.style.get("size").and_then(|v| v.num()).filter(|s| *s > 0.0).unwrap_or(1.0);
    let bold = t.style.get("bold").and_then(|v| v.boolean()).unwrap_or(false);
    let face = rnl_render::Face::of(bold);
    let (asc, desc) = (face.ascender() * size, face.descender().abs() * size);
    let top = (asc - CAP * size).max(0.0);
    // the way the text reads, turned with its owner: vertical when it reads up
    let r = t.rotate.to_radians();
    let u = t.orient.apply((libm_round(r.cos()), libm_round(r.sin())));
    let vertical = u.0.abs() < 0.5;
    let out = if vertical {
        // a vertical text's glyphs have their tops to the left
        [b[0] + top, b[1], b[2] - desc, b[3]]
    } else {
        [b[0], b[1] + desc, b[2], b[3] - top]
    };
    if out[2] - out[0] < EPS || out[3] - out[1] < EPS {
        b
    } else {
        out
    }
}

fn libm_round(v: f64) -> f64 {
    v.round()
}

// ---------------------------------------------------------------- names

/// How a hint names the objects of a level.
struct Names<'a> {
    by_id: &'a HashMap<&'a str, &'a ObjectInfo>,
    /// The terminal names of the host instance each item draws, by the item's id.
    terminals: HashMap<&'a str, &'a [Option<String>]>,
}

impl<'a> Names<'a> {
    fn new(lv: &'a LevelView, by_id: &'a HashMap<&'a str, &'a ObjectInfo>) -> Names<'a> {
        let terminals = lv
            .items
            .iter()
            .filter_map(|it| Some((it.object_id.as_str(), lv.terminal_names.get(*it.host.first()?)?.as_slice())))
            .collect();
        Names { by_id, terminals }
    }

    /// An object by its id: as it is, `#M1` or `%a_port`, when it has a name; a usage without one
    /// by its definition and what it is attached to, `the $IO_PORT on &A`; a part made in the body
    /// of a usage by that usage, `the ground stub of #M3:s`.
    fn object(&self, id: &str) -> String {
        if !id.starts_with('@') {
            return id.to_string();
        }
        let Some(o) = self.by_id.get(id) else { return id.to_string() };
        let Some(def) = &o.definition else { return id.to_string() };
        if let Some(parent) = o.parent.as_deref().and_then(|p| self.by_id.get(p)) {
            if parent.id.starts_with('@') {
                if let Some(pdef) = &parent.definition {
                    return format!("the {} of {}", words(pdef), self.attached(parent));
                }
            } else {
                return format!("the {} of {}", words(def), parent.id);
            }
        }
        format!("the {def} on {}", self.attached(o))
    }

    /// What a usage's roles are attached to.
    fn attached(&self, o: &ObjectInfo) -> String {
        let all: Vec<String> = o.attach.iter().map(|(_, a)| self.attach(a)).collect();
        all.join(" ")
    }

    fn attach(&self, a: &Attach) -> String {
        match a {
            Attach::Object(id) => self.object_or_terminal(id),
            Attach::At(p) => pt(*p),
            Attach::Set(items) => format!("{{{}}}", items.iter().map(|a| self.attach(a)).collect::<Vec<_>>().join(" ")),
        }
    }

    fn object_or_terminal(&self, id: &str) -> String {
        if id.starts_with('#') && id.contains(':') {
            self.terminal(id)
        } else {
            self.object(id)
        }
    }

    /// A terminal of an instance by its name, `#M3:s`, for `#M3:3`.
    fn terminal(&self, t: &str) -> String {
        let Some((base, index)) = t.rsplit_once(':') else { return t.to_string() };
        let Ok(index) = index.parse::<usize>() else { return t.to_string() };
        let named = self.terminals.get(base).and_then(|names| names.get(index.wrapping_sub(1)).cloned().flatten());
        match named {
            Some(n) => format!("{base}:{n}"),
            None => t.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_meet_cross_and_pass_through_boxes() {
        assert!(cross((0.0, 0.0), (2.0, 0.0), (1.0, -1.0), (1.0, 1.0)));
        // a touch is no crossing
        assert!(!cross((0.0, 0.0), (2.0, 0.0), (1.0, 0.0), (1.0, 1.0)));
        assert!(meet((0.0, 0.0), (2.0, 0.0), (1.0, 0.0), (1.0, 1.0)));
        assert!(through_box((-1.0, 0.0), (1.0, 0.0), [-0.5, -0.5, 0.5, 0.5], 0.0));
        // along the border is not inside
        assert!(!through_box((-1.0, 0.5), (1.0, 0.5), [-0.5, -0.5, 0.5, 0.5], 0.0));
        assert!(on_segment((1.0, 0.0), (0.0, 0.0), (2.0, 0.0)));
        assert!(!inside_segment((0.0, 0.0), (0.0, 0.0), (2.0, 0.0)));
    }

    #[test]
    fn a_place_is_read_from_an_id() {
        assert_eq!(site("@a.sp:12:3"), Some(Loc::new("a.sp", 12, 3)));
        assert_eq!(site("@D:/w/a.sp:12:3#2"), Some(Loc::new("D:/w/a.sp", 12, 3)));
        assert_eq!(site("#M1"), None);
        assert_eq!(words("$GROUND_STUB"), "ground stub");
    }
}
