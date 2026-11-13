//! Compare semantic values, excluding source locations and package version metadata.

use std::collections::BTreeMap;

use rnl_runtime::problem::{FactKind, PointRef, Position, Sense};
use rnl_runtime::Interpretation;
use serde_json::{json, Value};

use super::Difference;

type Key = (String, String, String, String);

fn canonical_fact(kind: &FactKind) -> FactKind {
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

fn symbol(table: &[rnl_runtime::model::SymbolView], index: usize, depth: usize) -> Value {
    let Some(s) = table.get(index).filter(|_| depth < 65) else { return Value::Null };
    json!({"box":s.bbox,
        "pins":s.pins.iter().map(|p|json!({"at":p.pos,"direction":p.dir,"label":p.label})).collect::<Vec<_>>(),
        "labels":s.labels.iter().map(|l|json!({"key":l.key,"text":l.text,"at":l.at,"halign":l.halign,"valign":l.valign,"rotate":l.rotate,"fix_rotate":l.fix_rotate.unwrap_or(false),"upright":l.upright.unwrap_or(true),"style":format!("{:?}",l.style)})).collect::<Vec<_>>(),
        "primitives":s.prims.iter().map(|p|json!({"order":p.order,"shape":p.shape.as_str(),"points":p.points,"params":format!("{:?}",p.params),"style":format!("{:?}",p.style),"text":p.text,"halign":p.halign,"valign":p.valign,"rotate":p.rotate,"fix_rotate":p.fix_rotate,"upright":p.upright.unwrap_or(true)})).collect::<Vec<_>>(),
        "nested":s.nested.iter().map(|n|json!({"order":n.order,"at":n.at,"orientation":n.orient.as_str(),"symbol":symbol(table,n.symbol,depth+1)})).collect::<Vec<_>>()})
}

/// A position that only the symbol gives (a label at its default place) keeps its `local` offset
/// apart, so that [`without_look`] can set it aside; every other position is compared as written.
fn position(position: &Position) -> Value {
    match position {
        Position::InSymbol { owner, local, replaced: None } => {
            json!({"symbol_given": {"owner": owner, "local": local}})
        }
        other => json!({"written": format!("{other:?}")}),
    }
}

/// The value without what only the look of the standard package decides: the style properties, the
/// labels and drawing primitives of a symbol, and the offset of a label that its symbol places.
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
        "objects" | "unplaced-drawings" | "texts" | "shapes" => {
            map.remove("style");
            for key in ["geometry", "symbol"] {
                if let Some(symbol) = map.get_mut(key).and_then(Value::as_object_mut) {
                    // Of a label only its style, place and turning are look; its key and text are not.
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
        }
        _ => {}
    }
    value
}

fn snapshot(read: &Interpretation) -> BTreeMap<Key, Value> {
    let mut out = BTreeMap::new();
    for level in &read.doc.levels {
        if !level.circuit {
            continue;
        }
        let mut put = |category: &str, key: String, value: Value| {
            out.insert((level.path.clone(), level.title(), category.into(), key), value);
        };
        put("netlist", "ports".into(), json!(level.ports));
        for instance in &level.instances {
            put(
                "netlist",
                instance.name.clone(),
                json!({"nodes":instance.nodes,"model":instance.model,"value":instance.value,"parameters":instance.params}),
            );
        }
    }
    for level in &read.model.levels {
        let file = &read.doc.levels[level.level].path;
        let mut put = |category: &str, key: String, value: Value| {
            out.insert((file.clone(), level.title.clone(), category.into(), key), value);
        };
        // Earlier interfaces expose object pins only after placement. Unplaced local
        // geometry and connectivity are compared separately below.
        for (index, object) in level.objects.iter().enumerate() {
            let id = if object.id.starts_with('@') { format!("object-{index}") } else { object.id.clone() };
            put(
                "objects",
                id,
                json!({"kind":object.kind,"host":object.host,"definition":object.definition,
                    "at":object.at,"orientation":object.orient.as_str(),
                    "symbol":object.symbol.as_ref().map(|s| &s.name),
                    "roles":format!("{:?}",object.attach),
                    "parameters":format!("{:?}",object.params),"disabled":object.disabled,
                    "style":format!("{:?}",object.own_style),
                    "geometry":object.symbol_view.map(|i|symbol(&level.symbols,i,0)),
                    "pins":object.pins.iter().filter(|p| p.at.is_some()).map(|p| json!({"at":p.at,"nets":p.nets,"direction":p.dir})).collect::<Vec<_>>()}),
            );
        }
        // Objects with free positions still have meaningful local geometry,
        // label templates and pin networks. They are absent from drawn symbols.
        for (index, item) in level.unplaced_items.iter().enumerate() {
            put(
                "unplaced-drawings",
                index.to_string(),
                json!({
                    "kind":item.kind,"host":item.host,"orientation":item.orient.as_str(),
                    "style":format!("{:?}",item.style),"layer":item.layer.as_str(),
                    "texts":format!("{:?}",item.texts),"pin_nets":item.pin_nets,
                    "symbol":symbol(&level.unplaced_symbols,item.symbol,0)
                }),
            );
        }
        for (index, point) in level.points.iter().enumerate() {
            let id = if point.id.starts_with('@') { format!("point-{index}") } else { point.id.clone() };
            put("points", id, json!(point.at));
        }
        for (index, text) in level.texts.iter().enumerate() {
            put(
                "texts",
                index.to_string(),
                json!({"at":text.anchor,"markup":text.markup,"lines":format!("{:?}",text.lines),"halign":text.halign,"valign":text.valign,"rotate":text.rotate,"fix_rotate":text.fix_rotate.unwrap_or(false),"upright":text.upright.unwrap_or(true),"orientation":text.orient.as_str(),"style":format!("{:?}",text.style),"layer":text.layer.as_str()}),
            );
        }
        for (index, shape) in level.shapes.iter().enumerate() {
            put(
                "shapes",
                index.to_string(),
                json!({"shape":shape.shape.as_str(),"points":shape.points,"params":format!("{:?}",shape.params),"style":format!("{:?}",shape.style),"layer":shape.layer.as_str()}),
            );
        }
    }
    for problem in &read.problems {
        let file = &read.doc.levels[problem.level].path;
        let mut put = |category: &str, key: String, value: Value| {
            out.insert((file.clone(), problem.title.clone(), category.into(), key), value);
        };
        for (index, object) in problem.objects.iter().enumerate() {
            put(
                "positions",
                index.to_string(),
                json!({"kind":object.kind.as_str(),"position":position(&object.position),"orientation":format!("{:?}",object.orient)}),
            );
        }
        for (index, fact) in problem.facts.iter().enumerate() {
            put(
                "facts",
                index.to_string(),
                json!({"kind":format!("{:?}",canonical_fact(&fact.kind)),"priority":format!("{:?}",fact.priority),"chain":fact.chain}),
            );
        }
        for net in &problem.nets {
            put("networks", net.name.clone(), json!({"pins":format!("{:?}",net.pins),"port":net.port}));
        }
        for (index, wire) in problem.wires.iter().enumerate() {
            put(
                "wires",
                index.to_string(),
                json!({"nets":wire.nets,"anchors":format!("{:?}",wire.anchors),"mode":wire.mode,"bus":wire.bus}),
            );
        }
        for (index, route) in problem.routes.iter().enumerate() {
            put("routes", index.to_string(), json!(format!("{route:?}")));
        }
        for (index, group) in problem.groups.iter().enumerate() {
            put("groups", index.to_string(), json!(group.members));
        }
    }
    out
}

/// The differences, and apart from them the changes that only the default look of the standard
/// package of 0.4 explains (see [`without_look`]). What is not sure to be one stays a difference.
pub(super) fn compare(before: &Interpretation, after: &Interpretation) -> (Vec<Difference>, Vec<Difference>) {
    let mut before = snapshot(before);
    let after = snapshot(after);
    let mut differences = Vec::new();
    let mut look = Vec::new();
    for ((file, level, category, item), value) in after {
        let old = before.remove(&(file.clone(), level.clone(), category.clone(), item.clone()));
        if old.as_ref() == Some(&value) {
            continue;
        }
        let only_look = old.as_ref().is_some_and(|old| without_look(&category, old) == without_look(&category, &value));
        let difference = Difference { file, level, category, item, before: old, after: Some(value) };
        if only_look {
            look.push(difference);
        } else {
            differences.push(difference);
        }
    }
    differences.extend(before.into_iter().map(|((file, level, category, item), value)| Difference {
        file,
        level,
        category,
        item,
        before: Some(value),
        after: None,
    }));
    (differences, look)
}
