//! One drawing level: its objects and facts, sets and selections (spec-draft 5.2, 8.2, 10).
//!
//! Phase 3 happens while statements are evaluated; the later phases are in `phases.rs`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::diag::DiagId;
use crate::geom::Orient;
use crate::host::{glob, LevelId};

use super::ctx::Ctx;
use super::eval::{Stop, R};
use super::obj::*;
use super::unit::{DeviceDecl, Place, UnitKind};
use super::Interp;

/// A statement whose input is a selection, run at its phase (8.2, 10.4).
pub type Task = Rc<dyn Fn(&mut Interp, SceneId, usize)>;

/// What a statement adds to each object of its single-object position.
#[derive(Clone, Debug)]
pub enum TargetOp {
    Place { x: Option<f64>, y: Option<f64>, priority: Option<Priority>, off: usize },
    Orient { code: Orient, priority: Option<Priority>, off: usize },
    Layer { layer: String, off: usize },
    Symbol { symbol: ObjId, off: usize },
    Style { style: ObjId, off: usize },
    Current { direction: String, off: usize },
    Priority { priority: Priority, off: usize },
    Route { mode: String, style: StyleMap, off: usize },
}

#[derive(Clone, Debug)]
pub enum FactData {
    Coord(char, f64),
    Orient(Orient),
    Layer(String),
    Symbol(ObjId),
    Style(ObjId),
    /// Style parameters written on the object itself (8.3).
    OwnStyle(StyleMap),
    Priority(Priority),
}

#[derive(Clone, Debug)]
pub struct Fact {
    pub pos: Pos,
    pub target: ObjId,
    pub data: FactData,
    pub priority: Option<Priority>,
    pub ctx: Option<Rc<Ctx>>,
    pub off: usize,
    /// Directly names the object, as opposed to reaching it through a selection (8.3).
    pub direct: bool,
    /// The usages whose expansion made it, outermost first (14.8).
    pub chain: Vec<ObjId>,
}

pub struct Scene {
    pub level: LevelId,
    pub levelobj: ObjId,
    pub phase: usize,
    pub insts: Vec<ObjId>,
    pub inst_keys: HashMap<String, ObjId>,
    pub nets: Vec<ObjId>,
    pub net_keys: HashMap<String, ObjId>,
    pub terms: HashMap<(ObjId, u32), ObjId>,
    pub points: Vec<ObjId>,
    pub usages: Vec<ObjId>,
    pub prims: Vec<ObjId>,
    pub texts: Vec<ObjId>,
    /// The key texts already in `texts`.
    pub listed_keytexts: HashSet<ObjId>,
    pub wires: Vec<ObjId>,
    /// The wires on each net, in the order of `wires`; `scene_add_wire` keeps it.
    pub wires_by_net: HashMap<ObjId, Vec<ObjId>>,
    pub bundles: Vec<ObjId>,
    pub constraints: Vec<ObjId>,
    pub routes: Vec<ObjId>,
    pub keytexts: HashMap<(ObjId, String), ObjId>,
    pub at: Vec<Fact>,
    /// The indexes in `at` of each object's facts, in order; `add_at` keeps it.
    pub at_by_target: HashMap<ObjId, Vec<usize>>,
    pub orient: Vec<Fact>,
    pub appear: Vec<Fact>,
    pub styles: Vec<Fact>,
    pub layers: Vec<Fact>,
    pub prioritys: Vec<Fact>,
    pub currents: Vec<(Pos, ObjId, String)>,
    pub groups: Vec<(Pos, ObjId)>,
    pub tasks: Vec<Vec<(Pos, Task)>>,
    pub created: HashMap<ObjId, Pos>,
    pub items_cache: HashMap<(ObjId, usize), Vec<ObjId>>,
    pub pos_cache: HashMap<ObjId, ObjId>,
    pub symbol: HashMap<ObjId, ObjId>,
    pub default_symbol: HashSet<ObjId>,
    pub pos: HashMap<ObjId, [Option<f64>; 2]>,
    pub ori: HashMap<ObjId, Orient>,
    pub style: HashMap<ObjId, BTreeMap<String, (Lit, Pos)>>,
    pub layer: HashMap<ObjId, (String, Pos)>,
    pub unplaced: Vec<String>,
    pub failed_facts: HashSet<(u8, usize)>,
    /// The symbol facts weighed for some object, and those that won (8.3), by index in `appear`.
    pub symbol_weighed: HashSet<usize>,
    pub symbol_won: HashSet<usize>,
    /// The fact kept per (object, axis) and per object's orientation: (priority, position, index).
    pub at_best: HashMap<(ObjId, char), (Priority, Pos, usize)>,
    pub ori_best: HashMap<ObjId, (Priority, Pos, usize)>,
    pub devices: Option<Vec<DeviceDecl>>,
    pub busy: HashSet<(ObjId, usize)>,
    pub hinted: HashSet<ObjId>,
    pub dropped: usize,
}

impl Scene {
    /// Add a coordinate fact (PLACE, or the coordinates of a point).
    pub(crate) fn add_at(&mut self, f: Fact) {
        self.at_by_target.entry(f.target).or_default().push(self.at.len());
        self.at.push(f);
    }

    fn new(level: LevelId, levelobj: ObjId) -> Scene {
        Scene {
            level,
            levelobj,
            phase: 3,
            insts: Vec::new(),
            inst_keys: HashMap::new(),
            nets: Vec::new(),
            net_keys: HashMap::new(),
            terms: HashMap::new(),
            points: Vec::new(),
            usages: Vec::new(),
            prims: Vec::new(),
            texts: Vec::new(),
            listed_keytexts: HashSet::new(),
            wires: Vec::new(),
            wires_by_net: HashMap::new(),
            bundles: Vec::new(),
            constraints: Vec::new(),
            routes: Vec::new(),
            keytexts: HashMap::new(),
            at: Vec::new(),
            at_by_target: HashMap::new(),
            orient: Vec::new(),
            appear: Vec::new(),
            styles: Vec::new(),
            layers: Vec::new(),
            prioritys: Vec::new(),
            currents: Vec::new(),
            groups: Vec::new(),
            tasks: (0..10).map(|_| Vec::new()).collect(),
            created: HashMap::new(),
            items_cache: HashMap::new(),
            pos_cache: HashMap::new(),
            symbol: HashMap::new(),
            default_symbol: HashSet::new(),
            pos: HashMap::new(),
            ori: HashMap::new(),
            style: HashMap::new(),
            layer: HashMap::new(),
            unplaced: Vec::new(),
            failed_facts: HashSet::new(),
            symbol_weighed: HashSet::new(),
            symbol_won: HashSet::new(),
            at_best: HashMap::new(),
            ori_best: HashMap::new(),
            devices: None,
            busy: HashSet::new(),
            hinted: HashSet::new(),
            dropped: 0,
        }
    }
}

/// A net name condition: `*` patterns against the net, otherwise the same node (6.3, 10.1).
pub(crate) fn name_match(it: &Interp, pattern: &str, key: &str) -> bool {
    if pattern.contains('*') {
        glob(pattern, key, it.doc.dialect.case_sensitive)
    } else {
        key == it.doc.net_key(pattern)
    }
}

pub(crate) fn dedup(objs: Vec<ObjId>) -> Vec<ObjId> {
    let mut seen = HashSet::new();
    objs.into_iter().filter(|o| seen.insert(*o)).collect()
}

impl Interp<'_> {
    // ------------------------------------------------------------------ the scene of a level

    pub(crate) fn create_scene(&mut self, level: LevelId) -> SceneId {
        let s = self.scenes.len();
        let levelobj = self.new_obj(ObjData::LevelObj { scene: s });
        self.scenes.push(Scene::new(level, levelobj));
        self.level_scene[level] = Some(s);
        let n = self.doc.levels[level].instances.len();
        for idx in 0..n {
            let o = self.new_obj(ObjData::Inst { scene: s, idx, bundle: None });
            let key = self.doc.key(&self.doc.levels[level].instances[idx].name);
            self.scenes[s].insts.push(o);
            self.scenes[s].inst_keys.insert(key, o);
        }
        let ports = self.doc.levels[level].ports.clone();
        for p in ports {
            self.net_obj(s, &p);
        }
        for idx in 0..n {
            let nodes = self.doc.levels[level].instances[idx].nodes.clone();
            for nd in nodes {
                self.net_obj(s, &nd);
            }
        }
        s
    }

    pub(crate) fn net_obj(&mut self, s: SceneId, name: &str) -> ObjId {
        let key = self.doc.net_key(name);
        if let Some(&o) = self.scenes[s].net_keys.get(&key) {
            return o;
        }
        let display = if key == "0" { "0".to_string() } else { name.to_string() };
        let o = self.new_obj(ObjData::Net { scene: s, key: key.clone(), display });
        self.scenes[s].nets.push(o);
        self.scenes[s].net_keys.insert(key, o);
        o
    }

    pub(crate) fn scene_inst(&self, s: SceneId, name: &str) -> Option<ObjId> {
        self.scenes[s].inst_keys.get(&self.doc.key(name)).copied()
    }

    pub(crate) fn scene_net(&self, s: SceneId, name: &str) -> Option<ObjId> {
        self.scenes[s].net_keys.get(&self.doc.net_key(name)).copied()
    }

    pub(crate) fn inst_of(&self, id: ObjId) -> &crate::host::Instance {
        match self.o(id) {
            ObjData::Inst { scene, idx, .. } => &self.doc.levels[self.scenes[*scene].level].instances[*idx],
            _ => panic!("not an instance"),
        }
    }

    pub(crate) fn bundle_field(&self, id: ObjId) -> Option<ObjId> {
        match self.o(id) {
            ObjData::Inst { bundle, .. } => *bundle,
            _ => None,
        }
    }

    /// Terminal names of a host instance: subcircuit ports or DEVICE (6.4, 6.12).
    pub(crate) fn term_names(&mut self, s: SceneId, inst: ObjId) -> Vec<String> {
        let i = self.inst_of(inst).clone();
        if i.is_subckt {
            if let Some(m) = &i.model {
                if let Some(sub) = self.doc.subckt(m) {
                    return self.doc.levels[sub].ports.clone();
                }
            }
        }
        self.device_pins(s, &i).unwrap_or_default()
    }

    fn device_pins(&mut self, s: SceneId, inst: &crate::host::Instance) -> Option<Vec<String>> {
        if self.scenes[s].devices.is_none() {
            let lv = self.scenes[s].level;
            let d = self.devices_for(lv);
            self.scenes[s].devices = Some(d);
        }
        let cs = inst.case_sensitive;
        let mut found = None;
        for d in self.scenes[s].devices.as_ref().unwrap() {
            let model_ok = match &d.model {
                None => true,
                Some(m) => inst.model.as_ref().is_some_and(|im| glob(m, im, cs)),
            };
            if glob(&d.name, &inst.name, cs) && model_ok {
                found = Some(
                    d.terminals
                        .iter()
                        .map(|t| match t {
                            Lit::Str(s) => s.as_str().to_string(),
                            Lit::Num(n) => crate::geom::fmt_num(*n),
                            other => other.show(),
                        })
                        .collect(),
                );
            }
        }
        found.or_else(|| inst.default_terminals.clone())
    }

    /// The terminals of an owner: (index from 1, name).
    pub(crate) fn owner_terms(&mut self, s: SceneId, owner: ObjId) -> Vec<(u32, Option<String>)> {
        match self.o(owner).clone() {
            ObjData::Inst { .. } => {
                let n = self.inst_of(owner).nodes.len();
                let names = self.term_names(s, owner);
                (0..n).map(|k| (k as u32 + 1, names.get(k).cloned())).collect()
            }
            ObjData::Bundle { members, .. } => match members.first() {
                Some(m) => self.owner_terms(s, *m),
                None => Vec::new(),
            },
            ObjData::LevelObj { .. } => {
                let lv = self.scenes[s].level;
                self.doc.levels[lv].ports.iter().enumerate().map(|(k, p)| (k as u32 + 1, Some(p.clone()))).collect()
            }
            ObjData::Usage(u) => {
                let d = self.def(u.definition);
                d.roles
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| !r.is_set)
                    .map(|(k, r)| (k as u32 + 1, Some(r.name.clone())))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    fn host_case(&self, owner: ObjId) -> bool {
        match self.o(owner) {
            ObjData::Usage(_) => true,
            _ => self.doc.dialect.case_sensitive,
        }
    }

    pub(crate) fn scene_term(
        &mut self,
        s: SceneId,
        owner: ObjId,
        name: Option<&str>,
        number: Option<u32>,
    ) -> Option<ObjId> {
        let cs = self.host_case(owner);
        for (idx, tname) in self.owner_terms(s, owner) {
            let by_num = number.is_some_and(|n| n == idx);
            let by_name = match (&tname, name) {
                (Some(t), Some(n)) => {
                    if cs {
                        t == n
                    } else {
                        t.eq_ignore_ascii_case(n)
                    }
                }
                _ => false,
            };
            if by_num || by_name {
                return Some(self.term_obj(s, owner, idx, tname));
            }
        }
        None
    }

    pub(crate) fn term_obj(&mut self, s: SceneId, owner: ObjId, idx: u32, name: Option<String>) -> ObjId {
        if let Some(&t) = self.scenes[s].terms.get(&(owner, idx)) {
            return t;
        }
        let t = self.new_obj(ObjData::Term { owner, index: idx, name });
        self.scenes[s].terms.insert((owner, idx), t);
        t
    }

    pub(crate) fn terms_of(&mut self, s: SceneId, owner: ObjId) -> Vec<ObjId> {
        self.owner_terms(s, owner).into_iter().map(|(idx, n)| self.term_obj(s, owner, idx, n)).collect()
    }

    pub(crate) fn term_net(&mut self, s: SceneId, t: ObjId, ctx: &Ctx, off: usize) -> R<ObjId> {
        let ObjData::Term { owner, index, name } = self.o(t).clone() else { unreachable!() };
        match self.o(owner).clone() {
            ObjData::Inst { .. } => {
                let node = self.inst_of(owner).nodes[index as usize - 1].clone();
                return Ok(self.net_obj(s, &node));
            }
            ObjData::LevelObj { .. } => {
                let lv = self.scenes[s].level;
                let p = self.doc.levels[lv].ports[index as usize - 1].clone();
                return Ok(self.net_obj(s, &p));
            }
            ObjData::Bundle { members, .. } => {
                let mut nets = Vec::new();
                for m in members {
                    let node = self.inst_of(m).nodes[index as usize - 1].clone();
                    nets.push(self.net_obj(s, &node));
                }
                if nets.iter().all(|n| *n == nets[0]) {
                    return Ok(nets[0]);
                }
                return Ok(self.new_obj(ObjData::Set(Box::new(SetData::plain(
                    nets.into_iter().map(|n| (n, true)).collect(),
                    false,
                )))));
            }
            ObjData::Usage(u) => {
                if let Some(mem) = name.as_ref().and_then(|n| u.role(n)) {
                    match self.o(mem) {
                        ObjData::Net { .. } => return Ok(mem),
                        ObjData::Term { .. } => return self.term_net(s, mem, ctx, off),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        let l = self.label(t);
        self.fail(ctx, format!("{l} is on no net (6.5)"), off)
    }

    /// The nets a terminal is on (several for a bundle's terminal).
    pub(crate) fn tnets(&mut self, s: SceneId, t: ObjId) -> Vec<ObjId> {
        let ObjData::Term { owner, index, .. } = self.o(t).clone() else { return Vec::new() };
        match self.o(owner).clone() {
            ObjData::Bundle { members, .. } => members
                .iter()
                .map(|m| {
                    let node = self.inst_of(*m).nodes[index as usize - 1].clone();
                    self.net_obj(s, &node)
                })
                .collect(),
            ObjData::Inst { .. } => {
                let node = self.inst_of(owner).nodes[index as usize - 1].clone();
                vec![self.net_obj(s, &node)]
            }
            ObjData::LevelObj { .. } => {
                let lv = self.scenes[s].level;
                let p = self.doc.levels[lv].ports[index as usize - 1].clone();
                vec![self.net_obj(s, &p)]
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn scene_pos_of(&mut self, s: SceneId, obj: ObjId) -> ObjId {
        if let Some(&p) = self.scenes[s].pos_cache.get(&obj) {
            return p;
        }
        let p = self.new_obj(ObjData::Pos { parent: obj });
        self.scenes[s].pos_cache.insert(obj, p);
        p
    }

    pub(crate) fn scene_keytext(&mut self, s: SceneId, obj: ObjId, key: &str) -> ObjId {
        if let Some(&k) = self.scenes[s].keytexts.get(&(obj, key.to_string())) {
            return k;
        }
        let k = self.new_obj(ObjData::KeyText {
            attach: obj,
            key: key.to_string(),
            layers: Vec::new(),
            from_symbol: false,
            sym_pos: None,
            generated: false,
            failed: false,
            site: None,
            off: 0,
        });
        self.scenes[s].keytexts.insert((obj, key.to_string()), k);
        k
    }

    /// The ordering position of a member of a unit in this scene (8.3, 8.5).
    pub(crate) fn position_of(&mut self, s: SceneId, u: UnitId, rec: Option<RecId>, k: Option<usize>) -> Pos {
        let lv = self.scenes[s].level;
        let tail = [rec.map(|r| r as i64).unwrap_or(0), k.map(|k| k as i64).unwrap_or(0)];
        if self.units[u].kind == UnitKind::Level {
            let ul = self.units[u].level;
            if ul == Some(lv) {
                return pos(&[3, 0, tail[0], tail[1]]);
            }
            let anc = self.doc.ancestors(lv);
            if let Some(i) = anc.iter().position(|a| Some(*a) == ul) {
                let depth = (anc.len() - i) as i64;
                return pos(&[2, depth, tail[0], tail[1]]);
            }
            return pos(&[9, 0, tail[0], tail[1]]);
        }
        for e in self.effective_packages(lv) {
            if e.unit == u {
                match e.place {
                    Place::Root(k0) => return pos(&[0, k0 as i64, tail[0], tail[1]]),
                    Place::Level(at, upos) => {
                        let decl = e.decl.unwrap();
                        let au = self.level_unit[at];
                        let mut base = self.position_of(s, au, decl.rec, Some(upos.1));
                        base.push(PosElem::Tag("pkg"));
                        base.push(PosElem::Int(tail[0]));
                        base.push(PosElem::Int(tail[1]));
                        return base;
                    }
                }
            }
        }
        pos(&[9, 1, tail[0], tail[1]])
    }

    // ------------------------------------------------------------------ creation (phase 3)

    pub(crate) fn scene_add_point(&mut self, s: SceneId, p: ObjId, at: Pos) {
        let (x, y) = match self.o(p) {
            ObjData::Point { x, y, .. } => (*x, *y),
            _ => return,
        };
        let sc = &mut self.scenes[s];
        sc.points.push(p);
        sc.created.insert(p, at.clone());
        let mut fp = at;
        fp.push(PosElem::Tag("p"));
        for (axis, v) in [('x', x), ('y', y)] {
            if let Some(v) = v {
                sc.add_at(Fact {
                    pos: fp.clone(),
                    target: p,
                    data: FactData::Coord(axis, v),
                    priority: None,
                    ctx: None,
                    off: 0,
                    direct: true,
                    chain: Vec::new(),
                });
            }
        }
    }

    pub(crate) fn scene_add_usage(&mut self, s: SceneId, a: ObjId, at: Pos) {
        self.scenes[s].usages.push(a);
        self.scenes[s].created.insert(a, at);
    }

    pub(crate) fn scene_add_prim(&mut self, s: SceneId, prim: ObjId, at: Pos, ctx: &Ctx) {
        let style = match self.o(prim) {
            ObjData::Prim { style, .. } => style.clone(),
            _ => StyleMap::new(),
        };
        let sc = &mut self.scenes[s];
        sc.prims.push(prim);
        sc.created.insert(prim, at.clone());
        if !style.is_empty() {
            sc.styles.push(Fact {
                pos: at,
                target: prim,
                data: FactData::OwnStyle(style),
                priority: None,
                ctx: Some(Rc::new(ctx.clone())),
                off: 0,
                direct: true,
                chain: Vec::new(),
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn scene_add_text(
        &mut self,
        s: SceneId,
        att: ObjId,
        key: Option<String>,
        items: TextItems,
        style: StyleMap,
        at: Pos,
        ctx: &Ctx,
        generated: bool,
        key_unit: UnitId,
        off: usize,
    ) -> R<ObjId> {
        if let ObjData::Failed(d) = self.o(att) {
            let d = *d;
            let id = self.err_cause(ctx, "the object the TEXT is attached to failed", 0, d);
            return Err(Stop::Fail(Some(id)));
        }
        let site = Some(Rc::new(ctx.clone()));
        let kind = self.o(att).clone();
        let t = match kind {
            ObjData::Point { .. } | ObjData::Pos { .. } => {
                if key.is_some() {
                    return self.fail(ctx, "a text attached to a point takes no key= (13.3)", 0);
                }
                if items.text.is_none() {
                    return self.fail(ctx, "the text= of TEXT must be written (13.3)", 0);
                }
                self.new_obj(ObjData::Text {
                    attach: att,
                    items,
                    style,
                    from_symbol: false,
                    sym_pos: None,
                    generated,
                    failed: false,
                    site,
                    off: 0,
                })
            }
            ObjData::Inst { .. }
            | ObjData::Usage(_)
            | ObjData::Bundle { .. }
            | ObjData::Term { .. }
            | ObjData::Wire { .. }
            | ObjData::Prim { .. }
            | ObjData::Net { .. } => {
                if let (Some(k), ObjData::Usage(_)) = (&key, &kind) {
                    // not spelled like a role or local name of the usage, or an alias of one (6.5, 13.3)
                    if let Some(clash) = self.key_clash(k, key_unit, Some(att)) {
                        return self.fail(ctx, clash, off);
                    }
                }
                if let Some(k) = &key {
                    let kt = self.scene_keytext(s, att, k);
                    if let ObjData::KeyText { layers, site: ks, generated: g, .. } = self.om(kt) {
                        layers.push((at.clone(), items, style));
                        *ks = site;
                        *g = generated;
                    }
                    let sc = &mut self.scenes[s];
                    sc.created.entry(kt).or_insert(at);
                    if sc.listed_keytexts.insert(kt) {
                        sc.texts.push(kt);
                    }
                    return Ok(kt);
                }
                if items.text.is_none() {
                    return self.fail(ctx, "the text= of TEXT must be written (13.3)", 0);
                }
                let has_offset = items.dx.is_some() || items.dy.is_some();
                if has_offset && matches!(kind, ObjData::Wire { .. } | ObjData::Net { .. }) {
                    return self.fail(ctx, "a text attached to a wire or a net takes no dx, dy (13.3)", 0);
                }
                self.new_obj(ObjData::Text {
                    attach: att,
                    items,
                    style,
                    from_symbol: false,
                    sym_pos: None,
                    generated,
                    failed: false,
                    site,
                    off: 0,
                })
            }
            _ => {
                let l = self.label(att);
                return self.fail(ctx, format!("TEXT cannot be attached to {l} (13.3)"), 0);
            }
        };
        self.scenes[s].texts.push(t);
        self.scenes[s].created.insert(t, at);
        Ok(t)
    }

    pub(crate) fn scene_add_wire(&mut self, s: SceneId, w: ObjId, at: Pos, ctx: &Ctx) {
        let (style, nets) = match self.o(w) {
            ObjData::Wire { style, nets, .. } => (style.clone(), nets.clone()),
            _ => (StyleMap::new(), Vec::new()),
        };
        let sc = &mut self.scenes[s];
        sc.wires.push(w);
        // a wire's nets do not change after it is made
        for (k, n) in nets.iter().enumerate() {
            if !nets[..k].contains(n) {
                sc.wires_by_net.entry(*n).or_default().push(w);
            }
        }
        sc.created.insert(w, at.clone());
        if !style.is_empty() {
            sc.styles.push(Fact {
                pos: at,
                target: w,
                data: FactData::OwnStyle(style),
                priority: None,
                ctx: Some(Rc::new(ctx.clone())),
                off: 0,
                direct: true,
                chain: Vec::new(),
            });
        }
    }

    pub(crate) fn scene_add_bundle(&mut self, s: SceneId, b: ObjId, at: Pos) {
        self.scenes[s].bundles.push(b);
        self.scenes[s].created.insert(b, at);
    }

    pub(crate) fn scene_add_constraint(&mut self, s: SceneId, c: ObjId, at: Pos) {
        self.scenes[s].constraints.push(c);
        self.scenes[s].created.insert(c, at);
    }

    /// Apply a single-object statement to one object.
    pub(crate) fn apply_op(
        &mut self,
        s: SceneId,
        op: &TargetOp,
        obj: ObjId,
        at: Pos,
        direct: bool,
        ctx: &Ctx,
    ) -> R<()> {
        let chain = self.apply_chain(ctx);
        let site = Some(Rc::new(ctx.clone()));
        match op {
            TargetOp::Place { x, y, priority, off } => {
                let mut px = at.clone();
                px.push(PosElem::Int(0));
                let mut py = at;
                py.push(PosElem::Int(1));
                if let Some(x) = x {
                    self.scenes[s].add_at(Fact {
                        pos: px,
                        target: obj,
                        data: FactData::Coord('x', *x),
                        priority: *priority,
                        ctx: site.clone(),
                        off: *off,
                        direct,
                        chain: chain.clone(),
                    });
                }
                if let Some(y) = y {
                    self.scenes[s].add_at(Fact {
                        pos: py,
                        target: obj,
                        data: FactData::Coord('y', *y),
                        priority: *priority,
                        ctx: site,
                        off: *off,
                        direct,
                        chain,
                    });
                }
            }
            TargetOp::Orient { code, priority, off } => {
                self.scenes[s].orient.push(Fact {
                    pos: at,
                    target: obj,
                    data: FactData::Orient(*code),
                    priority: *priority,
                    ctx: site,
                    off: *off,
                    direct,
                    chain,
                });
            }
            TargetOp::Layer { layer, off } => {
                self.scenes[s].layers.push(Fact {
                    pos: at,
                    target: obj,
                    data: FactData::Layer(layer.clone()),
                    priority: None,
                    ctx: site,
                    off: *off,
                    direct,
                    chain: Vec::new(),
                });
            }
            TargetOp::Symbol { symbol, off } => {
                self.scenes[s].appear.push(Fact {
                    pos: at,
                    target: obj,
                    data: FactData::Symbol(*symbol),
                    priority: None,
                    ctx: site,
                    off: *off,
                    direct,
                    chain: Vec::new(),
                });
            }
            TargetOp::Style { style, off } => {
                self.scenes[s].styles.push(Fact {
                    pos: at,
                    target: obj,
                    data: FactData::Style(*style),
                    priority: None,
                    ctx: site,
                    off: *off,
                    direct,
                    chain: Vec::new(),
                });
            }
            TargetOp::Current { direction, off } => {
                let ok = match self.o(obj) {
                    ObjData::Term { owner, .. } => {
                        matches!(self.o(*owner), ObjData::Inst { .. } | ObjData::LevelObj { .. })
                    }
                    _ => false,
                };
                if !ok {
                    let l = self.label(obj);
                    return self.fail(ctx, format!("CURRENT takes host terminals only, not {l} (15.2)"), *off);
                }
                self.scenes[s].currents.push((at, obj, direction.clone()));
            }
            TargetOp::Priority { priority, off } => {
                if !matches!(self.o(obj), ObjData::Constraint(_) | ObjData::Usage(_) | ObjData::Set(_)) {
                    let l = self.label(obj);
                    return self.fail(
                        ctx,
                        format!("PRIORITY takes constraints, usages and grouped sets only, not {l} (14.8)"),
                        *off,
                    );
                }
                self.scenes[s].prioritys.push(Fact {
                    pos: at,
                    target: obj,
                    data: FactData::Priority(*priority),
                    priority: None,
                    ctx: site,
                    off: *off,
                    direct,
                    chain: Vec::new(),
                });
            }
            TargetOp::Route { mode, style, off } => {
                if !matches!(self.o(obj), ObjData::Net { .. }) {
                    let l = self.label(obj);
                    return self.fail(ctx, format!("ROUTE takes nets, not {l} (13.7)"), *off);
                }
                let r = self.new_obj(ObjData::Route { net: obj, mode: mode.clone(), style: style.clone() });
                self.scenes[s].routes.push(r);
                self.scenes[s].created.insert(r, at);
                self.made_routes.push(r);
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------ sets (5.2, 10.3)

    /// Leaf members, sets inside sets spread all the way (done one by one, 5.2.1).
    pub(crate) fn flatten(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: Option<&Ctx>) -> Vec<ObjId> {
        let mut out = Vec::new();
        for m in self.items_of(s, obj, phase, ctx) {
            if self.is_set(m) {
                out.extend(self.flatten(s, m, phase, ctx));
            } else {
                out.push(m);
            }
        }
        out
    }

    /// Members of a set one level deep: references to sets are spread, written `{…}` stay.
    pub(crate) fn items_of(&mut self, s: SceneId, obj: ObjId, phase: usize, ctx: Option<&Ctx>) -> Vec<ObjId> {
        if obj == LEVEL {
            return vec![self.scenes[s].levelobj];
        }
        if !self.is_set(obj) {
            return vec![obj];
        }
        let key = (obj, phase);
        if let Some(v) = self.scenes[s].items_cache.get(&key) {
            return v.clone();
        }
        if self.scenes[s].busy.contains(&key) {
            if self.set(obj).removes {
                let msg =
                    "a cycle of sets with removals has no definite result; the whole interpretation fails (5.2.2)";
                self.failed_doc = Some(msg.to_string());
                let loc = ctx.and_then(|c| self.rec_loc(c.rec, 0));
                self.sink.error(msg, loc);
            }
            return Vec::new();
        }
        self.scenes[s].busy.insert(key);
        // this scene's edits as they are now, in the order of their positions; the edits of the
        // other scenes are not looked at
        let mods: Vec<(bool, ObjId, Rc<Ctx>)> = {
            let sd = self.set(obj);
            let mut ks = sd.mods_by_scene.get(&s).cloned().unwrap_or_default();
            ks.sort_by(|a, b| sd.mods[*a].pos.cmp(&sd.mods[*b].pos));
            ks.into_iter().map(|k| (sd.mods[k].add, sd.mods[k].source, sd.mods[k].ctx.clone())).collect()
        };
        let mut base: Vec<ObjId> = match &self.set(obj).kind {
            SetKind::Select { .. } => self.select(s, obj, phase, ctx),
            SetKind::Gen { by_scene } => {
                if phase >= 5 {
                    by_scene.get(&s).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            &SetKind::Union { source } => {
                let mut seen = HashSet::new();
                let mut out = Vec::new();
                for m in self.items_of(s, source, phase, ctx) {
                    let xs =
                        if self.is_set(m) && !self.set(m).explicit { self.items_of(s, m, phase, ctx) } else { vec![m] };
                    for x in xs {
                        if seen.insert(x) {
                            out.push(x);
                        }
                    }
                }
                out
            }
            SetKind::Plain => {
                let mut out = Vec::new();
                for (o, spread) in self.set(obj).items.clone() {
                    if self.is_failed(o) {
                        continue;
                    }
                    if o == LEVEL {
                        out.push(self.scenes[s].levelobj);
                    } else if self.is_set(o) && (spread || !self.set(o).explicit) {
                        out.extend(self.items_of(s, o, phase, ctx));
                    } else {
                        out.push(o);
                    }
                }
                out
            }
        };
        for (add, source, mctx) in mods {
            let srcm = if self.is_set(source) { self.items_of(s, source, phase, Some(&mctx)) } else { vec![source] };
            if add {
                base.extend(srcm);
            } else {
                let ids: HashSet<ObjId> = srcm.into_iter().collect();
                base.retain(|x| !ids.contains(x));
            }
        }
        let base: Vec<ObjId> = base.into_iter().filter(|x| self.belongs(s, *x)).collect();
        self.scenes[s].busy.remove(&key);
        self.scenes[s].items_cache.insert(key, base.clone());
        base
    }

    fn belongs(&self, s: SceneId, x: ObjId) -> bool {
        match self.o(x) {
            ObjData::Inst { scene, .. } | ObjData::Net { scene, .. } => *scene == s,
            ObjData::Term { owner, .. } => match self.o(*owner) {
                ObjData::LevelObj { .. } => *owner == self.scenes[s].levelobj,
                _ => self.belongs(s, *owner),
            },
            ObjData::LevelObj { .. } => x == self.scenes[s].levelobj,
            ObjData::Usage(_)
            | ObjData::Wire { .. }
            | ObjData::Prim { .. }
            | ObjData::Text { .. }
            | ObjData::Bundle { .. }
            | ObjData::Constraint(_)
            | ObjData::Point { .. } => self.scenes[s].created.contains_key(&x) || self.scenes[s].points.contains(&x),
            _ => true,
        }
    }

    // ------------------------------------------------------------------ selections (10.1)

    fn select(&mut self, s: SceneId, sel: ObjId, phase: usize, ctx: Option<&Ctx>) -> Vec<ObjId> {
        let SetKind::Select { keyword, scope, params, writer, definition, site, off } = self.set(sel).kind.clone()
        else {
            unreachable!()
        };
        if params.get("recursive").and_then(Lit::boolean) == Some(false) {
            let w = &self.units[writer];
            if w.kind == UnitKind::Level && w.level != Some(self.scenes[s].level) {
                return Vec::new();
            }
        }
        let snapshot = phase <= 5; // generated objects are not seen before step 6 (10.5)
        let pool: Option<Vec<ObjId>> = if scope == LEVEL || matches!(self.o(scope), ObjData::LevelObj { .. }) {
            None
        } else if self.is_set(scope) {
            Some(self.flatten(s, scope, phase, ctx))
        } else {
            Some(vec![scope])
        };
        if keyword == "SELECT-USAGE" || keyword == "SELECT-ANCESTOR-USAGE" {
            return self.select_usages(s, sel, &keyword, pool, definition, phase, ctx, snapshot, site, off);
        }
        let role = params.get("role").cloned();
        let (pool, via) = match pool {
            Some(p) => {
                let (p, v) = self.walk_usages(&p, role.as_ref(), writer);
                (Some(p), v)
            }
            None => (None, HashSet::new()),
        };
        match keyword.as_str() {
            "SELECT-INSTANCE" => {
                let cands = match &pool {
                    None => self.inst_like(s),
                    Some(pool) => {
                        // a terminal in the scope gives its device, a net the devices with a terminal on it; as for
                        // SELECT-TERMINAL, only terminals with a pin count once the symbols are known (TR11, 9.2.4)
                        let mut cands = Vec::new();
                        for &o in pool {
                            match self.o(o).clone() {
                                ObjData::Inst { .. } | ObjData::Bundle { .. } => cands.push(self.bundle_of(o)),
                                _ if via.contains(&o) => {}
                                ObjData::Term { owner, .. }
                                    if matches!(self.o(owner), ObjData::Inst { .. } | ObjData::Bundle { .. }) =>
                                {
                                    if self.drawn(s, o, phase) {
                                        cands.push(self.bundle_of(owner));
                                    }
                                }
                                ObjData::Net { .. } => {
                                    for ow in self.inst_like(s) {
                                        let mut hit = false;
                                        for t in self.terms_of(s, ow) {
                                            if self.tnet_has(s, t, o) && self.drawn(s, t, phase) {
                                                hit = true;
                                                break;
                                            }
                                        }
                                        if hit {
                                            cands.push(ow);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        cands
                    }
                };
                let out: Vec<ObjId> = cands.into_iter().filter(|o| self.inst_match(*o, &params, false)).collect();
                dedup(out)
            }
            "SELECT-TERMINAL" => {
                let terms: Vec<ObjId> = match &pool {
                    None => {
                        let mut out = Vec::new();
                        for o in self.inst_like(s) {
                            out.extend(self.terms_of(s, o));
                        }
                        out
                    }
                    Some(pool) => {
                        let mut out = Vec::new();
                        for &o in pool {
                            let is_term = matches!(self.o(o), ObjData::Term { .. });
                            if via.contains(&o) && !is_term {
                                continue; // members are taken by kind, not used as a scope again (10.1, PR16)
                            }
                            match self.o(o).clone() {
                                ObjData::Net { .. } => {
                                    for ow in self.inst_like(s) {
                                        for t in self.terms_of(s, ow) {
                                            if self.tnet_has(s, t, o) {
                                                out.push(t);
                                            }
                                        }
                                    }
                                }
                                ObjData::Inst { .. } | ObjData::Bundle { .. } => {
                                    let b = self.bundle_of(o);
                                    out.extend(self.terms_of(s, b));
                                }
                                ObjData::Term { .. } => out.push(o),
                                _ => {}
                            }
                        }
                        out
                    }
                };
                let ends = if params.contains_key("wired") { self.wire_ends(s) } else { HashMap::new() };
                let mut out = Vec::new();
                for t in terms {
                    let ObjData::Term { owner, index, name } = self.o(t).clone() else { continue };
                    if !self.inst_match(owner, &params, true) {
                        continue;
                    }
                    if let Some(pv) = params.get("terminal") {
                        let ok = match pv {
                            Lit::Num(n) => index == *n as u32,
                            Lit::Str(w) => name.as_ref().is_some_and(|n| n.eq_ignore_ascii_case(w.as_str())),
                            _ => false,
                        };
                        if !ok {
                            continue;
                        }
                    }
                    if let Some(net) = params.get("net").and_then(Lit::str) {
                        let net = net.to_string();
                        let nets = self.tnets(s, t);
                        let ok = nets.iter().any(|n| match self.o(*n) {
                            ObjData::Net { key, .. } => name_match(self, &net, key),
                            _ => false,
                        });
                        if !ok {
                            continue;
                        }
                    }
                    if let Some(w) = params.get("wired").and_then(Lit::boolean) {
                        if self.wired(s, t, &ends) != w {
                            continue;
                        }
                    }
                    if let Some(c) = params.get("current").and_then(Lit::str) {
                        if self.current_of(s, t) != c {
                            continue;
                        }
                    }
                    if phase >= 5 && !self.has_pin(s, t) {
                        continue;
                    }
                    out.push(t);
                }
                dedup(out)
            }
            "SELECT-NET" => {
                let cands = match &pool {
                    None => self.scenes[s].nets.clone(),
                    Some(pool) => {
                        // a device, terminal or wire in the scope gives the nets it is on (TR11)
                        let mut cands = Vec::new();
                        for &o in pool {
                            match self.o(o).clone() {
                                ObjData::Net { .. } => cands.push(o),
                                _ if via.contains(&o) => {}
                                ObjData::Inst { .. } | ObjData::Bundle { .. } => {
                                    let b = self.bundle_of(o);
                                    for t in self.terms_of(s, b) {
                                        if self.drawn(s, t, phase) {
                                            cands.extend(self.tnets(s, t));
                                        }
                                    }
                                }
                                ObjData::Term { owner, .. } => {
                                    // a port of this level names its net; other terminals count with a pin (TR11)
                                    if matches!(self.o(owner), ObjData::LevelObj { .. }) || self.drawn(s, o, phase) {
                                        cands.extend(self.tnets(s, o));
                                    }
                                }
                                ObjData::Wire { nets, .. } => cands.extend(nets),
                                _ => {}
                            }
                        }
                        dedup(cands)
                    }
                };
                let lv = self.scenes[s].level;
                let ports: Vec<String> = self.doc.levels[lv].ports.iter().map(|p| self.doc.net_key(p)).collect();
                let ends = if params.contains_key("wired") { self.wire_ends(s) } else { HashMap::new() };
                let mut out = Vec::new();
                for n in cands {
                    let key = match self.o(n) {
                        ObjData::Net { key, .. } => key.clone(),
                        _ => continue,
                    };
                    if let Some(name) = params.get("name").and_then(Lit::str) {
                        if !name_match(self, name, &key) {
                            continue;
                        }
                    }
                    if let Some(port) = params.get("port").and_then(Lit::boolean) {
                        if ports.contains(&key) != port {
                            continue;
                        }
                    }
                    if let Some(g) = params.get("global").and_then(Lit::boolean) {
                        if (key == "0" || self.doc.globals.contains(&key)) != g {
                            continue;
                        }
                    }
                    if let Some(w) = params.get("wired").and_then(Lit::boolean) {
                        if self.net_wired(s, n, &ends) != w {
                            continue;
                        }
                    }
                    out.push(n);
                }
                out
            }
            "SELECT-WIRE" => {
                let wires: Vec<ObjId> = self
                    .drawables(s, snapshot)
                    .into_iter()
                    .filter(|o| matches!(self.o(*o), ObjData::Wire { .. }))
                    .collect();
                let cands: Vec<ObjId> = match &pool {
                    None => wires,
                    Some(pool) => {
                        // a net in the scope gives the wires drawn on it; nets that are only usage members are not scopes (PR16)
                        let nets: HashSet<ObjId> = pool
                            .iter()
                            .filter(|o| matches!(self.o(**o), ObjData::Net { .. }) && !via.contains(o))
                            .copied()
                            .collect();
                        let ids: HashSet<ObjId> =
                            pool.iter().filter(|o| matches!(self.o(**o), ObjData::Wire { .. })).copied().collect();
                        wires
                            .into_iter()
                            .filter(|w| ids.contains(w) || matches!(self.o(*w), ObjData::Wire { nets: wn, .. } if wn.iter().any(|n| nets.contains(n))))
                            .collect()
                    }
                };
                let mut out = Vec::new();
                for w in cands {
                    let ObjData::Wire { nets, bus, .. } = self.o(w).clone() else { continue };
                    if let Some(net) = params.get("net").and_then(Lit::str) {
                        let ok = nets
                            .iter()
                            .any(|n| matches!(self.o(*n), ObjData::Net { key, .. } if name_match(self, net, key)));
                        if !ok {
                            continue;
                        }
                    }
                    if let Some(b) = params.get("bus").and_then(Lit::boolean) {
                        if bus != b {
                            continue;
                        }
                    }
                    out.push(w);
                }
                out
            }
            "SELECT-LABEL" => {
                let labels: Vec<ObjId> =
                    self.drawables(s, snapshot).into_iter().filter(|o| self.obj_kind(*o) == Some("label")).collect();
                let cands: Vec<ObjId> = match &pool {
                    None => labels,
                    Some(pool) => {
                        // an object in the scope gives the labels attached to it; a label in the scope is taken itself
                        let ids: HashSet<ObjId> = pool
                            .iter()
                            .filter(|o| {
                                !via.contains(o)
                                    || matches!(self.o(**o), ObjData::Text { .. } | ObjData::KeyText { .. })
                            })
                            .copied()
                            .collect();
                        labels.into_iter().filter(|x| ids.contains(x) || ids.contains(&self.text_attach(*x))).collect()
                    }
                };
                let mut out = Vec::new();
                for x in cands {
                    if let Some(k) = params.get("key").and_then(Lit::str) {
                        let ok = match self.o(x) {
                            ObjData::KeyText { key, .. } => glob(k, key, true),
                            _ => false,
                        };
                        if !ok {
                            continue;
                        }
                    }
                    out.push(x);
                }
                out
            }
            "SELECT-ANNOTATION" => {
                // the graphics written in the scene, and texts attached to points (13.4, TR3)
                let objs = match pool {
                    None => self.drawables(s, snapshot),
                    Some(p) => p,
                };
                objs.into_iter().filter(|o| self.obj_kind(*o) == Some("annotation")).collect()
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn text_attach(&self, t: ObjId) -> ObjId {
        match self.o(t) {
            ObjData::Text { attach, .. } | ObjData::KeyText { attach, .. } => *attach,
            _ => NOTHING,
        }
    }

    /// SELECT-USAGE looks inward, SELECT-ANCESTOR-USAGE outward (10.1).
    #[allow(clippy::too_many_arguments)]
    fn select_usages(
        &mut self,
        s: SceneId,
        sel: ObjId,
        kw: &str,
        pool: Option<Vec<ObjId>>,
        dfn: Option<ObjId>,
        phase: usize,
        ctx: Option<&Ctx>,
        snapshot: bool,
        site: Option<Rc<Ctx>>,
        off: usize,
    ) -> Vec<ObjId> {
        let usages: Vec<ObjId> = self.scenes[s]
            .usages
            .clone()
            .into_iter()
            .filter(|a| {
                let u = self.usage(*a);
                !u.in_symbol && !(snapshot && u.generated)
            })
            .collect();
        if kw == "SELECT-USAGE" {
            let Some(pool) = pool else {
                return usages.into_iter().filter(|a| dfn.is_none_or(|d| self.usage(*a).definition == d)).collect();
            };
            let mut found = Vec::new();
            let mut seen = HashSet::new();
            let others: Vec<ObjId> =
                pool.iter().filter(|o| !matches!(self.o(**o), ObjData::Usage(_))).copied().collect();
            let mut stack: Vec<ObjId> =
                pool.iter().filter(|o| matches!(self.o(**o), ObjData::Usage(_))).copied().rev().collect();
            // depth-first, in order
            fn inward(
                it: &mut Interp,
                s: SceneId,
                u: ObjId,
                phase: usize,
                ctx: Option<&Ctx>,
                seen: &mut HashSet<ObjId>,
                found: &mut Vec<ObjId>,
            ) {
                if !seen.insert(u) {
                    return;
                }
                found.push(u);
                for m in it.usage_members(s, u, phase, ctx) {
                    if matches!(it.o(m), ObjData::Usage(_)) {
                        inward(it, s, m, phase, ctx, seen, found);
                    }
                }
            }
            stack.reverse();
            for u in stack {
                inward(self, s, u, phase, ctx, &mut seen, &mut found);
            }
            if let (Some(first), Some(site)) = (others.first(), site) {
                if self.scenes[s].hinted.insert(sel) {
                    let l = self.label(*first);
                    self.hint_at(&site, format!("SELECT-USAGE looks inward and {l} is not a usage, so nothing is selected; to find the usages it is in, use SELECT-ANCESTOR-USAGE (10.1)"), off);
                }
            }
            let ids: HashSet<ObjId> = usages.into_iter().collect();
            return found
                .into_iter()
                .filter(|a| dfn.is_none_or(|d| self.usage(*a).definition == d) && ids.contains(a))
                .collect();
        }
        let ids: HashSet<ObjId> = pool.unwrap_or_default().into_iter().collect();
        let mut out = Vec::new();
        for a in usages {
            if Some(self.usage(a).definition) != dfn || ids.contains(&a) {
                continue;
            }
            let mut seen = HashSet::new();
            let members = self.all_members(s, a, phase, ctx, &mut seen);
            if members.iter().any(|m| ids.contains(m)) {
                out.push(a);
            }
        }
        out
    }

    pub(crate) fn usage_members(&mut self, s: SceneId, u: ObjId, phase: usize, ctx: Option<&Ctx>) -> Vec<ObjId> {
        let roles = self.usage(u).roles.clone();
        let mut out = Vec::new();
        for (_, v) in roles {
            if self.is_set(v) {
                out.extend(self.flatten(s, v, phase, ctx));
            } else {
                out.push(v);
            }
        }
        out
    }

    /// Role members of a usage, and of the usages among them, all the way in.
    fn all_members(
        &mut self,
        s: SceneId,
        u: ObjId,
        phase: usize,
        ctx: Option<&Ctx>,
        seen: &mut HashSet<ObjId>,
    ) -> Vec<ObjId> {
        if !seen.insert(u) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for m in self.usage_members(s, u, phase, ctx) {
            out.push(m);
            if matches!(self.o(m), ObjData::Usage(_)) {
                out.extend(self.all_members(s, m, phase, ctx, seen));
            }
        }
        out
    }

    /// Selections scoped by usages go to their role members, all the way in (K3.12). Returns the
    /// objects and those reached only as members, which are taken by kind (PR16).
    fn walk_usages(&mut self, pool: &[ObjId], role: Option<&Lit>, writer: UnitId) -> (Vec<ObjId>, HashSet<ObjId>) {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut direct = HashSet::new();
        let mut member = HashSet::new();
        let role_name = role.and_then(Lit::str).map(String::from);
        let unit = match role {
            Some(Lit::Str(w)) => w.unit.unwrap_or(writer),
            _ => writer,
        };
        #[allow(clippy::too_many_arguments)]
        fn walk(
            it: &mut Interp,
            o: ObjId,
            top: bool,
            role: &Option<String>,
            unit: UnitId,
            seen: &mut HashSet<ObjId>,
            out: &mut Vec<ObjId>,
            direct: &mut HashSet<ObjId>,
            member: &mut HashSet<ObjId>,
        ) {
            if let ObjData::Usage(u) = it.o(o).clone() {
                if !seen.insert(o) {
                    return;
                }
                let want = match (top, role) {
                    (true, Some(r)) => {
                        if u.role(r).is_some() {
                            Some(r.clone())
                        } else {
                            let sc = it.scoped_for(unit, u.definition);
                            Some(sc.iter().find(|(a, _)| a == r).map(|(_, o)| o.clone()).unwrap_or(r.clone()))
                        }
                    }
                    _ => None,
                };
                for (r, v) in &u.roles {
                    if top && role.is_some() && Some(r) != want.as_ref() {
                        continue;
                    }
                    let items: Vec<ObjId> =
                        if it.is_set(*v) { it.set(*v).items.iter().map(|(x, _)| *x).collect() } else { vec![*v] };
                    for m in items {
                        walk(it, m, false, role, unit, seen, out, direct, member);
                    }
                }
            } else {
                out.push(o);
                if top {
                    direct.insert(o);
                } else {
                    member.insert(o);
                }
            }
        }
        for &o in pool {
            walk(self, o, true, &role_name, unit, &mut seen, &mut out, &mut direct, &mut member);
        }
        let via: HashSet<ObjId> = member.difference(&direct).copied().collect();
        (out, via)
    }

    pub(crate) fn inst_like(&mut self, s: SceneId) -> Vec<ObjId> {
        let insts = self.scenes[s].insts.clone();
        dedup(insts.into_iter().map(|i| self.bundle_of(i)).collect())
    }

    pub(crate) fn bundle_of(&self, o: ObjId) -> ObjId {
        self.bundle_field(o).unwrap_or(o)
    }

    fn inst_match(&self, o: ObjId, p: &BTreeMap<String, Lit>, owner_only: bool) -> bool {
        let o = match self.o(o) {
            ObjData::Bundle { members, .. } => members[0],
            _ => o,
        };
        if !matches!(self.o(o), ObjData::Inst { .. }) {
            return if owner_only { !["name", "model", "type"].iter().any(|k| p.contains_key(*k)) } else { false };
        }
        let inst = self.inst_of(o);
        let cs = inst.case_sensitive;
        // name conditions take `*` and compare by the host's case rule (10.1)
        if let Some(n) = p.get("name").and_then(Lit::str) {
            if !glob(n, &inst.name, cs) {
                return false;
            }
        }
        // model: the name in the model position, a `.model` name or a subcircuit name (TR1)
        if let Some(m) = p.get("model").and_then(Lit::str) {
            match &inst.model {
                Some(im) if glob(m, im, cs) => {}
                _ => return false,
            }
        }
        if let Some(t) = p.get("type").and_then(Lit::str) {
            if inst.mtype.as_deref().unwrap_or("") != t.to_lowercase() {
                return false;
            }
        }
        true
    }

    pub(crate) fn tnet_has(&mut self, s: SceneId, t: ObjId, net: ObjId) -> bool {
        self.tnets(s, t).contains(&net)
    }

    /// The terminals that explicit wires touch, by owner: what `wired` looks among (10.2).
    pub(crate) fn wire_ends(&self, s: SceneId) -> HashMap<ObjId, Vec<ObjId>> {
        let mut ends: HashMap<ObjId, Vec<ObjId>> = HashMap::new();
        for &w in &self.scenes[s].wires {
            let ObjData::Wire { points, generated: false, .. } = self.o(w) else { continue };
            for &q in points {
                if let ObjData::Term { owner, .. } = self.o(q) {
                    ends.entry(*owner).or_default().push(q);
                }
            }
        }
        ends
    }

    /// 10.2: a terminal is wired when an explicit wire touches it (or its bus pin). `ends` is what
    /// `wire_ends` gives while the wires stay as they are.
    pub(crate) fn wired(&mut self, s: SceneId, t: ObjId, ends: &HashMap<ObjId, Vec<ObjId>>) -> bool {
        let ObjData::Term { owner, .. } = *self.o(t) else { return false };
        for &q in ends.get(&owner).into_iter().flatten() {
            if self.same_pin(s, q, t) {
                return true;
            }
        }
        false
    }

    fn same_pin(&mut self, s: SceneId, a: ObjId, b: ObjId) -> bool {
        if a == b {
            return true;
        }
        let (ObjData::Term { owner: oa, .. }, ObjData::Term { owner: ob, .. }) = (self.o(a).clone(), self.o(b).clone())
        else {
            return false;
        };
        if oa != ob {
            return false;
        }
        if !self.scenes[s].symbol.contains_key(&oa) {
            return false;
        }
        match (self.pin_for(s, oa, a), self.pin_for(s, oa, b)) {
            (Some(pa), Some(pb)) => pa == pb,
            _ => false,
        }
    }

    pub(crate) fn net_wired(&mut self, s: SceneId, n: ObjId, ends: &HashMap<ObjId, Vec<ObjId>>) -> bool {
        for ow in self.inst_like(s) {
            for t in self.terms_of(s, ow) {
                if self.tnet_has(s, t, n) && self.wired(s, t, ends) {
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn current_of(&self, s: SceneId, t: ObjId) -> String {
        let mut best: Option<(&Pos, &String)> = None;
        for (p, term, d) in &self.scenes[s].currents {
            if *term == t && best.is_none_or(|b| p > b.0) {
                best = Some((p, d));
            }
        }
        best.map(|b| b.1.clone()).unwrap_or_else(|| "unspecified".into())
    }

    pub(crate) fn has_pin(&mut self, s: SceneId, t: ObjId) -> bool {
        let ObjData::Term { owner, .. } = self.o(t).clone() else { return false };
        self.pin_for(s, owner, t).is_some()
    }

    /// Selections see only terminals with a pin; before step 5 the symbols are not known yet (8.2, 10.1).
    fn drawn(&mut self, s: SceneId, t: ObjId, phase: usize) -> bool {
        phase < 5 || self.has_pin(s, t)
    }

    /// The index of the pin of `owner`'s symbol that draws terminal `t`.
    pub(crate) fn pin_for(&mut self, s: SceneId, owner: ObjId, t: ObjId) -> Option<usize> {
        let sym = *self.scenes[s].symbol.get(&owner)?;
        let ObjData::Term { index, .. } = self.o(t).clone() else { return None };
        let terms = self.owner_terms(s, owner);
        let cs = self.host_case(owner);
        let pins = self.symbol(sym).pins.clone();
        for (k, pin) in pins.iter().enumerate() {
            for key in &pin.terms {
                match key {
                    TermKey::Num(n) if *n == index => return Some(k),
                    TermKey::Name(name) => {
                        for (idx, tn) in &terms {
                            if *idx == index {
                                if let Some(tn) = tn {
                                    let same = if cs { tn == name } else { tn.eq_ignore_ascii_case(name) };
                                    if same {
                                        return Some(k);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub(crate) fn obj_kind(&self, o: ObjId) -> Option<&'static str> {
        match self.o(o) {
            ObjData::Inst { .. } | ObjData::Bundle { .. } => Some("instance"),
            ObjData::Usage(_) => Some("usage"),
            ObjData::Wire { .. } => Some("wire"),
            ObjData::Prim { .. } => Some("annotation"),
            ObjData::Text { attach, .. } | ObjData::KeyText { attach, .. } => {
                if matches!(self.o(*attach), ObjData::Point { .. } | ObjData::Pos { .. }) {
                    Some("annotation")
                } else {
                    Some("label")
                }
            }
            _ => None,
        }
    }

    pub(crate) fn drawables(&mut self, s: SceneId, snapshot: bool) -> Vec<ObjId> {
        let mut out = self.inst_like(s);
        let sc = &self.scenes[s];
        for &a in &sc.usages {
            let u = self.usage(a);
            if !u.in_symbol && !(snapshot && u.generated) {
                out.push(a);
            }
        }
        out.extend(sc.wires.iter().copied());
        out.extend(sc.prims.iter().copied());
        for &t in &sc.texts {
            if !(snapshot && self.is_generated(t)) {
                out.push(t);
            }
        }
        out
    }

    pub(crate) fn diag_fact(&mut self, sev: crate::diag::Severity, f: &Fact, msg: String) -> Option<DiagId> {
        let c = f.ctx.clone()?;
        if sev == crate::diag::Severity::Error {
            self.note_failed_at(c.rec, f.off, false);
        }
        let loc = self.rec_loc(c.rec, f.off);
        // shown like the document's own, as the later phases' diagnostics are (7.6)
        Some(self.sink.add(
            sev,
            msg,
            loc,
            crate::diag::Extra { cause: None, origin: c.origin.to_string(), in_package: String::new() },
        ))
    }
}
