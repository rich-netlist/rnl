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
use rnl_runtime::problem::{
    FactKind, LevelProblem, ObjectKind, OrientSlot, PointRef, Position, Reference, Sense, Slot,
};
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

/// What says which terminal a pin is. The terminal that the netlist numbers (`#M1:3`) is the same in
/// both versions, where a pin's place in the symbol is not (a symbol may order its pins otherwise
/// from one version to the next), so pins are compared by terminal.
#[derive(Default, Clone)]
struct Pins {
    /// The pin of an object that a terminal written by name is, from the level's own references
    /// (a `Pin` names its terminal; a `SymbolPin` written `#M::s` names `s`).
    by_name: BTreeMap<(String, String), usize>,
    /// The terminal that a pin of an object is, by the object's written id and the pin's place,
    /// from the pins of the object in the model (`#M1:3` is the number `3`).
    number: BTreeMap<(String, usize), String>,
}

fn pins(read: &Interpretation, problem: &LevelProblem) -> Pins {
    let mut out = Pins::default();
    for level in read.model.levels.iter().filter(|l| l.title == problem.title) {
        for object in level.objects.iter().filter(|o| !o.id.starts_with('@')) {
            for (index, pin) in object.pins.iter().enumerate() {
                let terminal = pin.terminals.iter().find_map(|t| match t.rsplit_once(':') {
                    Some((owner, number)) => (owner == object.id).then(|| number.to_string()),
                    None => Some(t.clone()),
                });
                if let Some(terminal) = terminal {
                    out.number.insert((object.id.clone(), index), terminal);
                }
            }
        }
    }
    let mut numbers = Vec::new();
    let mut visit = |p: &PointRef| {
        let (object, pin, terminal) = match p {
            PointRef::Pin { object, pin, terminal } => (*object, *pin, terminal.clone()),
            PointRef::SymbolPin { object, pin, reference: Reference::Name(name) } => match name.rsplit_once("::") {
                Some((_, terminal)) => (*object, *pin, terminal.to_string()),
                None => return,
            },
            _ => return,
        };
        if let Some(id) = problem.ids.objects.get(object).filter(|id| !id.starts_with('@')) {
            // A terminal that a reference gives by its number is the netlist's own.
            if matches!(p, PointRef::Pin { .. }) && terminal.chars().all(|c| c.is_ascii_digit()) {
                numbers.push(((id.clone(), pin), terminal.clone()));
            }
            out.by_name.insert((id.clone(), terminal), pin);
        }
    };
    for net in &problem.nets {
        net.pins.iter().for_each(&mut visit);
    }
    for wire in &problem.wires {
        wire.anchors.iter().for_each(&mut visit);
    }
    for (key, terminal) in numbers {
        out.number.entry(key).or_insert(terminal);
    }
    out
}

/// The identities of the objects of a problem, by index, with the terminal of each of their pins.
struct Names {
    names: Vec<String>,
    terminal: BTreeMap<(usize, usize), String>,
}

impl std::ops::Deref for Names {
    type Target = [String];
    fn deref(&self) -> &[String] {
        &self.names
    }
}

impl Names {
    /// A pin as the terminal it is; by its place in the symbol only when no terminal is known.
    fn pin(&self, object: usize, pin: usize) -> String {
        match self.terminal.get(&(object, pin)) {
            Some(t) => format!("t{t}"),
            None => format!("pin{pin}"),
        }
    }
}

fn identities(read: &Interpretation, problem: &LevelProblem) -> Names {
    let pins = pins(read, problem);
    // A pin of a point is the terminal that the level's own references name (a `Pin` its terminal,
    // a `SymbolPin` written `#M::s` its `s`), the same in both versions where the pin's place in the
    // symbol is not.
    let terminal = pins
        .by_name
        .iter()
        .filter_map(|((id, name), pin)| {
            let index = problem.ids.objects.iter().position(|o| o == id)?;
            // The roles of a usage are the package's and may be renamed with it; its pins stay by place.
            (problem.objects[index].kind == ObjectKind::Instance).then(|| ((index, *pin), name.clone()))
        })
        .collect();
    Names { names: names(problem, &pins), terminal }
}

/// The identities of the objects of a problem, by index. An object with a name has its id; one
/// without (the id is where it was written, which an upgrade moves) is what it is, with its place
/// among the like ones. Objects that a rule made and ones that were written are counted apart, so
/// a rule's object does not shift the number of a written one.
fn names(problem: &LevelProblem, pins: &Pins) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    problem
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| {
            if let Some(id) = problem.ids.objects.get(index).filter(|id| !id.starts_with('@')) {
                return terminals(id, pins);
            }
            let base =
                format!("~{}:{}:{}", object.kind.as_str(), object.label, if object.generated { "made" } else { "" });
            let count = seen.entry(base.clone()).or_default();
            *count += 1;
            format!("{base}#{count}")
        })
        .collect()
}

/// A pin of an object as its terminal: `pin(#M1,2)`, how 0.4 writes the pin of a symbol, and `#M1:d`,
/// the terminal by its name, are `#M1:3` when the object's pins say that terminal (the number is the
/// netlist's, the same in both versions). What cannot be resolved stays as written, and a
/// difference of writing is then reported rather than hidden.
fn terminals(text: &str, pins: &Pins) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("pin(") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        let converted = after.find(')').and_then(|end| {
            let (name, index) = after[..end].rsplit_once(',')?;
            let index: usize = index.trim().parse().ok()?;
            let terminal = pins.number.get(&(name.trim().to_string(), index))?;
            Some((format!("{}:{terminal}", name.trim()), end + 1))
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
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let chars: Vec<char> = out.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            let mut j = i + 1;
            while j < chars.len() && word(chars[j]) {
                j += 1;
            }
            if j + 1 < chars.len() && chars[j] == ':' && chars[j + 1] != ':' && word(chars[j + 1]) {
                let mut k = j + 1;
                while k < chars.len() && word(chars[k]) {
                    k += 1;
                }
                let name: String = chars[i..j].iter().collect();
                let terminal: String = chars[j + 1..k].iter().collect();
                let number =
                    pins.by_name.get(&(name.clone(), terminal)).and_then(|pin| pins.number.get(&(name.clone(), *pin)));
                if let Some(number) = number {
                    result.push_str(&format!("{name}:{number}"));
                    i = k;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

fn point(names: &Names, p: &PointRef) -> String {
    let name = |k: &usize| names.get(*k).map_or_else(|| format!("object {k}"), |n| mark(n));
    match p {
        PointRef::Origin(k) => name(k),
        PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. } => {
            format!("{}.{}", name(object), names.pin(*object, *pin))
        }
        PointRef::PinPosition { object, pin, .. } => format!("{}.at{}", name(object), names.pin(*object, *pin)),
        PointRef::Centre(points) => {
            format!("centre({})", points.iter().map(|q| point(names, q)).collect::<Vec<_>>().join(","))
        }
    }
}

fn points(names: &Names, list: &[PointRef]) -> String {
    list.iter().map(|p| point(names, p)).collect::<Vec<_>>().join(",")
}

/// The facts that a fact says. A DISTANCE between sets with a direction and a lower bound only
/// (nonnegative) is the same as one between every pair of their points (14.5): 0.3 reads it as
/// the one fact over the extents, 0.4 as the pairs, so both are the pairs.
fn facts(names: &Names, kind: &FactKind) -> Vec<String> {
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

/// Whether a fact has too few members to say anything, such as the column of the port tags of a
/// level without ports that the fill of 0.4 writes: it holds by itself, as a ROUTE of one pin does.
fn says_nothing(kind: &FactKind) -> bool {
    match kind {
        FactKind::Align { members, .. } | FactKind::Order { members, .. } | FactKind::Distribute { members, .. } => {
            members.len() < 2
        }
        FactKind::Distance { from, to, .. } => from.is_empty() || to.is_empty(),
        _ => false,
    }
}

/// The fact as it says, with identities in place of indexes and without its rank.
fn fact(names: &Names, kind: &FactKind) -> String {
    let n = |k: &usize| names.get(*k).map_or_else(|| format!("object {k}"), |n| mark(n));
    let pin = |d: &rnl_runtime::problem::DirectedPin| {
        format!("{}.{}@{}", n(&d.object), names.pin(d.object, d.pin), d.direction)
    };
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

fn position(names: &Names, position: &Position) -> Value {
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
                    // The drawing of a symbol that comes from a package (any package, not only the
                    // standard one) is that package's default look;
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
        .map(|p| {
            (
                p.title.clone(),
                (names(p, &pins(before, p)).into_iter().collect(), p.ids.usages.iter().cloned().collect()),
            )
        })
        .collect();
    let mut out = BTreeMap::new();
    for problem in &after.problems {
        let (known, usages) = old.get(&problem.title).cloned().unwrap_or_default();
        let set: BTreeSet<String> = names(problem, &pins(after, problem))
            .into_iter()
            .zip(&problem.objects)
            .filter(|(name, object)| {
                // Made in the body of another usage (`$MATCHED` inside `$DIFFERENTIAL_PAIR_N`) is the
                // definition's expansion, as a rule's is.
                let in_body = object.usage.and_then(|u| problem.usages.get(u)).is_some_and(|u| u.parent.is_some());
                !known.contains(name) && (object.generated || in_body || usages.contains(name))
            })
            .map(|(name, _)| name)
            .collect();
        out.insert(problem.title.clone(), set);
    }
    // What a rule made in either reading is the default fill of that version, which differs
    // between versions (0.3 draws a ground at each ground pin, 0.4 a stub).
    for read in [before, after] {
        for problem in &read.problems {
            out.entry(problem.title.clone()).or_default().extend(
                names(problem, &pins(read, problem))
                    .into_iter()
                    .zip(&problem.objects)
                    .filter(|(_, o)| o.generated)
                    .map(|(n, _)| n),
            );
        }
    }
    out
}

/// Whether the upgrade's own tables explain a definition becoming another: a rename of the legacy
/// tables, or a conversion of an approximate rule. Any other change of definition is not the
/// upgrade's doing on purpose.
fn explained(old: &str, new: &str) -> bool {
    let (old, new) = (old.trim_start_matches('$'), new.trim_start_matches('$'));
    [super::legacy::renamed(old), super::legacy::renamed_03_definition(old), super::legacy::renamed_03_symbol(old)]
        .contains(&Some(new))
        || super::approximate::CONVERSIONS.iter().any(|(from, to)| *from == old && *to == new)
}

/// The identities of the named usages whose definition the upgrade changed on purpose, per level
/// title: a legacy rename, or a conversion of an approximate rule (`$IO_PORT` into `$NET_LABEL`,
/// which has no pin and no `name` key). What changes with them (their texts, the ROUTEs and wires
/// that went to their pin) is the rule's doing, which the report names in the changes of the file;
/// it goes to `rule_changes`, still listed. A change of definition the tables do not explain is a
/// difference.
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
                !id.starts_with('@') && old.get(id).is_some_and(|d| d != definition && explained(d, definition))
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
        // A network is what its author's pins say; the pins that a rule made are kept apart in it
        // (`default_pins`) and are what `without_look` leaves out.
        "networks" => false,
        _ => mentions(fill, item) || mentions(fill, &value.to_string()),
    }
}

fn mentions(defaults: &BTreeSet<String>, text: &str) -> bool {
    defaults.iter().any(|name| text.contains(&mark(name)))
}

fn snapshot(read: &Interpretation, added: &BTreeMap<String, BTreeSet<String>>) -> Items {
    let mut out = Items { items: BTreeMap::new(), seen: BTreeMap::new() };
    let pinmaps: BTreeMap<String, Pins> = read.problems.iter().map(|p| (p.title.clone(), pins(read, p))).collect();
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
        let pins = pinmaps.get(title).cloned().unwrap_or_default();
        let mut put = |category: &str, key: &str, value: Value| out.put(file, title, category, key, value);
        // An id that is where a statement is written moves with the upgrade; such an object is
        // known by what it is.
        let id = |id: &str, kind: &str, extra: &str| {
            if id.starts_with('@') || id.is_empty() {
                format!("~{kind}:{extra}")
            } else {
                terminals(id, &pins)
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
                    "roles":terminals(&format!("{:?}", object.attach), &pins),
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
                    terminals(text.attach.as_deref().unwrap_or(""), &pins),
                    text.key.as_deref().unwrap_or("")
                )
            } else {
                format!("label|{}", terminals(text.object_id.split("::").next().unwrap_or(""), &pins))
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
        let names = identities(read, problem);
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
            if says_nothing(&item.kind) {
                continue;
            }
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
            // A net with fewer than two pins has nothing to join: the ROUTE is a no-op.
            if route.pins.len() < 2 {
                continue;
            }
            let pins = points(&names, &route.pins);
            let key = format!("{} {pins}", route.net);
            if asked.insert(key.clone()) {
                put("routes", &key, json!({"net":route.net,"mode":route.mode,"pins":pins}));
            }
        }
        for group in &problem.groups {
            if group.members.is_empty() {
                continue;
            }
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
    let mut before = snapshot(before, &added).items;
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
        // The upgrade writes a default that 0.3 had by itself (`@ATTACH(gap=2)`) as a parameter,
        // so that the drawing stays: the object is the same but for parameters it did not list.
        let explicit_default = category == "objects"
            && old.as_ref().is_some_and(|old| {
                let (mut a, mut b) = (old.clone(), value.clone());
                let listed = a["parameters"] == "[]" && b["parameters"] != "[]";
                a["parameters"] = Value::Null;
                b["parameters"] = Value::Null;
                listed && a == b
            });
        let by_rule_here = !only_look
            && (explicit_default
                || about(ids, &item, &format!("{}{}", old.as_ref().map_or(String::new(), Value::to_string), value)));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pins_of() -> Pins {
        // A symbol whose first pin is the netlist's terminal 2 (named `d`), the second terminal 1.
        let mut pins = Pins::default();
        pins.by_name.insert(("#M1".into(), "d".into()), 0);
        pins.number.insert(("#M1".into(), 0), "2".into());
        pins.number.insert(("#M1".into(), 1), "1".into());
        pins
    }

    #[test]
    fn a_pin_is_known_by_its_terminal_not_by_its_place_in_the_symbol() {
        let pins = pins_of();
        assert_eq!(terminals("pin(#M1,0)", &pins), "#M1:2");
        assert_eq!(terminals("#M1:d", &pins), "#M1:2");
        // The wrong pin of the same symbol is another terminal, so a mix-up shows as a difference.
        assert_ne!(terminals("pin(#M1,1)", &pins), terminals("#M1:d", &pins));
        // A pin that no reference says stays as written.
        assert_eq!(terminals("pin(#M2,0)", &pins), "pin(#M2,0)");
        assert_eq!(terminals("#M2:g", &pins), "#M2:g");
    }

    #[test]
    fn a_point_and_a_text_spell_one_terminal_alike() {
        let pins = pins_of();
        let terminal = pins.number.iter().map(|((_, pin), t)| ((0, *pin), t.clone())).collect();
        let names = Names { names: vec!["#M1".into()], terminal };
        // The pin of a point is the terminal number, the number that a text resolves to.
        assert_eq!(names.pin(0, 0), "t2");
        assert!(terminals("#M1:d", &pins).ends_with(":2"));
        // Without a known terminal the place in the symbol stands in.
        assert_eq!(names.pin(0, 5), "pin5");
    }

    #[test]
    fn only_definitions_the_upgrade_explains_are_conversions() {
        assert!(explained("$IO_PORT", "$NET_LABEL"));
        assert!(explained("$CURRENT_MIRROR", "$CURRENT_MIRROR_N"));
        assert!(explained("$FLOW", "$SIGNAL_FLOW"));
        assert!(!explained("$GROUND", "$SUPPLY"));
        assert!(!explained("$FLOW", "$IO_PORT"));
        assert!(!explained("$IO_PORT", "$NET_LABEL_N"));
    }
}
