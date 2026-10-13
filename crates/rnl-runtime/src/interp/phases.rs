//! The phases after creation (spec-draft 8.2–8.5): symbol queue (4, 6), generation (5), geometry
//! (7), style queue (8). There is no solver here: positions come from PLACE and the coordinates of
//! points; constraints are checked against them, and the ones that lose in the order of 8.5 are
//! reported.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::diag::{Loc, Severity};
use crate::geom::{fmt_num, route, Orient, Pt, TOL};

use super::obj::*;
use super::scene::{Fact, FactData};
use super::Interp;

fn level(p: Priority) -> u8 {
    p as u8
}

/// The style facts of a scene by target, each list in the order of the facts (8.3).
#[derive(Default)]
struct StyleIndex {
    /// Style parameters written on the object itself, with where they were written.
    own: HashMap<ObjId, Vec<(Pos, StyleMap)>>,
    /// The indexes of the SET-STYLE facts whose style is a style object.
    styles: HashMap<ObjId, Vec<usize>>,
}

impl Interp<'_> {
    pub(crate) fn run_scene(&mut self, s: SceneId) {
        for ph in [3usize, 4] {
            self.scenes[s].phase = ph;
            self.run_tasks(s, ph);
        }
        self.resolve_bundles(s);
        self.scenes[s].phase = 4;
        self.resolve_symbols(s, false);
        self.scenes[s].phase = 5;
        self.run_tasks(s, 5);
        self.scenes[s].phase = 6;
        self.run_tasks(s, 6);
        self.resolve_symbols(s, true);
        self.note_overridden_symbols(s);
        self.symbol_labels(s);
        self.scenes[s].phase = 7;
        self.run_tasks(s, 7);
        self.resolve_geometry(s);
        self.scenes[s].phase = 8;
        self.run_tasks(s, 8);
        self.resolve_styles(s);
        self.resolve_layers(s);
    }

    fn run_tasks(&mut self, s: SceneId, ph: usize) {
        let mut tasks = std::mem::take(&mut self.scenes[s].tasks[ph]);
        tasks.sort_by(|a, b| a.0.cmp(&b.0));
        for (_, t) in tasks {
            t(self, s, ph);
        }
    }

    fn site_loc(&self, site: &Option<std::rc::Rc<super::ctx::Ctx>>, off: usize) -> (Option<Loc>, String, String) {
        match site {
            Some(c) => (self.rec_loc(c.rec, off), c.origin.to_string(), self.ctx_pkg(c)),
            None => (None, String::new(), String::new()),
        }
    }

    /// A diagnostic of the later phases. It concerns what the statement does in this document, even
    /// when the statement is written in a package, so it is shown like the document's own (7.6).
    fn diag_site(&mut self, sev: Severity, site: &Option<std::rc::Rc<super::ctx::Ctx>>, off: usize, msg: String) {
        if sev == Severity::Error {
            if let Some(c) = site {
                self.note_failed_at(c.rec, off, false);
            }
        }
        let (loc, origin, _pkg) = self.site_loc(site, off);
        self.sink.add(sev, msg, loc, crate::diag::Extra { cause: None, origin, in_package: String::new() });
    }

    // ---- bundles (11.8)

    fn resolve_bundles(&mut self, s: SceneId) {
        let mut bundles = self.scenes[s].bundles.clone();
        bundles.sort_by(|a, b| self.scenes[s].created[a].cmp(&self.scenes[s].created[b]));
        let mut owner: HashMap<ObjId, ObjId> = HashMap::new();
        for &b in &bundles {
            let ObjData::Bundle { members, site, .. } = self.o(b).clone() else { continue };
            for m in members {
                if let Some(prev) = owner.get(&m) {
                    if *prev != b {
                        let l = self.label(m);
                        self.diag_site(
                            Severity::Hint,
                            &site,
                            0,
                            format!("{l} is written into several bundles and goes to the later one (11.8)"),
                        );
                    }
                }
                owner.insert(m, b);
            }
        }
        for b in self.scenes[s].bundles.clone() {
            let ObjData::Bundle { members, site, .. } = self.o(b).clone() else { continue };
            let kept: Vec<ObjId> = members.into_iter().filter(|m| owner.get(m) == Some(&b)).collect();
            let empty = kept.is_empty();
            if empty {
                self.diag_site(
                    Severity::Warning,
                    &site,
                    0,
                    "every member of the bundle was taken by later bundles; the empty bundle is not drawn (11.8)"
                        .into(),
                );
            }
            for &m in &kept {
                if let ObjData::Inst { bundle, .. } = self.om(m) {
                    *bundle = Some(b);
                }
            }
            if let Some(&first) = kept.first() {
                let f = self.inst_of(first).clone();
                for &m in &kept[1..] {
                    let i = self.inst_of(m).clone();
                    let same = i.letter == f.letter
                        && i.model.as_ref().map(|x| x.to_lowercase()) == f.model.as_ref().map(|x| x.to_lowercase())
                        && i.nodes.len() == f.nodes.len()
                        && i.params == f.params;
                    if !same {
                        self.diag_site(
                            Severity::Warning,
                            &site,
                            0,
                            format!("the bundle member {} is not the same kind as {} (11.8)", i.name, f.name),
                        );
                    }
                }
            }
            if let ObjData::Bundle { members: ms, empty: e, .. } = self.om(b) {
                *ms = kept;
                *e = empty;
            }
        }
    }

    pub(crate) fn bundle_empty(&self, b: ObjId) -> bool {
        matches!(self.o(b), ObjData::Bundle { empty: true, .. })
    }

    // ---- symbol queue (8.3)

    fn resolve_symbols(&mut self, s: SceneId, generated: bool) {
        let mut objs = Vec::new();
        if !generated {
            for o in self.inst_like(s) {
                if !self.bundle_empty(o) {
                    objs.push(o);
                }
            }
            for &a in &self.scenes[s].usages {
                let u = self.usage(a);
                if !u.generated && !u.in_symbol {
                    objs.push(a);
                }
            }
        } else {
            for &a in &self.scenes[s].usages {
                let u = self.usage(a);
                if u.generated && !u.in_symbol {
                    objs.push(a);
                }
            }
        }
        let appear = self.scenes[s].appear.clone();
        // the SET-SYMBOL facts by target, each list in the order of `appear`
        let mut by_target: HashMap<ObjId, Vec<usize>> = HashMap::new();
        for (k, f) in appear.iter().enumerate() {
            if let FactData::Symbol(_) = f.data {
                by_target.entry(f.target).or_default().push(k);
            }
        }
        for o in objs {
            // each candidate with the index of its fact in `appear`; a block symbol has none
            let mut direct: Vec<(Pos, ObjId, Option<usize>)> = Vec::new();
            let mut other: Vec<(Pos, ObjId, Option<usize>)> = Vec::new();
            let (base, members) = match self.o(o) {
                ObjData::Bundle { members, .. } => (members[0], members.clone()),
                _ => (o, Vec::new()),
            };
            if let ObjData::Inst { .. } = self.o(base) {
                let inst = self.inst_of(base).clone();
                if inst.is_subckt {
                    if let Some(m) = &inst.model {
                        let key = self.doc.key(m);
                        if let Some(blocks) = self.block_symbols.get(&key) {
                            for (p, sym) in blocks.values() {
                                let mut pp = vec![PosElem::Int(1)];
                                pp.extend(p.iter().cloned());
                                other.push((pp, *sym, None));
                            }
                        }
                    }
                }
            }
            for &k in by_target.get(&o).into_iter().flatten() {
                let f = &appear[k];
                let FactData::Symbol(sym) = f.data else { continue };
                self.scenes[s].symbol_weighed.insert(k);
                if f.direct {
                    direct.push((f.pos.clone(), sym, Some(k)));
                } else {
                    other.push((f.pos.clone(), sym, Some(k)));
                }
            }
            // a symbol given to a member of a bundle fails, in the order of the facts (11.8)
            let mut ks: Vec<usize> = members.iter().filter_map(|m| by_target.get(m)).flatten().copied().collect();
            ks.sort_unstable();
            ks.dedup();
            for k in ks {
                self.member_draw_error(s, 'a', k, &appear[k], appear[k].target);
            }
            let pick = if !direct.is_empty() {
                direct.into_iter().max_by(|a, b| a.0.cmp(&b.0))
            } else {
                other.into_iter().max_by(|a, b| a.0.cmp(&b.0))
            };
            if let Some((_, sym, fact)) = pick {
                if let Some(k) = fact {
                    self.scenes[s].symbol_won.insert(k);
                }
                self.set_symbol(s, o, sym);
            } else if matches!(self.o(o), ObjData::Inst { .. } | ObjData::Bundle { .. }) {
                let d = self.default_symbol(s, base);
                self.set_symbol(s, o, d);
                self.scenes[s].default_symbol.insert(o);
            }
        }
    }

    /// A statement whose symbols lose in the queue of every object it gives one to takes no
    /// effect: it is overridden (7.5, 8.3).
    fn note_overridden_symbols(&mut self, s: SceneId) {
        let mut won: HashMap<(Option<RecId>, usize), bool> = HashMap::new();
        for (k, f) in self.scenes[s].appear.iter().enumerate() {
            if !matches!(f.data, FactData::Symbol(_)) || !self.scenes[s].symbol_weighed.contains(&k) {
                continue;
            }
            let Some(c) = &f.ctx else { continue };
            *won.entry((c.rec, f.off)).or_insert(false) |= self.scenes[s].symbol_won.contains(&k);
        }
        for ((rec, off), w) in won {
            if !w {
                self.note_failed_at(rec, off, true);
            }
        }
    }

    fn set_symbol(&mut self, s: SceneId, o: ObjId, sym: ObjId) {
        if !self.scenes[s].symbol.contains_key(&o) {
            self.symbol_order.entry(s).or_default().push(o);
        }
        self.scenes[s].symbol.insert(o, sym);
    }

    pub(crate) fn member_draw_error(&mut self, s: SceneId, list: char, k: usize, f: &Fact, member: ObjId) {
        if f.ctx.is_none() || !self.scenes[s].failed_facts.insert((list as u8, k)) {
            return;
        }
        let l = self.label(member);
        self.diag_fact(
            Severity::Error,
            f,
            format!("{l} is in a bundle and is not drawn on its own: this description fails (11.8)"),
        );
    }

    /// The renderer's own default for an instance no symbol is given to (9.2.6, F18; principle 2.7).
    fn default_symbol(&mut self, s: SceneId, inst: ObjId) -> ObjId {
        let terms = self.owner_terms(s, inst);
        let n = terms.len();
        let left: Vec<_> = terms[..n.div_ceil(2)].to_vec();
        let right: Vec<_> = terms[n.div_ceil(2)..].to_vec();
        let h = (left.len().max(right.len()).max(1) * 2) as f64;
        let mut sym = SymbolDef { default: true, ..SymbolDef::default() };
        let mut dashed = StyleMap::new();
        dashed.insert("linestyle".into(), Lit::Str(WStr::new("dashed", None)));
        sym.prims.push(SymPrim {
            shape: Shape::Rectangle,
            points: vec![(-2.0, -h / 2.0), (2.0, h / 2.0)],
            params: PrimParams::default(),
            style: dashed,
            text: None,
        });
        for (side, x0, x1, dir) in [(&left, -3.0, -2.0, 180u16), (&right, 3.0, 2.0, 0u16)] {
            for (k, (idx, name)) in side.iter().enumerate() {
                let y = h / 2.0 - 1.0 - 2.0 * k as f64;
                sym.pins.push(Pin {
                    terms: vec![TermKey::Num(*idx)],
                    pos: (x0, y),
                    label: name.clone(),
                    dir: Some(dir),
                });
                sym.prims.push(SymPrim {
                    shape: Shape::Line,
                    points: vec![(x0, y), (x1, y)],
                    params: PrimParams::default(),
                    style: StyleMap::new(),
                    text: None,
                });
            }
        }
        sym.labels.push(SymLabel {
            key: Some("name".into()),
            items: TextItems {
                text: Some(WStr::new("${name}", None)),
                halign: Some("middle".into()),
                valign: Some("bottom".into()),
                ..TextItems::default()
            },
            position: Some((0.0, h / 2.0 + 0.3)),
            style: StyleMap::new(),
            unit: 0,
            site: None,
            off: 0,
        });
        let id = self.new_obj(ObjData::Symbol(Box::new(sym)));
        self.objs[id.0 as usize].name = "(viewer default)".into();
        id
    }

    /// Texts written in a symbol under a key are sub-objects of what the symbol draws (13.3).
    fn symbol_labels(&mut self, s: SceneId) {
        let order = self.symbol_order.get(&s).cloned().unwrap_or_default();
        for owner in order {
            let sym = self.scenes[s].symbol[&owner];
            let labels = self.symbol(sym).labels.clone();
            for lab in labels {
                if let Some(key) = &lab.key {
                    if matches!(self.o(owner), ObjData::Usage(_)) && lab.site.is_some() {
                        // a key written in the symbol may not be spelled like a role or local name of the usage
                        // it is drawn on, or an alias of one; the text is left out (6.5, 13.3)
                        if let Some(clash) = self.key_clash(key, lab.unit, Some(owner)) {
                            let l = self.label(owner);
                            self.diag_site(
                                Severity::Error,
                                &lab.site,
                                lab.off,
                                format!("{clash}; a text written in the symbol, drawn on {l}"),
                            );
                            continue;
                        }
                    }
                    let kt = self.scene_keytext(s, owner, key);
                    if let ObjData::KeyText { layers, sym_pos, from_symbol, .. } = self.om(kt) {
                        layers.insert(0, (pos(&[-1]), lab.items.clone(), lab.style.clone()));
                        *sym_pos = lab.position;
                        *from_symbol = true;
                    }
                    if self.scenes[s].listed_keytexts.insert(kt) {
                        self.scenes[s].texts.push(kt);
                        let c = self.scenes[s].created.get(&owner).cloned().unwrap_or_else(|| pos(&[0]));
                        self.scenes[s].created.insert(kt, c);
                    }
                } else {
                    let t = self.new_obj(ObjData::Text {
                        attach: owner,
                        items: lab.items.clone(),
                        style: lab.style.clone(),
                        from_symbol: true,
                        sym_pos: lab.position,
                        generated: false,
                        failed: false,
                        site: None,
                        off: 0,
                    });
                    self.scenes[s].texts.push(t);
                    let c = self.scenes[s].created.get(&owner).cloned().unwrap_or_else(|| pos(&[0]));
                    self.scenes[s].created.insert(t, c);
                }
            }
        }
    }

    // ---- priorities (14.8)

    /// From low to high: header default < own parameter < PRIORITY statements < the priority of the
    /// enclosing usages (outermost that has one) < PRIORITY written on the local from outside.
    pub(crate) fn eff_priority(
        &self,
        s: SceneId,
        priority: Option<Priority>,
        chain: &[ObjId],
        own: Option<ObjId>,
    ) -> Priority {
        let sc = &self.scenes[s];
        let mut p = priority;
        let mut inner: Vec<&Fact> = Vec::new();
        let mut outer: Vec<&Fact> = Vec::new();
        if let Some(own) = own {
            for g in &sc.prioritys {
                if g.target == own {
                    if self.reaches_from_outside(g, chain) {
                        outer.push(g);
                    } else {
                        inner.push(g);
                    }
                }
            }
        }
        let best = |v: &[&Fact]| -> Option<Priority> {
            v.iter().max_by(|a, b| a.pos.cmp(&b.pos)).and_then(|g| match g.data {
                FactData::Priority(p) => Some(p),
                _ => None,
            })
        };
        if !inner.is_empty() {
            p = best(&inner);
        }
        for &a in chain {
            let mods: Vec<&Fact> = sc.prioritys.iter().filter(|g| g.target == a).collect();
            if !mods.is_empty() {
                p = best(&mods);
                break;
            }
            if let Some(ap) = self.usage(a).priority {
                p = Some(ap);
                break;
            }
        }
        if !outer.is_empty() {
            p = best(&outer);
        }
        p.unwrap_or(Priority::Normal)
    }

    /// A PRIORITY reaches a definition's local from outside when it is not written in that usage's body.
    fn reaches_from_outside(&self, g: &Fact, chain: &[ObjId]) -> bool {
        let Some(last) = chain.last() else { return false };
        let mut inside = g.ctx.as_ref().and_then(|c| c.frame_usage());
        while let Some(u) = inside {
            if u == *last {
                return false;
            }
            inside = self.usage(u).parent;
        }
        true
    }

    fn fact_priority(&self, s: SceneId, f: &Fact) -> Priority {
        self.eff_priority(s, f.priority, &f.chain, None)
    }

    // ---- geometry (8.5, 11.x, 12.1, 14.x)

    fn resolve_geometry(&mut self, s: SceneId) {
        // 1. check what PLACE, ORIENT and constraints reach (7.1, 12.1, 11.8)
        let at = self.scenes[s].at.clone();
        let mut live_at = Vec::new();
        for (k, f) in at.iter().enumerate() {
            if self.geom_target_ok(s, 'p', k, f, "PLACE") {
                live_at.push(k);
            }
        }
        let orient = self.scenes[s].orient.clone();
        let mut live_or = Vec::new();
        for (k, f) in orient.iter().enumerate() {
            if self.geom_target_ok(s, 'o', k, f, "ORIENT") {
                live_or.push(k);
            }
        }
        // 2. coordinates: per object and axis the fact with the highest priority (8.5)
        let mut best: HashMap<(ObjId, char), (Priority, Pos, usize)> = HashMap::new();
        let mut best_order: Vec<(ObjId, char)> = Vec::new();
        for k in live_at {
            let f = &at[k];
            let p = self.fact_priority(s, f);
            if p == Priority::Disabled {
                continue;
            }
            let FactData::Coord(axis, v) = f.data else { continue };
            let key = (f.target, axis);
            match best.get(&key).cloned() {
                None => {
                    best.insert(key, (p, f.pos.clone(), k));
                    best_order.push(key);
                }
                Some(cur) => {
                    let FactData::Coord(_, cv) = at[cur.2].data else { continue };
                    if (level(p), &f.pos) > (level(cur.0), &cur.1) {
                        if (cv - v).abs() > TOL {
                            self.drop_fact(s, &at[cur.2], f);
                        }
                        best.insert(key, (p, f.pos.clone(), k));
                    } else if (cv - v).abs() > TOL {
                        self.drop_fact(s, f, &at[cur.2]);
                    }
                }
            }
        }
        let mut targets: Vec<(ObjId, [Option<f64>; 2])> = Vec::new();
        let mut target_at: HashMap<ObjId, usize> = HashMap::new();
        for key in &best_order {
            let (_, _, k) = best[key];
            let FactData::Coord(axis, v) = at[k].data else { continue };
            let i = if axis == 'x' { 0 } else { 1 };
            match target_at.get(&key.0) {
                Some(&t) => targets[t].1[i] = Some(v),
                None => {
                    let mut xy = [None, None];
                    xy[i] = Some(v);
                    target_at.insert(key.0, targets.len());
                    targets.push((key.0, xy));
                }
            }
        }
        for (o, xy) in &targets {
            if matches!(self.o(*o), ObjData::Pos { .. }) {
                continue;
            }
            self.scenes[s].pos.insert(*o, *xy);
        }
        self.scenes[s].at_best = best.clone();
        // orientation
        let mut ob: HashMap<ObjId, (Priority, Pos, usize)> = HashMap::new();
        for k in live_or {
            let f = &orient[k];
            let p = self.fact_priority(s, f);
            if p == Priority::Disabled {
                continue;
            }
            let FactData::Orient(code) = f.data else { continue };
            match ob.get(&f.target).cloned() {
                None => {
                    ob.insert(f.target, (p, f.pos.clone(), k));
                }
                Some(cur) => {
                    let FactData::Orient(cc) = orient[cur.2].data else { continue };
                    if (level(p), &f.pos) > (level(cur.0), &cur.1) {
                        if cc != code {
                            self.drop_fact(s, &orient[cur.2], f);
                        }
                        ob.insert(f.target, (p, f.pos.clone(), k));
                    } else if cc != code {
                        self.drop_fact(s, f, &orient[cur.2]);
                    }
                }
            }
        }
        for (o, (_, _, k)) in &ob {
            if let FactData::Orient(c) = orient[*k].data {
                self.scenes[s].ori.insert(*o, c);
            }
        }
        self.scenes[s].ori_best = ob;
        // `X::position` of an object is the object itself (6.5)
        for key in &best_order {
            let (_, _, k) = best[key];
            let obj = at[k].target;
            if let ObjData::Pos { parent } = self.o(obj).clone() {
                if !self.is_set(parent) {
                    let FactData::Coord(axis, v) = at[k].data else { continue };
                    let e = self.scenes[s].pos.entry(parent).or_insert([None, None]);
                    e[if axis == 'x' { 0 } else { 1 }] = Some(v);
                }
            }
        }
        // 3. usages drawn on their main object (12.1 rule 5)
        for a in self.scenes[s].usages.clone() {
            if self.usage(a).in_symbol {
                continue;
            }
            if self.at_point(a) {
                if let Some(m) = self.main_of(a) {
                    if let Some(pt) = self.point_of(s, m) {
                        self.scenes[s].pos.insert(a, [Some(pt.0), Some(pt.1)]);
                    }
                }
            }
        }
        // 4. check constraints against the positions (no solver)
        self.check_constraints(s);
        // 5. failures that need symbols: wires to pinless terminals, labels on symbol-less usages
        for w in self.scenes[s].wires.clone() {
            let ObjData::Wire { nets, points, generated, site, off, .. } = self.o(w).clone() else { continue };
            if site.is_none() || generated {
                continue;
            }
            for q in &points {
                if !matches!(self.o(*q), ObjData::Term { .. }) {
                    continue;
                }
                let tn = self.term_nets(s, *q);
                if !tn.is_empty() && !tn.iter().any(|n| nets.contains(n)) {
                    let wn: Vec<String> = nets.iter().map(|n| self.net_display(*n)).collect();
                    let names: Vec<String> = tn.iter().map(|n| self.net_display(*n)).collect();
                    let ql = self.label(*q);
                    self.diag_site(
                        Severity::Warning,
                        &site,
                        off,
                        format!(
                            "the wire is marked on net {}, but the terminal {ql} it reaches is on {} (13.1)",
                            wn.join(","),
                            names.join(",")
                        ),
                    );
                }
            }
        }
        for w in self.scenes[s].wires.clone() {
            let ObjData::Wire { points, site, off, .. } = self.o(w).clone() else { continue };
            for q in &points {
                let ObjData::Term { owner, .. } = self.o(*q).clone() else { continue };
                if !self.term_drawable(s, *q, &site, off, "wire") {
                    self.set_failed(w);
                    break;
                }
                if self.bundle_field(owner).is_some() {
                    let l = self.label(owner);
                    self.diag_site(
                        Severity::Error,
                        &site,
                        off,
                        format!("{l} is in a bundle; wires go to the terminals of the bundle (11.8)"),
                    );
                    self.set_failed(w);
                    break;
                }
            }
        }
        for t in self.scenes[s].texts.clone() {
            let (att, from_symbol, site, off) = match self.o(t) {
                ObjData::Text { attach, from_symbol, site, off, .. } => (*attach, *from_symbol, site.clone(), *off),
                ObjData::KeyText { attach, from_symbol, site, off, .. } => (*attach, *from_symbol, site.clone(), *off),
                _ => continue,
            };
            let mut failed = false;
            match self.o(att).clone() {
                ObjData::Usage(_) if !self.scenes[s].symbol.contains_key(&att) && !from_symbol => {
                    if site.is_some() {
                        let l = self.label(att);
                        self.diag_site(
                            Severity::Error,
                            &site,
                            off,
                            format!("{l} has no symbol and no drawing meaning: this TEXT fails (12.1, 13.3)"),
                        );
                    }
                    failed = true;
                }
                ObjData::Term { .. } if !self.has_pin(s, att) => {
                    if site.is_some() {
                        let l = self.label(att);
                        self.diag_site(
                            Severity::Error,
                            &site,
                            off,
                            format!("{l} has no pin in its symbol: this TEXT fails (9.2.4, 13.3)"),
                        );
                    }
                    failed = true;
                }
                ObjData::Inst { bundle: Some(_), .. } if site.is_some() => {
                    let l = self.label(att);
                    self.diag_site(
                        Severity::Error,
                        &site,
                        off,
                        format!("{l} is in a bundle and is not drawn on its own: this TEXT fails (11.8)"),
                    );
                    failed = true;
                }
                _ => {}
            }
            if let ObjData::KeyText { layers, .. } = self.o(t) {
                if layers.is_empty() {
                    failed = true;
                }
            }
            if failed {
                self.set_failed(t);
            } else {
                self.check_attrs(s, t, &site, off);
            }
            if let ObjData::KeyText { layers, .. } = self.o(t).clone() {
                if !layers.is_empty() && !layers.iter().any(|(_, it, _)| it.text.is_some()) {
                    if site.is_some() {
                        let l = self.label(t);
                        self.diag_site(
                            Severity::Error,
                            &site,
                            off,
                            format!(
                                "{l} has no content: the symbol has no such key and the scene wrote no text= (13.3)"
                            ),
                        );
                    }
                    self.set_failed(t);
                }
            }
        }
        // 6. unplaced drawn objects
        let mut un = Vec::new();
        for o in self.inst_like(s) {
            if self.bundle_empty(o) {
                continue;
            }
            if self.xy(s, o).is_none() {
                un.push(self.label(o));
            }
        }
        for a in self.scenes[s].usages.clone() {
            if self.usage(a).in_symbol || !self.scenes[s].symbol.contains_key(&a) {
                continue;
            }
            if self.xy(s, a).is_none() {
                un.push(self.label(a));
            }
        }
        if !un.is_empty() {
            let lv = &self.doc.levels[self.scenes[s].level];
            let title = lv.title();
            let loc = Loc::line(lv.file.clone(), lv.line.max(1));
            let shown: Vec<&String> = un.iter().take(12).collect();
            let more = if un.len() > 12 { ", …" } else { "" };
            self.sink.hint(
                format!(
                    "{title}: {} objects have no position and are left to a solver; they are not drawn: {}{more}",
                    un.len(),
                    shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ),
                Some(loc),
            );
            self.scenes[s].unplaced = un;
        }
    }

    pub(crate) fn set_failed(&mut self, o: ObjId) {
        match self.om(o) {
            ObjData::Wire { failed, .. } | ObjData::Text { failed, .. } | ObjData::KeyText { failed, .. } => {
                *failed = true
            }
            ObjData::Constraint(c) => c.failed = true,
            _ => {}
        }
    }

    pub(crate) fn is_obj_failed(&self, o: ObjId) -> bool {
        match self.o(o) {
            ObjData::Wire { failed, .. } | ObjData::Text { failed, .. } | ObjData::KeyText { failed, .. } => *failed,
            ObjData::Constraint(c) => c.failed,
            _ => false,
        }
    }

    pub(crate) fn net_display(&self, n: ObjId) -> String {
        match self.o(n) {
            ObjData::Net { display, .. } => display.clone(),
            _ => self.label(n),
        }
    }

    /// 13.6: a `${…}` path that reaches nothing, or ends on something that cannot be shown, fails the text.
    fn check_attrs(&mut self, s: SceneId, t: ObjId, site: &Option<std::rc::Rc<super::ctx::Ctx>>, off: usize) {
        let items = self.text_items(t);
        let Some(text) = items.text else { return };
        let unit = text.unit;
        let att = self.text_attach(t);
        let mut bad = Vec::new();
        for path in crate::markup::attr_refs(text.as_str()) {
            if let Err(e) = self.eval_path(s, att, &path, unit) {
                bad.push(format!("`${{{path}}}`: {e}"));
            }
        }
        if bad.is_empty() {
            return;
        }
        self.set_failed(t);
        let msg = format!("{}: this text fails (13.6)", bad.join("; "));
        if site.is_some() {
            self.diag_site(Severity::Error, site, off, msg);
        } else {
            let sym = self.scenes[s].symbol.get(&att).map(|x| self.label(*x)).unwrap_or_else(|| "?".into());
            let lv = &self.doc.levels[self.scenes[s].level];
            let loc = Loc::line(lv.file.clone(), lv.line.max(1));
            self.sink.error(format!("{msg}, the text is written in the symbol {sym}"), Some(loc));
        }
    }

    fn drop_fact(&mut self, s: SceneId, loser: &Fact, winner: &Fact) {
        if self.fact_priority(s, loser) == Priority::Low || loser.ctx.is_none() {
            return;
        }
        self.scenes[s].dropped += 1;
        if let Some(c) = &loser.ctx {
            self.note_failed_at(c.rec, loser.off, true);
        }
        let a = self.fact_text(loser);
        let b = self.fact_text(winner);
        self.diag_fact(
            Severity::Hint,
            loser,
            format!("{a} conflicts with {b} and is dropped in the order of 8.5 (8.5)"),
        );
    }

    fn fact_text(&self, f: &Fact) -> String {
        match &f.data {
            FactData::Coord(axis, v) => format!("the {axis} of {} = {}", self.label(f.target), fmt_num(*v)),
            FactData::Orient(c) => format!("the orientation {} of {}", c.as_str(), self.label(f.target)),
            _ => self.label(f.target),
        }
    }

    fn geom_target_ok(&mut self, s: SceneId, list: char, k: usize, f: &Fact, what: &str) -> bool {
        if f.ctx.is_none() {
            return true;
        }
        let obj = f.target;
        match self.o(obj).clone() {
            ObjData::Inst { bundle: Some(_), .. } => {
                self.member_draw_error(s, list, k, f, obj);
                return false;
            }
            ObjData::Usage(_) => {
                if !self.scenes[s].symbol.contains_key(&obj) {
                    let l = self.label(obj);
                    self.diag_fact(
                        Severity::Error,
                        f,
                        format!("{l} has no symbol and no drawing meaning: {what} fails (12.1)"),
                    );
                    return false;
                }
                if what == "PLACE" && self.at_point(obj) {
                    let l = self.label(obj);
                    self.diag_fact(Severity::Error, f, format!("{l} is drawn on the point of its main object and has no position of its own: PLACE fails (12.1)"));
                    return false;
                }
            }
            ObjData::Pos { parent } if matches!(self.o(parent), ObjData::Usage(_)) && self.at_point(parent) => {
                let l = self.label(parent);
                self.diag_fact(
                    Severity::Error,
                    f,
                    format!("{l} is drawn on the point of its main object: PLACE fails (12.1)"),
                );
                return false;
            }
            _ => {}
        }
        let place_ok = matches!(
            self.o(obj),
            ObjData::Inst { .. }
                | ObjData::Usage(_)
                | ObjData::Bundle { .. }
                | ObjData::Point { .. }
                | ObjData::Text { .. }
                | ObjData::KeyText { .. }
                | ObjData::Pos { .. }
        );
        if what == "PLACE" && !place_ok {
            let l = self.label(obj);
            self.diag_fact(Severity::Error, f, format!("PLACE does not take {l} (11.1)"));
            return false;
        }
        if what == "ORIENT" && !matches!(self.o(obj), ObjData::Inst { .. } | ObjData::Usage(_) | ObjData::Bundle { .. })
        {
            let l = self.label(obj);
            self.diag_fact(Severity::Error, f, format!("ORIENT does not take {l} (11.2)"));
            return false;
        }
        true
    }

    pub(crate) fn main_of(&self, a: ObjId) -> Option<ObjId> {
        let u = self.usage(a);
        let d = self.def(u.definition);
        let first = d.roles.first()?;
        u.role(&first.name)
    }

    pub(crate) fn at_point(&self, a: ObjId) -> bool {
        match self.main_of(a) {
            Some(m) => matches!(
                self.o(m),
                ObjData::Term { .. }
                    | ObjData::Inst { .. }
                    | ObjData::Point { .. }
                    | ObjData::Pos { .. }
                    | ObjData::Bundle { .. }
            ),
            None => false,
        }
    }

    fn term_drawable(
        &mut self,
        s: SceneId,
        t: ObjId,
        site: &Option<std::rc::Rc<super::ctx::Ctx>>,
        off: usize,
        what: &str,
    ) -> bool {
        let ObjData::Term { owner, .. } = self.o(t).clone() else { return true };
        if matches!(self.o(owner), ObjData::Usage(_)) && !self.scenes[s].symbol.contains_key(&owner) {
            let l = self.label(owner);
            self.diag_site(
                Severity::Error,
                site,
                off,
                format!("{l} has no symbol: a {what} to its terminal fails (12.1)"),
            );
            return false;
        }
        if !self.has_pin(s, t) {
            let l = self.label(t);
            self.diag_site(
                Severity::Error,
                site,
                off,
                format!("{l} has no pin in its symbol: the {what} fails (9.2.4)"),
            );
            return false;
        }
        true
    }

    // positions of things

    pub(crate) fn orientation(&self, s: SceneId, o: ObjId) -> Orient {
        self.scenes[s].ori.get(&o).copied().unwrap_or_default()
    }

    pub(crate) fn xy(&self, s: SceneId, o: ObjId) -> Option<Pt> {
        match self.scenes[s].pos.get(&o) {
            Some([Some(x), Some(y)]) => Some((*x, *y)),
            _ => None,
        }
    }

    /// The coordinate point of a locatable object (14.1), or None when it is free.
    pub(crate) fn point_of(&mut self, s: SceneId, o: ObjId) -> Option<Pt> {
        match self.o(o).clone() {
            ObjData::Point { .. } => self.xy(s, o),
            ObjData::Pos { parent } => {
                if self.is_set(parent) {
                    self.set_center(s, parent)
                } else {
                    self.point_of(s, parent)
                }
            }
            ObjData::Inst { bundle: Some(b), .. } => self.point_of(s, b),
            ObjData::Inst { .. } | ObjData::Bundle { .. } | ObjData::Usage(_) => self.xy(s, o),
            ObjData::Term { owner, index, name } => {
                let mut owner = owner;
                let mut t = o;
                if let Some(b) = self.bundle_field(owner) {
                    owner = b;
                    t = self.term_obj(s, owner, index, name);
                }
                let pin = self.pin_for(s, owner, t)?;
                let base = self.xy(s, owner)?;
                let sym = self.scenes[s].symbol[&owner];
                let pp = self.symbol(sym).pins[pin].pos;
                let d = self.orientation(s, owner).apply(pp);
                Some((base.0 + d.0, base.1 + d.1))
            }
            ObjData::Text { .. } | ObjData::KeyText { .. } => self.text_anchor(s, o),
            _ => None,
        }
    }

    pub(crate) fn set_center(&mut self, s: SceneId, set: ObjId) -> Option<Pt> {
        let members = self.flatten(s, set, 7, None);
        if members.is_empty() {
            return None;
        }
        let mut pts = Vec::new();
        for m in members {
            pts.push(self.point_of(s, m)?);
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for (x, y) in pts {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        Some(((x0 + x1) / 2.0, (y0 + y1) / 2.0))
    }

    /// Where the text's `dx` or `dy` was last written (13.3).
    fn offset_pos(&self, s: SceneId, t: ObjId, dx: bool) -> Pos {
        let mut p = self.scenes[s].created.get(&t).cloned().unwrap_or_else(|| pos(&[0]));
        if let ObjData::KeyText { layers, .. } = self.o(t) {
            let mut ls: Vec<&(Pos, TextItems, StyleMap)> = layers.iter().collect();
            ls.sort_by(|a, b| a.0.cmp(&b.0));
            for (lp, it, _) in ls {
                if (dx && it.dx.is_some()) || (!dx && it.dy.is_some()) {
                    p = lp.clone();
                }
            }
        }
        p
    }

    pub(crate) fn text_sym_pos(&self, t: ObjId) -> Option<(f64, f64)> {
        match self.o(t) {
            ObjData::Text { sym_pos, .. } | ObjData::KeyText { sym_pos, .. } => *sym_pos,
            _ => None,
        }
    }

    pub(crate) fn text_anchor(&mut self, s: SceneId, t: ObjId) -> Option<Pt> {
        let att = self.text_attach(t);
        let items = self.text_items(t);
        let base = match self.o(att) {
            ObjData::Point { .. }
            | ObjData::Pos { .. }
            | ObjData::Term { .. }
            | ObjData::Inst { .. }
            | ObjData::Usage(_)
            | ObjData::Bundle { .. } => self.point_of(s, att),
            _ => None,
        };
        let sym_pos = self.text_sym_pos(t);
        if let Some(b) = base {
            if (items.dx.is_some() || items.dy.is_some()) && sym_pos.is_none() {
                // one axis is either an absolute position (PLACE) or an offset (dx, dy): whichever was written
                // later replaces the other, whatever the priority (13.3)
                let cur = self.scenes[s].pos.get(&t).copied().unwrap_or([None, None]);
                let mut xy = cur;
                for (k, axis, item) in [(0usize, 'x', items.dx), (1, 'y', items.dy)] {
                    let Some(d) = item else { continue };
                    let sc = &self.scenes[s];
                    let ats: Vec<Pos> = sc
                        .at_by_target
                        .get(&t)
                        .into_iter()
                        .flatten()
                        .map(|&j| &sc.at[j])
                        .filter(|f| {
                            matches!(f.data, FactData::Coord(a, _) if a == axis)
                                && self.fact_priority(s, f) != Priority::Disabled
                        })
                        .map(|f| f.pos.clone())
                        .collect();
                    let off = self.offset_pos(s, t, k == 0);
                    if ats.is_empty() || off > *ats.iter().max().unwrap() {
                        xy[k] = Some(if k == 0 { b.0 } else { b.1 } + d);
                    }
                }
                if let [Some(x), Some(y)] = xy {
                    return Some((x, y));
                }
            }
        }
        if let Some(p) = self.xy(s, t) {
            return Some(p);
        }
        if let (Some(sp), Some(b)) = (sym_pos, base) {
            if matches!(self.o(att), ObjData::Inst { .. } | ObjData::Usage(_) | ObjData::Bundle { .. }) {
                let d = self.orientation(s, att).apply(sp);
                return Some((b.0 + d.0, b.1 + d.1));
            }
        }
        if matches!(self.o(att), ObjData::Point { .. } | ObjData::Pos { .. }) {
            return base;
        }
        if let Some(b) = base {
            return Some((b.0 + 1.2, b.1 + 1.2)); // this implementation's own default place
        }
        match self.o(att).clone() {
            ObjData::Net { .. } => {
                // the first wire on the net that has its points
                let ws = self.scenes[s].wires_by_net.get(&att).cloned().unwrap_or_default();
                for w in ws {
                    if !matches!(self.o(w), ObjData::Wire { failed: false, .. }) {
                        continue;
                    }
                    if let Some(pts) = self.wire_points(s, w) {
                        return pts.first().copied();
                    }
                }
                None
            }
            ObjData::Wire { .. } => {
                let pts = self.wire_points(s, att)?;
                let k = pts.len() / 2;
                let (a, b) = (if k > 0 { pts[k - 1] } else { pts[0] }, pts[k]);
                Some(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0 + 0.6))
            }
            _ => None,
        }
    }

    pub(crate) fn text_items(&self, t: ObjId) -> TextItems {
        match self.o(t) {
            ObjData::KeyText { layers, .. } => {
                let mut ls: Vec<&(Pos, TextItems, StyleMap)> = layers.iter().collect();
                ls.sort_by(|a, b| a.0.cmp(&b.0));
                let mut items = TextItems::default();
                for (_, it, _) in ls {
                    items.update(it);
                }
                items
            }
            ObjData::Text { items, .. } => items.clone(),
            _ => TextItems::default(),
        }
    }

    /// The pins a ROUTE still has to connect, as segments of a minimum spanning tree (the viewer's choice).
    pub(crate) fn flywires(&mut self, s: SceneId, r: ObjId) -> Vec<(Pt, Pt)> {
        let ObjData::Route { net, .. } = self.o(r).clone() else { return Vec::new() };
        let mut nodes: Vec<Pt> = Vec::new();
        let mut index: HashMap<(u8, ObjId, u32), usize> = HashMap::new();
        let mut node = |key: (u8, ObjId, u32), xy: Option<Pt>, nodes: &mut Vec<Pt>| -> Option<usize> {
            if let (false, Some(p)) = (index.contains_key(&key), xy) {
                index.insert(key, nodes.len());
                nodes.push(p);
            }
            index.get(&key).copied()
        };
        for ow in self.inst_like(s) {
            for t in self.terms_of(s, ow) {
                if self.tnet_has(s, t, net) && self.has_pin(s, t) {
                    let ObjData::Term { index: i, .. } = self.o(t).clone() else { continue };
                    let p = self.point_of(s, t);
                    node((0, ow, i), p, &mut nodes);
                }
            }
        }
        for a in self.scenes[s].usages.clone() {
            if self.usage(a).in_symbol || !self.scenes[s].symbol.contains_key(&a) {
                continue;
            }
            for t in self.terms_of(s, a) {
                let ObjData::Term { index: i, name, .. } = self.o(t).clone() else { continue };
                let role = name.as_ref().and_then(|n| self.usage(a).role(n));
                if role == Some(net) && self.has_pin(s, t) {
                    let p = self.point_of(s, t);
                    node((0, a, i), p, &mut nodes);
                }
            }
        }
        let mut parent: Vec<usize> = (0..nodes.len()).collect();
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        for w in self.scenes[s].wires.clone() {
            let ObjData::Wire { nets, points, failed, .. } = self.o(w).clone() else { continue };
            if !nets.contains(&net) || failed {
                continue;
            }
            let mut keys = Vec::new();
            for q in points {
                let k = match self.o(q).clone() {
                    ObjData::Term { owner, index: i, .. } => {
                        let ow = self.bundle_field(owner).unwrap_or(owner);
                        let p = self.point_of(s, q);
                        node((0, ow, i), p, &mut nodes)
                    }
                    _ => {
                        let p = self.point_of(s, q);
                        node((1, q, 0), p, &mut nodes)
                    }
                };
                if let Some(k) = k {
                    keys.push(k);
                }
            }
            while parent.len() < nodes.len() {
                parent.push(parent.len());
            }
            for &k in keys.iter().skip(1) {
                let (a, b) = (find(&mut parent, k), find(&mut parent, keys[0]));
                parent[a] = b;
            }
        }
        while parent.len() < nodes.len() {
            parent.push(parent.len());
        }
        let mut edges = Vec::new();
        for i in 0..nodes.len() {
            for j in i + 1..nodes.len() {
                let d = (nodes[i].0 - nodes[j].0).abs() + (nodes[i].1 - nodes[j].1).abs();
                edges.push((d, i, j));
            }
        }
        edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut out = Vec::new();
        for (_, i, j) in edges {
            let (a, b) = (find(&mut parent, i), find(&mut parent, j));
            if a != b {
                parent[a] = b;
                out.push((nodes[i], nodes[j]));
            }
        }
        out
    }

    pub(crate) fn wire_points(&mut self, s: SceneId, w: ObjId) -> Option<Vec<Pt>> {
        let ObjData::Wire { points, mode, .. } = self.o(w).clone() else { return None };
        let mut pts = Vec::new();
        for q in points {
            pts.push(self.point_of(s, q)?);
        }
        Some(route(&pts, &mode))
    }

    // ---- constraints (14)

    fn check_constraints(&mut self, s: SceneId) {
        // the kept coordinate facts of each object, for the constraints that are violated
        let mut kept: HashMap<ObjId, Vec<usize>> = HashMap::new();
        for (&(o, _), &(_, _, k)) in &self.scenes[s].at_best {
            kept.entry(o).or_default().push(k);
        }
        for c in self.scenes[s].constraints.clone() {
            let ObjData::Constraint(cd) = self.o(c).clone() else { continue };
            let site = cd.site.clone();
            let p = self.eff_priority(s, cd.priority_param, &cd.chain, Some(c));
            self.constraint_mut(c).eff = Some(p);
            if p == Priority::Disabled {
                self.constraint_mut(c).status = Some("disabled");
                continue;
            }
            let relation = cd.kind == ConstraintKind::OrientRelation;
            let ctx = site.as_deref();
            let members: Vec<ObjId> = if cd.kind == ConstraintKind::Distance || relation {
                let mut out = Vec::new();
                for a in &cd.args {
                    if self.is_set(*a) {
                        out.extend(self.flatten(s, *a, 7, ctx));
                    } else {
                        out.push(*a);
                    }
                }
                out
            } else if self.is_set(cd.args[0]) {
                self.items_of(s, cd.args[0], 7, ctx)
            } else {
                vec![cd.args[0]]
            };
            let leaves = self.leaves(s, &members);
            let mut bad = None;
            for &m in &leaves {
                let why = if relation { self.orientable(s, m) } else { self.locatable(s, m) };
                if why.is_some() {
                    bad = why;
                    break;
                }
            }
            if let Some(why) = bad {
                let l = self.label(c);
                self.diag_site(
                    Severity::Error,
                    &site,
                    cd.off,
                    format!("a member of {l}: {why}; this constraint fails (14.1)"),
                );
                self.constraint_mut(c).failed = true;
                continue;
            }
            let ok = self.satisfied(s, c, &members);
            match ok {
                None => {
                    self.constraint_mut(c).status = Some("unsolved");
                    continue;
                }
                Some(true) => {
                    self.constraint_mut(c).status = Some("ok");
                    continue;
                }
                Some(false) => {}
            }
            // violated: compare its priority with the coordinates it involves (8.5)
            let pr = (level(p), self.scenes[s].created[&c].clone());
            let mut involved: Vec<Fact> = Vec::new();
            for m in &leaves {
                if relation {
                    if let Some((_, _, k)) = self.scenes[s].ori_best.get(m) {
                        involved.push(self.scenes[s].orient[*k].clone());
                    }
                } else {
                    let target = match self.o(*m) {
                        ObjData::Term { owner, .. } => *owner,
                        _ => *m,
                    };
                    let mut ks = kept.get(&target).cloned().unwrap_or_default();
                    ks.sort();
                    for k in ks {
                        involved.push(self.scenes[s].at[k].clone());
                    }
                }
            }
            let what = if relation { "orientations" } else { "coordinates" };
            let weaker = involved.iter().any(|f| (level(self.fact_priority(s, f)), f.pos.clone()) < pr);
            let l = self.label(c);
            if !involved.is_empty() && !weaker {
                self.constraint_mut(c).status = Some("dropped");
                if p != Priority::Low {
                    if let Some(sc) = &site {
                        self.note_failed_at(sc.rec, cd.off, true);
                    }
                    self.diag_site(
                        Severity::Hint,
                        &site,
                        cd.off,
                        format!("{l} conflicts with fixed {what} and is dropped in the order of 8.5 (8.5)"),
                    );
                }
            } else {
                self.constraint_mut(c).status = Some("needs-solver");
                self.diag_site(
                    Severity::Hint,
                    &site,
                    cd.off,
                    format!("{l} comes before some of the {what} it involves in the order of 8.5, so they should change; without a solver the written {what} are drawn"),
                );
            }
        }
    }

    pub(crate) fn constraint_mut(&mut self, c: ObjId) -> &mut ConstraintData {
        match self.om(c) {
            ObjData::Constraint(cd) => cd,
            _ => panic!("not a constraint"),
        }
    }

    fn leaves(&mut self, s: SceneId, members: &[ObjId]) -> Vec<ObjId> {
        let mut out = Vec::new();
        for &m in members {
            if self.is_set(m) {
                let f = self.flatten(s, m, 7, None);
                out.extend(self.leaves(s, &f));
            } else {
                out.push(m);
            }
        }
        out
    }

    fn locatable(&mut self, s: SceneId, m: ObjId) -> Option<String> {
        let l = self.label(m);
        match self.o(m).clone() {
            ObjData::Usage(_) => {
                if !self.scenes[s].symbol.contains_key(&m) {
                    return Some(format!("{l} has no symbol"));
                }
                if self.at_point(m) {
                    return Some(format!("{l} is drawn on the point of its main object"));
                }
                None
            }
            ObjData::Wire { .. } | ObjData::Net { .. } | ObjData::Constraint(_) => {
                Some(format!("{l} cannot be located"))
            }
            ObjData::Term { .. } if !self.has_pin(s, m) => Some(format!("{l} has no pin in its symbol")),
            ObjData::Inst { bundle: Some(_), .. } => Some(format!("{l} is in a bundle")),
            _ => None,
        }
    }

    /// ORIENT-RELATION takes what ORIENT takes (11.2, 14.9).
    fn orientable(&self, s: SceneId, m: ObjId) -> Option<String> {
        let l = self.label(m);
        match self.o(m) {
            ObjData::Inst { bundle, .. } => bundle.map(|_| format!("{l} is in a bundle")),
            ObjData::Bundle { .. } => None,
            ObjData::Usage(_) => {
                if self.scenes[s].symbol.contains_key(&m) {
                    None
                } else {
                    Some(format!("{l} has no symbol"))
                }
            }
            _ => Some(format!("{l} has no orientation")),
        }
    }

    fn coord_of(&mut self, s: SceneId, m: ObjId, axis: char) -> Option<f64> {
        let p = if self.is_set(m) { self.set_center(s, m) } else { self.point_of(s, m) }?;
        Some(if axis == 'x' { p.0 } else { p.1 })
    }

    /// The range of the members along an axis: None when unknown, Some(None) when empty.
    fn range(&mut self, s: SceneId, a: ObjId, axis: char) -> Option<Option<(f64, f64)>> {
        let leaves = if self.is_set(a) { self.flatten(s, a, 7, None) } else { vec![a] };
        if leaves.is_empty() {
            return Some(None);
        }
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        for m in leaves {
            let p = self.point_of(s, m)?;
            let v = if axis == 'x' { p.0 } else { p.1 };
            lo = lo.min(v);
            hi = hi.max(v);
        }
        Some(Some((lo, hi)))
    }

    fn satisfied(&mut self, s: SceneId, c: ObjId, members: &[ObjId]) -> Option<bool> {
        let ObjData::Constraint(cd) = self.o(c).clone() else { return Some(true) };
        if cd.kind == ConstraintKind::OrientRelation {
            // each side shares one orientation; `to` is `from` turned by the relation in the drawing's frame (14.9)
            let mut sides = Vec::new();
            for a in &cd.args {
                sides.push(if self.is_set(*a) { self.flatten(s, *a, 7, None) } else { vec![*a] });
            }
            if sides.iter().any(|x| x.is_empty()) {
                return Some(true);
            }
            let mut fixed = Vec::new();
            for side in &sides {
                let codes: HashSet<Orient> = side.iter().filter_map(|m| self.scenes[s].ori.get(m).copied()).collect();
                if codes.len() > 1 {
                    return Some(false);
                }
                fixed.push(codes.into_iter().next());
            }
            if let (Some(a), Some(b), Some(rel)) = (fixed[0], fixed[1], cd.params.relation) {
                if b != a.then(rel) {
                    return Some(false);
                }
            }
            if sides.iter().flatten().any(|m| !self.scenes[s].ori.contains_key(m)) {
                return None;
            }
            return Some(true);
        }
        let axis = cd.params.axis.unwrap_or('x');
        if cd.kind == ConstraintKind::Distance {
            let ra = self.range(s, cd.args[0], axis)?;
            let rb = self.range(s, cd.args[1], axis)?;
            let (Some(ra), Some(rb)) = (ra, rb) else { return Some(true) };
            let d = match cd.params.sense {
                Some("increasing") => rb.0 - ra.1,
                Some("decreasing") => ra.0 - rb.1,
                _ => (rb.0 - ra.1).max(ra.0 - rb.1),
            };
            let lo_ok = cd.params.min.is_none_or(|lo| d >= lo - TOL);
            let hi_ok = cd.params.max.is_none_or(|hi| d <= hi + TOL);
            return Some(lo_ok && hi_ok);
        }
        let mut vals = Vec::new();
        for &m in members {
            vals.push(self.coord_of(s, m, axis)?);
        }
        match cd.kind {
            ConstraintKind::Align => Some(vals.len() < 2 || vals.iter().all(|v| (v - vals[0]).abs() <= TOL)),
            ConstraintKind::Order => {
                if vals.len() < 2 {
                    return Some(true);
                }
                let inc = vals.windows(2).all(|w| w[1] - w[0] >= 1.0 - TOL);
                let dec = vals.windows(2).all(|w| w[0] - w[1] >= 1.0 - TOL);
                Some(match cd.params.sense {
                    Some("increasing") => inc,
                    Some("decreasing") => dec,
                    _ => inc || dec,
                })
            }
            ConstraintKind::Distribute => {
                if vals.len() < 3 {
                    return Some(true);
                }
                let d0 = vals[1] - vals[0];
                Some(vals.windows(2).all(|w| ((w[1] - w[0]) - d0).abs() <= TOL))
            }
            _ => Some(true),
        }
    }

    // ---- style queue (8.3, 9.3, 11.3)

    fn resolve_styles(&mut self, s: SceneId) {
        // neither the facts nor the objects change in this pass, so the index and the entries kept
        // in `done` stay valid until its end
        let mut idx = StyleIndex::default();
        for (k, f) in self.scenes[s].styles.iter().enumerate() {
            match &f.data {
                FactData::OwnStyle(m) => idx.own.entry(f.target).or_default().push((f.pos.clone(), m.clone())),
                FactData::Style(sty) if matches!(self.o(*sty), ObjData::Style(_)) => {
                    idx.styles.entry(f.target).or_default().push(k)
                }
                _ => {}
            }
        }
        let mut objs = self.drawables(s, false);
        let mut listed: HashSet<ObjId> = objs.iter().copied().collect();
        for &t in &self.scenes[s].texts {
            if matches!(self.o(t), ObjData::KeyText { .. }) && listed.insert(t) {
                objs.push(t);
            }
        }
        let mut done = HashMap::new();
        for o in objs {
            let e = self.style_entries(s, o, &idx, &mut done).clone();
            self.scenes[s].style.insert(o, e);
        }
    }

    /// The style entries of an object, computed once per object and kept in `done`.
    fn style_entries<'a>(
        &mut self,
        s: SceneId,
        o: ObjId,
        idx: &StyleIndex,
        done: &'a mut HashMap<ObjId, BTreeMap<String, (Lit, Pos)>>,
    ) -> &'a BTreeMap<String, (Lit, Pos)> {
        if done.contains_key(&o) {
            return &done[&o];
        }
        let mut direct: BTreeMap<String, (Lit, Pos)> = BTreeMap::new();
        let mut other: BTreeMap<String, (Lit, Pos)> = BTreeMap::new();
        fn put(d: &mut BTreeMap<String, (Lit, Pos)>, props: &StyleMap, p: &Pos) {
            for (k, v) in props {
                let replace = match d.get(k) {
                    None => true,
                    Some(cur) => *p >= cur.1,
                };
                if replace {
                    d.insert(k.clone(), (v.clone(), p.clone()));
                }
            }
        }
        if let Some(v) = idx.own.get(&o) {
            for (p, props) in v {
                put(&mut direct, props, p);
            }
        }
        match self.o(o).clone() {
            ObjData::KeyText { layers, .. } => {
                for (p, _, st) in &layers {
                    put(&mut direct, st, p);
                }
            }
            ObjData::Text { style, .. } if !style.is_empty() => {
                let p = self.scenes[s].created.get(&o).cloned().unwrap_or_else(|| pos(&[0]));
                put(&mut direct, &style, &p);
            }
            _ => {}
        }
        for &k in idx.styles.get(&o).into_iter().flatten() {
            let f = &self.scenes[s].styles[k];
            let FactData::Style(sty) = f.data else { continue };
            let ObjData::Style(props) = self.o(sty) else { continue };
            if f.direct {
                put(&mut direct, props, &f.pos);
            } else {
                put(&mut other, props, &f.pos);
            }
        }
        // a style given to a member of a bundle fails, in the order of the facts (11.8)
        if let ObjData::Bundle { members, .. } = self.o(o) {
            let mut ks: Vec<usize> = members.iter().filter_map(|m| idx.styles.get(m)).flatten().copied().collect();
            ks.sort_unstable();
            ks.dedup();
            for k in ks {
                let f = self.scenes[s].styles[k].clone();
                self.member_draw_error(s, 's', k, &f, f.target);
            }
        }
        // passed down: labels take the style of the object they are attached to (8.3)
        let src = match self.o(o) {
            ObjData::Text { attach, .. } | ObjData::KeyText { attach, .. } => {
                if matches!(
                    self.o(*attach),
                    ObjData::Inst { .. }
                        | ObjData::Usage(_)
                        | ObjData::Bundle { .. }
                        | ObjData::Wire { .. }
                        | ObjData::Prim { .. }
                ) {
                    Some(*attach)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(src) = src {
            for (k, v) in self.style_entries(s, src, idx, done) {
                let replace = match other.get(k) {
                    None => true,
                    Some(cur) => v.1 >= cur.1,
                };
                if replace {
                    other.insert(k.clone(), v.clone());
                }
            }
        }
        let mut out = other;
        for (k, v) in direct {
            out.insert(k, v);
        }
        done.entry(o).or_insert(out)
    }

    fn resolve_layers(&mut self, s: SceneId) {
        let layers = self.scenes[s].layers.clone();
        for (k, f) in layers.iter().enumerate() {
            let o = f.target;
            match self.o(o) {
                ObjData::Point { .. } | ObjData::Net { .. } | ObjData::Constraint(_) => {
                    let l = self.label(o);
                    self.diag_fact(Severity::Error, f, format!("LAYER does not take {l} (11.6)"));
                    continue;
                }
                ObjData::Inst { bundle: Some(_), .. } => {
                    self.member_draw_error(s, 'l', k, f, o);
                    continue;
                }
                _ => {}
            }
            let FactData::Layer(name) = &f.data else { continue };
            let replace = match self.scenes[s].layer.get(&o) {
                None => true,
                Some(cur) => f.pos > cur.1,
            };
            if replace {
                self.scenes[s].layer.insert(o, (name.clone(), f.pos.clone()));
            }
        }
    }
}
