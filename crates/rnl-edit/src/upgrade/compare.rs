//! Compare semantic values, excluding source locations and package version metadata.

use std::collections::BTreeMap;

use rnl_runtime::problem::{FactKind, PointRef, Sense};
use rnl_runtime::Interpretation;
use serde_json::{json, Value};

use super::Difference;

type Key = (String, String, String, String);

fn canonical_fact(kind: &FactKind) -> FactKind {
    let mut kind = kind.clone();
    if let FactKind::Distance { sense, min, max, from, to, .. } = &mut kind {
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
        "labels":s.labels.iter().map(|l|json!({"key":l.key,"text":l.text,"at":l.at,"halign":l.halign,"valign":l.valign,"rotate":l.rotate,"style":format!("{:?}",l.style)})).collect::<Vec<_>>(),
        "primitives":s.prims.iter().map(|p|json!({"order":p.order,"shape":p.shape.as_str(),"points":p.points,"params":format!("{:?}",p.params),"style":format!("{:?}",p.style),"text":p.text,"halign":p.halign,"valign":p.valign,"rotate":p.rotate})).collect::<Vec<_>>(),
        "nested":s.nested.iter().map(|n|json!({"order":n.order,"at":n.at,"orientation":n.orient.as_str(),"symbol":symbol(table,n.symbol,depth+1)})).collect::<Vec<_>>()})
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
                    "pins":object.pins.iter().map(|p| json!({"at":p.at,"nets":p.nets,"direction":p.dir})).collect::<Vec<_>>()}),
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
                json!({"at":text.anchor,"markup":text.markup,"lines":format!("{:?}",text.lines),"halign":text.halign,"valign":text.valign,"rotate":text.rotate,"orientation":text.orient.as_str(),"style":format!("{:?}",text.style),"layer":text.layer.as_str()}),
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
                json!({"kind":object.kind.as_str(),"position":format!("{:?}",object.position),"orientation":format!("{:?}",object.orient)}),
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

pub(super) fn compare(before: &Interpretation, after: &Interpretation) -> Vec<Difference> {
    let mut before = snapshot(before);
    let after = snapshot(after);
    let mut differences = Vec::new();
    for ((file, level, category, item), value) in after {
        let old = before.remove(&(file.clone(), level.clone(), category.clone(), item.clone()));
        if old.as_ref() != Some(&value) {
            differences.push(Difference { file, level, category, item, before: old, after: Some(value) });
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
    differences
}
