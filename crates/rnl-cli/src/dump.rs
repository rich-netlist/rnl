//! The effective model and the diagnostics as JSON.

use serde_json::{json, Value};

use rnl_runtime::interp::obj::Lit;
use rnl_runtime::markup::{Line, Run};
use rnl_runtime::model::*;
use rnl_runtime::Interpretation;

fn lit(v: &Lit) -> Value {
    match v {
        Lit::Num(n) => json!(n),
        Lit::Str(s) => json!(s.as_str()),
        Lit::Bool(b) => json!(b),
        Lit::Arr(items) => Value::Array(items.iter().map(lit).collect()),
    }
}

fn style(s: &Style) -> Value {
    Value::Object(s.iter().map(|(k, v)| (k.clone(), lit(v))).collect())
}

fn runs(rs: &[Run]) -> Value {
    Value::Array(
        rs.iter()
            .map(|r| match r {
                Run::Text { text, style } => json!({"text": text, "level": style.level, "bold": style.bold, "italic": style.italic, "overline": style.overline}),
                Run::Frac { num, den, .. } => json!({"frac": [runs(num), runs(den)]}),
            })
            .collect(),
    )
}

fn lines(ls: &[Line]) -> Value {
    Value::Array(ls.iter().map(|l| runs(l)).collect())
}

fn pt(p: &(f64, f64)) -> Value {
    json!([p.0, p.1])
}

pub fn level(lv: &LevelView) -> Value {
    json!({
        "title": lv.title,
        "kind": lv.kind,
        "file": lv.file,
        "symbols": lv.symbols.iter().map(|s| json!({
            "name": s.name,
            "box": s.bbox,
            "box_written": s.box_written,

            "default": s.default,
            "pins": s.pins.iter().map(|p| json!({"pos": pt(&p.pos), "terminals": p.terminals, "label": p.label, "dir": p.dir})).collect::<Vec<_>>(),
            "prims": s.prims.iter().map(|p| json!({"shape": p.shape.as_str(), "points": p.points.iter().map(pt).collect::<Vec<_>>(), "style": style(&p.style)})).collect::<Vec<_>>(),
            "nested": s.nested.iter().map(|n| json!({"symbol": n.symbol, "at": pt(&n.at), "orient": n.orient.as_str()})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "items": lv.items.iter().map(|i| json!({
            "id": i.id, "kind": i.kind, "symbol": i.symbol, "at": pt(&i.at), "orient": i.orient.as_str(),
            "style": style(&i.style), "layer": i.layer.as_str(),
            "texts": i.texts.iter().map(|(k, l)| (k.clone(), lines(l))).collect::<serde_json::Map<_, _>>(),
        })).collect::<Vec<_>>(),
        "wires": lv.wires.iter().map(|w| json!({
            "id": w.id, "nets": w.nets, "points": w.points.iter().map(pt).collect::<Vec<_>>(), "mode": w.mode,
            "style": style(&w.style), "layer": w.layer.as_str(),
        })).collect::<Vec<_>>(),
        "shapes": lv.shapes.iter().map(|s| json!({
            "shape": s.shape.as_str(), "points": s.points.iter().map(pt).collect::<Vec<_>>(), "style": style(&s.style), "layer": s.layer.as_str(),
        })).collect::<Vec<_>>(),
        "texts": lv.texts.iter().map(|t| json!({
            "id": t.id, "anchor": pt(&t.anchor), "lines": lines(&t.lines), "halign": t.halign, "valign": t.valign,
            "rotate": t.rotate, "orient": t.orient.as_str(), "style": style(&t.style), "layer": t.layer.as_str(),
        })).collect::<Vec<_>>(),
        "flylines": lv.flylines.iter().map(|(a, b)| json!([pt(a), pt(b)])).collect::<Vec<_>>(),
        "constraints": lv.constraints.iter().map(|c| json!({"label": c.label, "kind": c.kind, "status": c.status, "priority": c.priority})).collect::<Vec<_>>(),
        "unplaced": lv.unplaced,
    })
}

pub fn interpretation(r: &Interpretation) -> Value {
    json!({
        "failed": r.model.failed,
        "levels": r.model.levels.iter().map(level).collect::<Vec<_>>(),
        "diagnostics": r.diagnostics().iter().map(|d| json!({
            "severity": d.severity.as_str(),
            "message": d.message,
            "file": d.loc.as_ref().map(|l| l.file.clone()),
            "line": d.loc.as_ref().map(|l| l.line),
            "col": d.loc.as_ref().map(|l| l.col),
            "origin": d.origin,
        })).collect::<Vec<_>>(),
    })
}

/// What the conformance corpus records (7.6): the effective model, and each diagnostic's level
/// and place. Messages are left out: their wording is the implementation's.
pub fn conformance(r: &Interpretation) -> Value {
    json!({
        "failed": r.model.failed.is_some(),
        "levels": r.model.levels.iter().map(level).collect::<Vec<_>>(),
        "diagnostics": r.diagnostics().iter().map(|d| json!({
            "severity": d.severity.as_str(),
            "file": d.loc.as_ref().map(|l| l.file.clone()),
            "line": d.loc.as_ref().map(|l| l.line),
            "col": d.loc.as_ref().map(|l| l.col),
        })).collect::<Vec<_>>(),
    })
}
