//! Compare the semantics of a level before and after an upgrade.
//!
//! Items are paired by identity, never by index: an object by its id (or, for one without a name, by
//! what it is and where it comes among the like ones), a fact, wire, ROUTE or GROUP by what it says
//! about those identities. Writing differences of one fact (a pin reached as `Pin` or `SymbolPin`,
//! the fact numbers inside a position, whether a ROUTE is an already resolved tree) are removed
//! before the comparison. What is left falls into three lists:
//!
//! - differences: an item that disappeared, appeared, or changed in anything but the default look;
//! - default look: a change that only the default look of the standard package explains, and what
//!   the upgrade added by default (junction dots and the like, which only the new version makes).

use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::geom::fmt_num;
use rnl_runtime::model::{SymbolOrigin, UsageOrigin};
use rnl_runtime::problem::{FactKind, LevelProblem, ObjectKind, OrientSlot, PointRef, Position, Sense, Slot};
use rnl_runtime::Interpretation;
use serde_json::{json, Value};

use super::{Difference, DiscardedPlacements};

type Key = (String, String, String, String);

/// Marks an identity inside canonical text, so that `#M1` is not found in `#M10`.
fn mark(name: &str) -> String {
    format!("⟦{name}⟧")
}

fn slot(slot: &Slot) -> String {
    match slot {
        Slot::Free => "free".into(),
        Slot::Fixed { value, .. } => fmt_num(*value),
    }
}

/// The identities of the objects of a problem, by index. An object with a name has its id; one
/// without (the id is where it was written, which an upgrade moves) is what it is, with its place
/// among the like ones.
fn names(problem: &LevelProblem) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    problem
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| {
            if let Some(id) = problem.ids.objects.get(index).filter(|id| !id.starts_with('@')) {
                return terminals(id);
            }
            let base = format!("~{}:{}", object.kind.as_str(), object.label);
            let count = seen.entry(base.clone()).or_default();
            *count += 1;
            format!("{base}#{count}")
        })
        .collect()
}

/// `pin(#M1,2)`, how 0.4 writes the third pin, as `#M1:3`, how 0.3 writes the terminal.
fn terminals(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("pin(") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        let converted = after.find(')').and_then(|end| {
            let (name, index) = after[..end].rsplit_once(',')?;
            let index: usize = index.trim().parse().ok()?;
            Some((format!("{}:{}", name.trim(), index + 1), end + 1))
        });
        match converted {
            Some((text, used)) => {
                out.push_str(&text);
                rest = &after[used..];
            }
            None => {
                out.push_str("pin(");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn point(names: &[String], p: &PointRef) -> String {
    let name = |k: &usize| names.get(*k).map_or_else(|| format!("object {k}"), |n| mark(n));
    match p {
        PointRef::Origin(k) => name(k),
        PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. } => {
            format!("{}.pin{pin}", name(object))
        }
        PointRef::PinPosition { object, pin, .. } => format!("{}.at{pin}", name(object)),
        PointRef::Centre(points) => {
            format!("centre({})", points.iter().map(|q| point(names, q)).collect::<Vec<_>>().join(","))
        }
    }
}

fn points(names: &[String], list: &[PointRef]) -> String {
    list.iter().map(|p| point(names, p)).collect::<Vec<_>>().join(",")
}

/// The facts that a fact says. A DISTANCE between sets with a direction and a lower bound only
/// (nonnegative) is the same as one between every pair of their points (14.5): 0.3 reads it as
/// the one fact over the extents, 0.4 as the pairs, so both are the pairs.
fn facts(names: &[String], kind: &FactKind) -> Vec<String> {
    if let FactKind::Distance { axis, sense, min, max: None, from, to } = kind {
        let plain = |list: &[PointRef]| !list.iter().any(|p| matches!(p, PointRef::Centre(_)));
        if *sense != Sense::Either
            && min.is_some_and(|n| n >= 0.0)
            && from.len() * to.len() > 1
            && plain(from)
            && plain(to)
        {
            return from
                .iter()
                .flat_map(|a| to.iter().map(move |b| (a, b)))
                .map(|(a, b)| {
                    let pair = FactKind::Distance {
                        axis: *axis,
                        sense: *sense,
                        min: *min,
                        max: None,
                        from: vec![a.clone()],
                        to: vec![b.clone()],
                    };
                    fact(names, &pair)
                })
                .collect();
        }
    }
    vec![fact(names, kind)]
}

/// The fact as it says, with identities in place of indexes and without its rank.
fn fact(names: &[String], kind: &FactKind) -> String {
    let n = |k: &usize| names.get(*k).map_or_else(|| format!("object {k}"), |n| mark(n));
    let pin = |d: &rnl_runtime::problem::DirectedPin| format!("{}.pin{}@{}", n(&d.object), d.pin, d.direction);
    match canonical(kind) {
        FactKind::Place { at, axis, value } => format!("place {} {axis:?} {}", point(names, &at), fmt_num(value)),
        FactKind::PointCoordinate { point: p, axis, value } => format!("point {} {axis:?} {}", n(&p), fmt_num(value)),
        FactKind::Offset { text, from, axis, value } => {
            format!("offset {} from {} {axis:?} {}", n(&text), point(names, &from), fmt_num(value))
        }
        FactKind::Orient { object, code } => format!("orient {} {code:?}", n(&object)),
        FactKind::PinOrient { pin: p, codes } => format!("pin-orient {} {codes:?}", pin(&p)),
        FactKind::PinOrientRelation { from, to, relations } => {
            format!("pin-orient-relation {} {} {relations:?}", pin(&from), pin(&to))
        }
        FactKind::Align { axis, members } => format!("align {axis:?} {}", points(names, &members)),
        FactKind::Order { axis, sense, members } => format!("order {axis:?} {sense:?} {}", points(names, &members)),
        FactKind::Distribute { axis, sense, members } => {
            format!("distribute {axis:?} {sense:?} {}", points(names, &members))
        }
        FactKind::Distance { axis, sense, min, max, from, to } => format!(
            "distance {axis:?} {sense:?} {:?} {:?} [{}] [{}]",
            min.map(fmt_num),
            max.map(fmt_num),
            points(names, &from),
            points(names, &to)
        ),
        FactKind::Table { axis, rows } => {
            let rows = rows.try_map(&mut |p| Some(point(names, p)));
            format!("table {axis:?} {rows:?}")
        }
        FactKind::OrientRelation { relation, from, to } => format!(
            "orient-relation {relation:?} {} {}",
            from.iter().map(&n).collect::<Vec<_>>().join(","),
            to.iter().map(&n).collect::<Vec<_>>().join(",")
        ),
    }
}

fn canonical(kind: &FactKind) -> FactKind {
    let mut kind = kind.clone();
    if let FactKind::Distance { sense, min, max, from, to, .. } = &mut kind {
        if *sense == Sense::Either
            && *min == Some(0.0)
            && *max == Some(0.0)
            && from.len() == 1
            && to.len() == 1
            && !matches!(from[0], PointRef::Centre(_))
            && !matches!(to[0], PointRef::Centre(_))
        {
            *sense = Sense::Increasing;
        }
        if *sense == Sense::Decreasing
            && from.len() == 1
            && to.len() == 1
            && !matches!(from[0], PointRef::Centre(_))
            && !matches!(to[0], PointRef::Centre(_))
        {
            let old_min = *min;
            *min = max.map(|n| if n == 0.0 { 0.0 } else { -n });
            *max = old_min.map(|n| if n == 0.0 { 0.0 } else { -n });
            *sense = Sense::Increasing;
        }
    }
    kind
}

fn position(names: &[String], position: &Position) -> Value {
    let n = |k: &usize| names.get(*k).map_or_else(|| format!("object {k}"), |n| mark(n));
    match position {
        Position::InSymbol { owner, local, replaced: None } => {
            json!({"symbol_given": {"owner": n(owner), "local": local}})
        }
        Position::InSymbol { owner, local, replaced: Some((axis, s)) } => {
            json!({"written": format!("symbol {} {local:?} {axis:?} {}", n(owner), slot(s))})
        }
        Position::Own { x, y } => json!({"written": format!("own {} {}", slot(x), slot(y))}),
        // 0.4 gives such an object a point of its own, kept at the other by facts (see `facts`).
        Position::At(_) => json!({"written": "own free free"}),
    }
}

fn symbol(table: &[rnl_runtime::model::SymbolView], index: usize, depth: usize) -> Value {
    let Some(s) = table.get(index).filter(|_| depth < 65) else { return Value::Null };
    json!({"box":s.bbox,
        "pins":s.pins.iter().map(|p|json!({"at":p.pos,"direction":p.dir,"label":p.label})).collect::<Vec<_>>(),
        "labels":s.labels.iter().map(|l|json!({"key":l.key,"text":l.text,"at":l.at,"halign":l.halign,"valign":l.valign,"rotate":l.rotate,"fix_rotate":l.fix_rotate.unwrap_or(false),"upright":l.upright.unwrap_or(true),"style":format!("{:?}",l.style)})).collect::<Vec<_>>(),
        "primitives":s.prims.iter().map(|p|json!({"order":p.order,"shape":p.shape.as_str(),"points":p.points,"params":format!("{:?}",p.params),"style":format!("{:?}",p.style),"text":p.text,"halign":p.halign,"valign":p.valign,"rotate":p.rotate,"fix_rotate":p.fix_rotate,"upright":p.upright.unwrap_or(true)})).collect::<Vec<_>>(),
        "nested":s.nested.iter().map(|n|json!({"order":n.order,"at":n.at,"orientation":n.orient.as_str(),"symbol":symbol(table,n.symbol,depth+1)})).collect::<Vec<_>>()})
}

/// The value without what only the look of the standard package decides: the style properties, the
/// place and turning of the labels of a symbol, and the offset of a label that its symbol places.
/// Names, kinds, hosts, orientations, pin positions, pin networks, the box, authored positions and
/// everything else stay. Two values equal after this differ only in default appearance.
fn without_look(category: &str, value: &Value) -> Value {
    let mut value = value.clone();
    let Some(map) = value.as_object_mut() else { return value };
    match category {
        "positions" => {
            if let Some(given) =
                map.get_mut("position").and_then(|p| p.get_mut("symbol_given")).and_then(Value::as_object_mut)
            {
                given.remove("local");
            }
        }
        "objects" => {
            map.remove("style");
            let standard = map.get("standard_symbol") == Some(&Value::Bool(true));
            if let Some(symbol) = map.get_mut("geometry").and_then(Value::as_object_mut) {
                if standard {
                    // The drawing of a symbol of the standard package is the package's default look;
                    // its box and pins stay, as connections and geometry.
                    for part in ["labels", "primitives", "nested"] {
                        symbol.remove(part);
                    }
                }
                // Of a label of any symbol only its style, place and turning are look.
                for label in symbol.get_mut("labels").and_then(Value::as_array_mut).into_iter().flatten() {
                    if let Some(label) = label.as_object_mut() {
                        for field in ["style", "at", "halign", "valign", "rotate", "fix_rotate", "upright"] {
                            label.remove(field);
                        }
                    }
                }
                // Of a primitive only its style; how many there are and what they draw is geometry.
                for primitive in symbol.get_mut("primitives").and_then(Value::as_array_mut).into_iter().flatten() {
                    if let Some(primitive) = primitive.as_object_mut() {
                        primitive.remove("style");
                    }
                }
            }
        }
        "texts" => {
            map.remove("style");
            // Where a text is drawn follows from the facts that place it and the symbol, which are
            // compared on their own.
            map.remove("at");
            // A label that only its symbol gives is the package's: what it says and where it is are look.
            if map.get("written") == Some(&Value::Bool(false)) {
                for field in
                    ["at", "markup", "lines", "halign", "valign", "rotate", "fix_rotate", "upright", "orientation"]
                {
                    map.remove(field);
                }
            }
        }
        "shapes" => {
            map.remove("style");
        }
        "networks" => {
            map.remove("default_pins");
        }
        _ => {}
    }
    value
}

/// Items of one category, by identity. Items that would share a key are numbered in order.
struct Items {
    items: BTreeMap<Key, Value>,
    seen: BTreeMap<Key, usize>,
}

impl Items {
    fn put(&mut self, file: &str, level: &str, category: &str, key: &str, value: Value) {
        let base = (file.to_string(), level.to_string(), category.to_string(), key.to_string());
        let count = self.seen.entry(base.clone()).or_default();
        *count += 1;
        let mut full = base;
        if *count > 1 {
            full.3 = format!("{key}#{count}");
        }
        self.items.insert(full, value);
    }
}

/// The identities of the objects that only the upgraded text has: what a rule made there (junction
/// dots, default texts), and what 0.4 counts as an object where 0.3 had only a usage of that name
/// (a `$GROUND_STUB` or `$FLOW`, which now has a point of its own). Per level title.
fn defaults(before: &Interpretation, after: &Interpretation) -> BTreeMap<String, BTreeSet<String>> {
    let old: BTreeMap<_, (BTreeSet<_>, BTreeSet<_>)> = before
        .problems
        .iter()
        .map(|p| (p.title.clone(), (names(p).into_iter().collect(), p.ids.usages.iter().cloned().collect())))
        .collect();
    let mut out = BTreeMap::new();
    for problem in &after.problems {
        let (known, usages) = old.get(&problem.title).cloned().unwrap_or_default();
        let set: BTreeSet<String> = names(problem)
            .into_iter()
            .zip(&problem.objects)
            .filter(|(name, object)| !known.contains(name) && (object.generated || usages.contains(name)))
            .map(|(name, _)| name)
            .collect();
        out.insert(problem.title.clone(), set);
    }
    // What a rule made in either reading is the default fill of that version, which differs
    // between versions (0.3 draws a ground at each ground pin, 0.4 a stub).
    for read in [before, after] {
        for problem in &read.problems {
            out.entry(problem.title.clone())
                .or_default()
                .extend(names(problem).into_iter().zip(&problem.objects).filter(|(_, o)| o.generated).map(|(n, _)| n));
        }
    }
    out
}

/// The identities of the usages that a conversion rule of the upgrade turns into another usage on
/// purpose: `$IO_PORT` becomes `$NET_LABEL`, which has no pin (and no `name` key). What changes with
/// them (their texts, the ROUTEs and wires that went to their pin) is the rule's doing, which the
/// report names in the changes of the file; it is not a difference that the upgrade did by mistake.
fn converted(before: &Interpretation, after: &Interpretation) -> BTreeMap<String, BTreeSet<String>> {
    let definitions = |read: &Interpretation, title: &str| -> BTreeMap<String, String> {
        read.model
            .levels
            .iter()
            .filter(|l| l.title == title)
            .flat_map(|l| &l.objects)
            .filter_map(|o| Some((o.id.clone(), o.definition.clone()?)))
            .collect()
    };
    let mut out = BTreeMap::new();
    for level in &after.model.levels {
        let (old, new) = (definitions(before, &level.title), definitions(after, &level.title));
        let set = new
            .into_iter()
            .filter(|(id, definition)| {
                !id.starts_with('@') && definition == "$NET_LABEL" && old.get(id).is_some_and(|d| d == "$IO_PORT")
            })
            .map(|(id, _)| id)
            .collect();
        out.insert(level.title.clone(), set);
    }
    out
}

/// The ids of the texts that only a symbol gives, in either reading, per level title. Such a text
/// appearing, going or changing is the package's default look, whatever the text says.
fn package_texts(before: &Interpretation, after: &Interpretation) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for read in [before, after] {
        for level in &read.model.levels {
            out.entry(level.title.clone())
                .or_default()
                .extend(level.texts.iter().filter(|t| !t.written).map(|t| t.object_id.clone()));
        }
    }
    out
}

fn about(ids: &BTreeSet<String>, item: &str, text: &str) -> bool {
    mentions(ids, text) || item.split('|').any(|part| part.split("::").next().is_some_and(|id| ids.contains(id)))
}

/// Whether an item is, or is about, an object that only a rule made.
fn is_fill(fill: &BTreeSet<String>, category: &str, item: &str, value: &Value) -> bool {
    match category {
        "objects" => value["rule_made"] == Value::Bool(true),
        "positions" => fill.contains(item),
        _ => mentions(fill, item) || mentions(fill, &value.to_string()),
    }
}

fn mentions(defaults: &BTreeSet<String>, text: &str) -> bool {
    defaults.iter().any(|name| text.contains(&mark(name)))
}

fn snapshot(read: &Interpretation, added: &BTreeMap<String, BTreeSet<String>>) -> Items {
    let mut out = Items { items: BTreeMap::new(), seen: BTreeMap::new() };
    for level in &read.doc.levels {
        if !level.circuit {
            continue;
        }
        let title = level.title();
        out.put(&level.path, &title, "netlist", "ports", json!(level.ports));
        for instance in &level.instances {
            out.put(
                &level.path,
                &title,
                "netlist",
                &instance.name,
                json!({"nodes":instance.nodes,"model":instance.model,"value":instance.value,"parameters":instance.params}),
            );
        }
    }
    for level in &read.model.levels {
        let file = &read.doc.levels[level.level].path;
        let title = &level.title;
        let mut put = |category: &str, key: &str, value: Value| out.put(file, title, category, key, value);
        // An id that is where a statement is written moves with the upgrade; such an object is
        // known by what it is.
        let id = |id: &str, kind: &str, extra: &str| {
            if id.starts_with('@') || id.is_empty() {
                format!("~{kind}:{extra}")
            } else {
                terminals(id)
            }
        };
        // Every object of the level, drawn or not, is one item: where it is drawn is a matter of
        // the facts, compared on their own. What is left is what it is and what it looks like.
        for object in &level.objects {
            let key = id(&object.id, object.kind, object.definition.as_deref().unwrap_or(""));
            let drawn = object.symbol_view.map(|i| symbol(&level.symbols, i, 0));
            let undrawn = || {
                level
                    .unplaced_items
                    .iter()
                    .find(|item| item.object_id == object.id)
                    .map(|item| symbol(&level.unplaced_symbols, item.symbol, 0))
            };
            let geometry = drawn.or_else(undrawn);
            let standard = matches!(object.symbol.as_ref().map(|s| &s.origin), Some(SymbolOrigin::Package(_)));
            put(
                "objects",
                &key,
                json!({"kind":object.kind,"host":object.host,"definition":object.definition,
                    "symbol":object.symbol.as_ref().map(|s| &s.name),"standard_symbol":standard,"rule_made":!matches!(object.origin, UsageOrigin::Written | UsageOrigin::Solver),
                    "roles":terminals(&format!("{:?}", object.attach)),
                    "parameters":format!("{:?}",object.params),"disabled":object.disabled,
                    "style":format!("{:?}",object.own_style),
                    "geometry":geometry}),
            );
        }
        for point in &level.points {
            put("points", &id(&point.id, "point", ""), json!({"junction":point.junction}));
        }
        for text in &level.texts {
            // A label that only a symbol gives to a usage that no one named is the package's: it
            // has no identity that an upgrade keeps (it is known by where the package writes it).
            if !text.written && (text.object_id.starts_with('@') || text.object_id.is_empty()) {
                continue;
            }
            // A label that only its symbol gives is known by its owner: the package may rename its key.
            let key = if text.written {
                format!(
                    "{}|{}|{}",
                    id(&text.object_id, "text", ""),
                    terminals(text.attach.as_deref().unwrap_or("")),
                    text.key.as_deref().unwrap_or("")
                )
            } else {
                format!("label|{}", terminals(text.object_id.split("::").next().unwrap_or("")))
            };
            put(
                "texts",
                &key,
                json!({"written":text.written,"at":text.anchor,"markup":text.markup,"lines":format!("{:?}",text.lines),"halign":text.halign,"valign":text.valign,"rotate":text.rotate,"fix_rotate":text.fix_rotate.unwrap_or(false),"upright":text.upright.unwrap_or(true),"orientation":text.orient.as_str(),"style":format!("{:?}",text.style),"layer":text.layer.as_str()}),
            );
        }
        for shape in &level.shapes {
            put(
                "shapes",
                &format!("{}|{}", shape.shape.as_str(), shape.layer.as_str()),
                json!({"points":shape.points,"params":format!("{:?}",shape.params),"style":format!("{:?}",shape.style),"layer":shape.layer.as_str()}),
            );
        }
    }
    for problem in &read.problems {
        let file = &read.doc.levels[problem.level].path;
        let title = &problem.title;
        let names = names(problem);
        let none = BTreeSet::new();
        let default = added.get(title).unwrap_or(&none);
        let mut put = |category: &str, key: &str, value: Value| out.put(file, title, category, key, value);
        for (index, object) in problem.objects.iter().enumerate() {
            // The place of a label that only its symbol gives is the package's (default look).
            if object.kind == ObjectKind::Text && matches!(object.position, Position::InSymbol { replaced: None, .. }) {
                continue;
            }
            let orient = match &object.orient {
                None => "none".to_string(),
                Some(OrientSlot::Free) => "free".to_string(),
                Some(OrientSlot::Fixed { code, .. }) => format!("{code:?}"),
            };
            put(
                "positions",
                &names[index],
                json!({"kind":object.kind.as_str(),"position":position(&names, &object.position),"orientation":orient}),
            );
        }
        for item in &problem.facts {
            for text in facts(&names, &item.kind) {
                put("facts", &format!("{text} {:?}", item.priority), json!(text));
            }
        }
        // An object drawn at a point (0.3) is the same as one with a point of its own that two
        // ALIGN facts keep there (0.4).
        for (index, object) in problem.objects.iter().enumerate() {
            if let Position::At(target) = &object.position {
                for axis in ["X", "Y"] {
                    let text = format!("align {axis} {},{}", mark(&names[index]), point(&names, target));
                    put("facts", &format!("{text} Normal"), json!(text));
                }
            }
        }
        for net in &problem.nets {
            let (default_pins, kept): (Vec<_>, Vec<_>) =
                net.pins.iter().map(|p| point(&names, p)).partition(|p| mentions(default, p));
            let mut kept = kept;
            kept.sort();
            put("networks", &net.name, json!({"pins":kept,"default_pins":default_pins,"port":net.port}));
        }
        for wire in &problem.wires {
            let anchors = points(&names, &wire.anchors);
            put(
                "wires",
                &format!("{:?} {anchors}", wire.nets),
                json!({"nets":wire.nets,"anchors":anchors,"mode":wire.mode,"bus":wire.bus}),
            );
        }
        let mut asked = BTreeSet::new();
        for route in &problem.routes {
            // Whether the ROUTE is an already resolved tree only says how a version of the
            // language wrote it; the net, the hint and the pins are what it asks for, and asking
            // twice for the same is asking once.
            let pins = points(&names, &route.pins);
            let key = format!("{} {pins}", route.net);
            if asked.insert(key.clone()) {
                put("routes", &key, json!({"net":route.net,"mode":route.mode,"pins":pins}));
            }
        }
        for group in &problem.groups {
            let mut members: Vec<_> = group.members.iter().map(|m| mark(&names[*m])).collect();
            members.sort();
            put("groups", &members.join(","), json!(members));
        }
    }
    out
}

/// The differences, and apart from them the changes that only the default look of the standard
/// package of 0.4 explains (see [`without_look`]), including what the upgrade adds by default.
/// What is not sure to be one stays a difference.
pub(super) fn compare(
    before: &Interpretation,
    after: &Interpretation,
) -> (Vec<Difference>, Vec<Difference>, Vec<Difference>) {
    let added = defaults(before, after);
    let rules = converted(before, after);
    let package = package_texts(before, after);
    let mut before = snapshot(before, &BTreeMap::new()).items;
    let after = snapshot(after, &added).items;
    let mut differences = Vec::new();
    let mut look = Vec::new();
    let mut by_rule = Vec::new();
    let no_ids = BTreeSet::new();
    for ((file, level, category, item), value) in after {
        let old = before.remove(&(file.clone(), level.clone(), category.clone(), item.clone()));
        let default = added.get(&level);
        let only_look = match &old {
            Some(old) => {
                old != &value && {
                    let (mut a, mut b) = (without_look(&category, old), without_look(&category, &value));
                    // A symbol of the package that one side does not draw has no geometry to compare.
                    if category == "objects"
                        && (a["geometry"].is_null() || b["geometry"].is_null())
                        && a["standard_symbol"] == Value::Bool(true)
                    {
                        a["geometry"] = Value::Null;
                        b["geometry"] = Value::Null;
                    }
                    a == b
                }
            }
            None => false,
        };
        if old.as_ref() == Some(&value) {
            continue;
        }
        let package_text = matches!(category.as_str(), "texts" | "positions")
            && package.get(&level).is_some_and(|ids| {
                item.starts_with("label|")
                    || ids.contains(&item)
                    || item.split('|').next().is_some_and(|first| ids.contains(first))
            });
        let fill = default.is_some_and(|d| is_fill(d, &category, &item, &value))
            || old.as_ref().is_some_and(|old| is_fill(default.unwrap_or(&no_ids), &category, &item, old));
        let only_look = only_look || package_text || fill;
        let ids = rules.get(&level).unwrap_or(&no_ids);
        let by_rule_here = !only_look
            && about(ids, &item, &format!("{}{}", old.as_ref().map_or(String::new(), Value::to_string), value));
        let difference = Difference { file, level, category, item, before: old, after: Some(value) };
        if only_look {
            look.push(difference);
        } else if by_rule_here {
            by_rule.push(difference);
        } else {
            differences.push(difference);
        }
    }
    for ((file, level, category, item), value) in before {
        let ids = rules.get(&level).unwrap_or(&no_ids);
        let by_rule_here = about(ids, &item, &value.to_string());
        let package_text = matches!(category.as_str(), "texts" | "positions")
            && package.get(&level).is_some_and(|ids| {
                item.starts_with("label|")
                    || ids.contains(&item)
                    || item.split('|').next().is_some_and(|first| ids.contains(first))
            });
        let fill = added.get(&level).is_some_and(|d| is_fill(d, &category, &item, &value));
        let difference = Difference { file, level, category, item, before: Some(value), after: None };
        if package_text || fill {
            look.push(difference);
        } else if by_rule_here {
            by_rule.push(difference);
        } else {
            differences.push(difference);
        }
    }
    (differences, look, by_rule)
}

/// The facts that only the records of the solver gave: those of `full`, read with them, that
/// `authored`, read without them, does not have, per level.
pub(super) fn discarded(full: &Interpretation, authored: &Interpretation) -> Vec<DiscardedPlacements> {
    full.problems
        .iter()
        .filter_map(|problem| {
            let own = authored.problems.iter().find(|p| p.title == problem.title && p.file == problem.file)?;
            let facts = problem.facts.len().saturating_sub(own.facts.len());
            (facts > 0).then(|| DiscardedPlacements {
                file: full.doc.levels[problem.level].path.clone(),
                level: problem.title.clone(),
                facts,
            })
        })
        .collect()
}
