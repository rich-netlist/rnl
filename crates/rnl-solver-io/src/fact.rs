use crate::{
    data,
    json::{Node, Result},
    point::{orientation, point, point_value, reference, reference_value},
};
use rnl_runtime::problem::*;
use serde_json::{json, Value};

pub(crate) fn axis(n: Node<'_>) -> Result<Axis> {
    match n.string()? {
        "x" => Ok(Axis::X),
        "y" => Ok(Axis::Y),
        _ => Err(n.error("unknown axis")),
    }
}
fn sense(n: Node<'_>) -> Result<Sense> {
    match n.string()? {
        "increasing" => Ok(Sense::Increasing),
        "decreasing" => Ok(Sense::Decreasing),
        "either" => Ok(Sense::Either),
        _ => Err(n.error("unknown direction sense")),
    }
}
fn points_value(ps: &[PointRef]) -> Value {
    json!(ps.iter().map(point_value).collect::<Vec<_>>())
}
fn points(n: Node<'_>) -> Result<Vec<PointRef>> {
    n.list(|n| point(n, 0))
}
fn directed_value(p: DirectedPin) -> Value {
    json!({"object":p.object,"pin":p.pin,"direction":p.direction})
}
fn directed(n: Node<'_>) -> Result<DirectedPin> {
    Ok(DirectedPin {
        object: n.field("object")?.index()?,
        pin: n.field("pin")?.index()?,
        direction: n.field("direction")?.u16()?,
    })
}
fn table_value(t: &TableCell<PointRef>) -> Value {
    match t {
        TableCell::Point(p) => json!({"kind":"point","point":point_value(p)}),
        TableCell::List(items) => json!({"kind":"list","items":items.iter().map(table_value).collect::<Vec<_>>()}),
    }
}
fn table(n: Node<'_>, depth: usize) -> Result<TableCell<PointRef>> {
    if depth > 64 {
        return Err(n.error("TABLE nesting exceeds 64 levels"));
    }
    match n.field("kind")?.string()? {
        "point" => Ok(TableCell::Point(point(n.field("point")?, 0)?)),
        "list" => Ok(TableCell::List(n.field("items")?.list(|n| table(n, depth + 1))?)),
        _ => Err(n.error("unknown TABLE cell kind")),
    }
}
pub(crate) fn kind_value(f: &FactKind) -> Value {
    match f {
        FactKind::Place { at, axis, value } => {
            json!({"kind":"place","at":point_value(at),"axis":axis.as_str(),"value":value})
        }
        FactKind::PointCoordinate { point, axis, value } => {
            json!({"kind":"point-coordinate","point":point,"axis":axis.as_str(),"value":value})
        }
        FactKind::Offset { text, from, axis, value } => {
            json!({"kind":"offset","text":text,"from":point_value(from),"axis":axis.as_str(),"value":value})
        }
        FactKind::Orient { object, code } => json!({"kind":"orient","object":object,"code":code.as_str()}),
        FactKind::PinOrient { pin, codes } => {
            json!({"kind":"pin-orient","pin":directed_value(*pin),"codes":codes.iter().map(|c|c.as_str()).collect::<Vec<_>>()})
        }
        FactKind::PinOrientRelation { from, to, relations } => {
            json!({"kind":"pin-orient-relation","from":directed_value(*from),"to":directed_value(*to),"relations":relations.iter().map(|c|c.as_str()).collect::<Vec<_>>()})
        }
        FactKind::Align { axis, members } => {
            json!({"kind":"align","axis":axis.as_str(),"members":points_value(members)})
        }
        FactKind::Order { axis, sense, members } => {
            json!({"kind":"order","axis":axis.as_str(),"sense":sense.as_str(),"members":points_value(members)})
        }
        FactKind::Distribute { axis, sense, members } => {
            json!({"kind":"distribute","axis":axis.as_str(),"sense":sense.as_str(),"members":points_value(members)})
        }
        FactKind::Distance { axis, sense, min, max, from, to } => {
            json!({"kind":"distance","axis":axis.as_str(),"sense":sense.as_str(),"min":min,"max":max,"from":points_value(from),"to":points_value(to)})
        }
        FactKind::Table { axis, rows } => json!({"kind":"table","axis":axis.as_str(),"rows":table_value(rows)}),
        FactKind::OrientRelation { relation, from, to } => {
            json!({"kind":"orient-relation","relation":relation.as_str(),"from":from,"to":to})
        }
    }
}
fn kind(n: Node<'_>) -> Result<FactKind> {
    Ok(match n.field("kind")?.string()? {
        "place" => FactKind::Place {
            at: point(n.field("at")?, 0)?,
            axis: axis(n.field("axis")?)?,
            value: n.field("value")?.number()?,
        },
        "point-coordinate" => FactKind::PointCoordinate {
            point: n.field("point")?.index()?,
            axis: axis(n.field("axis")?)?,
            value: n.field("value")?.number()?,
        },
        "offset" => FactKind::Offset {
            text: n.field("text")?.index()?,
            from: point(n.field("from")?, 0)?,
            axis: axis(n.field("axis")?)?,
            value: n.field("value")?.number()?,
        },
        "orient" => FactKind::Orient { object: n.field("object")?.index()?, code: orientation(n.field("code")?)? },
        "pin-orient" => {
            FactKind::PinOrient { pin: directed(n.field("pin")?)?, codes: n.field("codes")?.list(orientation)? }
        }
        "pin-orient-relation" => FactKind::PinOrientRelation {
            from: directed(n.field("from")?)?,
            to: directed(n.field("to")?)?,
            relations: n.field("relations")?.list(orientation)?,
        },
        "align" => FactKind::Align { axis: axis(n.field("axis")?)?, members: points(n.field("members")?)? },
        "order" => FactKind::Order {
            axis: axis(n.field("axis")?)?,
            sense: sense(n.field("sense")?)?,
            members: points(n.field("members")?)?,
        },
        "distribute" => FactKind::Distribute {
            axis: axis(n.field("axis")?)?,
            sense: sense(n.field("sense")?)?,
            members: points(n.field("members")?)?,
        },
        "distance" => FactKind::Distance {
            axis: axis(n.field("axis")?)?,
            sense: sense(n.field("sense")?)?,
            min: n.field("min")?.nullable(|n| n.number())?,
            max: n.field("max")?.nullable(|n| n.number())?,
            from: points(n.field("from")?)?,
            to: points(n.field("to")?)?,
        },
        "table" => FactKind::Table { axis: axis(n.field("axis")?)?, rows: table(n.field("rows")?, 0)? },
        "orient-relation" => FactKind::OrientRelation {
            relation: orientation(n.field("relation")?)?,
            from: data::indices(n.field("from")?)?,
            to: data::indices(n.field("to")?)?,
        },
        _ => return Err(n.field("kind")?.error("unknown fact kind")),
    })
}
pub(crate) fn value(f: &ProblemFact) -> Value {
    json!({"kind":kind_value(&f.kind),"priority":f.priority.as_str(),"label":f.label,"reference":f.reference.as_ref().map(reference_value),"source":f.source.as_ref().map(data::loc_value),"chain":f.chain})
}
pub(crate) fn decode(n: Node<'_>) -> Result<ProblemFact> {
    Ok(ProblemFact {
        kind: kind(n.field("kind")?)?,
        priority: data::priority(n.field("priority")?)?,
        label: n.field("label")?.string()?.into(),
        reference: n.field("reference")?.nullable(reference)?,
        source: n.field("source")?.nullable(data::loc)?,
        chain: data::indices(n.field("chain")?)?,
    })
}
