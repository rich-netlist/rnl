//! The solver's view of each level (rnl_runtime::problem) as JSON, for `rnl problem`.

use serde_json::{json, Value};

use rnl_runtime::interp::obj::Lit;
use rnl_runtime::markup::plain;
use rnl_runtime::problem::*;

fn lit(v: &Lit) -> Value {
    match v {
        Lit::Num(n) => json!(n),
        Lit::Str(s) => json!(s.as_str()),
        Lit::Bool(b) => json!(b),
        Lit::Arr(items) => Value::Array(items.iter().map(lit).collect()),
    }
}

fn point(r: &PointRef) -> Value {
    match r {
        PointRef::Origin(k) => json!({"origin": k}),
        PointRef::Pin { object, pin, terminal } => json!({"object": object, "pin": pin, "terminal": terminal}),
        PointRef::Centre(points) => json!({"centre": points.iter().map(point).collect::<Vec<_>>()}),
    }
}

fn points(rs: &[PointRef]) -> Value {
    Value::Array(rs.iter().map(point).collect())
}

fn slot(s: &Slot) -> Value {
    match s {
        Slot::Free => json!("free"),
        Slot::Fixed { value, fact } => json!({"value": value, "fact": fact}),
    }
}

fn reference(r: &Reference) -> Value {
    match r {
        Reference::Name(n) => json!(n),
        Reference::Generated { usage, path } => json!({"generated_by": usage, "path": path.concat()}),
        Reference::Anonymous => Value::Null,
    }
}

fn filler(f: &Filler) -> Value {
    match f {
        Filler::Object(k) => json!({"object": k}),
        Filler::Terminal { object, terminal, pin } => json!({"object": object, "terminal": terminal, "pin": pin}),
        Filler::Usage(k) => json!({"usage": k}),
        Filler::Net(n) => json!({"net": n}),
        Filler::Wire(k) => json!({"wire": k}),
        Filler::Other(l) => json!({"other": l}),
    }
}

fn position(p: &Position) -> Value {
    match p {
        Position::Own { x, y } => json!({"x": slot(x), "y": slot(y)}),
        Position::At(r) => json!({"at": point(r)}),
        Position::InSymbol { owner, local } => json!({"in_symbol": owner, "local": [local.0, local.1]}),
    }
}

fn fact(f: &ProblemFact) -> Value {
    let mut v = json!({"kind": f.kind.name()});
    let m = v.as_object_mut().unwrap();
    match &f.kind {
        FactKind::Place { at, value, .. } => {
            m.insert("at".into(), point(at));
            m.insert("value".into(), json!(value));
        }
        FactKind::PointCoordinate { point: k, value, .. } => {
            m.insert("point".into(), json!(k));
            m.insert("value".into(), json!(value));
        }
        FactKind::Offset { text, from, value, .. } => {
            m.insert("text".into(), json!(text));
            m.insert("from".into(), point(from));
            m.insert("value".into(), json!(value));
        }
        FactKind::Orient { object, code } => {
            m.insert("object".into(), json!(object));
            m.insert("code".into(), json!(code.as_str()));
        }
        FactKind::Align { axis, members } | FactKind::Distribute { axis, members } => {
            m.insert("axis".into(), json!(axis.as_str()));
            m.insert("members".into(), points(members));
        }
        FactKind::Order { axis, sense, members } => {
            m.insert("axis".into(), json!(axis.as_str()));
            m.insert("sense".into(), json!(sense.as_str()));
            m.insert("members".into(), points(members));
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            m.insert("axis".into(), json!(axis.as_str()));
            m.insert("sense".into(), json!(sense.as_str()));
            m.insert("min".into(), json!(min));
            m.insert("max".into(), json!(max));
            m.insert("from".into(), points(from));
            m.insert("to".into(), points(to));
        }
        FactKind::OrientRelation { relation, from, to } => {
            m.insert("relation".into(), json!(relation.as_str()));
            m.insert("from".into(), json!(from));
            m.insert("to".into(), json!(to));
        }
    }
    m.insert("priority".into(), json!(f.priority.as_str()));
    m.insert("label".into(), json!(f.label));
    if let Some(r) = &f.reference {
        m.insert("reference".into(), reference(r));
    }
    m.insert(
        "source".into(),
        match &f.source {
            Some(l) => json!({"file": l.file, "line": l.line, "col": l.col}),
            None => Value::Null,
        },
    );
    m.insert("chain".into(), json!(f.chain));
    v
}

/// One level's problem, with its `sig1` hash for the solver options given.
pub fn level(p: &LevelProblem, solver_options: &str) -> Value {
    json!({
        "level": p.level,
        "title": p.title,
        "kind": p.kind,
        "name": p.name,
        "file": p.file,
        "line": p.line,
        "end_line": p.end_line,
        "hash": sig1(p, solver_options),
        "packages": p.packages.iter().map(|(n, v)| json!([n, v])).collect::<Vec<_>>(),
        "names": p.names,
        "objects": p.objects.iter().enumerate().map(|(k, o)| {
            let mut v = json!({
                "id": k,
                "kind": o.kind.as_str(),
                "label": o.label,
                "reference": reference(&o.reference),
                "generated": o.generated,
                "symbol": o.symbol,
                "position": position(&o.position),
            });
            let m = v.as_object_mut().unwrap();
            match &o.orient {
                Some(OrientSlot::Free) => { m.insert("orient".into(), json!("free")); }
                Some(OrientSlot::Fixed { code, fact }) => { m.insert("orient".into(), json!({"code": code.as_str(), "fact": fact})); }
                None => {}
            }
            if let Some(u) = o.usage {
                m.insert("usage".into(), json!(u));
            }
            if let Some(t) = &o.text {
                m.insert("text".into(), json!({
                    "attach": filler(&t.attach), "key": t.key,
                    "lines": t.lines.iter().map(|l| plain(l)).collect::<Vec<_>>(),
                    "halign": t.halign, "valign": t.valign, "rotate": t.rotate,
                    "style": t.style.iter().map(|(k, v)| (k.clone(), lit(v))).collect::<serde_json::Map<_, _>>(),
                }));
            }
            v
        }).collect::<Vec<_>>(),
        "symbols": p.symbols.iter().map(|s| json!({
            "name": s.name,
            "box": s.bbox,
            "box_written": s.box_written,
            "default": s.default,
            "pins": s.pins.iter().map(|q| json!({"pos": [q.pos.0, q.pos.1], "dir": q.dir, "terminals": q.terminals, "label": q.label})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "facts": p.facts.iter().map(fact).collect::<Vec<_>>(),
        "wires": p.wires.iter().map(|w| json!({
            "label": w.label, "reference": reference(&w.reference), "nets": w.nets, "bus": w.bus,
            "anchors": points(&w.anchors), "mode": w.mode, "generated": w.generated,
        })).collect::<Vec<_>>(),
        "routes": p.routes.iter().map(|r| json!({
            "net": r.net, "mode": r.mode,
            "style": r.style.iter().map(|(k, v)| (k.clone(), lit(v))).collect::<serde_json::Map<_, _>>(),
            "pins": points(&r.pins), "wires": r.wires,
        })).collect::<Vec<_>>(),
        "groups": p.groups.iter().map(|g| json!({"label": g.label, "members": g.members})).collect::<Vec<_>>(),
        "usages": p.usages.iter().map(|u| json!({
            "definition": u.definition,
            "label": u.label,
            "reference": reference(&u.reference),
            "generated": u.generated,
            "rule_made": u.rule_made,
            "roles": u.roles.iter().map(|r| json!({
                "role": r.role, "members": r.members.iter().map(filler).collect::<Vec<_>>(), "written": r.written,
            })).collect::<Vec<_>>(),
            "params": u.params.iter().map(|(k, v)| (k.clone(), lit(v))).collect::<serde_json::Map<_, _>>(),
            "priority": u.priority.map(|p| p.as_str()),
            "parent": u.parent,
            "object": u.object,
        })).collect::<Vec<_>>(),
    })
}
