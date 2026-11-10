//! Direction witnesses confined to one ordinary definition body.
use super::approximate::Choice;
use rnl_runtime::host::Record;
use rnl_runtime::syntax::{parse_record_features, Coord, Member, ObjExpr, SetExpr, Value};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Scoped(BTreeMap<String, BTreeMap<usize, Choice>>);
struct Constraint {
    at: usize,
    axis: char,
    sense: Option<bool>,
    witness: bool,
    items: Vec<String>,
    coordinates: Option<Vec<f64>>,
}
fn items(object: &ObjExpr, axis: char, text: &str, names: &mut Vec<String>, at: &mut Vec<Option<f64>>) {
    match object {
        ObjExpr::Ref(r) => {
            names.push(text.chars().skip(r.start).take(r.end - r.start).collect());
            at.push(None);
        }
        ObjExpr::Point(p) => {
            names.push(text.chars().skip(p.start).take(p.end - p.start).collect());
            at.push(match if axis == 'x' { &p.x } else { &p.y } {
                Coord::Num(n) => Some(*n),
                _ => None,
            });
        }
        ObjExpr::Set(s) => {
            for m in &s.members {
                for seg in &m.segments {
                    if seg.call.is_none() {
                        for a in &seg.args {
                            items(&a.obj, axis, text, names, at);
                        }
                    }
                }
            }
        }
    }
}
fn inspect(body: &SetExpr, text: &str, out: &mut BTreeMap<usize, Choice>) {
    let mut constraints = Vec::new();
    for m in &body.members {
        for s in &m.segments {
            let Some(call) = s.call.as_ref().filter(|c| matches!(c.kw.as_str(), "ORDER" | "DISTANCE" | "DISTRIBUTE"))
            else {
                continue;
            };
            if s.piped {
                continue;
            }
            let Some(p) = &call.params else { continue };
            let Some(Value::Str(d)) =
                p.positional.first().or_else(|| p.get(if call.kw == "DISTRIBUTE" { "axis" } else { "direction" }))
            else {
                continue;
            };
            let (axis, sense) = match d.as_str() {
                "x" => ('x', None),
                "y" => ('y', None),
                "left-to-right" => ('x', Some(true)),
                "right-to-left" => ('x', Some(false)),
                "bottom-to-top" => ('y', Some(true)),
                "top-to-bottom" => ('y', Some(false)),
                _ => continue,
            };
            let mut names = Vec::new();
            let mut at = Vec::new();
            for a in &s.args {
                items(&a.obj, axis, text, &mut names, &mut at);
            }
            let witness = call.kw == "ORDER"
                || (call.kw == "DISTANCE"
                    && matches!(p.get("min").or_else(||p.get("exact")),Some(Value::Num{value,..}) if *value>=0.0));
            constraints.push(Constraint {
                at: call.start,
                axis,
                sense,
                witness,
                items: names,
                coordinates: at.into_iter().collect(),
            });
        }
    }
    for c in &constraints {
        if c.sense.is_some() {
            continue;
        }
        let positions = c.coordinates.as_ref().and_then(|v| {
            if v.len() < 2 || v.first() == v.last() {
                None
            } else if v.windows(2).all(|w| w[0] <= w[1]) {
                Some(true)
            } else if v.windows(2).all(|w| w[0] >= w[1]) {
                Some(false)
            } else {
                None
            }
        });
        let mut witnesses = BTreeSet::new();
        for other in constraints.iter().filter(|o| o.witness && o.axis == c.axis && o.sense.is_some()) {
            for i in 0..c.items.len() {
                for j in i + 1..c.items.len() {
                    if let (Some(a), Some(b)) = (
                        other.items.iter().position(|n| n == &c.items[i]),
                        other.items.iter().position(|n| n == &c.items[j]),
                    ) {
                        if a != b {
                            witnesses.insert((a < b) == other.sense.unwrap());
                        }
                    }
                }
            }
        }
        let (increasing, evidence) = if let Some(p) = positions {
            (p, "literal positions inside the same definition body")
        } else if witnesses.len() == 1 {
            (*witnesses.first().unwrap(), "an authored directed ORDER or DISTANCE inside the same definition body")
        } else {
            (c.axis == 'x', "written role order inside the definition (no unambiguous local witness)")
        };
        out.insert(
            c.at,
            Choice {
                direction: match (c.axis, increasing) {
                    ('x', true) => "left-to-right",
                    ('x', false) => "right-to-left",
                    (_, true) => "bottom-to-top",
                    (_, false) => "top-to-bottom",
                }
                .into(),
                evidence,
            },
        );
    }
}
fn visit(member: &Member, text: &str, out: &mut BTreeMap<usize, Choice>) {
    for s in &member.segments {
        for a in &s.args {
            if let ObjExpr::Set(body) = &a.obj {
                if s.call.as_ref().is_some_and(|c| c.kw == "DEFINE") {
                    inspect(body, text, out);
                }
                for m in &body.members {
                    visit(m, text, out);
                }
            }
        }
    }
}
impl Scoped {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut out = Self::default();
        for level in &input.doc.levels {
            for record in &level.records {
                if let Some(body) = parse_record_features(&record.text, true, false).body {
                    let choices = out.0.entry(record.text.clone()).or_default();
                    for m in &body.members {
                        visit(m, &record.text, choices);
                    }
                }
            }
        }
        out
    }
    pub(super) fn get(&self, record: &Record, at: usize) -> Option<&Choice> {
        self.0.get(&record.text)?.get(&at)
    }
}
