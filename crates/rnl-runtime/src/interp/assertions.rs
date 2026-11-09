//! The assertions ASSERT-INSTANCE, ASSERT-TERMINAL and ASSERT-NET (15.3): a check that what a file
//! or a definition says agrees with the circuit. Each takes a set of objects of its kind and checks
//! that every member meets the conditions written, which have the names and meanings of the
//! conditions of the selection keyword of that kind (10.1), that the attributes listed in `same`
//! are the same for all members and those in `different` different for any two, and, for
//! instances, that every member has the attributes listed in `has`. A check that does not hold
//! gives a violation (7.6), which names the members, the attribute and the statement's `message`:
//! nothing fails, it makes no fact and changes nothing drawn.
//!
//! **Where and when.** In any level of a file of 0.4, and in the body of a general definition,
//! where it is checked for each application, on the objects that fill the roles: the diagnostic
//! names the usage and its definition. Assertions are checked after the geometry phase (8.2), when
//! every condition can be read and every PRIORITY is in. A set with no members holds trivially; in
//! a record with `priority="disabled"`, and in the body of a usage that is disabled, nothing is
//! checked.
//!
//! **What is checked.** For instances `name`, `model`, `type` and `terminals`. For terminals
//! `name`, `model` and `type` of the instance, `terminal`, `net` and `current`. For nets `name`,
//! `port`, `wired` and `global`. The attributes of `same`, `different` and `has` are those of
//! `${…}` (13.6): for instances `name`, `model`, `type`, `value` and the names of their parameters,
//! which are open; for terminals `net` only, for nets `name` only, and any other is a wrong
//! parameter value. Names compare by the case rules of the host, the values of parameters as the
//! host writes them, without evaluation (3.7). An attribute a member lacks is a value of its own,
//! "absent": members that lack it are equal to one another and differ from every written value.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{codes, Code, Extra, Severity};
use crate::host::glob;
use crate::syntax::{Segment, Value};
use crate::words::words;

use super::ctx::Ctx;
use super::eval::{get, ArgV, Stop, R};
use super::obj::*;
use super::scene::{name_match, Task};
use super::Interp;

/// One assertion statement, as it is checked after the geometry phase.
#[derive(Clone, Debug)]
struct Assertion {
    kw: String,
    /// The conditions written, in the order of the word list (10.1).
    conds: Vec<(String, Lit)>,
    same: Vec<String>,
    different: Vec<String>,
    /// For ASSERT-INSTANCE, the attributes every member is to have (15.3).
    has: Vec<String>,
    message: Option<String>,
    members: ObjId,
    site: Rc<Ctx>,
    off: usize,
    /// The usages whose bodies it is in, outermost first: one of them disabled, it is not checked.
    chain: Vec<ObjId>,
}

/// The conditions each assertion checks, by keyword, those of its selection keyword (10.1).
fn conditions(kw: &str) -> &'static [&'static str] {
    match kw {
        "ASSERT-INSTANCE" => &["name", "model", "type", "terminals"],
        "ASSERT-TERMINAL" => &["name", "model", "type", "terminal", "net", "current"],
        _ => &["name", "port", "wired", "global"],
    }
}

/// The kind of object an assertion checks.
fn kind_of_assertion(kw: &str) -> &'static str {
    match kw {
        "ASSERT-INSTANCE" => "instance",
        "ASSERT-TERMINAL" => "terminal",
        _ => "net",
    }
}

/// A value as a message shows it.
fn quoted(v: &str) -> String {
    format!("{v:?}")
}

/// Names joined for a message: `#M1`, `#M1 and #M2`, `#M1, #M2 and #M3`.
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn has_have(n: usize) -> &'static str {
    if n == 1 {
        "has"
    } else {
        "have"
    }
}

/// The value of an attribute of a member: as it shows, and as it compares.
struct AttrValue {
    shown: String,
    key: String,
}

impl Interp<'_> {
    /// `{members} @ASSERT-INSTANCE(…)`, `@ASSERT-TERMINAL(…)`, `@ASSERT-NET(…)` (15.3): the
    /// statement's parameters checked now, its members after the geometry phase.
    pub(crate) fn kw_assert(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let at = seg.call.as_ref().map_or(seg.start, |c| c.start);
        // `same="model"`: a list is asked for, which the check of the parameter's type only calls the
        // wrong type
        if let Some(ps) = seg.call.as_ref().and_then(|c| c.params.as_ref()) {
            for n in &ps.named {
                let std = if n.quoted { n.name.clone() } else { self.word(&n.name, ctx.unit) };
                if let ("same" | "different" | "has", Value::Str(s)) = (std.as_str(), &n.value) {
                    return self.fail(
                        ctx,
                        codes::ASSERT_LIST_WRITTEN_AS_STRING,
                        format!("{std}= of @{kw} is a list of attribute names, written [{s:?}], not {s:?} (15.3)"),
                        n.off,
                    );
                }
            }
        }
        let spec = words().keyword(kw).map(|s| s.param_list().to_vec()).unwrap_or_default();
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let mut lists: [Vec<String>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for (i, key) in ["same", "different", "has"].into_iter().enumerate() {
            let Some(v) = get(&p, key) else { continue };
            let items = match v {
                Lit::Arr(items) => items.clone(),
                other => vec![other.clone()],
            };
            for item in items {
                match item.str() {
                    Some(a) if !a.is_empty() => {
                        if !lists[i].iter().any(|x| x == a) {
                            lists[i].push(a.to_string());
                        }
                    }
                    _ => {
                        return self.fail(
                            ctx,
                            codes::ASSERT_LIST_ITEM_NOT_NAME,
                            format!(
                                "{key}= of @{kw} is a list of attribute names, such as {key}=[\"model\"], and {} is none (15.3)",
                                item.show()
                            ),
                            at,
                        )
                    }
                }
            }
        }
        let [same, different, has] = lists;
        // the attributes of terminals and nets are fixed, and terminals have no name (13.6, 15.3)
        let allowed: Option<(&str, &str)> = match kw {
            "ASSERT-TERMINAL" => Some(("net", "a terminal has the attribute net")),
            "ASSERT-NET" => Some(("name", "a net has the attribute name")),
            _ => None,
        };
        if let Some((allowed, what)) = allowed {
            for (key, list) in [("same", &same), ("different", &different)] {
                if let Some(a) = list.iter().find(|a| *a != allowed) {
                    return self.fail(
                        ctx,
                        codes::ASSERT_ATTRIBUTE_NOT_OF_KIND,
                        format!("{key}= of @{kw} names the attribute {a}, but {what} only (13.6, 15.3)"),
                        at,
                    );
                }
            }
        }
        if let Some(n) = get(&p, "terminals").and_then(Lit::num) {
            if n < 0.0 || n.fract() != 0.0 {
                return self.fail(
                    ctx,
                    codes::ASSERT_TERMINALS_NOT_WHOLE,
                    format!(
                        "terminals= of @{kw} is a whole number of terminals, such as terminals=4, not {} (10.1, 15.3)",
                        crate::geom::fmt_num(n)
                    ),
                    at,
                );
            }
        }
        let message = get(&p, "message").and_then(Lit::str).map(String::from);
        let conds: Vec<(String, Lit)> =
            conditions(kw).iter().filter_map(|c| get(&p, c).map(|v| (c.to_string(), v.clone()))).collect();
        let positions = words().keyword(kw).map(|s| s.position_list()).unwrap_or(&[]);
        let mapped = self.map_args(kw, args, seg, ctx, positions)?;
        let members = mapped["members"].obj;
        if let Some(d) = self.failed_obj(members).or_else(|| self.failed_written_member(members)) {
            let id = self.err_cause(
                ctx,
                codes::ASSERT_MEMBER_FAILED,
                format!("a member of @{kw} failed, so the assertion checks nothing (7.3, 15.3)"),
                seg.start,
                d,
            );
            return Err(Stop::Fail(Some(id)));
        }
        // in a record with priority="disabled" an assertion is not checked (15.3)
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
        let a = Assertion {
            kw: kw.to_string(),
            conds,
            same,
            different,
            has,
            message,
            members,
            site: Rc::new(ctx.clone()),
            off: at,
            chain: self.apply_chain(ctx),
        };
        // checked after the geometry phase, when every condition can be read and every PRIORITY is in
        let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, _| it.check_assertion(s, &a));
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

    /// A failed object among the members written in a plain set, also in a nested one.
    fn failed_written_member(&self, set: ObjId) -> Option<Option<crate::diag::DiagId>> {
        let ObjData::Set(sd) = self.o(set) else { return None };
        if !matches!(sd.kind, SetKind::Plain) {
            return None;
        }
        sd.items.iter().find_map(|(o, _)| self.failed_obj(*o).or_else(|| self.failed_written_member(*o)))
    }

    /// A diagnostic of an assertion, at the statement. A violation concerns the document's circuit,
    /// so it is shown also when the statement is in a package the document uses (7.6).
    fn assertion_diag(&mut self, sev: Severity, code: Code, a: &Assertion, msg: String) {
        if sev == Severity::Error {
            self.note_failed_at(a.site.rec, a.off, false);
        }
        let loc = self.rec_loc(a.site.rec, a.off);
        let extra = Extra { cause: None, origin: a.site.origin.to_string(), in_package: String::new() };
        self.sink.add(sev, code, msg, loc, extra);
    }

    /// Where an assertion is checked, as its message says it: in the body of a usage, the usage, its
    /// definition and where the usage is written, such as ` in %dp1, a usage of $PAIR at t.sp:12`.
    fn assertion_where(&self, site: &Ctx) -> String {
        let Some(u) = site.frame_usage() else { return String::new() };
        let ud = self.usage(u);
        let def = self.label(ud.definition);
        let at = ud
            .site
            .as_ref()
            .and_then(|c| self.rec_loc(c.rec, ud.off))
            .map(|l| format!(" at {}:{}", crate::host::file_name(&l.file), l.line))
            .unwrap_or_default();
        format!(" in {}, a usage of {def}{at}", self.label(u))
    }

    /// The instance whose attributes an instance member has: a bundle by its first member (11.8).
    fn asserted_instance(&self, m: ObjId) -> Option<ObjId> {
        match self.o(m) {
            ObjData::Inst { .. } => Some(m),
            ObjData::Bundle { members, .. } => members.first().copied(),
            _ => None,
        }
    }

    /// How a member fails a condition, if it does: what it has instead.
    fn condition_fails(&mut self, s: SceneId, kw: &str, m: ObjId, cond: &str, v: &Lit) -> Option<String> {
        let lv = self.scenes[s].level;
        match kw {
            "ASSERT-INSTANCE" | "ASSERT-TERMINAL" if matches!(cond, "name" | "model" | "type") => {
                let owner = match self.o(m) {
                    ObjData::Term { owner, .. } => *owner,
                    _ => m,
                };
                let Some(i) = self.asserted_instance(owner) else {
                    return Some("is a terminal of no device".to_string());
                };
                let inst = self.inst_of(i).clone();
                let want = v.str().unwrap_or_default();
                let cs = inst.case_sensitive;
                // as SELECT-INSTANCE and SELECT-TERMINAL compare them (10.1)
                let (has, ok) = match cond {
                    "name" => (Some(inst.name.clone()), glob(want, &inst.name, cs)),
                    "model" => (inst.model.clone(), inst.model.as_deref().is_some_and(|x| glob(want, x, cs))),
                    _ => (inst.mtype.clone(), inst.mtype.as_deref().unwrap_or("") == want.to_lowercase()),
                };
                let of = if kw == "ASSERT-TERMINAL" { "is a terminal of a device with" } else { "has" };
                (!ok).then(|| match has {
                    Some(x) => format!("{of} {cond} {}", quoted(&x)),
                    None => format!("{of} no {cond}"),
                })
            }
            "ASSERT-INSTANCE" => {
                let i = self.asserted_instance(m)?;
                (cond == "terminals").then_some(())?;
                let n = self.inst_of(i).nodes.len();
                let want = v.num().unwrap_or(0.0);
                (n as f64 != want).then(|| format!("has {n} terminal{}", if n == 1 { "" } else { "s" }))
            }
            "ASSERT-TERMINAL" => {
                let ObjData::Term { owner, index, name } = self.o(m).clone() else { return None };
                match cond {
                    // as SELECT-TERMINAL compares it (10.1)
                    "terminal" => {
                        let ok = match v {
                            Lit::Num(n) => index == *n as u32,
                            Lit::Str(w) => name.as_ref().is_some_and(|n| {
                                if self.host_case(owner) {
                                    n == w.as_str()
                                } else {
                                    n.eq_ignore_ascii_case(w.as_str())
                                }
                            }),
                            _ => false,
                        };
                        (!ok).then(|| match &name {
                            Some(n) => format!("is terminal {index} ({n})"),
                            None => format!("is terminal {index}"),
                        })
                    }
                    "net" => {
                        let want = v.str().unwrap_or_default().to_string();
                        let nets = self.tnets(s, m);
                        let ok = nets.iter().any(|n| match self.o(*n) {
                            ObjData::Net { key, .. } => name_match(self, lv, &want, key),
                            _ => false,
                        });
                        (!ok).then(|| {
                            let shown: Vec<String> = nets.iter().map(|n| self.label(*n)).collect();
                            if shown.is_empty() {
                                "is on no net".to_string()
                            } else {
                                format!("is on {}", and_list(&shown))
                            }
                        })
                    }
                    "current" => {
                        let has = self.current_of(s, m);
                        (Some(has.as_str()) != v.str()).then(|| format!("has current {}", quoted(&has)))
                    }
                    _ => None,
                }
            }
            _ => {
                let ObjData::Net { key, .. } = self.o(m).clone() else { return None };
                let want = v.boolean();
                let (yes, no, is) = match cond {
                    "name" => {
                        let pat = v.str().unwrap_or_default().to_string();
                        return (!name_match(self, lv, &pat, &key))
                            .then(|| format!("is named {}", quoted(&self.net_display(m))));
                    }
                    "port" => {
                        let file = self.doc.levels[lv].file.clone();
                        let port = self.doc.levels[lv].ports.iter().any(|p| self.doc.net_key_in(lv, p, &file) == key);
                        ("is a port of this level", "is not a port of this level", port)
                    }
                    "global" => ("is a global net", "is not a global net", self.doc.is_global(lv, &key)),
                    // as SELECT-NET reads it (10.2)
                    "wired" => {
                        let ends = self.wire_ends(s);
                        ("is wired", "is not wired", self.net_wired(s, m, &ends))
                    }
                    _ => return None,
                };
                (Some(is) != want).then(|| if is { yes } else { no }.to_string())
            }
        }
    }

    /// What a condition asks, as a message says it after "every member should".
    fn condition_asked(kw: &str, cond: &str, v: &Lit) -> String {
        let yes = v.boolean() == Some(true);
        let either = |a: &str, b: &str| if yes { a.to_string() } else { b.to_string() };
        match cond {
            "name" if kw == "ASSERT-NET" => format!("be named {}", v.show()),
            "name" | "model" | "type" if kw == "ASSERT-TERMINAL" => {
                format!("be a terminal of a device with {cond} {}", v.show())
            }
            "name" | "model" | "type" => format!("have {cond} {}", v.show()),
            "terminals" => format!("have {} terminals", v.show()),
            "terminal" => format!("be terminal {}", v.show()),
            "net" => format!("be on net {}", v.show()),
            "current" => format!("have current {}", v.show()),
            "port" => either("be a port of this level", "not be a port of this level"),
            "wired" => either("be wired", "not be wired"),
            "global" => either("be a global net", "not be a global net"),
            _ => format!("meet {cond}={}", v.show()),
        }
    }

    /// The value of an attribute of a member (13.6), none when it has none.
    fn attribute_of(&mut self, s: SceneId, kw: &str, m: ObjId, attr: &str) -> Option<AttrValue> {
        match kw {
            "ASSERT-INSTANCE" => {
                let i = self.asserted_instance(m)?;
                let inst = self.inst_of(i).clone();
                let fold = |x: &str| if inst.case_sensitive { x.to_string() } else { x.to_lowercase() };
                let (shown, key) = match attr {
                    "name" => (inst.name.clone(), fold(&inst.name)),
                    "model" => {
                        let x = inst.model.clone()?;
                        (x.clone(), fold(&x))
                    }
                    "type" => {
                        let x = inst.mtype.clone()?;
                        (x.clone(), x.to_lowercase())
                    }
                    "value" => {
                        let x = inst.value.clone()?;
                        (x.clone(), x)
                    }
                    _ => {
                        let x = inst.param(attr)?.to_string();
                        (x.clone(), x)
                    }
                };
                Some(AttrValue { shown: quoted(&shown), key })
            }
            "ASSERT-TERMINAL" => {
                let nets = self.tnets(s, m);
                if nets.is_empty() {
                    return None;
                }
                let shown: Vec<String> = nets.iter().map(|n| self.label(*n)).collect();
                let mut ids: Vec<u32> = nets.iter().map(|n| n.0).collect();
                ids.sort_unstable();
                Some(AttrValue { shown: format!("net {}", shown.join(",")), key: format!("{ids:?}") })
            }
            _ => {
                let ObjData::Net { key, .. } = self.o(m).clone() else { return None };
                Some(AttrValue { shown: quoted(&self.net_display(m)), key })
            }
        }
    }

    /// What an attribute is called in a message: a parameter of an instance as such.
    fn attribute_called(kw: &str, attr: &str) -> String {
        match (kw, attr) {
            ("ASSERT-INSTANCE", "name" | "model" | "type" | "value") | (_, "net" | "name") => attr.to_string(),
            _ => format!("parameter {attr}"),
        }
    }

    /// An assertion checked in a scene (15.3): a diagnostic for each check that does not hold.
    fn check_assertion(&mut self, s: SceneId, a: &Assertion) {
        // in the body of a usage that is disabled, as in a disabled record, nothing is checked (15.3)
        if !a.chain.is_empty() && self.eff_priority(s, None, &a.chain, None) == Priority::Disabled {
            return;
        }
        let kw = a.kw.as_str();
        let members: Vec<ObjId> =
            if self.is_set(a.members) { self.flatten(s, a.members, 7, Some(&a.site)) } else { vec![a.members] };
        let want = kind_of_assertion(kw);
        // a terminal converts to its net (5.2.4)
        let members: Vec<ObjId> = members
            .into_iter()
            .flat_map(|m| match (want, self.o(m)) {
                ("net", ObjData::Term { owner, .. }) if !matches!(self.o(*owner), ObjData::Usage(_)) => {
                    let ns = self.tnets(s, m);
                    if ns.is_empty() {
                        vec![m]
                    } else {
                        ns
                    }
                }
                _ => vec![m],
            })
            .collect();
        let wrong: Vec<String> = members
            .iter()
            .filter(|&&m| !match want {
                "instance" => matches!(self.o(m), ObjData::Inst { .. } | ObjData::Bundle { .. }),
                "terminal" => {
                    matches!(self.o(m), ObjData::Term { owner, .. } if !matches!(self.o(*owner), ObjData::Usage(_)))
                }
                _ => matches!(self.o(m), ObjData::Net { .. }),
            })
            .map(|&m| {
                let kind = self.kind_of(m);
                let a = if kind.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
                format!("{} ({a} {kind})", self.label(m))
            })
            .collect();
        if !wrong.is_empty() {
            let msg = format!(
                "@{kw} checks {want}s, and {} {} not: the assertion is not checked (15.3)",
                and_list(&wrong),
                if wrong.len() == 1 { "is" } else { "are" }
            );
            self.assertion_diag(Severity::Error, codes::ASSERT_MEMBER_WRONG_KIND, a, msg);
            return;
        }
        if members.is_empty() {
            return;
        }
        let place = self.assertion_where(&a.site);
        let note = a.message.as_ref().map(|m| format!("; {m}")).unwrap_or_default();
        let mut problems: Vec<(Code, String)> = Vec::new();
        for (cond, v) in &a.conds {
            let mut bad = Vec::new();
            for &m in &members {
                if let Some(has) = self.condition_fails(s, kw, m, cond, v) {
                    bad.push(format!("{} {has}", self.label(m)));
                }
            }
            if !bad.is_empty() {
                let asked = Self::condition_asked(kw, cond, v);
                problems
                    .push((codes::VIOLATION_CONDITION, format!("every member should {asked}, but {}", and_list(&bad))));
            }
        }
        for attr in &a.same {
            let called = Self::attribute_called(kw, attr);
            let mut by_value: Vec<(String, String, Vec<String>)> = Vec::new();
            let mut without = Vec::new();
            for &m in &members {
                let l = self.label(m);
                match self.attribute_of(s, kw, m, attr) {
                    None => without.push(l),
                    Some(v) => match by_value.iter_mut().find(|(k, _, _)| *k == v.key) {
                        Some((_, _, ls)) => ls.push(l),
                        None => by_value.push((v.key, v.shown, vec![l])),
                    },
                }
            }
            // an attribute a member lacks is a value of its own, "absent": members that all lack it
            // agree (15.3)
            if by_value.len() + usize::from(!without.is_empty()) <= 1 {
                continue;
            }
            let mut parts: Vec<String> = by_value
                .iter()
                .map(|(_, shown, ls)| format!("{} {} {shown}", and_list(ls), has_have(ls.len())))
                .collect();
            if !without.is_empty() {
                parts.push(format!("{} {} no {called}", and_list(&without), has_have(without.len())));
            }
            let msg = format!("the members should have the same {called}, but {}", and_list(&parts));
            problems.push((codes::VIOLATION_SAME, msg));
        }
        for attr in &a.different {
            let called = Self::attribute_called(kw, attr);
            let mut seen: HashMap<String, (String, Vec<String>)> = HashMap::new();
            let mut order: Vec<String> = Vec::new();
            let mut without = Vec::new();
            let mut twice: Vec<String> = Vec::new();
            let mut met: HashSet<ObjId> = HashSet::new();
            for &m in &members {
                let l = self.label(m);
                if !met.insert(m) {
                    if !twice.contains(&l) {
                        twice.push(l);
                    }
                    continue;
                }
                match self.attribute_of(s, kw, m, attr) {
                    None => without.push(l),
                    Some(v) => {
                        let e = seen.entry(v.key.clone()).or_insert_with(|| {
                            order.push(v.key.clone());
                            (v.shown.clone(), Vec::new())
                        });
                        e.1.push(l);
                    }
                }
            }
            let mut parts: Vec<String> = twice.iter().map(|l| format!("{l} is among them more than once")).collect();
            for k in &order {
                let (shown, ls) = &seen[k];
                if ls.len() > 1 {
                    parts.push(format!("{} have the same {called} {shown}", and_list(ls)));
                }
            }
            // "absent" is a value too: one member without it differs from the others, two are equal (15.3)
            if without.len() > 1 {
                parts.push(format!("{} have no {called}", and_list(&without)));
            }
            if !parts.is_empty() {
                let msg = format!("any two members should have a different {called}, but {}", and_list(&parts));
                problems.push((codes::VIOLATION_DIFFERENT, msg));
            }
        }
        for attr in &a.has {
            let called = Self::attribute_called(kw, attr);
            let mut without = Vec::new();
            for &m in &members {
                if self.attribute_of(s, kw, m, attr).is_none() {
                    without.push(self.label(m));
                }
            }
            if !without.is_empty() {
                let msg = format!(
                    "every member should have a {called}, but {} {} none",
                    and_list(&without),
                    has_have(without.len())
                );
                problems.push((codes::VIOLATION_HAS, msg));
            }
        }
        for (code, problem) in problems {
            let msg = format!("@{kw} does not hold{place}: {problem}{note} (15.3)");
            self.assertion_diag(Severity::Violation, code, a, msg);
        }
    }
}
