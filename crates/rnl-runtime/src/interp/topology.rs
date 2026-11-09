//! ASSERT-TOPOLOGY (15.4): a check that some members are connected in a given way, in series, in
//! parallel, as a star or as a single loop. It is an assertion as those of 15.3: it only reports,
//! with a violation, and is checked where, when and under which priority they are.
//!
//! **The members** are the items of the set given, in order (5.2): an object is one member, a
//! `{…}` or a reference to a set one member, a black box; a set given alone is split one level
//! first (5.2.1). A member is an instance (a bundle by its first member, 11.8), a usage or a set.
//!
//! **Their nets.** An instance: the nets of its terminals that `exclude` leaves. A usage or a set,
//! a black box: the terminals it brings, followed inward (a usage brings what fills its roles, a
//! set its members; an instance all its terminals, a terminal itself; nets and points nothing), and
//! of their nets those that are also used outside it, by a terminal it does not bring (of the
//! other members of a bundle too) or as a port of the level. `exclude=[[type, terminal], …]` takes
//! out the terminals of instances of that type, the terminal compared as the condition `terminal`
//! of SELECT-TERMINAL (10.1).

use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

use crate::diag::{codes, Code, Extra, Severity};
use crate::syntax::Segment;
use crate::words::words;

use super::assertions::and_list;
use super::ctx::Ctx;
use super::eval::{get, ArgV, Stop, R};
use super::obj::*;
use super::scene::Task;
use super::Interp;

/// One ASSERT-TOPOLOGY, as it is checked after the geometry phase.
#[derive(Clone, Debug)]
struct Topology {
    /// "series", "parallel", "star" or "loop".
    topology: String,
    /// The terminals that do not count: a type, lower case, and a terminal (15.4).
    exclude: Vec<(String, TermKey)>,
    message: Option<String>,
    members: ObjId,
    site: Rc<Ctx>,
    off: usize,
    chain: Vec<ObjId>,
}

impl Interp<'_> {
    /// `members @ASSERT-TOPOLOGY(topology, exclude=…, message=…)` (15.4): the parameters checked now,
    /// the members after the geometry phase, as for the assertions of 15.3.
    pub(crate) fn kw_assert_topology(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let at = seg.call.as_ref().map_or(seg.start, |c| c.start);
        let spec = words().keyword(kw).map(|s| s.param_list().to_vec()).unwrap_or_default();
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let topology = get(&p, "topology").and_then(Lit::str).unwrap_or("series").to_string();
        let mut exclude = Vec::new();
        if let Some(v) = get(&p, "exclude") {
            let items = match v {
                Lit::Arr(items) => items.clone(),
                other => vec![other.clone()],
            };
            for item in items {
                let pair = match &item {
                    Lit::Arr(pair) if pair.len() == 2 => Some(pair.clone()),
                    _ => None,
                };
                // a terminal by name, or by number (10.1)
                let term = pair.as_ref().and_then(|pair| match &pair[1] {
                    Lit::Str(t) if !t.as_str().is_empty() => Some(TermKey::Name(t.as_str().to_string())),
                    Lit::Num(n) if *n >= 1.0 && n.fract() == 0.0 => Some(TermKey::Num(*n as u32)),
                    _ => None,
                });
                match (pair.as_ref().and_then(|pair| pair[0].str()), term) {
                    (Some(ty), Some(term)) if !ty.is_empty() => exclude.push((ty.to_lowercase(), term)),
                    _ => {
                        return self.fail(
                            ctx,
                            codes::ASSERT_TOPOLOGY_EXCLUDE_NOT_PAIR,
                            format!(
                                "exclude= of @{kw} is a list of pairs of a type and a terminal, such as exclude=[[\"nmos\", \"g\"], [\"nmos\", 4]], and {} is none (15.4)",
                                item.show()
                            ),
                            at,
                        )
                    }
                }
            }
        }
        let message = get(&p, "message").and_then(Lit::str).map(String::from);
        let positions = words().keyword(kw).map(|s| s.position_list()).unwrap_or(&[]);
        let mapped = self.map_args(kw, args, seg, ctx, positions)?;
        let members = mapped["members"].obj;
        if let Some(d) = self.failed_obj(members) {
            let id = self.err_cause(
                ctx,
                codes::ASSERT_MEMBER_FAILED,
                format!("a member of @{kw} failed, so the assertion checks nothing (7.3, 15.4)"),
                seg.start,
                d,
            );
            return Err(Stop::Fail(Some(id)));
        }
        // in a record with priority="disabled" it is not checked (15.3, 15.4)
        if ctx.header.priority == Some(Priority::Disabled) {
            return Ok((NOTHING, false));
        }
        let lazy = self.is_lazy(members);
        let Some(s) = ctx.scene else {
            if lazy {
                self.rules.insert(ctx.member_key.clone());
                return Ok((NOTHING, true));
            }
            return self.scene_only(ctx, seg.start);
        };
        let t = Topology {
            topology,
            exclude,
            message,
            members,
            site: Rc::new(ctx.clone()),
            off: at,
            chain: self.apply_chain(ctx),
        };
        // checked as the assertions of 15.3 are: after the geometry phase
        let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, _| it.check_topology(s, &t));
        if self.scenes[s].phase >= 8 {
            task(self, s, 8);
        } else {
            self.scenes[s].tasks[8].push((ctx.pos.clone(), task));
        }
        if lazy {
            self.rules.insert(ctx.member_key.clone());
        }
        Ok((NOTHING, false))
    }

    /// Does `exclude` take out terminal `t` (15.4)?
    fn excluded(&self, exclude: &[(String, TermKey)], t: ObjId) -> bool {
        if exclude.is_empty() {
            return false;
        }
        let ObjData::Term { owner, index, name } = self.o(t).clone() else { return false };
        let inst = match self.o(owner) {
            ObjData::Inst { .. } => owner,
            ObjData::Bundle { members, .. } => members[0],
            _ => return false,
        };
        let Some(ty) = self.inst_of(inst).mtype.clone() else { return false };
        let cs = self.host_case(owner);
        exclude.iter().any(|(xt, key)| {
            *xt == ty.to_lowercase()
                && match key {
                    TermKey::Num(n) => *n == index,
                    TermKey::Name(w) => {
                        name.as_ref().is_some_and(|n| if cs { n == w } else { n.eq_ignore_ascii_case(w) })
                    }
                }
        })
    }

    /// The terminals an object brings into a black box (15.4), followed inward.
    fn brought(&mut self, s: SceneId, o: ObjId, out: &mut Vec<ObjId>, seen: &mut HashSet<ObjId>) {
        if !seen.insert(o) {
            return;
        }
        match self.o(o).clone() {
            ObjData::Inst { .. } => out.extend(self.terms_of(s, o)),
            ObjData::Bundle { members, .. } => out.extend(self.terms_of(s, members[0])),
            ObjData::Term { owner, .. } if matches!(self.o(owner), ObjData::Inst { .. } | ObjData::Bundle { .. }) => {
                out.push(o)
            }
            ObjData::Usage(_) => {
                for m in self.usage_members(s, o, 7, None) {
                    self.brought(s, m, out, seen);
                }
            }
            _ if self.is_set(o) => {
                for m in self.items_of(s, o, 7, None) {
                    self.brought(s, m, out, seen);
                }
            }
            _ => {}
        }
    }

    /// The nets of each terminal of the level's instances, those of every member of a bundle.
    fn level_terminals(&mut self, s: SceneId) -> Vec<(ObjId, Vec<ObjId>)> {
        let mut out = Vec::new();
        for o in self.inst_like(s) {
            let insts = match self.o(o).clone() {
                ObjData::Bundle { members, .. } => members,
                _ => vec![o],
            };
            for i in insts {
                for t in self.terms_of(s, i) {
                    let nets = self.tnets(s, t);
                    out.push((t, nets));
                }
            }
        }
        out
    }

    /// The nets of a member (15.4), or None when it is no member of a topology.
    fn member_nets(
        &mut self,
        s: SceneId,
        m: ObjId,
        exclude: &[(String, TermKey)],
        all: &[(ObjId, Vec<ObjId>)],
        ports: &HashSet<ObjId>,
    ) -> Option<BTreeSet<ObjId>> {
        let inst = match self.o(m).clone() {
            ObjData::Inst { .. } => Some(m),
            ObjData::Bundle { members, .. } => Some(members[0]),
            ObjData::Usage(_) => None,
            _ if self.is_set(m) => None,
            _ => return None,
        };
        if let Some(i) = inst {
            let mut nets = BTreeSet::new();
            for t in self.terms_of(s, i) {
                if !self.excluded(exclude, t) {
                    nets.extend(self.tnets(s, t));
                }
            }
            return Some(nets);
        }
        // a black box: the nets of what it brings that are used outside it too
        let mut terms = Vec::new();
        self.brought(s, m, &mut terms, &mut HashSet::new());
        let inside: HashSet<ObjId> = terms.iter().copied().collect();
        let mut nets = BTreeSet::new();
        for &t in &terms {
            if self.excluded(exclude, t) {
                continue;
            }
            for n in self.tnets(s, t) {
                let outside = ports.contains(&n) || all.iter().any(|(u, ns)| !inside.contains(u) && ns.contains(&n));
                if outside {
                    nets.insert(n);
                }
            }
        }
        Some(nets)
    }

    /// A set whose one item, as written, is a set: that set, split one level (5.2.1).
    fn single_item_set(&self, s: SceneId, obj: ObjId) -> ObjId {
        let ObjData::Set(set) = self.o(obj) else { return obj };
        if !matches!(set.kind, SetKind::Plain) || set.mods_by_scene.get(&Some(s)).is_some_and(|m| !m.is_empty()) {
            return obj;
        }
        let items: Vec<ObjId> =
            set.items.iter().map(|(o, _)| *o).filter(|o| *o != LEVEL && !self.is_failed(*o)).collect();
        match items.as_slice() {
            [one] if self.is_set(*one) => *one,
            _ => obj,
        }
    }

    /// The members of an ASSERT-TOPOLOGY (15.4): the items of the set as written, a set among them
    /// one member, not spread; the result of a selection one member for each object it selects.
    fn topology_items(&mut self, s: SceneId, root: ObjId, site: &Ctx) -> Vec<ObjId> {
        if !self.is_set(root) {
            return vec![root];
        }
        let sd = self.set(root);
        let edited = sd.mods_by_scene.get(&Some(s)).is_some_and(|m| !m.is_empty());
        if matches!(sd.kind, SetKind::Plain) && !edited {
            let items: Vec<ObjId> = sd.items.iter().map(|(o, _)| *o).collect();
            return items.into_iter().filter(|o| *o != LEVEL && !self.is_failed(*o)).collect();
        }
        self.items_of(s, root, 7, Some(site))
    }

    /// An ASSERT-TOPOLOGY checked in a scene (15.4): a violation when the members are not connected
    /// as it says.
    fn check_topology(&mut self, s: SceneId, t: &Topology) {
        if !t.chain.is_empty() && self.eff_priority(s, None, &t.chain, None) == Priority::Disabled {
            return;
        }
        let root = self.single_item_set(s, t.members);
        let items = self.topology_items(s, root, &t.site);
        let lv = self.scenes[s].level;
        let ports: HashSet<ObjId> = self.doc.levels[lv].ports.clone().iter().map(|p| self.net_obj(s, p)).collect();
        let all = self.level_terminals(s);
        let mut nets: Vec<BTreeSet<ObjId>> = Vec::new();
        let mut wrong = Vec::new();
        for &m in &items {
            match self.member_nets(s, m, &t.exclude, &all, &ports) {
                Some(n) => nets.push(n),
                None => wrong.push(self.label(m)),
            }
        }
        if !wrong.is_empty() {
            let msg = format!(
                "@ASSERT-TOPOLOGY takes instances, bundles, usages and sets, and {} {} not: the assertion is not checked (15.4)",
                and_list(&wrong),
                if wrong.len() == 1 { "is" } else { "are" }
            );
            self.topology_diag(Severity::Error, codes::ASSERT_TOPOLOGY_MEMBER_WRONG_KIND, t, msg);
            return;
        }
        if items.len() < 2 {
            return;
        }
        let names: Vec<String> = items.iter().map(|m| self.label(*m)).collect();
        let shown = |it: &Self, ns: &BTreeSet<ObjId>| -> String {
            if ns.is_empty() {
                "no net".to_string()
            } else {
                ns.iter().map(|n| it.label(*n)).collect::<Vec<_>>().join(", ")
            }
        };
        let share = |a: usize, b: usize| -> BTreeSet<ObjId> { nets[a].intersection(&nets[b]).copied().collect() };
        let pair = |a: usize, b: usize, ns: &BTreeSet<ObjId>, it: &Self| -> String {
            format!("{} and {} share {}", names[a], names[b], shown(it, ns))
        };
        let common = || -> BTreeSet<ObjId> {
            nets.iter().skip(1).fold(nets[0].clone(), |acc, x| acc.intersection(x).copied().collect())
        };
        let n = items.len();
        let mut problem: Option<String> = None;
        match t.topology.as_str() {
            "series" => {
                let mut bad = Vec::new();
                for a in 0..n {
                    for b in a + 1..n {
                        let ns = share(a, b);
                        let next = b == a + 1;
                        if (next && ns.len() != 1) || (!next && !ns.is_empty()) {
                            bad.push(pair(a, b, &ns, self));
                        }
                    }
                }
                if !bad.is_empty() {
                    problem = Some(format!(
                        "the members should be in series, each sharing exactly one net with the next and none with any other, but {}",
                        and_list(&bad)
                    ));
                }
            }
            "parallel" => {
                // at least two nets, the ends, shared by all; a member's other nets, such as a tap a
                // branch brings out, are not checked (15.4)
                let common = common();
                if common.len() < 2 {
                    let each: Vec<String> =
                        (0..n).map(|k| format!("{} is on {}", names[k], shown(self, &nets[k]))).collect();
                    problem = Some(format!(
                        "the members should be in parallel, sharing at least two nets, the ends, but the nets all of them are on are {}: {}",
                        shown(self, &common),
                        and_list(&each)
                    ));
                }
            }
            "star" => {
                let common = common();
                let mut bad = Vec::new();
                if common.len() != 1 {
                    bad.push(format!("the nets all of them are on are {}", shown(self, &common)));
                }
                for a in 0..n {
                    for b in a + 1..n {
                        let ns = share(a, b);
                        if common.len() == 1 && ns != common {
                            bad.push(pair(a, b, &ns, self));
                        }
                    }
                }
                if !bad.is_empty() {
                    problem = Some(format!(
                        "the members should be a star, all on one net and no two sharing another, but {}",
                        and_list(&bad)
                    ));
                }
            }
            _ if n == 2 => {
                problem = Some(format!(
                    "a loop has at least three members, and {} and {} are two; for two members write \"parallel\"",
                    names[0], names[1]
                ));
            }
            _ => {
                let mut bad = Vec::new();
                let mut joints: Vec<BTreeSet<ObjId>> = Vec::new();
                for a in 0..n {
                    for b in a + 1..n {
                        let ns = share(a, b);
                        let next = b == a + 1 || (a == 0 && b == n - 1);
                        if next {
                            if ns.len() != 1 || joints.contains(&ns) {
                                bad.push(pair(a, b, &ns, self));
                            }
                            joints.push(ns);
                        } else if !ns.is_empty() {
                            bad.push(pair(a, b, &ns, self));
                        }
                    }
                }
                if !bad.is_empty() {
                    problem = Some(format!(
                        "the members should go round in a loop, each sharing exactly one net with the next, the last with the first, these nets all different, and none with any other, but {}",
                        and_list(&bad)
                    ));
                }
            }
        }
        if let Some(problem) = problem {
            let place = self.assertion_where(&t.site);
            let note = t.message.as_ref().map(|m| format!("; {m}")).unwrap_or_default();
            let msg = format!("@ASSERT-TOPOLOGY does not hold{place}: {problem}{note} (15.4)");
            self.topology_diag(Severity::Violation, codes::VIOLATION_TOPOLOGY, t, msg);
        }
    }

    /// A diagnostic of an ASSERT-TOPOLOGY, at the statement; a violation is shown also when the
    /// statement is in a package the document uses (7.6).
    fn topology_diag(&mut self, sev: Severity, code: Code, t: &Topology, msg: String) {
        if sev == Severity::Error {
            self.note_failed_at(t.site.rec, t.off, false);
        }
        let loc = self.rec_loc(t.site.rec, t.off);
        let extra = Extra { cause: None, origin: t.site.origin.to_string(), in_package: String::new() };
        self.sink.add(sev, code, msg, loc, extra);
    }
}
