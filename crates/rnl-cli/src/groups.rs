//! `rnl groups`: the tree of groups (11.5) of each drawing level, with the parts each node holds
//! directly, as `rnl_runtime::group_nodes` gives them: the level at the root, each group under the
//! smallest group of the tree that holds it. A part is a device, an instance or a bundle, or a
//! virtual part, a usage with a symbol of its own, counted apart. The groups outside the tree
//! follow it: one with the members of another, one that crosses another (they share members
//! without either holding the other), one with no member with a place of its own, and one PRIORITY
//! turns off. A level of a language before 0.4 has no tree of groups: each group is listed with all
//! its members. Nothing is judged here; `rnl check` has the hint of a crowded node.

use rnl_runtime::diag::{codes, Code, Loc};
use rnl_runtime::group_nodes::{applied_at, group_name, held, label, member_ids, statement_at, tree, Held, Node};
use rnl_runtime::model::{GroupInfo, GroupPlace, LevelView, UsageOrigin};
use rnl_runtime::Interpretation;
use serde_json::{json, Value};

/// Where a group comes from and where it is written, as the end of its line says it: for one the
/// body of a definition makes, where the document applies the definition
/// (`group_nodes::applied_at`), or else where its GROUP is.
fn source(r: &Interpretation, lv: &LevelView, k: usize) -> (String, Value) {
    let g = &lv.groups[k];
    // where a statement is written: its span begins after the end of the one before it
    let written_at = |(start, end): &(Loc, Loc)| {
        let text = r.sources.iter().find(|(f, _)| *f == start.file).map_or("", |(_, t)| t.as_str());
        let at = statement_at(text, start, end);
        format!("{}:{}", at.file, at.line)
    };
    let at = g.source.as_ref().map(written_at).unwrap_or_else(|| "-".into());
    match &g.origin {
        UsageOrigin::Written if g.id.starts_with('%') => {
            (format!("written at {at}"), json!({"kind": "written", "at": at}))
        }
        UsageOrigin::Written => ("written".into(), json!({"kind": "written", "at": at})),
        UsageOrigin::Solver => (format!("written back by a solver at {at}"), json!({"kind": "solver", "at": at})),
        UsageOrigin::Rule { package, at } => {
            let rule = format!("{}:{}", at.file, at.line);
            let text = if package.is_empty() {
                format!("made by the rule at {rule}")
            } else {
                format!("made by the rule of {package} at {rule}")
            };
            (text, json!({"kind": "rule", "package": package, "rule": rule}))
        }
        UsageOrigin::Body => {
            let written = |file: &str| r.doc.sources.iter().any(|(f, _)| f == file);
            let applied = applied_at(lv, k, &written).map(written_at);
            // a usage that a rule of a package makes, up the chain, is written in no file
            let mut rule = None;
            let mut usage = g.parent.as_ref();
            for _ in 0..=lv.objects.len() {
                let Some(o) = usage.and_then(|u| lv.objects.iter().find(|o| &o.id == u)) else { break };
                if let UsageOrigin::Rule { package, .. } = &o.origin {
                    rule = Some(package.clone());
                    break;
                }
                usage = o.parent.as_ref();
            }
            match (applied, rule) {
                (Some(at), _) => (format!("applied at {at}"), json!({"kind": "body", "usage": g.parent, "at": at})),
                (None, Some(p)) if !p.is_empty() => {
                    (format!("made by a rule of {p}"), json!({"kind": "body", "usage": g.parent, "package": p}))
                }
                (None, _) => (g.id.clone(), json!({"kind": "body", "usage": g.parent, "at": g.id})),
            }
        }
    }
}

/// What a node holds, as its line says it: the devices, then the virtual parts, all of them or
/// how many, then what else is a member.
fn held_text(lv: &LevelView, h: &Held, all: bool) -> String {
    let devices: Vec<String> = h.devices.iter().map(|&k| lv.objects[k].id.clone()).collect();
    let mut out = devices.join(" ");
    if !h.virtuals.is_empty() {
        let virtuals = if all {
            let names: Vec<String> = h.virtuals.iter().map(|&k| label(&lv.objects[k])).collect();
            format!("{} virtual: {}", h.virtuals.len(), names.join(" "))
        } else {
            format!("{} virtual", h.virtuals.len())
        };
        if out.is_empty() {
            out = virtuals;
        } else {
            out.push_str(&format!(" + {virtuals}"));
        }
    }
    if !h.other.is_empty() {
        if !out.is_empty() {
            out.push_str("; ");
        }
        out.push_str(&format!("also {}", h.other.join(" ")));
    }
    out
}

fn devices(n: usize) -> String {
    if n == 1 {
        "1 device".into()
    } else {
        format!("{n} devices")
    }
}

fn held_json(lv: &LevelView, h: &Held) -> Value {
    let virtuals: Vec<Value> =
        h.virtuals.iter().map(|&k| json!({"id": lv.objects[k].id, "label": label(&lv.objects[k])})).collect();
    json!({
        "devices": h.devices.iter().map(|&k| lv.objects[k].id.clone()).collect::<Vec<_>>(),
        "virtual": virtuals,
        "other": h.other,
    })
}

/// A group outside the tree: what it is, `same`, `crosses`, `empty` or `disabled`, or `flat` in a
/// level without a tree, and the group of the tree it is the same as or crosses.
fn standing(g: &GroupInfo, tree: bool) -> (&'static str, Option<usize>) {
    match g.place {
        _ if g.disabled => ("disabled", None),
        _ if !tree => ("flat", None),
        GroupPlace::Same(t) => ("same", Some(t)),
        GroupPlace::Crosses(t) => ("crosses", Some(t)),
        GroupPlace::Empty => ("empty", None),
        _ => ("disabled", None),
    }
}

/// The levels `rnl groups` lists: every drawing level, or the one titled `title`. A title that no
/// level has is an error that lists the titles.
pub fn levels<'r>(r: &'r Interpretation, title: Option<&str>) -> Result<Vec<&'r LevelView>, (Code, String)> {
    if let Some(t) = title {
        if !r.model.levels.iter().any(|lv| lv.title == t) {
            let mut titles: Vec<&str> = r.model.levels.iter().map(|lv| lv.title.as_str()).collect();
            titles.sort();
            titles.dedup();
            let text = format!("no level is titled {t}; the levels are {}", titles.join(", "));
            return Err((codes::CLI_EXPLAIN_LEVEL_NOT_FOUND, text));
        }
    }
    Ok(r.model.levels.iter().filter(|lv| title.is_none_or(|t| lv.title == t)).collect())
}

fn count(nodes: &[Node]) -> usize {
    nodes.iter().map(|n| 1 + count(&n.children)).sum()
}

fn groups(n: usize) -> String {
    if n == 1 {
        "1 group".into()
    } else {
        format!("{n} groups")
    }
}

/// The groups of `lv` outside its tree, in their order: all of them when it has no tree.
fn outside(lv: &LevelView, root: Option<&Node>) -> Vec<usize> {
    fn taken(n: &Node, out: &mut Vec<usize>) {
        out.extend(n.group);
        for c in &n.children {
            taken(c, out);
        }
    }
    let mut inside = Vec::new();
    if let Some(r) = root {
        taken(r, &mut inside);
    }
    (0..lv.groups.len()).filter(|k| !inside.contains(k)).collect()
}

/// The parts of a level without a tree that are in none of its groups.
fn ungrouped(lv: &LevelView) -> Held {
    let grouped: Vec<String> = lv.groups.iter().flat_map(member_ids).collect();
    let mut h = held(lv, lv.objects.iter().filter(|o| !grouped.contains(&o.id)).map(|o| o.id.clone()));
    h.other.clear();
    h
}

/// The lines `rnl groups` prints: for each level, its title and language, then its tree, a line
/// for each node indented by its depth, `name · what it holds directly · N devices (where)`; the
/// level first, `(level)`, with the parts in no group; then the groups outside the tree. The
/// virtual parts are counted, and named too with `all`.
pub fn lines(r: &Interpretation, levels: &[&LevelView], all: bool) -> Vec<String> {
    let mut out = Vec::new();
    for (k, lv) in levels.iter().enumerate() {
        if k > 0 {
            out.push(String::new());
        }
        let root = tree(lv);
        let left = outside(lv, root.as_ref());
        match &root {
            Some(r) => {
                let mut head = format!("{} · language {} · {}", lv.title, lv.language, groups(count(&r.children)));
                if !left.is_empty() {
                    head.push_str(&format!(", {} outside the tree", left.len()));
                }
                out.push(head);
            }
            None => out.push(format!(
                "{} · language {} · {}; no tree of groups before 0.4, each with all its members",
                lv.title,
                lv.language,
                groups(lv.groups.len())
            )),
        }
        let root_held = root.as_ref().map_or_else(|| ungrouped(lv), |r| r.held.clone());
        out.push(line(lv, "(level)", &root_held, all, None, 1));
        fn node(r: &Interpretation, lv: &LevelView, n: &Node, all: bool, depth: usize, out: &mut Vec<String>) {
            let g = n.group.unwrap();
            let name = group_name(lv, g);
            out.push(line(lv, &name, &n.held, all, Some(source(r, lv, g).0), depth));
            for c in &n.children {
                node(r, lv, c, all, depth + 1, out);
            }
        }
        if let Some(top) = &root {
            for c in &top.children {
                node(r, lv, c, all, 2, &mut out);
            }
            if !left.is_empty() {
                out.push("  outside the tree:".into());
            }
        }
        let depth = if root.is_some() { 2 } else { 1 };
        for &g in &left {
            let group = &lv.groups[g];
            let mut name = group_name(lv, g);
            match standing(group, root.is_some()) {
                ("same", Some(t)) => {
                    name.push_str(&format!(" · the members of {}, which stands for it", group_name(lv, t)))
                }
                ("crosses", Some(t)) => {
                    let theirs = member_ids(&lv.groups[t]);
                    let shared: Vec<String> = member_ids(group).into_iter().filter(|m| theirs.contains(m)).collect();
                    name.push_str(&format!(
                        " · CROSSES {}: they share {} and neither holds the other",
                        group_name(lv, t),
                        shared.join(" ")
                    ))
                }
                ("empty", _) => name.push_str(" · no member with a place of its own"),
                ("disabled", _) => name.push_str(" · turned off by PRIORITY"),
                _ => (),
            }
            out.push(line(lv, &name, &held(lv, member_ids(group)), all, Some(source(r, lv, g).0), depth));
        }
    }
    out
}

/// A node's line: `name · what it holds · N devices (where)`.
fn line(lv: &LevelView, name: &str, h: &Held, all: bool, at: Option<String>, depth: usize) -> String {
    let what = held_text(lv, h, all);
    let mut out = format!("{}{name}", "  ".repeat(depth));
    if !what.is_empty() {
        out.push_str(&format!(" · {what}"));
    }
    out.push_str(&format!(" · {}", devices(h.devices.len())));
    if let Some(at) = at {
        out.push_str(&format!(" ({at})"));
    }
    out
}

/// What `rnl groups --json` prints: for each level, its title, its language, whether it has a tree,
/// the root with the parts in no group and the groups of the tree under it, nested, and the groups
/// outside the tree with what they are. Every virtual part is listed, with the text that names it.
pub fn json(r: &Interpretation, levels: &[&LevelView]) -> Value {
    fn node(r: &Interpretation, lv: &LevelView, n: &Node) -> Value {
        let g = n.group.unwrap();
        let mut v = held_json(lv, &n.held);
        v["id"] = json!(lv.groups[g].id);
        v["name"] = json!(group_name(lv, g));
        v["source"] = source(r, lv, g).1;
        v["groups"] = Value::Array(n.children.iter().map(|c| node(r, lv, c)).collect());
        v
    }
    let levels: Vec<Value> = levels
        .iter()
        .map(|lv| {
            let root = tree(lv);
            let mut first = held_json(lv, &root.as_ref().map_or_else(|| ungrouped(lv), |r| r.held.clone()));
            first["groups"] = Value::Array(root.iter().flat_map(|r| &r.children).map(|c| node(r, lv, c)).collect());
            let outside: Vec<Value> = outside(lv, root.as_ref())
                .into_iter()
                .map(|g| {
                    let group = &lv.groups[g];
                    let (state, with) = standing(group, root.is_some());
                    let mut v = held_json(lv, &held(lv, member_ids(group)));
                    v["id"] = json!(group.id);
                    v["name"] = json!(group_name(lv, g));
                    v["source"] = source(r, lv, g).1;
                    v["state"] = json!(state);
                    v["with"] = json!(with.map(|t| lv.groups[t].id.clone()));
                    v
                })
                .collect();
            json!({
                "level": lv.title,
                "language": lv.language,
                "tree": root.is_some(),
                "root": first,
                "outside": outside,
            })
        })
        .collect();
    json!({ "levels": levels })
}
