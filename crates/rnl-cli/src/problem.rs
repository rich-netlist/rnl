//! The solver's view of each level (rnl_runtime::problem) as JSON, for `rnl problem`.

use serde_json::{json, Value};

use rnl_runtime::geom::Pt;
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::interp::unit::DRAFT_VERSION;
use rnl_runtime::markup::plain;
use rnl_runtime::previous::Previous;
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

/// A reference; a number only in a level of a file of the draft 0.3, where the record of the
/// solver writes it (5.9), and otherwise none, as for an object without a reference.
fn reference(r: &Reference, numbers: bool) -> Value {
    match r {
        Reference::Name(n) => json!(n),
        Reference::Generated { usage, path } => json!({"generated_by": usage, "path": path.concat()}),
        Reference::Number { number, path } if numbers => json!({"number": number, "path": path.concat()}),
        Reference::GeneratedNumber { usage, index, path } if numbers => {
            json!({"generated_by": usage, "number": index, "path": path.concat()})
        }
        Reference::Number { .. } | Reference::GeneratedNumber { .. } | Reference::Anonymous => Value::Null,
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
        Position::InSymbol { owner, local, replaced } => {
            let mut v = json!({"in_symbol": owner, "local": [local.0, local.1]});
            if let Some((axis, s)) = replaced {
                v.as_object_mut().unwrap().insert(axis.as_str().into(), slot(s));
            }
            v
        }
    }
}

fn fact(f: &ProblemFact, numbers: bool) -> Value {
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
    match &f.reference {
        Some(Reference::Number { .. } | Reference::GeneratedNumber { .. }) if !numbers => {}
        Some(r) => {
            m.insert("reference".into(), reference(r, numbers));
        }
        None => {}
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

/// One level's problem, with its `sig2` hash for the solver options given. The references are
/// numbers, `{"number": 3, "path": "::mid"}` and `{"generated_by": 2, "number": 1, "path": ""}`,
/// only in a level of a file of the draft 0.3, which alone has `authored_numbers` and the
/// `written_numbers` of each usage as well.
pub fn level(p: &LevelProblem, solver_options: &str) -> Value {
    let numbers = p.version == DRAFT_VERSION;
    let mut v = json!({
        "level": p.level,
        "title": p.title,
        "kind": p.kind,
        "name": p.name,
        "file": p.file,
        "line": p.line,
        "end_line": p.end_line,
        "hash": sig2(p, solver_options),
        "packages": p.packages.iter().map(|(n, v)| json!([n, v])).collect::<Vec<_>>(),
        "names": p.names,
        "objects": p.objects.iter().enumerate().map(|(k, o)| {
            let mut v = json!({
                "id": k,
                "kind": o.kind.as_str(),
                "label": o.label,
                "reference": reference(&o.reference, numbers),
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
            if !o.nets.is_empty() {
                m.insert("nets".into(), json!(o.nets));
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
        "facts": p.facts.iter().map(|f| fact(f, numbers)).collect::<Vec<_>>(),
        "nets": p.nets.iter().map(|n| json!({
            "name": n.name, "reference": reference(&n.reference, numbers), "pins": points(&n.pins),
            "wires": n.wires, "routes": n.routes, "port": n.port,
        })).collect::<Vec<_>>(),
        "wires": p.wires.iter().map(|w| json!({
            "label": w.label, "reference": reference(&w.reference, numbers), "nets": w.nets, "bus": w.bus,
            "anchors": points(&w.anchors), "mode": w.mode, "generated": w.generated,
        })).collect::<Vec<_>>(),
        "routes": p.routes.iter().map(|r| json!({
            "net": r.net, "mode": r.mode,
            "style": r.style.iter().map(|(k, v)| (k.clone(), lit(v))).collect::<serde_json::Map<_, _>>(),
            "pins": points(&r.pins), "wires": r.wires,
        })).collect::<Vec<_>>(),
        "groups": p.groups.iter().map(|g| json!({"label": g.label, "members": g.members})).collect::<Vec<_>>(),
        "usages": p.usages.iter().map(|u| {
            let mut entry = json!({
                "definition": u.definition,
                "label": u.label,
                "reference": reference(&u.reference, numbers),
                "generated": u.generated,
                "rule_made": u.rule_made,
                "roles": u.roles.iter().map(|r| json!({
                    "role": r.role, "members": r.members.iter().map(filler).collect::<Vec<_>>(), "written": r.written,
                })).collect::<Vec<_>>(),
                "params": u.params.iter().map(|(k, v)| (k.clone(), lit(v))).collect::<serde_json::Map<_, _>>(),
                "priority": u.priority.map(|p| p.as_str()),
                "parent": u.parent,
                "object": u.object,
                "source": u.source.as_ref().map(|l| json!({"file": l.file, "line": l.line, "col": l.col})),
            });
            if numbers {
                entry.as_object_mut().unwrap().insert("written_numbers".into(), json!(u.written_numbers));
            }
            entry
        }).collect::<Vec<_>>(),
    });
    if numbers {
        v.as_object_mut().unwrap().insert("authored_numbers".into(), json!(p.authored_numbers));
    }
    v
}

/// The previous result of a level, for `rnl problem --previous`: null when the level has no record
/// of the solver, otherwise what the record holds that maps onto the problem, by the indexes of the
/// problem's objects and wires, and whether it is up to date.
pub fn previous(p: Option<&Previous>) -> Value {
    let Some(p) = p else { return Value::Null };
    let path = |points: &[Pt]| points.iter().map(|(x, y)| json!([x, y])).collect::<Vec<_>>();
    json!({
        "hash": p.hash,
        "same_input": p.same_input,
        "numbering": p.numbering,
        "placements": p.placements.iter().map(|pl| json!({
            "object": pl.object,
            "x": pl.x,
            "y": pl.y,
            "orient": pl.orient.map(|o| o.as_str()),
        })).collect::<Vec<_>>(),
        "wire_routes": p.wire_routes.iter().map(|r| json!({"wire": r.wire, "points": path(&r.points)})).collect::<Vec<_>>(),
        "wires": p.wires.iter().map(|w| json!({"nets": w.nets, "points": path(&w.points)})).collect::<Vec<_>>(),
    })
}
