//! Compare semantic values, excluding source locations and package version metadata.

use std::collections::BTreeMap;

use rnl_runtime::Interpretation;
use serde_json::{json, Value};

use super::Difference;

type Key = (String, String, String, String);

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
                    "geometry":object.symbol_view.and_then(|i| level.symbols.get(i)).map(|s| json!({
                        "box":s.bbox,"primitives":s.prims.iter().map(|p| json!({
                            "shape":p.shape.as_str(),"points":p.points,"params":format!("{:?}",p.params),
                            "style":format!("{:?}",p.style),"text":p.text,"halign":p.halign,"valign":p.valign,"rotate":p.rotate
                        })).collect::<Vec<_>>(),"nested":format!("{:?}",s.nested)
                    })),
                    "pins":object.pins.iter().map(|p| json!({"at":p.at,"nets":p.nets,"direction":p.dir})).collect::<Vec<_>>()}),
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
                json!({"kind":format!("{:?}",fact.kind),"priority":format!("{:?}",fact.priority),"chain":fact.chain}),
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
