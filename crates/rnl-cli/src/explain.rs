//! `rnl explain`: the usages an object is a member of, from the Document's reverse index
//! (`rnl_view::Object::usages`): for each, a line that says the roles the object fills, of which
//! usage of which definition, where it is written, where it comes from and, when it does not take
//! effect, why, with the usages that contain that usage in turn. The groups (11.5) are listed the
//! same way: the groups the object is a member of, and of a group, the groups it is within.

use std::collections::HashSet;

use rnl_runtime::model::quoted;
use rnl_view::{Document, MemberOf};
use serde_json::{json, Value};

/// An object that a reference finds: its index in `Document::objects`, and the terminal the
/// reference names, if any.
#[derive(Debug, PartialEq)]
pub struct Found {
    pub object: usize,
    pub terminal: Option<String>,
}

/// An object's id, as a reference writes it: `#M1`, `&vdd`, `%dp1`.
fn id_of(doc: &Document, object: usize) -> String {
    let o = &doc.objects[object];
    match o.kind {
        "instance" => format!("#{}", quoted(&o.name)),
        "net" => format!("&{}", quoted(&o.name)),
        _ => o.name.clone(),
    }
}

/// The objects `reference` names, one in each level that has it: in the level titled `level` only,
/// when it is given. A reference that names none is an error that lists the names closest to it.
pub fn find(doc: &Document, reference: &str, level: Option<&str>) -> Result<Vec<Found>, String> {
    if let Some(t) = level {
        if !doc.objects.iter().any(|o| o.level_title == t) {
            let mut titles: Vec<&str> = doc.levels.iter().map(|l| l.title.as_str()).collect();
            titles.sort();
            titles.dedup();
            return Err(format!("no level is titled {t}; the levels are {}", titles.join(", ")));
        }
    }
    let in_level = |k: &usize| level.is_none_or(|t| doc.objects[*k].level_title == t);
    let candidates: Vec<usize> = (0..doc.objects.len()).filter(in_level).collect();
    // an instance's terminal: `#M5:d`, by name or by number
    let (base, terminal) = match reference.rsplit_once(':') {
        Some((b, t)) if b.starts_with('#') && !b.ends_with(':') && !t.is_empty() => (b, Some(t)),
        _ => (reference, None),
    };
    let by = |exact: bool| -> Vec<usize> {
        candidates
            .iter()
            .copied()
            .filter(|&k| {
                let id = id_of(doc, k);
                if exact {
                    id == base
                } else {
                    id.eq_ignore_ascii_case(base)
                }
            })
            .collect()
    };
    let mut found = by(true);
    if found.is_empty() {
        found = by(false);
    }
    if found.is_empty() {
        return Err(not_found(doc, reference, &candidates));
    }
    Ok(found.into_iter().map(|object| Found { object, terminal: terminal.map(String::from) }).collect())
}

/// The error for a reference that names no object, with up to five names closest to it.
fn not_found(doc: &Document, reference: &str, candidates: &[usize]) -> String {
    let mut ids: Vec<String> = candidates.iter().map(|&k| id_of(doc, k)).collect();
    ids.sort();
    ids.dedup();
    let want = reference.to_lowercase();
    let mut close: Vec<(usize, String)> = ids
        .into_iter()
        .map(|id| (distance(&want, &id.to_lowercase()), id))
        .filter(|(d, id)| *d <= 2.max(want.len() / 3) || id.to_lowercase().starts_with(&want))
        .collect();
    close.sort();
    let names: Vec<String> = close.into_iter().take(5).map(|(_, id)| id).collect();
    if names.is_empty() {
        format!("no object is {reference}")
    } else {
        format!("no object is {reference}; close names: {}", names.join(", "))
    }
}

/// The edit distance of two texts, by characters.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The entries of `found`: those of its terminal only, when the reference names one.
fn entries<'d>(doc: &'d Document, found: &Found) -> Vec<&'d MemberOf> {
    let usages = &doc.objects[found.object].usages;
    let Some(t) = &found.terminal else { return usages.iter().collect() };
    let number = format!("{}:{t}", id_of(doc, found.object));
    usages
        .iter()
        .filter(|e| {
            e.roles
                .iter()
                .any(|r| r.terminal.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(t)) || r.member == number)
        })
        .collect()
}

/// One line of an entry, said as a sentence: the roles the object fills, of which usage of which
/// definition, or the group it is a member of or within, then where it is written, where it comes
/// from and, when it does not take effect, its state with the reason. On a line of the chain
/// upward, `subject` is the usage or group that fills those roles.
fn line(e: &MemberOf, subject: Option<&str>) -> String {
    let roles: Vec<String> = e
        .roles
        .iter()
        .map(|r| {
            let role = if r.set { format!("one of {{{}}}", r.role) } else { r.role.clone() };
            match &r.terminal {
                Some(t) => format!("{role} (through terminal {t})"),
                None => role,
            }
        })
        .collect();
    let mut said = match (e.kind, subject) {
        // a group within this one: its members are all among this one's (5.2, 11.5)
        ("group", Some(s)) if roles.is_empty() => format!("{s} is in turn within group {}", e.id),
        ("group", None) if roles.is_empty() => format!("within group {}", e.id),
        ("group", _) => {
            let member = match e.roles.iter().find_map(|r| r.terminal.as_ref()) {
                Some(t) => format!("member (through terminal {t})"),
                None => "member".into(),
            };
            match subject {
                Some(s) => format!("{s} is in turn a {member} of group {}", e.id),
                None => format!("{member} of group {}", e.id),
            }
        }
        (_, Some(s)) => format!("{s} is in turn {} of usage {}", roles.join(" and "), e.id),
        (_, None) => format!("{} of usage {}", roles.join(" and "), e.id),
    };
    if !e.definition.is_empty() {
        said.push_str(&format!(" (definition {})", e.definition));
    }
    let at = match &e.file {
        Some(f) => format!("{f}:{}", e.line),
        None => "-".into(),
    };
    let source = match (e.source, &e.rule, &e.made_in) {
        ("rule", Some(r), _) if r.package.is_empty() => format!("made by the rule at {}:{}", r.file, r.line),
        ("rule", Some(r), _) => format!("made by the rule of {} at {}:{}", r.package, r.file, r.line),
        ("body", _, Some(p)) => format!("made by the body of {p}"),
        ("solver", _, _) => "written back by a solver".into(),
        (s, _, _) => s.to_string(),
    };
    let mut out = format!("{said} · {at} · {source}");
    if e.state != "in-effect" {
        out.push_str(" · ");
        out.push_str(&e.state.to_uppercase());
        if let Some(r) = &e.reason {
            out.push_str(&format!(": {r}"));
        }
    }
    out
}

/// The lines `rnl explain` prints for `found`: one for each usage, each followed by the usages
/// that contain that usage, indented, up the chain.
pub fn lines(doc: &Document, found: &[Found]) -> Vec<String> {
    let mut out = Vec::new();
    for f in found {
        let o = &doc.objects[f.object];
        let what = match &f.terminal {
            Some(t) => format!("{}:{t}", id_of(doc, f.object)),
            None => id_of(doc, f.object),
        };
        let list = entries(doc, f);
        let groups = list.iter().filter(|e| e.kind == "group").count();
        out.push(format!("{what} in {}: {}", o.level_title, count(list.len() - groups, groups)));
        for e in list {
            chain(doc, e, None, 1, &mut HashSet::new(), &mut out);
        }
    }
    out
}

fn count(usages: usize, groups: usize) -> String {
    let some = |n: usize, one: &str| match n {
        1 => format!("1 {one}"),
        n => format!("{n} {one}s"),
    };
    match (usages, groups) {
        (0, 0) => "in no usage or group".into(),
        (n, 0) => some(n, "usage"),
        (0, g) => some(g, "group"),
        (n, g) => format!("{}, {}", some(n, "usage"), some(g, "group")),
    }
}

fn chain(
    doc: &Document,
    e: &MemberOf,
    subject: Option<&str>,
    depth: usize,
    seen: &mut HashSet<usize>,
    out: &mut Vec<String>,
) {
    let mut text = format!("{}{}", "  ".repeat(depth), line(e, subject));
    let Some(u) = e.usage else {
        out.push(text);
        return;
    };
    // a usage already on the way up: the chain loops, and stops here
    if !seen.insert(u) {
        text.push_str(&format!(" · the chain loops back to {}", e.id));
        out.push(text);
        return;
    }
    out.push(text);
    for outer in &doc.objects[u].usages {
        chain(doc, outer, Some(&e.id), depth + 1, seen, out);
    }
    seen.remove(&u);
}

/// What `rnl explain --json` prints for `found`: for each object found, its level, its id and its
/// usages, each a `MemberOf` with `outer`, the usages that contain that usage, the same way.
pub fn json(doc: &Document, found: &[Found]) -> Value {
    let objects: Vec<Value> = found
        .iter()
        .map(|f| {
            let o = &doc.objects[f.object];
            let usages: Vec<Value> =
                entries(doc, f).into_iter().map(|e| entry_json(doc, e, &mut HashSet::new())).collect();
            json!({
                "level": o.level_title,
                "object": f.object,
                "id": id_of(doc, f.object),
                "kind": o.kind,
                "terminal": f.terminal,
                "usages": usages,
            })
        })
        .collect();
    json!({ "objects": objects })
}

fn entry_json(doc: &Document, e: &MemberOf, seen: &mut HashSet<usize>) -> Value {
    let mut v = serde_json::to_value(e).expect("an entry is JSON");
    let mut outer = Vec::new();
    if let Some(u) = e.usage {
        if seen.insert(u) {
            outer = doc.objects[u].usages.iter().map(|x| entry_json(doc, x, seen)).collect();
            seen.remove(&u);
        }
    }
    v["outer"] = Value::Array(outer);
    v
}
