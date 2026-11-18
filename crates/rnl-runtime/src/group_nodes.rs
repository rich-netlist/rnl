//! The parts each node of a level's tree of groups (11.5) holds directly, for a tool to list
//! (`rnl groups`) and for the hint of a crowded node ([`CROWDED`]).
//!
//! The tree is the one the solver lays out, as [`GroupInfo::place`] gives it: the level at the
//! root, each group of the tree under the smallest group of the tree that holds it, siblings by
//! their first member in the order of the level. A node holds directly the members of no group
//! under it; the root, the parts in no group of the tree. A part is a device, an instance or a
//! bundle, or a virtual part, a usage with a symbol of its own; anything else a group names, such
//! as a net, is neither. Objects PRIORITY turns off are left out.

use std::collections::{BTreeMap, BTreeSet};

use crate::diag::Loc;
use crate::model::{Attach, GroupInfo, GroupPlace, LevelView, ObjectInfo, UsageOrigin};

/// The number of devices a node may hold directly before the hint [`crate::diag::codes::CROWDED_NODE`]
/// suggests grouping its functional blocks: a node of more than this many devices has it. Virtual
/// parts do not count.
pub const CROWDED: usize = 20;

/// What a part is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    /// An instance or a bundle.
    Device,
    /// A usage with a symbol of its own.
    Virtual,
}

/// What `o` is as a part, if it is one.
pub fn part(o: &ObjectInfo) -> Option<Part> {
    match o.kind {
        "instance" | "bundle" => Some(Part::Device),
        "usage" if o.symbol.is_some() => Some(Part::Virtual),
        _ => None,
    }
}

/// The ids of the objects among the members of `g`, in order, each once; points are left out.
pub fn member_ids(g: &GroupInfo) -> Vec<String> {
    fn walk(members: &[Attach], out: &mut Vec<String>) {
        for m in members {
            match m {
                Attach::Object(id) if !out.contains(id) => out.push(id.clone()),
                Attach::Set(inner) => walk(inner, out),
                _ => (),
            }
        }
    }
    let mut out = Vec::new();
    walk(&g.members, &mut out);
    out
}

/// What a node holds directly: its devices and its virtual parts, by their index in
/// [`LevelView::objects`], in the order of the level, and the other members by id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Held {
    pub devices: Vec<usize>,
    pub virtuals: Vec<usize>,
    pub other: Vec<String>,
}

/// The objects of `ids` that `lv` has, sorted into parts, in the order of the level.
pub fn held(lv: &LevelView, ids: impl IntoIterator<Item = String>) -> Held {
    let index: BTreeMap<&str, usize> =
        lv.objects.iter().enumerate().filter(|(_, o)| !o.disabled).map(|(k, o)| (o.id.as_str(), k)).collect();
    let mut out = Held::default();
    let mut seen = BTreeSet::new();
    for id in ids {
        match index.get(id.as_str()).and_then(|&k| Some((k, part(&lv.objects[k])?))) {
            Some((k, Part::Device)) if seen.insert(k) => out.devices.push(k),
            Some((k, Part::Virtual)) if seen.insert(k) => out.virtuals.push(k),
            Some(_) => (),
            None => out.other.push(id),
        }
    }
    out.devices.sort_unstable();
    out.virtuals.sort_unstable();
    out
}

/// A node of the tree: the level (`group` none) or a group, by its index in [`LevelView::groups`],
/// with what it holds directly and the groups under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub group: Option<usize>,
    pub held: Held,
    pub children: Vec<Node>,
}

/// The tree of groups of a level, with what each node holds directly.
pub fn tree(lv: &LevelView) -> Node {
    let members: Vec<Vec<String>> = lv.groups.iter().map(member_ids).collect();
    let index: BTreeMap<&str, usize> = lv.objects.iter().enumerate().map(|(k, o)| (o.id.as_str(), k)).collect();
    let first = |k: usize| members[k].iter().filter_map(|id| index.get(id.as_str()).copied()).min();
    let parent = |k: usize| match lv.groups[k].place {
        GroupPlace::Tree { parent } if !lv.groups[k].disabled => Some(parent),
        _ => None,
    };
    fn build(
        at: Option<usize>,
        lv: &LevelView,
        members: &[Vec<String>],
        parent: &dyn Fn(usize) -> Option<Option<usize>>,
        first: &dyn Fn(usize) -> Option<usize>,
    ) -> Vec<Node> {
        let mut under: Vec<usize> = (0..lv.groups.len()).filter(|&k| parent(k) == Some(at)).collect();
        under.sort_by_key(|&k| (first(k).unwrap_or(usize::MAX), k));
        under
            .into_iter()
            .map(|k| {
                let children = build(Some(k), lv, members, parent, first);
                let below: BTreeSet<&String> = children.iter().flat_map(|c| &members[c.group.unwrap()]).collect();
                let direct = members[k].iter().filter(|m| !below.contains(m)).cloned();
                Node { group: Some(k), held: held(lv, direct), children }
            })
            .collect()
    }
    let children = build(None, lv, &members, &parent, &first);
    let top: BTreeSet<&String> = children.iter().flat_map(|c| &members[c.group.unwrap()]).collect();
    let parts = lv.objects.iter().filter(|o| !o.disabled && part(o).is_some() && !top.contains(&o.id));
    Node { group: None, held: held(lv, parts.map(|o| o.id.clone())), children }
}

/// The text that names an object: its id, or for a usage without a name of its own, whose id is
/// where it is written, its definition with what its first role is attached to, `$GROUND(&0)`.
pub fn label(o: &ObjectInfo) -> String {
    if !o.id.starts_with('@') {
        return o.id.clone();
    }
    let Some(definition) = &o.definition else { return o.id.clone() };
    match o.attach.first() {
        Some((_, Attach::Object(id))) => format!("{definition}({id})"),
        _ => definition.clone(),
    }
}

/// The name a person or a model knows a group by: its own name, `%core`; for one the body of a
/// definition makes, the usage that applies the definition and the definition, `%amp1 ·
/// $TRANSCONDUCTOR_Z_AMPLIFIER`, with `#1`, `#2` when that usage makes several, and for a usage
/// without a name, the definition and the usage it is made in, `$TRANSCONDUCTOR_Z_AMPLIFIER in
/// %miller`; otherwise where GROUP is written, `@file:line:col`.
pub fn group_name(lv: &LevelView, k: usize) -> String {
    let g = &lv.groups[k];
    if g.origin != UsageOrigin::Body || g.id.starts_with('%') {
        return g.id.clone();
    }
    let Some(usage) = &g.parent else { return g.id.clone() };
    let made = lv.objects.iter().find(|o| &o.id == usage);
    let definition = made.and_then(|o| o.definition.clone()).unwrap_or_default();
    let mut name = if usage.starts_with('%') && !definition.is_empty() {
        format!("{usage} · {definition}")
    } else if !definition.is_empty() {
        match made.and_then(|o| o.parent.as_ref()) {
            Some(outer) => {
                let outer = lv.objects.iter().find(|o| &o.id == outer).map_or(outer.clone(), label);
                format!("{definition} in {outer}")
            }
            None => definition,
        }
    } else {
        usage.clone()
    };
    let siblings: Vec<usize> = (0..lv.groups.len()).filter(|&j| lv.groups[j].parent.as_ref() == Some(usage)).collect();
    if siblings.len() > 1 {
        let n = siblings.iter().position(|&j| j == k).unwrap() + 1;
        name.push_str(&format!(" #{n}"));
    }
    name
}

/// The nodes of the tree of `lv` that hold more than [`CROWDED`] devices directly, with how many:
/// the level first, then the groups in the order of the tree.
pub fn crowded(lv: &LevelView) -> Vec<(Option<usize>, usize)> {
    fn walk(n: &Node, out: &mut Vec<(Option<usize>, usize)>) {
        if n.held.devices.len() > CROWDED {
            out.push((n.group, n.held.devices.len()));
        }
        for c in &n.children {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    walk(&tree(lv), &mut out);
    out
}

/// Where a statement from `start` to `end` of the file `text` begins as written: its first character after the
/// blanks, the `;` before it and the comment markers of the lines it is written on, which its
/// span takes in from the end of the statement before it.
pub fn statement_at(text: &str, start: &Loc, end: &Loc) -> Loc {
    let lines: Vec<&str> = text.lines().collect();
    for n in start.line..=end.line.max(start.line) {
        let Some(line) = lines.get(n.saturating_sub(1) as usize) else { break };
        let from = if n == start.line { start.col.saturating_sub(1) as usize } else { 0 };
        let mut rest: String = line.chars().skip(from).collect();
        let mut col = from;
        if n != start.line {
            // the comment marker that carries a record line in a netlist
            let trimmed = rest.trim_start();
            let marker = ["//", "*"].iter().find(|m| trimmed.starts_with(**m));
            if let Some(m) = marker {
                let skip = rest.chars().count() - trimmed.chars().count() + m.chars().count();
                rest = rest.chars().skip(skip).collect();
                col += skip;
            }
        }
        let blank = rest.chars().take_while(|c| c.is_whitespace() || *c == ';').count();
        if blank < rest.chars().count() {
            return Loc::new(start.file.clone(), n, (col + blank + 1) as u32);
        }
    }
    start.clone()
}

/// For a group the body of a definition makes, the statement that applies the definition in a file
/// of the document (`written` says which files are): that of the usage that applies it or, when the
/// body or a rule of a package makes that usage, that of the usage it is made in, and so on up;
/// none for another group, and when no usage up the chain is written in such a file.
pub fn applied_at<'l>(lv: &'l LevelView, k: usize, written: &dyn Fn(&str) -> bool) -> Option<&'l (Loc, Loc)> {
    let g = &lv.groups[k];
    if g.origin != UsageOrigin::Body {
        return None;
    }
    let mut usage = g.parent.as_ref()?;
    for _ in 0..=lv.objects.len() {
        let o = lv.objects.iter().find(|o| &o.id == usage)?;
        if let Some(span) = o.source.as_ref().filter(|(a, _)| written(&a.file)) {
            return Some(span);
        }
        usage = o.parent.as_ref()?;
    }
    None
}
