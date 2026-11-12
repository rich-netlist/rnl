use crate::json::{Node, Result};
use rnl_runtime::{
    geom::Orient,
    problem::{PointRef, Reference},
};
use serde_json::{json, Value};

pub(crate) fn orientation(n: Node<'_>) -> Result<Orient> {
    Orient::parse(n.string()?).ok_or_else(|| n.error("unknown orientation"))
}

pub(crate) fn reference_value(r: &Reference) -> Value {
    match r {
        Reference::Anonymous => json!({"kind":"anonymous"}),
        Reference::Name(name) => json!({"kind":"name", "name":name}),
        Reference::Number { number, path } => json!({"kind":"number", "number":number, "path":path}),
        Reference::Generated { usage, path } => json!({"kind":"generated", "usage":usage, "path":path}),
        Reference::GeneratedNumber { usage, index, path } => {
            json!({"kind":"generated-number", "usage":usage, "index":index, "path":path})
        }
    }
}

pub(crate) fn reference(n: Node<'_>) -> Result<Reference> {
    let path = || n.field("path")?.list(|s| Ok(s.string()?.to_owned()));
    Ok(match n.field("kind")?.string()? {
        "anonymous" => Reference::Anonymous,
        "name" => Reference::Name(n.field("name")?.string()?.to_owned()),
        "number" => Reference::Number { number: n.field("number")?.index()?, path: path()? },
        "generated" => Reference::Generated { usage: n.field("usage")?.index()?, path: path()? },
        "generated-number" => Reference::GeneratedNumber {
            usage: n.field("usage")?.index()?,
            index: n.field("index")?.index()?,
            path: path()?,
        },
        _ => return Err(n.field("kind")?.error("unknown reference kind")),
    })
}

pub(crate) fn point_value(p: &PointRef) -> Value {
    match p {
        PointRef::Origin(k) => json!({"kind":"origin", "object":k}),
        PointRef::Pin { object, pin, terminal } => {
            json!({"kind":"pin", "object":object, "pin":pin, "terminal":terminal})
        }
        PointRef::SymbolPin { object, pin, reference } => {
            json!({"kind":"symbol-pin", "object":object, "pin":pin, "reference":reference_value(reference)})
        }
        PointRef::PinPosition { object, pin, reference } => {
            json!({"kind":"pin-position", "object":object, "pin":pin, "reference":reference_value(reference)})
        }
        PointRef::Centre(members) => {
            json!({"kind":"centre", "members":members.iter().map(point_value).collect::<Vec<_>>()})
        }
    }
}

pub(crate) fn point(n: Node<'_>, depth: usize) -> Result<PointRef> {
    if depth > 64 {
        return Err(n.error("point nesting exceeds 64 levels"));
    }
    Ok(match n.field("kind")?.string()? {
        "origin" => PointRef::Origin(n.field("object")?.index()?),
        "pin" => PointRef::Pin {
            object: n.field("object")?.index()?,
            pin: n.field("pin")?.index()?,
            terminal: n.field("terminal")?.string()?.to_owned(),
        },
        "symbol-pin" => PointRef::SymbolPin {
            object: n.field("object")?.index()?,
            pin: n.field("pin")?.index()?,
            reference: reference(n.field("reference")?)?,
        },
        "pin-position" => PointRef::PinPosition {
            object: n.field("object")?.index()?,
            pin: n.field("pin")?.index()?,
            reference: reference(n.field("reference")?)?,
        },
        "centre" => PointRef::Centre(n.field("members")?.list(|n| point(n, depth + 1))?),
        _ => return Err(n.field("kind")?.error("unknown point kind")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_reference_paths_preserve_all_variants() {
        for r in [
            Reference::Anonymous,
            Reference::Name("%root::\"quoted name\"".into()),
            Reference::Number { number: 3, path: vec!["::first".into(), ":terminal".into()] },
            Reference::Generated { usage: 2, path: vec!["::part".into(), "::pin".into()] },
            Reference::GeneratedNumber { usage: 4, index: 2, path: vec!["::pin".into(), "::position".into()] },
        ] {
            let bytes = serde_json::to_vec(&reference_value(&r)).unwrap();
            let value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(reference(Node::root(&value)).unwrap(), r);
        }
    }
}
