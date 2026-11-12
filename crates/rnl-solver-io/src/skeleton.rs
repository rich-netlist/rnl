//! The final line-layout description, with checked dimensions and finite coordinates.
use crate::json::{Node, Result};
use rnl_solver::{LayoutSkeleton, LineKind, SkeletonItem, SkeletonLine, SkeletonTable, SkeletonTree};
use serde_json::{json, Value};

pub(crate) fn encode(s: &LayoutSkeleton) -> Value {
    json!({
        "fallback":s.fallback,
        "tables":s.tables.iter().map(|t| json!({"group":t.group,"rows":t.rows,"columns":t.columns,"xs":t.xs,"ys":t.ys,
            "items":t.items.iter().map(|i| json!({"label":i.label,"group":i.group,"rows":[i.rows.0,i.rows.1],"columns":[i.columns.0,i.columns.1]})).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "lines":s.lines.iter().map(|l|json!({"axis":l.axis,"kind":l.kind.name(),"rank":l.rank,"at":l.at,"owner":l.owner,"tree":l.tree})).collect::<Vec<_>>(),
        "trees":s.trees.iter().map(|t|json!({"route":t.route,"net":t.net,"xs":t.xs,"ys":t.ys})).collect::<Vec<_>>()
    })
}

fn numbers(n: Node<'_>) -> Result<Vec<f64>> {
    n.list(|v| v.number())
}
fn indices(n: Node<'_>) -> Result<Vec<usize>> {
    n.list(|v| v.index())
}
fn range(n: Node<'_>, bound: usize) -> Result<(usize, usize)> {
    let values = indices(n.clone())?;
    let [first, end] = values.as_slice() else {
        return Err(n.error("expected a two-index half-open range"));
    };
    if first > end || *end > bound {
        return Err(n.error("table item range is outside its table"));
    }
    Ok((*first, *end))
}

pub(crate) fn decode(n: Node<'_>) -> Result<LayoutSkeleton> {
    let tables = n.field("tables")?.list(|t| {
        let rows = t.field("rows")?.index()?;
        let columns = t.field("columns")?.index()?;
        let xs = numbers(t.field("xs")?)?;
        let ys = numbers(t.field("ys")?)?;
        if Some(xs.len()) != columns.checked_add(1) || Some(ys.len()) != rows.checked_add(1) {
            return Err(t.error("table cuts must have one more coordinate than rows or columns"));
        }
        let items = t.field("items")?.list(|i| {
            Ok(SkeletonItem {
                label: i.field("label")?.string()?.to_owned(),
                group: i.field("group")?.boolean()?,
                rows: range(i.field("rows")?, rows)?,
                columns: range(i.field("columns")?, columns)?,
            })
        })?;
        Ok(SkeletonTable {
            group: t.field("group")?.nullable(|v| Ok(v.string()?.to_owned()))?,
            rows,
            columns,
            xs,
            ys,
            items,
        })
    })?;
    let lines = n.field("lines")?.list(|l| {
        let axis = l.field("axis")?.index()?;
        if axis > 1 {
            return Err(l.field("axis")?.error("line axis must be zero or one"));
        }
        let kind = match l.field("kind")?.string()? {
            "side" => LineKind::Side,
            "pin" => LineKind::Pin,
            "wire" => LineKind::Wire,
            "group" => LineKind::Group,
            _ => return Err(l.field("kind")?.error("unknown skeleton line kind")),
        };
        Ok(SkeletonLine {
            axis: axis as u8,
            kind,
            rank: l.field("rank")?.nullable(|v| v.index())?,
            at: l.field("at")?.number()?,
            owner: l.field("owner")?.string()?.to_owned(),
            tree: l.field("tree")?.nullable(|v| v.index())?,
        })
    })?;
    let trees = n.field("trees")?.list(|t| {
        Ok(SkeletonTree {
            route: t.field("route")?.index()?,
            net: t.field("net")?.string()?.to_owned(),
            xs: indices(t.field("xs")?)?,
            ys: indices(t.field("ys")?)?,
        })
    })?;
    Ok(LayoutSkeleton { tables, lines, trees, fallback: n.field("fallback")?.boolean()? })
}
