//! The usages each object is a member of: the usage table of its level ([`super::Usage`]) turned
//! round, with where each usage comes from and whether it takes effect, and the applications that
//! failed and made no usage. The groups (11.5) are listed as usages too: of an object, those it is
//! a member of; of a group, the groups it is within.
use std::collections::{BTreeSet, HashMap};

use rnl_runtime::diag::Loc;
use rnl_runtime::model::{quoted, Attach, LevelView, ObjectInfo, UsageOrigin};
use rnl_runtime::Interpretation;
use serde::Serialize;

use super::Index;

/// A usage an object is a member of, in the object's level ([`super::Object::usages`]), or a group.
/// The usage's or group's own entry in `Document::objects` lists those it is a member of in turn:
/// the chain upward.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct MemberOf {
    /// The usage or group, an index into `Document::objects`; none for an application that failed,
    /// which made no usage.
    pub usage: Option<usize>,
    /// "usage", or "group", a set marked with GROUP (11.5).
    pub kind: &'static str,
    /// Its id: `%dp1`, or for one without a name of its own, where the statement that made it is,
    /// `@file:line:col`; for a group, where GROUP is.
    pub id: String,
    /// Its definition, such as `$DIFFERENTIAL_PAIR`; empty for a group, and when a failed statement
    /// names none.
    pub definition: String,
    /// The roles the object fills in it, in the order of the definition. For a group, `members`, a
    /// set role; none when the object is a group within this one: a reference to a set is spread
    /// in a set (5.2), so a group is within another when all its members are among the other's,
    /// and it is listed with the smallest such groups only.
    pub roles: Vec<RoleOf>,
    /// Where the statement that applies the definition is: its file, as `Usage::file` names it,
    /// and its first line.
    pub file: Option<String>,
    pub line: u32,
    /// Where the usage comes from: "written", a statement of the document's records; "solver", a
    /// record a solver wrote back (`type="generated"`, 2.8); "rule", a rule of a package or of the
    /// document (10.5), which `rule` names; "body", the body of the definition that the usage
    /// `made_in` applies (9.4).
    pub source: &'static str,
    /// For a usage a rule made: the rule's package, empty for the document's own, and where the
    /// rule is written.
    pub rule: Option<RuleAt>,
    /// For a usage the body of a definition made: the usage whose definition it is, by id.
    pub made_in: Option<String>,
    /// "in-effect"; "failed", the application failed and made no usage (7.2); "overridden", a fact
    /// its statement writes lost a conflict to one of higher priority (8.5); "disabled", its
    /// priority, or that of the usage it is made in, is "disabled" (14.8), and its facts are left
    /// out. A group is only "in-effect" or "disabled" (11.5).
    pub state: &'static str,
    /// Why it failed or was overridden: the message of the diagnostic.
    pub reason: Option<String>,
}

/// A role an object fills in a usage.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RoleOf {
    /// The role, such as `in_p`.
    pub role: String,
    /// The role is a set role (9.4), and the object one of its members.
    pub set: bool,
    /// How the role names the object, as the snapshot writes ids: `#M1`, `#M5:1` for a terminal of
    /// an instance by its number, `&vdd`, `%dp1`, `%g1:p` for a terminal of a usage.
    pub member: String,
    /// For a terminal: its name, such as `d`, when it has one; its number otherwise.
    pub terminal: Option<String>,
}

/// Where a rule is written.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct RuleAt {
    pub package: String,
    pub file: String,
    pub line: u32,
}

/// Give every object of every level the usages it is a member of.
pub(super) fn index_all(r: &Interpretation, ix: &mut Index<'_>) {
    let sources: HashMap<&str, Vec<&str>> = r.sources.iter().map(|(p, s)| (p.as_str(), s.lines().collect())).collect();
    for lv in &r.model.levels {
        for (object, entry) in level(r, lv, ix, &sources) {
            let list = &mut ix.objects[object].usages;
            match list.iter_mut().find(|e| e.id == entry.id && e.usage == entry.usage) {
                Some(e) => e.roles.extend(entry.roles),
                None => list.push(entry),
            }
        }
    }
}

/// The objects of a level that a role names, by the id it names them by: the document object and
/// the terminal's name, if a terminal.
struct Members<'l> {
    level: usize,
    view: &'l LevelView,
    instances: HashMap<String, (usize, usize)>,
    usages: HashMap<&'l str, usize>,
}

impl Members<'_> {
    fn find(&self, ix: &mut Index<'_>, id: &str) -> Option<(usize, Option<String>)> {
        if let Some(&(object, _)) = self.instances.get(id) {
            return Some((object, None));
        }
        if let Some(&object) = self.usages.get(id) {
            return Some((object, None));
        }
        if let Some(written) = id.strip_prefix('&') {
            let name =
                written.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(written).replace("\"\"", "\"");
            return Some((ix.net(self.level, &name), None));
        }
        // a terminal: an instance's by number, a usage's by role
        let (owner, terminal) = id.rsplit_once(':')?;
        if owner.ends_with(':') {
            return None;
        }
        if let Some(&(object, i)) = self.instances.get(owner) {
            let n: usize = terminal.parse().ok()?;
            let name = self.view.terminal_names.get(i).and_then(|t| t.get(n.checked_sub(1)?).cloned().flatten());
            return Some((object, Some(name.unwrap_or_else(|| terminal.to_string()))));
        }
        let &object = self.usages.get(owner)?;
        let name = terminal.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(terminal);
        Some((object, Some(name.replace("\"\"", "\""))))
    }
}

/// Each member of each role, with whether the role is a set role.
fn members(roles: &[(Option<String>, Attach)]) -> Vec<(String, bool, String)> {
    fn walk(role: &str, set: bool, a: &Attach, out: &mut Vec<(String, bool, String)>) {
        match a {
            Attach::Object(id) => out.push((role.to_string(), set, id.clone())),
            Attach::At(_) => {}
            Attach::Set(members) => {
                for m in members {
                    walk(role, true, m, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    for (role, a) in roles {
        walk(role.as_deref().unwrap_or_default(), false, a, &mut out);
    }
    out
}

/// The first line of a statement that holds some of its text, as `Usage::first_line` gives it: a
/// statement begins after the `{` or `;` before it, and lines that hold only a comment marker do
/// not count.
fn first_line((a, b): &(Loc, Loc), sources: &HashMap<&str, Vec<&str>>) -> u32 {
    let Some(lines) = sources.get(a.file.as_str()) else { return a.line };
    let blank = |s: &str| {
        let s = s.trim_start_matches([';', '{']).trim();
        s.trim_start_matches('*').trim_start_matches("//").trim().is_empty()
    };
    (a.line..=b.line)
        .find(|&n| {
            let line = lines.get(n.saturating_sub(1) as usize).copied().unwrap_or_default();
            let from = if n == a.line { a.col.saturating_sub(1) as usize } else { 0 };
            !blank(&line.chars().skip(from).collect::<String>())
        })
        .unwrap_or(a.line)
}

/// Whether a usage or group of the usage table takes effect, as `MemberOf::state` says, with the
/// reason for one overridden.
pub(super) fn state(r: &Interpretation, o: &ObjectInfo) -> (&'static str, Option<String>) {
    let within = |(a, b): &(Loc, Loc), at: &Loc| {
        at.file == a.file && (a.line, a.col) <= (at.line, at.col) && (at.line, at.col) <= (b.line, b.col)
    };
    if o.disabled {
        return ("disabled", None);
    }
    // GROUP takes no part in conflicts (11.5)
    let overridden = o
        .source
        .as_ref()
        .filter(|span| o.kind == "usage" && r.failed.iter().any(|f| f.conflict && within(span, &f.start)));
    match overridden {
        Some(span) => (
            "overridden",
            r.diagnostics()
                .into_iter()
                .find(|d| d.loc.as_ref().is_some_and(|l| within(span, l)))
                .map(|d| d.message.clone()),
        ),
        None => ("in-effect", None),
    }
}

fn level(
    r: &Interpretation,
    lv: &LevelView,
    ix: &mut Index<'_>,
    sources: &HashMap<&str, Vec<&str>>,
) -> Vec<(usize, MemberOf)> {
    let host = &ix.doc.levels[lv.level];
    let instances: HashMap<String, (usize, usize)> = (0..host.instances.len())
        .filter_map(|i| {
            let name = format!("#{}", quoted(&host.instances[i].name));
            ix.insts.get(&(lv.level, i)).map(|&o| (name, (o, i)))
        })
        .collect();
    let listed = super::usages::listed(lv);
    let mut usages = HashMap::new();
    for o in &listed {
        usages.insert(o.id.as_str(), ix.usage_of_kind(lv.level, &o.id, o.kind));
    }
    let found = Members { level: lv.level, view: lv, instances, usages };
    let mut out = Vec::new();
    let mut groups = Vec::new();
    for o in &listed {
        let (state, reason) = state(r, o);
        let (source, rule) = match &o.origin {
            UsageOrigin::Written => ("written", None),
            UsageOrigin::Solver => ("solver", None),
            UsageOrigin::Rule { package, at } => {
                ("rule", Some(RuleAt { package: package.clone(), file: at.file.clone(), line: at.line }))
            }
            UsageOrigin::Body => ("body", None),
        };
        let usage = found.usages[o.id.as_str()];
        let roles: Vec<_> = o.attach.iter().map(|(r, a)| (Some(r.clone()), a.clone())).collect();
        let entry = MemberOf {
            usage: Some(usage),
            kind: o.kind,
            id: o.id.clone(),
            definition: o.definition.clone().unwrap_or_default(),
            roles: Vec::new(),
            file: o.source.as_ref().map(|(a, _)| a.file.clone()),
            line: o.source.as_ref().map_or(0, |span| first_line(span, sources)),
            source,
            rule,
            made_in: o.parent.clone(),
            state,
            reason,
        };
        add(&found, ix, &entry, &roles, &mut out);
        if o.kind == "group" {
            let ids: BTreeSet<String> = members(&roles).into_iter().map(|(_, _, m)| m).collect();
            groups.push((entry, ids));
        }
    }
    // a group within others: the smallest of them
    for (inner, ids) in &groups {
        let outer: Vec<&(MemberOf, BTreeSet<String>)> =
            groups.iter().filter(|(_, o)| ids.len() < o.len() && ids.is_subset(o)).collect();
        for (e, o) in &outer {
            if !outer.iter().any(|(_, m)| m.len() < o.len() && m.is_subset(o)) {
                out.push((inner.usage.unwrap(), MemberOf { roles: Vec::new(), ..e.clone() }));
            }
        }
    }
    for f in &lv.failed_usages {
        let (a, _) = &f.source;
        let entry = MemberOf {
            usage: None,
            kind: "usage",
            id: f.id.clone().unwrap_or_else(|| format!("@{}:{}:{}", a.file, a.line, a.col)),
            definition: f.definition.clone().unwrap_or_default(),
            roles: Vec::new(),
            file: Some(a.file.clone()),
            line: first_line(&f.source, sources),
            source: "written",
            rule: None,
            made_in: None,
            state: "failed",
            reason: Some(f.reason.clone()),
        };
        add(&found, ix, &entry, &f.roles, &mut out);
    }
    out
}

/// `entry` for each object that a member of `roles` names, with the roles it fills.
fn add(
    found: &Members<'_>,
    ix: &mut Index<'_>,
    entry: &MemberOf,
    roles: &[(Option<String>, Attach)],
    out: &mut Vec<(usize, MemberOf)>,
) {
    for (role, set, member) in members(roles) {
        let Some((object, terminal)) = found.find(ix, &member) else { continue };
        let role = RoleOf { role, set, member, terminal };
        match out.iter_mut().find(|(o, e)| *o == object && e.id == entry.id && e.usage == entry.usage) {
            Some((_, e)) => {
                if !e.roles.contains(&role) {
                    e.roles.push(role);
                }
            }
            None => out.push((object, MemberOf { roles: vec![role], ..entry.clone() })),
        }
    }
}
