//! Evaluating members, segments and objects (spec-draft chapters 5 and 6).
//!
//! Names are resolved lazily: a reference evaluates the member that binds the name, once per
//! drawing level. A failed statement yields a failed object (7.2); what depends on it fails too.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::diag::{DiagId, Extra, Severity};
use crate::model::{FailedSpan, HostRef, RefTarget};
use crate::syntax::{Coord, Member, ObjExpr, Params, PathItem, Point, Ref, Segment, Sep, SetExpr, Value};
use crate::words::{words, ParamSpec, ParamType, Takes};

use super::ctx::{Ctx, Defining, FrameKind, Nested};
use super::obj::*;
use super::unit::{closest, Binding, UnitKind};
use super::{Interp, Memo};

/// Why evaluation stopped.
#[derive(Clone, Debug)]
pub enum Stop {
    /// The statement fails (7.2), with its diagnostic.
    Fail(Option<DiagId>),
    /// A rule written at an upper level reached an object of that level (10.5).
    LocalRef,
    /// A statement that needs a circuit, read where there is none (a package, an included file level).
    NoCircuit,
}

pub type R<T> = Result<T, Stop>;

/// An evaluated object in a statement: the object, its role label, whether it came through a
/// selection, where it is written, and whether it is a written `{…}`.
#[derive(Clone, Debug)]
pub struct ArgV {
    pub obj: ObjId,
    pub role: Option<String>,
    pub role_quoted: bool,
    pub sel: bool,
    pub off: usize,
    pub explicit: bool,
}

/// A name lookup result (6.2, 6.8, 6.9).
#[derive(Clone, Debug)]
pub enum Found {
    Binding(Binding),
    Local(ObjId),
    LocalMember(String),
    Nested(Nested),
    /// A body names the name it defines, and there is no outer definition of it (6.9).
    NoOuter,
}

pub type Bound = Vec<((char, String), ObjId)>;

/// Alias -> original.
pub type Aliases = Vec<(String, String)>;

/// Reads a written parameter name: (name, quoted, offset) -> the name it stands for.
pub type NameReader<'a, 'p> = &'a dyn Fn(&mut Interp<'p>, &str, bool, usize) -> R<String>;

pub fn ptr<T>(r: &T) -> usize {
    r as *const T as usize
}

impl<'p> Interp<'p> {
    // ------------------------------------------------------------------ objects

    pub(crate) fn new_obj(&mut self, data: ObjData) -> ObjId {
        self.objs.push(Obj { data, name: String::new() });
        ObjId(self.objs.len() as u32 - 1)
    }

    pub(crate) fn o(&self, id: ObjId) -> &ObjData {
        &self.objs[id.0 as usize].data
    }

    pub(crate) fn om(&mut self, id: ObjId) -> &mut ObjData {
        &mut self.objs[id.0 as usize].data
    }

    pub(crate) fn def(&self, id: ObjId) -> &DefData {
        match self.o(id) {
            ObjData::Definition(d) => d,
            _ => panic!("not a definition"),
        }
    }

    pub(crate) fn usage(&self, id: ObjId) -> &UsageData {
        match self.o(id) {
            ObjData::Usage(u) => u,
            _ => panic!("not a usage"),
        }
    }

    pub(crate) fn usage_mut(&mut self, id: ObjId) -> &mut UsageData {
        match self.om(id) {
            ObjData::Usage(u) => u,
            _ => panic!("not a usage"),
        }
    }

    pub(crate) fn set(&self, id: ObjId) -> &SetData {
        match self.o(id) {
            ObjData::Set(s) => s,
            _ => panic!("not a set"),
        }
    }

    pub(crate) fn set_mut(&mut self, id: ObjId) -> &mut SetData {
        match self.om(id) {
            ObjData::Set(s) => s,
            _ => panic!("not a set"),
        }
    }

    pub(crate) fn is_set(&self, id: ObjId) -> bool {
        matches!(self.o(id), ObjData::Set(_))
    }

    pub(crate) fn is_failed(&self, id: ObjId) -> bool {
        matches!(self.o(id), ObjData::Failed(_))
    }

    pub(crate) fn kind_of(&self, id: ObjId) -> &'static str {
        if id == LEVEL {
            return "level";
        }
        if id == OWNER {
            return "owner";
        }
        if id == NOTHING {
            return "nothing";
        }
        match self.o(id) {
            ObjData::Failed(_) => "failed",
            ObjData::Marker => "marker",
            ObjData::LevelObj { .. } => "level",
            ObjData::Inst { .. } => "instance",
            ObjData::Net { .. } => "net",
            ObjData::Term { .. } => "terminal",
            ObjData::Point { .. } => "point",
            ObjData::Pos { .. } => "position",
            ObjData::Wire { .. } => "wire",
            ObjData::Route { .. } => "route",
            ObjData::Prim { .. } => "annotation",
            ObjData::Text { .. } | ObjData::KeyText { .. } | ObjData::SymLabel { .. } => "text",
            ObjData::Usage(_) => "usage",
            ObjData::Bundle { .. } => "bundle",
            ObjData::Constraint(_) => "constraint",
            ObjData::Set(_) => "set",
            ObjData::Symbol(_) => "symbol",
            ObjData::Style(_) => "style",
            ObjData::Definition(_) => "definition",
        }
    }

    pub(crate) fn inst_name(&self, id: ObjId) -> String {
        match self.o(id) {
            ObjData::Inst { scene, idx, .. } => {
                let lv = self.scenes[*scene].level;
                self.doc.levels[lv].instances[*idx].name.clone()
            }
            _ => String::new(),
        }
    }

    /// How an object is shown in diagnostics.
    pub(crate) fn label(&self, id: ObjId) -> String {
        if id == LEVEL {
            return "#".into();
        }
        if id == OWNER {
            return "?".into();
        }
        if id == NOTHING {
            return "nothing".into();
        }
        let o = &self.objs[id.0 as usize];
        let named = |anon: &str| if o.name.is_empty() { anon.to_string() } else { format!("%{}", o.name) };
        match &o.data {
            ObjData::Failed(_) => "a failed object".into(),
            ObjData::Marker => "?".into(),
            ObjData::LevelObj { .. } => "#".into(),
            ObjData::Inst { .. } => format!("#{}", self.inst_name(id)),
            ObjData::Net { display, .. } => format!("&{display}"),
            ObjData::Term { owner, index, name } => {
                let base = if matches!(self.o(*owner), ObjData::LevelObj { .. }) {
                    "#".to_string()
                } else {
                    self.label(*owner)
                };
                match name {
                    Some(n) => format!("{base}:{n}"),
                    None => format!("{base}:{index}"),
                }
            }
            ObjData::Point { x, y, .. } => {
                if !o.name.is_empty() {
                    format!("%{}", o.name)
                } else {
                    let f = |v: &Option<f64>| v.map(crate::geom::fmt_num).unwrap_or_else(|| "_".into());
                    format!("({}, {})", f(x), f(y))
                }
            }
            ObjData::Pos { parent } => format!("{}::position", self.label(*parent)),
            ObjData::Wire { .. } => named("a wire"),
            ObjData::Route { net, .. } => {
                if o.name.is_empty() {
                    format!("the ROUTE of {}", self.label(*net))
                } else {
                    format!("%{}", o.name)
                }
            }
            ObjData::Prim { shape, .. } => shape.as_str().to_string(),
            ObjData::Text { attach, .. } => {
                let _ = attach;
                "a text".into()
            }
            ObjData::KeyText { attach, key, .. } => format!("{}::{key}", self.label(*attach)),
            ObjData::SymLabel { .. } => "a text".into(),
            ObjData::Usage(u) => {
                if o.name.is_empty() {
                    format!("a usage of {}", self.label(u.definition))
                } else {
                    format!("%{}", o.name)
                }
            }
            ObjData::Bundle { .. } => named("a bundle"),
            ObjData::Constraint(c) => {
                if o.name.is_empty() {
                    c.kind.as_str().to_string()
                } else {
                    format!("%{}", o.name)
                }
            }
            ObjData::Set(s) => {
                if o.name.is_empty() {
                    match &s.kind {
                        SetKind::Select { keyword, .. } => keyword.clone(),
                        SetKind::Gen { .. } => "a generated set".into(),
                        _ => "a set".into(),
                    }
                } else if o.name.starts_with('$') || matches!(s.kind, SetKind::Select { .. } | SetKind::Gen { .. }) {
                    o.name.clone()
                } else {
                    format!("%{}", o.name)
                }
            }
            ObjData::Symbol(_) => {
                if o.name.is_empty() {
                    "a symbol".into()
                } else {
                    o.name.clone()
                }
            }
            ObjData::Style(_) => {
                if o.name.is_empty() {
                    "a style".into()
                } else {
                    o.name.clone()
                }
            }
            ObjData::Definition(_) => {
                if o.name.is_empty() {
                    "a definition".into()
                } else {
                    o.name.clone()
                }
            }
        }
    }

    /// A selection or generated set, or a set that holds one: evaluated where and when used (10.3).
    pub(crate) fn is_lazy(&self, id: ObjId) -> bool {
        if id == LEVEL {
            return true;
        }
        match self.o(id) {
            ObjData::Set(s) => match &s.kind {
                SetKind::Select { .. } | SetKind::Gen { .. } => true,
                SetKind::Union { source } => self.is_lazy(*source),
                SetKind::Plain => {
                    if !s.mods.is_empty() && s.mods.iter().any(|m| self.is_lazy(m.source)) {
                        return false;
                    }
                    s.items.iter().any(|(o, _)| self.is_lazy(*o))
                }
            },
            _ => false,
        }
    }

    /// Is a value tied to one drawing level?
    pub(crate) fn scene_bound(&self, id: ObjId) -> bool {
        if id == NOTHING || id == LEVEL || id == OWNER {
            return false;
        }
        match self.o(id) {
            ObjData::Symbol(_) | ObjData::Style(_) | ObjData::Definition(_) | ObjData::Failed(_) => false,
            _ if self.is_lazy(id) => false,
            ObjData::Set(s) => {
                // edits from ADD-TO / REMOVE-FROM apply per scene; only a level's own set is tied to it by them
                let own = s.writer.is_some_and(|w| self.units[w].kind == UnitKind::Level);
                s.items.iter().any(|(o, _)| self.scene_bound(*o)) || (!s.mods.is_empty() && own)
            }
            _ => true,
        }
    }

    // ------------------------------------------------------------------ diagnostics in a context

    pub(crate) fn ctx_pkg(&self, ctx: &Ctx) -> String {
        self.units[ctx.unit].pkg().to_string()
    }

    pub(crate) fn err(&mut self, ctx: &Ctx, msg: impl Into<String>, off: usize) -> DiagId {
        self.err_cause(ctx, msg, off, None)
    }

    pub(crate) fn err_cause(&mut self, ctx: &Ctx, msg: impl Into<String>, off: usize, cause: Option<DiagId>) -> DiagId {
        let loc = self.rec_loc(ctx.rec, off);
        let pkg = self.ctx_pkg(ctx);
        self.sink.add(Severity::Error, msg, loc, Extra { cause, origin: ctx.origin.to_string(), in_package: pkg })
    }

    pub(crate) fn fail<T>(&mut self, ctx: &Ctx, msg: impl Into<String>, off: usize) -> R<T> {
        Err(Stop::Fail(Some(self.err(ctx, msg, off))))
    }

    pub(crate) fn warn(&mut self, ctx: &Ctx, msg: impl Into<String>, off: usize) {
        let loc = self.rec_loc(ctx.rec, off);
        let pkg = self.ctx_pkg(ctx);
        self.sink.add(
            Severity::Warning,
            msg,
            loc,
            Extra { cause: None, origin: ctx.origin.to_string(), in_package: pkg },
        );
    }

    pub(crate) fn hint_at(&mut self, ctx: &Ctx, msg: impl Into<String>, off: usize) {
        let loc = self.rec_loc(ctx.rec, off);
        let pkg = self.ctx_pkg(ctx);
        self.sink.add(Severity::Hint, msg, loc, Extra { cause: None, origin: ctx.origin.to_string(), in_package: pkg });
    }

    // ------------------------------------------------------------------ names (6.2–6.9)

    pub(crate) fn lookup(&mut self, prefix: char, name: &str, ctx: &Ctx) -> Option<Found> {
        let key = (prefix, name.to_string());
        let defining = ctx.defining.as_ref().and_then(|d| d.get(&key).cloned());
        if let Some(nested) = &ctx.nested {
            let hit = nested.borrow().get(&key).cloned();
            if let Some(hit) = hit {
                let is_defining = match (&defining, &hit) {
                    (Some((dm, _)), Nested::Member(m, _)) => m.same(dm),
                    _ => false,
                };
                if !is_defining {
                    return Some(Found::Nested(hit));
                }
            }
        }
        if let (Some(fr), '%') = (&ctx.frame, prefix) {
            let fr = fr.borrow();
            if let Some(v) = fr.local(name) {
                return Some(Found::Local(v));
            }
            if let Some((_, m, _)) = fr.local_members.iter().find(|(n, _, _)| n == name) {
                let is_defining = defining.as_ref().is_some_and(|(dm, _)| m.same(dm));
                if !is_defining {
                    return Some(Found::LocalMember(name.to_string()));
                }
            }
        }
        if let Some((member, unit)) = defining {
            return Some(self.outer_lookup(prefix, name, &member, unit));
        }
        let u = self.name_unit(ctx);
        self.lookup_unit(prefix, name, u, false).map(Found::Binding)
    }

    /// The unit whose names a reference is resolved with: a body's names are those of where the
    /// definition is written (6.9).
    pub(crate) fn name_unit(&self, ctx: &Ctx) -> UnitId {
        if ctx.frame.is_some() {
            if let Some(u) = ctx.def_unit {
                return u;
            }
        }
        ctx.unit
    }

    /// The outer definition of a name whose own binding is `member`: the next one in the lookup
    /// order of 6.8 and 16.2, or NoOuter (6.9).
    pub(crate) fn outer_lookup(&mut self, prefix: char, name: &str, member: &MemberRef, unit: UnitId) -> Found {
        if let Some(b) = self.units[unit].names.get(&(prefix, name.to_string())) {
            if !Rc::ptr_eq(&b.member, &member.m) {
                return Found::Binding(b.clone());
            }
        }
        match self.lookup_unit(prefix, name, unit, true) {
            Some(b) => Found::Binding(b),
            None => Found::NoOuter,
        }
    }

    /// skip_own: what a level's own binding hides, from the levels above and the packages (6.8,
    /// 16.2); in a package, the standard package in effect (16.2).
    pub(crate) fn lookup_unit(&mut self, prefix: char, name: &str, u: UnitId, skip_own: bool) -> Option<Binding> {
        let key = (prefix, name.to_string());
        if self.units[u].kind == UnitKind::Package {
            if !skip_own {
                if let Some(b) = self.units[u].names.get(&key) {
                    return Some(b.clone());
                }
            }
            if prefix == '$' {
                let std = match self.units[u].std {
                    Some(s) => Some(s),
                    None => self.std_package(),
                };
                if let Some(s) = std {
                    if s != u {
                        if let Some(b) = self.units[s].names.get(&key) {
                            return Some(b.clone());
                        }
                    }
                }
            }
            return None;
        }
        if !skip_own {
            if let Some(b) = self.units[u].names.get(&key) {
                return Some(b.clone());
            }
        }
        if prefix == '%' {
            return None;
        }
        let lv = self.units[u].level?;
        let pk = self.effective_packages(lv);
        // this level's USE-PACKAGE packages, later first
        let mut mine: Vec<(&(usize, usize), UnitId)> = pk
            .iter()
            .filter_map(|e| match &e.place {
                super::unit::Place::Level(l, p) if *l == lv => Some((p, e.unit)),
                _ => None,
            })
            .collect();
        mine.sort_by(|a, b| b.0.cmp(a.0));
        for (_, pu) in &mine {
            if let Some(b) = self.units[*pu].names.get(&key) {
                if b.prefix != '%' {
                    return Some(b.clone());
                }
            }
        }
        for anc in self.doc.ancestors(lv) {
            let au = self.level_unit[anc];
            if let Some(b) = self.units[au].names.get(&key) {
                return Some(b.clone());
            }
            let mut theirs: Vec<(&(usize, usize), UnitId)> = pk
                .iter()
                .filter_map(|e| match &e.place {
                    super::unit::Place::Level(l, p) if *l == anc => Some((p, e.unit)),
                    _ => None,
                })
                .collect();
            theirs.sort_by(|a, b| b.0.cmp(a.0));
            for (_, pu) in &theirs {
                if let Some(b) = self.units[*pu].names.get(&key) {
                    return Some(b.clone());
                }
            }
        }
        let mut roots: Vec<(usize, UnitId)> = pk
            .iter()
            .filter_map(|e| match &e.place {
                super::unit::Place::Root(k) => Some((*k, e.unit)),
                _ => None,
            })
            .collect();
        roots.sort_by_key(|r| std::cmp::Reverse(r.0));
        for (_, pu) in roots {
            if let Some(b) = self.units[pu].names.get(&key) {
                return Some(b.clone());
            }
        }
        None
    }

    /// The scene a host level's unit is drawn in.
    pub(crate) fn home_scene(&self, u: UnitId) -> Option<SceneId> {
        let unit = &self.units[u];
        if unit.kind == UnitKind::Level {
            if let Some(lv) = unit.level {
                if self.doc.levels[lv].circuit {
                    return self.level_scene[lv];
                }
            }
        }
        None
    }

    pub(crate) fn binding_value(&mut self, found: &Found, ctx: &Ctx) -> R<ObjId> {
        let b = match found {
            Found::Local(v) => return Ok(*v),
            Found::LocalMember(name) => return self.local_member(name, ctx),
            Found::Binding(b) => b.clone(),
            _ => return Ok(NOTHING),
        };
        let home = self.home_scene(b.owner);
        let scene = if home.is_some() && ctx.scene == home { ctx.scene } else { home };
        let mkey = b.key();
        let val =
            self.eval_member_at(b.owner, b.rec, b.index, b.embedded, &b.member, scene, b.header.clone(), None, "")?;
        let val = self.bound_value(&mkey, scene, b.prefix, &b.name, val);
        if ctx.scene.is_some() && scene != ctx.scene && self.scene_bound(val) {
            return Err(Stop::LocalRef);
        }
        Ok(val)
    }

    pub(crate) fn bound_value(
        &self,
        mkey: &MemberKey,
        scene: Option<SceneId>,
        prefix: char,
        name: &str,
        default: ObjId,
    ) -> ObjId {
        if let Some(bound) = self.bound_memo.get(&(mkey.clone(), scene)) {
            if let Some((_, v)) = bound.iter().find(|(k, _)| k.0 == prefix && k.1 == name) {
                return *v;
            }
        }
        default
    }

    // ------------------------------------------------------------------ evaluation entry

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn eval_member_at(
        &mut self,
        u: UnitId,
        rec: Option<RecId>,
        k: usize,
        embedded: bool,
        m: &Rc<Member>,
        scene: Option<SceneId>,
        header: Rc<super::unit::Header>,
        pos: Option<Pos>,
        origin: &str,
    ) -> R<ObjId> {
        let mkey = MemberKey::M { unit: u, rec: rec.unwrap_or(usize::MAX), k, embedded };
        let skey = (mkey.clone(), scene);
        if let Some(v) = self.memo.get(&skey) {
            return match v {
                Memo::Val(v) => Ok(*v),
                Memo::LocalRef => Err(Stop::LocalRef),
            };
        }
        if self.in_progress.contains(&skey) {
            let loc = self.rec_loc(rec, m.text_start());
            let d = self.sink.error("the name refers to itself in a cycle (6.6)", loc);
            return Err(Stop::Fail(Some(d)));
        }
        self.in_progress.insert(skey.clone());
        let pos = match pos {
            Some(p) => p,
            None => match scene {
                Some(s) => self.position_of(s, u, rec, if embedded { None } else { Some(k) }),
                None => pos_of_ints(&[9, 0, rec.map(|r| r as i64).unwrap_or(0), k as i64]),
            },
        };
        let ctx = Ctx {
            scene,
            unit: u,
            rec,
            header,
            pos,
            frame: None,
            origin: Rc::from(origin),
            nested: None,
            member_key: mkey,
            selected: false,
            defining: None,
            def_unit: None,
            scope_in: false,
        };
        let (r, bound) = self.eval_member(m, &ctx);
        self.in_progress.remove(&skey);
        let val = match r {
            Ok(v) => v,
            Err(Stop::LocalRef) => {
                self.memo.insert(skey, Memo::LocalRef);
                return Err(Stop::LocalRef);
            }
            Err(Stop::NoCircuit) => return Err(Stop::NoCircuit),
            Err(Stop::Fail(d)) => self.new_obj(ObjData::Failed(d)),
        };
        self.bound_memo.insert(skey.clone(), bound);
        self.memo.insert(skey, Memo::Val(val));
        Ok(val)
    }

    /// Segments in order; each `=> name` binds the handle its segment produced (5.5, PR8).
    pub(crate) fn eval_member(&mut self, m: &Rc<Member>, ctx: &Ctx) -> (R<ObjId>, Bound) {
        let mut val = NOTHING;
        let mut sel = false;
        let mut bound: Bound = Vec::new();
        for (k, seg) in m.segments.iter().enumerate() {
            let piped = if k > 0 { Some((val, sel)) } else { None };
            match self.eval_segment(seg, ctx, piped, m) {
                Ok((v, s)) => {
                    val = v;
                    sel = s;
                }
                Err(Stop::Fail(d)) => {
                    self.note_failed(ctx.rec, seg.start, seg.end, false);
                    let out = seg
                        .call
                        .as_ref()
                        .and_then(|c| words().keyword(&self.kw(&c.kw, ctx.unit)))
                        .map(|s| s.output.clone())
                        .unwrap_or_default();
                    let piped_ok = piped.is_some_and(|(p, _)| !self.is_failed(p));
                    if k > 0 && out == "pass" && piped_ok {
                        // a segment that only adds facts keeps the handle (5.5)
                    } else {
                        val = self.new_obj(ObjData::Failed(d));
                        sel = false;
                    }
                }
                Err(other) => return (Err(other), bound),
            }
            for b in m.binds.iter().filter(|b| b.after == k) {
                bound.push(((b.prefix, b.name.clone()), val));
                if val != NOTHING && val != LEVEL && val != OWNER {
                    let nameable = matches!(
                        self.o(val),
                        ObjData::Usage(_)
                            | ObjData::Bundle { .. }
                            | ObjData::Wire { .. }
                            | ObjData::Constraint(_)
                            | ObjData::Set(_)
                            | ObjData::Text { .. }
                            | ObjData::Prim { .. }
                            | ObjData::Point { .. }
                            | ObjData::Symbol(_)
                            | ObjData::Style(_)
                            | ObjData::Definition(_)
                    );
                    if nameable && self.objs[val.0 as usize].name.is_empty() {
                        self.objs[val.0 as usize].name =
                            if b.prefix == '%' { b.name.clone() } else { format!("${}", b.name) };
                    }
                }
            }
        }
        if let ObjData::Failed(d) = self.o(val) {
            return (Err(Stop::Fail(*d)), bound);
        }
        (Ok(val), bound)
    }

    pub(crate) fn eval_segment(
        &mut self,
        seg: &Segment,
        ctx: &Ctx,
        piped: Option<(ObjId, bool)>,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        let mut args: Vec<ArgV> = Vec::new();
        if let Some((p, s)) = piped {
            args.push(ArgV { obj: p, role: None, role_quoted: false, sel: s, off: seg.start, explicit: false });
        }
        let kw0 = seg.call.as_ref().map(|c| self.kw(&c.kw, ctx.unit));
        for a in &seg.args {
            if let Some(k) = &kw0 {
                if matches!(k.as_str(), "DEFINE-SYMBOL" | "DEFINE-STYLE" | "DEFINE" | "USE-PACKAGE") {
                    // a definition body is encapsulated: it is not evaluated as statements here (5.4)
                    args.push(ArgV {
                        obj: NOTHING,
                        role: a.role.clone(),
                        role_quoted: a.role_quoted,
                        sel: false,
                        off: a.start,
                        explicit: true,
                    });
                    continue;
                }
            }
            let (obj, sel) = self.eval_obj(&a.obj, ctx)?;
            args.push(ArgV {
                obj,
                role: a.role.clone(),
                role_quoted: a.role_quoted,
                sel,
                off: a.start,
                explicit: matches!(a.obj, ObjExpr::Set(_)),
            });
        }
        let Some(call) = &seg.call else {
            if args.len() == 1 {
                return Ok((args[0].obj, args[0].sel));
            }
            return Ok((NOTHING, false));
        };
        let kw = self.kw(&call.kw, ctx.unit);
        if !words().is_keyword(&kw) {
            let written = call.kw.clone();
            let fu = self.file_unit(ctx.unit);
            let mut known: Vec<String> = words().keywords.iter().map(|k| k.name.clone()).collect();
            if let Some(f) = fu {
                known.extend(self.units[f].aliases.keys().filter_map(|a| a.strip_prefix('@').map(String::from)));
            }
            if let Some(f) = fu {
                if let Some(src) = self.units[f].alias_rejected.get(&format!("@{written}")).cloned() {
                    return self.fail(
                        ctx,
                        format!("unknown keyword @{written}: the original of this alias in alias package {src} is not a standard keyword, so it is not used (16.5)"),
                        call.start,
                    );
                }
            }
            let pre = known
                .iter()
                .filter(|k| written.starts_with(k.as_str()) && written.len() > k.len())
                .max_by_key(|k| k.len())
                .cloned();
            if let Some(pre) = pre {
                return self.fail(
                    ctx,
                    format!("unknown keyword @{written}: is a space missing after `@{pre}`? (16.5)"),
                    call.start,
                );
            }
            let close = closest(&kw, words().keywords.iter().map(|k| k.name.as_str()));
            let mut msg = format!("unknown keyword @{written}");
            if let Some(c) = close {
                msg.push_str(&format!(", did you mean @{c}?"));
            }
            if !written.is_ascii() {
                msg.push_str(" Is an alias package not loaded?");
            }
            msg.push_str(" (16.4)");
            return self.fail(ctx, msg, call.start);
        }
        if matches!(kw.as_str(), "VERSION" | "PACKAGE" | "ALIAS" | "DEVICE" | "USE-ALIASES" | "USE-PACKAGE") {
            if kw == "USE-PACKAGE" {
                return Ok((NOTHING, false)); // handled when reading (16.2)
            }
            return self.fail(ctx, format!("@{kw} is written only in the header record (2.8)"), call.start);
        }
        self.keyword(&kw, args, seg, ctx, member)
    }

    // ------------------------------------------------------------------ objects (5.1, 6.x)

    pub(crate) fn eval_obj(&mut self, e: &ObjExpr, ctx: &Ctx) -> R<(ObjId, bool)> {
        match e {
            ObjExpr::Point(p) => Ok((self.make_point(p, ctx)?, false)),
            ObjExpr::Set(s) => self.make_set(s, ctx),
            ObjExpr::Ref(r) => self.resolve_ref(r, ctx),
        }
    }

    fn make_point(&mut self, e: &Point, ctx: &Ctx) -> R<ObjId> {
        let x = self.coord(&e.x, ctx, e.start)?;
        let y = self.coord(&e.y, ctx, e.start)?;
        let local = ctx.in_symbol();
        if local && (x.is_none() || y.is_none()) {
            return self.fail(ctx, "a point in a symbol body has numbers on both axes (9.2.1)", e.start);
        }
        let p = self.new_obj(ObjData::Point { x, y, local });
        if let (Some(s), false) = (ctx.scene, local) {
            self.scene_add_point(s, p, ctx.pos.clone());
        }
        Ok(p)
    }

    fn coord(&mut self, v: &Coord, ctx: &Ctx, off: usize) -> R<Option<f64>> {
        match v {
            Coord::Free => Ok(None),
            Coord::Num(x) => Ok(Some(*x)),
            Coord::Hole(name) => match self.hole_value(name, ctx, off)? {
                None => Ok(None),
                Some(Lit::Num(x)) => Ok(Some(x)),
                Some(_) => self.fail(ctx, format!("?{name} is not a number and cannot be a coordinate"), off),
            },
        }
    }

    fn hole_value(&mut self, name: &str, ctx: &Ctx, off: usize) -> R<Option<Lit>> {
        let fr = match &ctx.frame {
            Some(f) if f.borrow().kind == FrameKind::Definition => f.clone(),
            _ => {
                return self.fail(
                    ctx,
                    format!("?{name} is written only in the body of an ordinary definition (6.10)"),
                    off,
                )
            }
        };
        let v = fr.borrow().param(name).cloned();
        match v {
            Some(v) => Ok(v),
            None => self.fail(ctx, format!("?{name} is not a parameter (6.10)"), off),
        }
    }

    pub(crate) fn make_set(&mut self, e: &Rc<SetExpr>, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mut items: Vec<(ObjId, bool)> = Vec::new();
        let nested: super::ctx::NestedRef =
            Rc::new(RefCell::new(ctx.nested.as_ref().map(|n| n.borrow().clone()).unwrap_or_default()));
        let mut lazy = false;
        // names bound inside a nested set are local to it (D4)
        for (k, m) in e.members.iter().enumerate() {
            for b in &m.binds {
                nested
                    .borrow_mut()
                    .insert((b.prefix, b.name.clone()), Nested::Member(MemberRef { rec: ctx.rec, m: m.clone() }, k));
            }
        }
        for (k, m) in e.members.iter().enumerate() {
            if let Some(err) = &m.error {
                let msg = err.clone();
                self.err(ctx, msg, m.error_pos);
                self.note_failed(ctx.rec, m.start, m.end, false);
                let f = self.new_obj(ObjData::Failed(None));
                items.push((f, false));
                continue;
            }
            let mut sub = ctx.clone();
            sub.nested = Some(nested.clone());
            sub.pos = {
                let mut p = ctx.pos.clone();
                p.push(PosElem::Tag("s"));
                p.push(PosElem::Int(e.start as i64));
                p.push(PosElem::Int(k as i64));
                p
            };
            if m.segments.len() == 1 && m.segments[0].call.is_none() && m.bind().is_none() {
                for a in &m.segments[0].args {
                    let (obj, _sel) = match self.eval_obj(&a.obj, &sub) {
                        Ok(v) => v,
                        Err(Stop::Fail(d)) => (self.new_obj(ObjData::Failed(d)), false),
                        Err(o) => return Err(o),
                    };
                    lazy = lazy || self.is_lazy(obj);
                    items.push((obj, !matches!(a.obj, ObjExpr::Set(_))));
                }
                continue;
            }
            let obj = match self.nested_member(m, &sub, &nested) {
                Ok(v) => v,
                Err(Stop::Fail(d)) => self.new_obj(ObjData::Failed(d)),
                Err(o) => return Err(o),
            };
            lazy = lazy || self.is_lazy(obj);
            items.push((obj, false));
        }
        let mut sd = SetData::plain(items, true);
        if ctx.scene.is_some() && ctx.frame.is_none() {
            sd.writer = Some(ctx.unit);
        }
        let s = self.new_obj(ObjData::Set(Box::new(sd)));
        Ok((s, lazy))
    }

    pub(crate) fn nested_member(&mut self, m: &Rc<Member>, ctx: &Ctx, nested: &super::ctx::NestedRef) -> R<ObjId> {
        let fkey = ctx.frame.as_ref().map(|f| Rc::as_ptr(f) as usize).unwrap_or(0);
        let key = (ptr(&**m), fkey, ctx.scene);
        if let Some(v) = self.nested_memo.get(&key) {
            return v.clone();
        }
        let (r, bound) = self.eval_member(m, ctx);
        for ((p, n), v) in bound {
            nested.borrow_mut().insert((p, n), Nested::Val(v));
        }
        let r = match r {
            Err(Stop::LocalRef) | Err(Stop::NoCircuit) => return r,
            other => other,
        };
        self.nested_memo.insert(key, r.clone());
        r
    }

    pub(crate) fn resolve_ref(&mut self, e: &Ref, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mut sel = false;
        let mut obj;
        if e.prefix == '?' {
            obj = self.resolve_hole(e, ctx)?;
            sel = ctx.frame.as_ref().is_some_and(|f| f.borrow().selected);
        } else if e.prefix == '#' || e.prefix == '&' {
            obj = self.resolve_host(e, ctx)?;
        } else {
            let name = e.name.clone().unwrap_or_default();
            let found = self.lookup(e.prefix, &name, ctx);
            match found {
                Some(Found::NoOuter) => {
                    // in an ordinary definition's body this was reported once at the definition; each
                    // expansion just leaves the statement out (6.9, M5)
                    let d = self.body_errors.get(&(ptr(e))).copied();
                    let d = match d {
                        Some(d) => d,
                        None => {
                            let msg = no_outer_message(e);
                            self.err(ctx, msg, e.start)
                        }
                    };
                    return Err(Stop::Fail(Some(d)));
                }
                None => {
                    let close = {
                        let names: Vec<String> = self.units[ctx.unit]
                            .names
                            .keys()
                            .filter(|(p, _)| *p == e.prefix)
                            .map(|(_, n)| n.clone())
                            .collect();
                        closest(&name, names.iter().map(String::as_str))
                    };
                    let hint = close.map(|c| format!(", did you mean {}{c}?", e.prefix)).unwrap_or_default();
                    return self.fail(ctx, format!("{}{name} cannot be found{hint} (6.6)", e.prefix), e.start);
                }
                Some(Found::Nested(Nested::Member(m, k))) => {
                    let nested = ctx.nested.clone().unwrap();
                    let last = self.nested_member(&m.m, ctx, &nested)?;
                    let _ = k;
                    let hit = nested.borrow().get(&(e.prefix, name.clone())).cloned();
                    obj = match hit {
                        Some(Nested::Val(v)) => v,
                        _ => last,
                    };
                }
                Some(Found::Nested(Nested::Val(v))) => obj = v,
                Some(f) => obj = self.binding_value(&f, ctx)?,
            }
            if obj == NOTHING {
                return self.fail(ctx, format!("{}{name} produces no object", e.prefix), e.start);
            }
            if let ObjData::Failed(d) = self.o(obj) {
                let d = *d;
                let msg = format!("{}{name} failed", e.prefix);
                let id = self.err_cause(ctx, msg, e.start, d);
                return Err(Stop::Fail(Some(id)));
            }
            sel = self.is_lazy(obj);
        }
        for p in &e.path {
            obj = self.subobject(obj, p, ctx, e)?;
        }
        if e.prefix == '#' && e.path.first().is_some_and(|p| p.sep == Sep::Term) {
            self.note_terminal_ref(ctx, e, obj);
        }
        Ok((obj, sel))
    }

    fn resolve_hole(&mut self, e: &Ref, ctx: &Ctx) -> R<ObjId> {
        let fk = ctx.frame_kind();
        let Some(name) = &e.name else {
            if fk != Some(FrameKind::Symbol) {
                return self.fail(
                    ctx,
                    "a bare `?` is written only in a symbol body, meaning the object the symbol draws (13.3)",
                    e.start,
                );
            }
            return Ok(OWNER);
        };
        if fk != Some(FrameKind::Definition) {
            return self.fail(
                ctx,
                format!("?{name} is written only in the body of an ordinary definition (6.10)"),
                e.start,
            );
        }
        let fr = ctx.frame.clone().unwrap();
        let (role, is_param) = {
            let f = fr.borrow();
            (f.role(name), f.param(name).is_some())
        };
        if let Some(r) = role {
            return Ok(r);
        }
        if is_param {
            return self.fail(
                ctx,
                format!("?{name} is a parameter and is written only where a literal goes (6.10)"),
                e.start,
            );
        }
        self.fail(ctx, format!("?{name} is not in the role list or the parameter list (6.10)"), e.start)
    }

    fn resolve_host(&mut self, e: &Ref, ctx: &Ctx) -> R<ObjId> {
        if matches!(ctx.frame_kind(), Some(FrameKind::Definition) | Some(FrameKind::Symbol)) {
            return self.fail(
                ctx,
                "a definition body cannot reference host objects; they come in through roles (6.9)",
                e.start,
            );
        }
        if e.is_bare_level() {
            return Ok(LEVEL);
        }
        let Some(scene) = ctx.scene else {
            if self.units[ctx.unit].kind == UnitKind::Package {
                return self.fail(
                    ctx,
                    "a package has no circuit of its own: statements that name host objects fail (16.2)",
                    e.start,
                );
            }
            return self.fail(
                ctx,
                "the file level of an included file has no circuit of its own: host references fail (6.1)",
                e.start,
            );
        };
        if self.units[ctx.unit].level != Some(self.scenes[scene].level) && !ctx.scope_in {
            return Err(Stop::LocalRef);
        }
        let mut names: Vec<Option<String>> = vec![e.name.clone()];
        let mut group = false;
        if let (Some(sub), Some(n)) = (&e.sub, &e.name) {
            match sub.single() {
                None => {
                    names = sub.expand().into_iter().map(|k| Some(format!("{n}{}", sub.text(k)))).collect();
                    group = true;
                }
                Some(k) => names = vec![Some(format!("{n}{}", sub.text(k)))],
            }
        }
        let mut objs = Vec::new();
        for nm in names {
            let o = if e.prefix == '#' {
                match &nm {
                    None => self.scenes[scene].levelobj,
                    Some(n) => match self.scene_inst(scene, n) {
                        Some(o) => o,
                        None => return self.fail(ctx, format!("this level has no instance {n} (6.3)"), e.start),
                    },
                }
            } else {
                let n = nm.unwrap_or_default();
                match self.scene_net(scene, &n) {
                    Some(o) => o,
                    None => return self.fail(ctx, format!("this level has no net {n} (6.3)"), e.start),
                }
            };
            objs.push(o);
        }
        self.note_ref(ctx, e, scene, &objs);
        if group {
            let sd = SetData::plain(objs.into_iter().map(|o| (o, true)).collect(), false);
            return Ok(self.new_obj(ObjData::Set(Box::new(sd))));
        }
        Ok(objs[0])
    }

    /// Note a resolved host reference for tools (`HostRef`): where its name is written and the
    /// instances or nets it reaches.
    fn note_ref(&mut self, ctx: &Ctx, e: &Ref, scene: SceneId, objs: &[ObjId]) {
        let Some(rec) = ctx.rec else { return };
        let Some(loc) = self.rec_loc(Some(rec), e.start) else { return };
        // the prefix and the name, up to the `:` of a path outside quotes
        let mut len = 0u32;
        let mut quoted = false;
        for c in self.recs[rec].text.chars().skip(e.start).take(e.end.saturating_sub(e.start)) {
            if c == '"' {
                quoted = !quoted;
            }
            if c == ':' && !quoted && len > 0 {
                break;
            }
            len += 1;
        }
        let level = self.scenes[scene].level;
        for &o in objs {
            let target = match self.o(o) {
                ObjData::Inst { idx, .. } => RefTarget::Instance(*idx),
                ObjData::Net { key, .. } => RefTarget::Net(key.clone()),
                _ => continue,
            };
            let r = HostRef { loc: loc.clone(), len, level, target };
            if self.refs_seen.insert(r.clone()) {
                self.refs.push(r);
            }
        }
    }

    /// Note that the text `start..end` of a record does not take effect (`FailedSpan`): it failed,
    /// or with `conflict` it lost a conflict (8.5). Surrounding blanks are left out.
    pub(crate) fn note_failed(&mut self, rec: Option<RecId>, start: usize, end: usize, conflict: bool) {
        let Some(rec) = rec else { return };
        let text: Vec<char> = self.recs[rec].text.chars().collect();
        let (mut a, mut b) = (start.min(text.len()), end.min(text.len()));
        while a < b && text[a].is_whitespace() {
            a += 1;
        }
        while b > a && text[b - 1].is_whitespace() {
            b -= 1;
        }
        if a == b {
            return;
        }
        let f = FailedSpan { start: self.recs[rec].loc(a), end: self.recs[rec].loc(b - 1), conflict };
        if self.failed_seen.insert(f.clone()) {
            self.failed_spans.push(f);
        }
    }

    /// Note that the statement written at `off` does not take effect: the innermost segment of the
    /// record around it, or the statement when the place is outside its segments.
    pub(crate) fn note_failed_at(&mut self, rec: Option<RecId>, off: usize, conflict: bool) {
        let Some(r) = rec else { return };
        let ast = self.parse(r);
        let found = ast.body.as_ref().and_then(|b| innermost(b, off));
        if let Some((a, b)) = found {
            self.note_failed(rec, a, b, conflict);
        }
    }

    /// Note the nets a terminal reference such as `#M1:d` reaches, for tools (`HostRef`). The place
    /// runs from the `#` to the end of the terminal, so it covers the instance's name as well.
    fn note_terminal_ref(&mut self, ctx: &Ctx, e: &Ref, obj: ObjId) {
        let (Some(rec), Some(scene)) = (ctx.rec, ctx.scene) else { return };
        let Some(loc) = self.rec_loc(Some(rec), e.start) else { return };
        // up to the `::` of a further path, outside quotes
        let mut len = 0u32;
        let mut quoted = false;
        let mut colons = 0;
        for c in self.recs[rec].text.chars().skip(e.start).take(e.end.saturating_sub(e.start)) {
            if c == '"' {
                quoted = !quoted;
            }
            if c == ':' && !quoted {
                colons += 1;
                if colons == 2 {
                    break;
                }
            }
            len += 1;
        }
        let reached: Vec<ObjId> = match self.o(obj) {
            ObjData::Set(s) => s.items.iter().map(|(o, _)| *o).collect(),
            _ => vec![obj],
        };
        let mut nets = Vec::new();
        for o in reached {
            match self.o(o) {
                ObjData::Term { .. } => nets.extend(self.term_nets(scene, o)),
                ObjData::Net { .. } => nets.push(o),
                _ => {}
            }
        }
        let level = self.scenes[scene].level;
        for n in nets {
            let ObjData::Net { key, .. } = self.o(n) else { continue };
            let r = HostRef { loc: loc.clone(), len, level, target: RefTarget::Net(key.clone()) };
            if self.refs_seen.insert(r.clone()) {
                self.refs.push(r);
            }
        }
    }

    fn subobject(&mut self, obj: ObjId, p: &PathItem, ctx: &Ctx, e: &Ref) -> R<ObjId> {
        if let ObjData::Set(s) = self.o(obj) {
            if !self.is_lazy(obj) && !s.explicit {
                // an expanded group: take the sub-object of each member (6.11)
                let members: Vec<ObjId> = s.items.iter().map(|(o, _)| *o).collect();
                let mut items = Vec::new();
                for o in members {
                    items.push((self.subobject(o, p, ctx, e)?, true));
                }
                return Ok(self.new_obj(ObjData::Set(Box::new(SetData::plain(items, false)))));
            }
        }
        if p.sep == Sep::Term {
            return self.terminal(obj, p, ctx, e);
        }
        let name = self.subobject_name(obj, &p.name, ctx);
        if name == "position" {
            return Ok(match ctx.scene {
                Some(s) => self.scene_pos_of(s, obj),
                None => self.new_obj(ObjData::Pos { parent: obj }),
            });
        }
        if name == "net" {
            if matches!(self.o(obj), ObjData::Term { .. }) {
                let Some(s) = ctx.scene else { return self.fail(ctx, "there is no circuit here", e.start) };
                return self.term_net(s, obj, ctx, e.start);
            }
            let written = if p.name == "net" { "net".to_string() } else { format!("{} (net)", p.name) };
            let l = self.label(obj);
            return self.fail(ctx, format!("{l} has no sub-object {written} (6.5)"), e.start);
        }
        if let ObjData::Usage(u) = self.o(obj) {
            if let Some(r) = u.role(&name) {
                return Ok(r);
            }
            if let Some(l) = u.local(&name) {
                return Ok(l);
            }
        }
        let keyable = matches!(
            self.o(obj),
            ObjData::Inst { .. }
                | ObjData::Usage(_)
                | ObjData::Bundle { .. }
                | ObjData::Term { .. }
                | ObjData::Wire { .. }
                | ObjData::Prim { .. }
                | ObjData::Net { .. }
        );
        if keyable {
            let Some(s) = ctx.scene else { return self.fail(ctx, "there is no circuit here", e.start) };
            return Ok(self.scene_keytext(s, obj, &name));
        }
        if obj == OWNER {
            return self.fail(ctx, "the `?` of a symbol is only an object of TEXT (13.3)", e.start);
        }
        let written = if p.name == name { name.clone() } else { format!("{} ({name})", p.name) };
        let l = self.label(obj);
        self.fail(ctx, format!("{l} has no sub-object {written} (6.5)"), e.start)
    }

    /// The name after `::`, read through the aliases of the file that writes it (6.5, 16.5): the
    /// standard names as written; on a usage its own role and local names, then the aliases scoped
    /// to its definition; then an alias of a standard sub-object name; otherwise the name itself.
    pub(crate) fn subobject_name(&mut self, obj: ObjId, name: &str, ctx: &Ctx) -> String {
        if words().is_subobject(name) {
            return name.to_string();
        }
        if matches!(self.o(obj), ObjData::Usage(_)) {
            let (own, scoped) = self.usage_subnames(obj, ctx.unit);
            if own.iter().any(|(n, _)| n == name) {
                return name.to_string();
            }
            if let Some((_, o)) = scoped.iter().find(|(a, _)| a == name) {
                return o.clone();
            }
        }
        let std = self.word(name, ctx.unit);
        if words().is_subobject(&std) {
            std
        } else {
            name.to_string()
        }
    }

    /// What `::` reaches on a usage before its text keys (6.5, 16.5): its role and local names
    /// ("role name" or "local name"), and the aliases scoped to its definition of those names.
    pub(crate) fn usage_subnames(&mut self, obj: ObjId, u: UnitId) -> (Vec<(String, &'static str)>, Aliases) {
        let ud = self.usage(obj).clone();
        let dfn = ud.definition;
        let d = self.def(dfn).clone();
        let mut own: Vec<(String, &'static str)> = Vec::new();
        for n in ud.locals.iter().map(|(n, _)| n.clone()).chain(d.local_names()) {
            if !own.iter().any(|(x, _)| *x == n) {
                own.push((n, "local name"));
            }
        }
        for n in ud.roles.iter().map(|(n, _)| n.clone()).chain(d.roles.iter().map(|r| r.name.clone())) {
            if let Some(e) = own.iter_mut().find(|(x, _)| *x == n) {
                e.1 = "role name";
            } else {
                own.push((n, "role name"));
            }
        }
        let scoped: Vec<(String, String)> =
            self.scoped_for(u, dfn).into_iter().filter(|(_, o)| own.iter().any(|(n, _)| n == o)).collect();
        (own, scoped)
    }

    /// Why a text key cannot be used, or None (6.5, 13.3): it may not be spelled like a name that
    /// `::` finds before the text keys.
    pub(crate) fn key_clash(&mut self, key: &str, unit: UnitId, owner: Option<ObjId>) -> Option<String> {
        if words().is_subobject(key) {
            return Some(format!("a text key cannot be the standard sub-object name {key} (13.3)"));
        }
        if let Some(o) = owner {
            if matches!(self.o(o), ObjData::Usage(_)) {
                let (own, scoped) = self.usage_subnames(o, unit);
                let l = self.label(o);
                if let Some((_, what)) = own.iter().find(|(n, _)| n == key) {
                    return Some(format!("the text key {key} is spelled like a {what} of the usage {l}: `::` takes the {what} first and cannot reach this text (6.5, 13.3)"));
                }
                if let Some((_, orig)) = scoped.iter().find(|(a, _)| a == key) {
                    let what = own.iter().find(|(n, _)| n == orig).map(|(_, w)| *w).unwrap_or("name");
                    return Some(format!("the text key {key} is an alias of the {what} {orig} of the usage {l}: `::` takes the alias first and cannot reach this text (13.3, 16.5)"));
                }
            }
        }
        let std = self.word(key, unit);
        if std != key && words().is_subobject(&std) {
            return Some(format!("the text key {key} is an alias of the standard sub-object name {std}: `::` takes the alias first and cannot reach this text (13.3, 16.5)"));
        }
        None
    }

    fn terminal(&mut self, obj: ObjId, p: &PathItem, ctx: &Ctx, e: &Ref) -> R<ObjId> {
        let mut obj = obj;
        if obj == LEVEL {
            match ctx.scene {
                Some(s) => obj = self.scenes[s].levelobj,
                None => return self.fail(ctx, "there is no circuit here", e.start),
            }
        }
        let names: Vec<String> = match &p.sub {
            Some(sub) if sub.single().is_none() => {
                sub.expand().into_iter().map(|k| format!("{}{}", p.name, sub.text(k))).collect()
            }
            Some(sub) => vec![format!("{}{}", p.name, sub.text(sub.single().unwrap()))],
            None => vec![p.name.clone()],
        };
        let ok_owner = matches!(
            self.o(obj),
            ObjData::Usage(_) | ObjData::Inst { .. } | ObjData::Bundle { .. } | ObjData::LevelObj { .. }
        );
        if !ok_owner {
            let l = self.label(obj);
            return self.fail(ctx, format!("{l} has no terminals (6.4)"), e.start);
        }
        let mut out = Vec::new();
        for nm in &names {
            let t = match ctx.scene {
                Some(s) => self.scene_term(s, obj, Some(nm), p.number),
                None => None,
            };
            match t {
                Some(t) => out.push(t),
                None => {
                    let l = self.label(obj);
                    return self.fail(ctx, format!("{l} has no terminal {nm} (6.4, 6.12)"), e.start);
                }
            }
        }
        if out.len() == 1 && p.sub.as_ref().is_none_or(|s| s.single().is_some()) {
            return Ok(out[0]);
        }
        let sd = SetData::plain(out.into_iter().map(|t| (t, true)).collect(), false);
        Ok(self.new_obj(ObjData::Set(Box::new(sd))))
    }

    fn local_member(&mut self, name: &str, ctx: &Ctx) -> R<ObjId> {
        let fr = ctx.frame.clone().unwrap();
        let (m, k) = {
            let f = fr.borrow();
            let (_, m, k) = f.local_members.iter().find(|(n, _, _)| n == name).unwrap();
            (m.clone(), *k)
        };
        let val = self.definition_member(&m.m, k, ctx)?;
        let local = fr.borrow().local(name);
        Ok(local.unwrap_or(val))
    }

    // ------------------------------------------------------------------ objects to positions (5.2.1)

    /// Map written objects to signature positions: position name -> argument.
    pub(crate) fn map_args(
        &mut self,
        kw: &str,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        positions: &[crate::words::PositionSpec],
    ) -> R<HashMap<String, ArgV>> {
        let mut defs: Vec<&crate::words::PositionSpec> =
            positions.iter().filter(|p| matches!(p.takes, Takes::Def(_))).collect();
        let mut objs: Vec<ArgV> = args.to_vec();
        let mut mapped: HashMap<String, ArgV> = HashMap::new();
        if let Some(d0) = defs.first() {
            if d0.optional {
                let last_is_def = objs.last().is_some_and(|a| {
                    a.role.is_none() && {
                        let x = self.anon_def(a.obj);
                        matches!(self.o(x), ObjData::Definition(_) | ObjData::Symbol(_) | ObjData::Style(_))
                    }
                });
                if !last_is_def {
                    defs.clear(); // SELECT-USAGE may leave out its definition (10.1, TR3)
                }
            }
        }
        if let Some(d0) = defs.first() {
            if objs.is_empty() {
                return self.fail(ctx, format!("the definition of @{kw} is written last (5.6)"), seg.start);
            }
            if seg.piped && objs.len() == 1 {
                return self.fail(
                    ctx,
                    format!("a definition cannot come down a pipe: the piped object fills the first position, and the definition of @{kw} is written after it (5.6)"),
                    seg.start,
                );
            }
            let last = objs.pop().unwrap();
            if last.role.is_some() {
                return self.fail(ctx, "a definition is written last and carries no role (4.2 rule 7)", last.off);
            }
            mapped.insert(d0.name.clone(), last);
        }
        let rest: Vec<&crate::words::PositionSpec> =
            positions.iter().filter(|p| !matches!(p.takes, Takes::Def(_))).collect();
        let labeled = objs.iter().filter(|a| a.role.is_some()).count();
        if labeled > 0 && labeled != objs.len() {
            return self.fail(ctx, "label the roles of all objects or of none (4.2 rule 6)", seg.start);
        }
        if labeled > 0 {
            for a in &objs {
                let r = a.role.clone().unwrap();
                // a quoted role is the name itself; an unquoted one may be an alias of the slot name (16.5)
                let role = if a.role_quoted { r.clone() } else { self.word(&r, ctx.unit) };
                if !rest.iter().any(|p| p.name == role) {
                    let more = if !a.role_quoted && !r.is_ascii() { "; is an alias package not loaded?" } else { "" };
                    return self.fail(ctx, format!("@{kw} has no position {r}{more} (5.3)"), a.off);
                }
                if mapped.contains_key(&role) {
                    return self.fail(ctx, "a role is labelled only once (4.2 rule 6)", a.off);
                }
                mapped.insert(role, a.clone());
            }
            for p in &rest {
                if !mapped.contains_key(&p.name) && !p.optional {
                    return self.fail(ctx, format!("the {} of @{kw} is not filled (5.3)", p.name), seg.start);
                }
            }
            return Ok(mapped);
        }
        // 5.2.1 rule 2: a single set object is unpacked one level
        let mut unpacked_from: Option<ArgV> = None;
        let total = objs.len() + usize::from(!defs.is_empty());
        if total == 1 && objs.len() == 1 && self.is_set(objs[0].obj) && !self.is_lazy(objs[0].obj) {
            let a = objs[0].clone();
            let items = self.set(a.obj).items.clone();
            objs = items
                .into_iter()
                .map(|(o, spread)| ArgV {
                    obj: o,
                    role: None,
                    role_quoted: false,
                    sel: a.sel,
                    off: a.off,
                    explicit: !spread,
                })
                .collect();
            unpacked_from = Some(a);
        }
        if rest.len() == 1 {
            let p = rest[0];
            if objs.is_empty() {
                if let Some(a) = unpacked_from {
                    mapped.insert(p.name.clone(), a);
                    return Ok(mapped);
                }
                if p.optional {
                    return Ok(mapped);
                }
                return self.fail(ctx, format!("@{kw} has no object: the signature does not match (5.2.1)"), seg.start);
            }
            if objs.len() == 1 && unpacked_from.is_none() {
                mapped.insert(p.name.clone(), objs[0].clone());
            } else if let Some(a) = unpacked_from {
                mapped.insert(p.name.clone(), a);
            } else {
                let items: Vec<(ObjId, bool)> = objs.iter().map(|a| (a.obj, !a.explicit)).collect();
                let sel = objs.iter().any(|a| a.sel);
                let s = self.new_obj(ObjData::Set(Box::new(SetData::plain(items, false))));
                mapped.insert(
                    p.name.clone(),
                    ArgV { obj: s, role: None, role_quoted: false, sel, off: objs[0].off, explicit: false },
                );
            }
            return Ok(mapped);
        }
        if objs.len() != rest.len() {
            return self.fail(
                ctx,
                format!("@{kw} takes {} objects, {} are written (5.2.1)", rest.len(), objs.len()),
                seg.start,
            );
        }
        for (p, a) in rest.iter().zip(objs) {
            mapped.insert(p.name.clone(), a);
        }
        Ok(mapped)
    }

    /// A nested anonymous definition `{ @DEFINE-SYMBOL {…}}` in the definition position is that
    /// definition (5.6).
    pub(crate) fn anon_def(&self, obj: ObjId) -> ObjId {
        if obj == NOTHING || obj == LEVEL || obj == OWNER {
            return obj;
        }
        if let ObjData::Set(s) = self.o(obj) {
            if matches!(s.kind, SetKind::Plain) && s.items.len() == 1 {
                let inner = s.items[0].0;
                if matches!(self.o(inner), ObjData::Definition(_) | ObjData::Symbol(_) | ObjData::Style(_)) {
                    return inner;
                }
            }
        }
        obj
    }

    // ------------------------------------------------------------------ parameters (5.3)

    /// Evaluate and check parameters against the signature (5.3, 4.2 rule 9). `resolve` reads a
    /// written name; by default an unquoted name may be an alias of a standard word (16.5).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn params(
        &mut self,
        kw: &str,
        seg: &Segment,
        ctx: &Ctx,
        spec: &[ParamSpec],
        positional: Option<&[String]>,
        open_style: bool,
        resolve: Option<NameReader<'_, 'p>>,
    ) -> R<Vec<(String, Lit)>> {
        let call = seg.call.as_ref().unwrap();
        let mut out: Vec<(String, Option<Lit>)> = Vec::new();
        let Some(p) = &call.params else {
            self.require(kw, spec, &[], seg, ctx)?;
            return Ok(Vec::new());
        };
        let default_order = words().keyword(kw).map(|s| s.positional.clone()).unwrap_or_default();
        let order: Vec<String> = positional.map(|p| p.to_vec()).unwrap_or(default_order);
        if !p.positional.is_empty() {
            if order.is_empty() {
                let ex = if kw == "DEFINE" {
                    ", such as @DEFINE(roles=[\"a\", \"b\"], parameters=[[\"gap\", 2]])"
                } else {
                    ", as name=value"
                };
                return self.fail(
                    ctx,
                    format!(
                        "the parameters of @{kw} cannot be written by position; write each by name{ex} (4.2 rule 9)"
                    ),
                    p.start,
                );
            }
            if p.positional.len() > order.len() {
                return self.fail(
                    ctx,
                    format!(
                        "@{kw} takes at most {} parameters by position ({}), {} are written (4.2 rule 9)",
                        order.len(),
                        order.join(", "),
                        p.positional.len()
                    ),
                    p.start,
                );
            }
            for (name, v) in order.iter().zip(&p.positional) {
                let val = self.value(v, ctx, p.start)?;
                if val.is_none() {
                    // a named parameter left without a value counts as not written; a positional one has no
                    // definite meaning (9.4)
                    let hole = if let Value::Hole(h) = v { h.clone() } else { String::new() };
                    return self.fail(
                        ctx,
                        format!("?{hole} has no value passed and no default: in a positional parameter it has no definite meaning, so the statement fails (9.4)"),
                        p.start,
                    );
                }
                out.push((name.clone(), val));
            }
        }
        for n in &p.named {
            let std = match resolve {
                Some(f) => f(self, &n.name, n.quoted, n.off)?,
                None => {
                    if n.quoted {
                        n.name.clone()
                    } else {
                        self.word(&n.name, ctx.unit)
                    }
                }
            };
            let known = spec.iter().any(|s| s.name == std) || (open_style && words().is_style_prop(&std));
            if !known {
                let mut opts: Vec<&str> = spec.iter().map(|s| s.name.as_str()).collect();
                if open_style {
                    opts.extend(words().style_props.iter().map(|s| s.name.as_str()));
                }
                let close = closest(&std, opts);
                let mut msg = format!("@{kw} has no parameter {}", n.name);
                if let Some(c) = close {
                    msg.push_str(&format!(", did you mean {c}?"));
                }
                if !n.quoted && !n.name.is_ascii() {
                    msg.push_str(" Is an alias package not loaded?");
                }
                msg.push_str(" (16.4)");
                return self.fail(ctx, msg, n.off);
            }
            if out.iter().any(|(k, _)| *k == std) {
                // by position and by name, or under two names: the same parameter twice (4.2 rule 9)
                return self.fail(ctx, format!("parameter {std} of @{kw} is written twice (4.2 rule 9)"), n.off);
            }
            let v = self.value(&n.value, ctx, n.off)?;
            out.push((std, v));
        }
        let mut done = Vec::new();
        for (name, val) in out {
            let Some(val) = val else { continue };
            let sp = spec.iter().find(|s| s.name == name).cloned().or_else(|| {
                words().style_prop(&name).map(|s| ParamSpec { name: s.name.clone(), ty: s.ty.clone(), required: false })
            });
            let v = self.check_value(kw, &name, val, sp.as_ref(), ctx, p.start)?;
            done.push((name, v));
        }
        let names: Vec<String> = done.iter().map(|(n, _)| n.clone()).collect();
        self.require(kw, spec, &names, seg, ctx)?;
        Ok(done)
    }

    /// The parameters marked required in the word list must be there (5.3).
    fn require(&mut self, kw: &str, spec: &[ParamSpec], have: &[String], seg: &Segment, ctx: &Ctx) -> R<()> {
        for sp in spec {
            if sp.required && !have.contains(&sp.name) {
                let at = seg.call.as_ref().map(|c| c.start).unwrap_or(seg.start);
                return self.fail(ctx, format!("@{kw} lacks the parameter {} (5.3)", sp.name), at);
            }
        }
        Ok(())
    }

    fn check_value(&mut self, kw: &str, name: &str, val: Lit, sp: Option<&ParamSpec>, ctx: &Ctx, off: usize) -> R<Lit> {
        let Some(sp) = sp else { return Ok(val) };
        if let ParamType::Enum(vals) = &sp.ty {
            let v = match &val {
                // a value passed into a definition is read with the aliases of the file that wrote it (16.5)
                Lit::Str(s) => Lit::Str(WStr::new(&self.word(s.as_str(), s.unit.unwrap_or(ctx.unit)), s.unit)),
                other => other.clone(),
            };
            let ok = vals.iter().any(|x| match (x, &v) {
                (crate::words::Literal::Str(a), Lit::Str(b)) => a == b.as_str(),
                (crate::words::Literal::Num(a), Lit::Num(b)) => a == b,
                _ => false,
            });
            if !ok {
                let shown: Vec<String> = vals
                    .iter()
                    .map(|x| match x {
                        crate::words::Literal::Str(s) => format!("{s:?}"),
                        crate::words::Literal::Num(n) => crate::geom::fmt_num(*n),
                    })
                    .collect();
                return self.fail(
                    ctx,
                    format!("the {name} of @{kw} cannot be {}; one of {} (16.4)", val.show(), shown.join(", ")),
                    off,
                );
            }
            return Ok(v);
        }
        let ok = match sp.ty {
            ParamType::Number => matches!(val, Lit::Num(_)),
            ParamType::String => matches!(val, Lit::Str(_)),
            ParamType::Color => {
                if let Lit::Str(s) = &val {
                    if !is_color(s.as_str()) {
                        return self.fail(
                            ctx,
                            format!("a colour is written like \"#RRGGBB\" (9.3), not {:?}", s.as_str()),
                            off,
                        );
                    }
                    true
                } else {
                    false
                }
            }
            ParamType::Bool => matches!(val, Lit::Bool(_)),
            ParamType::Array => matches!(val, Lit::Arr(_)),
            ParamType::Term => matches!(val, Lit::Str(_) | Lit::Num(_) | Lit::Arr(_)),
            _ => true,
        };
        if !ok {
            return self.fail(ctx, format!("the {name} of @{kw} has the wrong type (5.3)"), off);
        }
        Ok(val)
    }

    /// A literal; None for a definition parameter neither passed nor given a default (9.4).
    pub(crate) fn value(&mut self, v: &Value, ctx: &Ctx, off: usize) -> R<Option<Lit>> {
        match v {
            Value::Num { value, .. } => Ok(Some(Lit::Num(*value))),
            Value::Str(s) => Ok(Some(Lit::Str(WStr::new(s, None)))),
            Value::Bool(b) => Ok(Some(Lit::Bool(*b))),
            Value::Arr(items) => {
                let mut out = Vec::new();
                for x in items {
                    match self.value(x, ctx, off)? {
                        Some(v) => out.push(v),
                        None => {
                            let hole = if let Value::Hole(h) = x { h.clone() } else { String::new() };
                            return self.fail(
                                ctx,
                                format!("?{hole} has no value passed and no default: as an array element it has no definite meaning, so the statement fails (9.4)"),
                                off,
                            );
                        }
                    }
                }
                Ok(Some(Lit::Arr(out)))
            }
            Value::Hole(name) => {
                let fr = match &ctx.frame {
                    Some(f) if f.borrow().kind == FrameKind::Definition => f.clone(),
                    _ => {
                        return self.fail(
                            ctx,
                            format!("?{name} is written only in the body of an ordinary definition (6.10)"),
                            off,
                        )
                    }
                };
                let (param, is_role) = {
                    let f = fr.borrow();
                    (f.param(name).cloned(), f.role(name).is_some())
                };
                if let Some(v) = param {
                    return Ok(v);
                }
                if is_role {
                    return self.fail(
                        ctx,
                        format!("?{name} is a role and cannot stand where a literal goes (6.10)"),
                        off,
                    );
                }
                self.fail(ctx, format!("?{name} is not in the parameter list (6.10)"), off)
            }
            Value::Word(w) => {
                let std = self.word(w, ctx.unit);
                match std.as_str() {
                    "true" => Ok(Some(Lit::Bool(true))),
                    "false" => Ok(Some(Lit::Bool(false))),
                    _ => self.fail(ctx, format!("`{w}` is written as a string: enumerations are in quotes (3.7)"), off),
                }
            }
        }
    }

    /// A statement that only draws a scene object, written where there is no circuit.
    pub(crate) fn scene_only<T>(&mut self, ctx: &Ctx, off: usize) -> R<T> {
        if self.units[ctx.unit].kind == UnitKind::Package {
            return self.fail(
                ctx,
                "a statement that only draws a scene object has no level to draw in, in a package (16.2)",
                off,
            );
        }
        self.fail(ctx, "the file level of an included file has no circuit: a statement that only draws a scene object has no level to draw in (6.1)", off)
    }

    /// A statement whose input is a selection: evaluated at its phase (8.2, 10.4).
    pub(crate) fn defer(&mut self, ctx: &Ctx, phase: usize, task: super::scene::Task) -> R<()> {
        self.rules.insert(ctx.member_key.clone());
        if ctx.frame_generated() {
            return self.fail(ctx, "generation does not recurse: a rule in what a generation expands fails (10.5)", 0);
        }
        if let Some(s) = ctx.scene {
            self.scenes[s].tasks[phase].push((ctx.pos.clone(), task));
        }
        Ok(())
    }

    /// The names a definition keyword's body cannot see (6.9): those bound to this segment's result,
    /// added to the ones of the bodies it is written in.
    pub(crate) fn defining_for(&self, ctx: &Ctx, seg: &Segment, member: &Rc<Member>) -> Rc<Defining> {
        let mut out: Defining = ctx.defining.as_ref().map(|d| (**d).clone()).unwrap_or_default();
        let k = member.segments.iter().position(|s| std::ptr::eq(s, seg));
        let unit = self.name_unit(ctx);
        if let Some(k) = k {
            for b in member.binds.iter().filter(|b| b.after == k) {
                out.insert((b.prefix, b.name.clone()), (MemberRef { rec: ctx.rec, m: member.clone() }, unit));
            }
        }
        Rc::new(out)
    }
}

pub fn no_outer_message(e: &Ref) -> String {
    let name = format!("{}{}", e.prefix, e.name.clone().unwrap_or_default());
    let written = e.written.clone().unwrap_or_else(|| name.clone());
    let shown = if written == name { name.clone() } else { format!("{written} ({name})") };
    if e.prefix == '$' {
        format!("a definition body does not see the name it defines: {shown} in the body means the outer definition of that name (above or in a package), and there is none (6.9)")
    } else {
        format!("a definition body does not see the name it defines: {shown} in the body means the outer definition of that name; `%` names are not inherited, and there is none (6.8, 6.9)")
    }
}

pub fn is_color(s: &str) -> bool {
    let Some(h) = s.strip_prefix('#') else { return false };
    matches!(h.len(), 3 | 4 | 6 | 8) && h.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn pos_of_ints(items: &[i64]) -> Pos {
    items.iter().map(|&k| PosElem::Int(k)).collect()
}

/// Parameters of a statement as a lookup table.
pub fn get<'a>(p: &'a [(String, Lit)], name: &str) -> Option<&'a Lit> {
    p.iter().find(|(n, _)| n == name).map(|(_, v)| v)
}

/// The parameters of an `@RNL(…)` header or a call, as written.
pub fn named_value<'a>(p: &'a Params, name: &str) -> Option<&'a Value> {
    p.get(name)
}

/// The innermost segment around a record offset, or the statement when it is outside its segments.
fn innermost(set: &SetExpr, off: usize) -> Option<(usize, usize)> {
    for m in &set.members {
        if off < m.start || off >= m.end.max(m.start + 1) {
            continue;
        }
        for seg in &m.segments {
            if off < seg.start || off >= seg.end {
                continue;
            }
            for a in &seg.args {
                if let ObjExpr::Set(s) = &a.obj {
                    if off >= s.start && off < s.end {
                        if let Some(x) = innermost(s, off) {
                            return Some(x);
                        }
                    }
                }
            }
            return Some((seg.start, seg.end));
        }
        return Some((m.start, m.end));
    }
    None
}
