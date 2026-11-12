use crate::{
    data, fact,
    json::{Node, Result},
    point::{orientation, point, point_value, reference, reference_value},
};
use rnl_runtime::{
    model::{PinView, SymbolPinIdentity},
    problem::*,
};
use serde_json::{json, Value};

pub(crate) fn filler_value(f: &Filler) -> Value {
    match f {
        Filler::Object(object) => json!({"kind":"object","object":object}),
        Filler::Terminal { object, terminal, pin } => {
            json!({"kind":"terminal","object":object,"terminal":terminal,"pin":pin})
        }
        Filler::Pin { object, pin } => json!({"kind":"pin","object":object,"pin":pin}),
        Filler::Usage(usage) => json!({"kind":"usage","usage":usage}),
        Filler::Net(net) => json!({"kind":"net","net":net}),
        Filler::Wire(wire) => json!({"kind":"wire","wire":wire}),
        Filler::Other(label) => json!({"kind":"other","label":label}),
    }
}
pub(crate) fn filler(n: Node<'_>) -> Result<Filler> {
    Ok(match n.field("kind")?.string()? {
        "object" => Filler::Object(n.field("object")?.index()?),
        "terminal" => Filler::Terminal {
            object: n.field("object")?.index()?,
            terminal: n.field("terminal")?.string()?.into(),
            pin: n.field("pin")?.nullable(|n| n.index())?,
        },
        "pin" => Filler::Pin { object: n.field("object")?.index()?, pin: n.field("pin")?.index()? },
        "usage" => Filler::Usage(n.field("usage")?.index()?),
        "net" => Filler::Net(n.field("net")?.string()?.into()),
        "wire" => Filler::Wire(n.field("wire")?.index()?),
        "other" => Filler::Other(n.field("label")?.string()?.into()),
        _ => return Err(n.field("kind")?.error("unknown filler kind")),
    })
}
fn slot_value(s: &Slot) -> Value {
    match s {
        Slot::Free => json!({"kind":"free"}),
        Slot::Fixed { value, fact } => json!({"kind":"fixed","value":value,"fact":fact}),
    }
}
fn slot(n: Node<'_>) -> Result<Slot> {
    Ok(match n.field("kind")?.string()? {
        "free" => Slot::Free,
        "fixed" => Slot::Fixed { value: n.field("value")?.number()?, fact: n.field("fact")?.index()? },
        _ => return Err(n.error("unknown coordinate slot kind")),
    })
}
fn position_value(p: &Position) -> Value {
    match p {
        Position::Own { x, y } => json!({"kind":"own","x":slot_value(x),"y":slot_value(y)}),
        Position::At(point) => json!({"kind":"at","point":point_value(point)}),
        Position::InSymbol { owner, local, replaced } => {
            json!({"kind":"in-symbol","owner":owner,"local":[local.0,local.1],"replaced":replaced.as_ref().map(|(axis,slot)|json!({"axis":axis.as_str(),"slot":slot_value(slot)}))})
        }
    }
}
fn position(n: Node<'_>) -> Result<Position> {
    Ok(match n.field("kind")?.string()? {
        "own" => Position::Own { x: slot(n.field("x")?)?, y: slot(n.field("y")?)? },
        "at" => Position::At(point(n.field("point")?, 0)?),
        "in-symbol" => Position::InSymbol {
            owner: n.field("owner")?.index()?,
            local: n.field("local")?.point()?,
            replaced: n
                .field("replaced")?
                .nullable(|n| Ok((fact::axis(n.field("axis")?)?, slot(n.field("slot")?)?)))?,
        },
        _ => return Err(n.error("unknown position kind")),
    })
}
fn text_value(t: &TextInfo) -> Value {
    let mut result = json!({"attach":filler_value(&t.attach),"key":t.key,"lines":t.lines.iter().map(|l|l.iter().map(data::run_value).collect::<Vec<_>>()).collect::<Vec<_>>(),"halign":t.halign,"valign":t.valign,"rotate":t.rotate,"fix_rotate":t.fix_rotate,"baseline_mirrors":t.baseline_mirrors,"column_position":t.column_position.map(|p|[p.0,p.1]),"style":data::pairs_value(&t.style)});
    if !t.upright {
        result["upright"] = json!(false);
    }
    result
}
fn text(n: Node<'_>) -> Result<TextInfo> {
    Ok(TextInfo {
        attach: filler(n.field("attach")?)?,
        key: data::optional_string(n.field("key")?)?,
        lines: n.field("lines")?.list(|n| n.list(|n| data::run(n, 0)))?,
        halign: n.field("halign")?.string()?.into(),
        valign: n.field("valign")?.string()?.into(),
        rotate: n.field("rotate")?.number()?,
        fix_rotate: n.field("fix_rotate")?.boolean()?,
        upright: match n.value.get("upright") {
            None => true,
            Some(_) => n.field("upright")?.boolean()?,
        },
        baseline_mirrors: n.field("baseline_mirrors")?.boolean()?,
        column_position: n.field("column_position")?.nullable(|n| n.point())?,
        style: data::pairs(n.field("style")?)?,
    })
}
pub(crate) fn object_value(o: &ProblemObject) -> Value {
    json!({"kind":o.kind.as_str(),"label":o.label,"reference":reference_value(&o.reference),"generated":o.generated,"symbol":o.symbol,"position":position_value(&o.position),"orient":o.orient.as_ref().map(|o|match o {OrientSlot::Free=>json!({"kind":"free"}),OrientSlot::Fixed {code,fact}=>json!({"kind":"fixed","code":code.as_str(),"fact":fact})}),"usage":o.usage,"text":o.text.as_ref().map(text_value),"nets":o.nets})
}
pub(crate) fn object(n: Node<'_>) -> Result<ProblemObject> {
    let kind = match n.field("kind")?.string()? {
        "instance" => ObjectKind::Instance,
        "bundle" => ObjectKind::Bundle,
        "usage" => ObjectKind::Usage,
        "point" => ObjectKind::Point,
        "text" => ObjectKind::Text,
        _ => return Err(n.error("unknown object kind")),
    };
    Ok(ProblemObject {
        kind,
        label: n.field("label")?.string()?.into(),
        reference: reference(n.field("reference")?)?,
        generated: n.field("generated")?.boolean()?,
        symbol: n.field("symbol")?.nullable(|n| n.index())?,
        position: position(n.field("position")?)?,
        orient: n.field("orient")?.nullable(|n| {
            Ok(match n.field("kind")?.string()? {
                "free" => OrientSlot::Free,
                "fixed" => OrientSlot::Fixed { code: orientation(n.field("code")?)?, fact: n.field("fact")?.index()? },
                _ => return Err(n.error("unknown orientation slot kind")),
            })
        })?,
        usage: n.field("usage")?.nullable(|n| n.index())?,
        text: n.field("text")?.nullable(text)?,
        nets: data::indices(n.field("nets")?)?,
    })
}
pub(crate) fn symbol_value(s: &ProblemSymbol) -> Value {
    json!({"name":s.name,"bbox":s.bbox,"box_written":s.box_written,"default":s.default,"pins":s.pins.iter().map(|p|json!({"identity":p.identity.as_ref().map(|i|json!({"names":i.names,"bus":i.bus})),"pos":[p.pos.0,p.pos.1],"label":p.label,"label_name_formats":p.label_name_formats,"dir":p.dir,"terminals":p.terminals})).collect::<Vec<_>>()})
}
pub(crate) fn symbol(n: Node<'_>) -> Result<ProblemSymbol> {
    let bbox = n.field("bbox")?;
    let values = bbox.list(|n| n.number())?;
    let bbox = values.try_into().map_err(|_| bbox.error("a symbol box requires four coordinates"))?;
    Ok(ProblemSymbol {
        name: n.field("name")?.string()?.into(),
        bbox,
        box_written: n.field("box_written")?.boolean()?,
        default: n.field("default")?.boolean()?,
        pins: n.field("pins")?.list(|n| {
            Ok(PinView {
                identity: n.field("identity")?.nullable(|n| {
                    Ok(SymbolPinIdentity { names: data::strings(n.field("names")?)?, bus: n.field("bus")?.boolean()? })
                })?,
                pos: n.field("pos")?.point()?,
                label: data::optional_string(n.field("label")?)?,
                label_name_formats: n.field("label_name_formats")?.boolean()?,
                dir: n.field("dir")?.nullable(|n| n.u16())?,
                terminals: data::strings(n.field("terminals")?)?,
            })
        })?,
    })
}
pub(crate) fn wire_value(w: &ProblemWire) -> Value {
    json!({"label":w.label,"reference":reference_value(&w.reference),"nets":w.nets,"bus":w.bus,"anchors":w.anchors.iter().map(point_value).collect::<Vec<_>>(),"mode":w.mode,"default_auto":w.default_auto,"generated":w.generated})
}
pub(crate) fn wire(n: Node<'_>) -> Result<ProblemWire> {
    let mode = n.field("mode")?.string()?.to_string();
    let default_auto = n.field("default_auto")?.boolean()?;
    if default_auto && mode != "auto" {
        return Err(n.field("default_auto")?.error("default_auto requires auto mode"));
    }
    Ok(ProblemWire {
        label: n.field("label")?.string()?.into(),
        reference: reference(n.field("reference")?)?,
        nets: data::strings(n.field("nets")?)?,
        bus: n.field("bus")?.boolean()?,
        anchors: n.field("anchors")?.list(|n| point(n, 0))?,
        mode,
        default_auto,
        generated: n.field("generated")?.boolean()?,
    })
}
pub(crate) fn route_value(r: &RouteIntent) -> Value {
    json!({"tree":r.tree,"net":r.net,"mode":r.mode,"style":data::pairs_value(&r.style),"pins":r.pins.iter().map(point_value).collect::<Vec<_>>(),"wires":r.wires})
}
pub(crate) fn route(n: Node<'_>) -> Result<RouteIntent> {
    Ok(RouteIntent {
        tree: n.field("tree")?.boolean()?,
        net: n.field("net")?.string()?.into(),
        mode: n.field("mode")?.string()?.into(),
        style: data::pairs(n.field("style")?)?,
        pins: n.field("pins")?.list(|n| point(n, 0))?,
        wires: data::indices(n.field("wires")?)?,
    })
}
pub(crate) fn group_value(g: &ProblemGroup) -> Value {
    json!({"identity":g.identity,"first":g.first,"direct":g.direct,"spacing_x":g.spacing_x,"spacing_y":g.spacing_y,"label":g.label,"members":g.members})
}
pub(crate) fn group(n: Node<'_>) -> Result<ProblemGroup> {
    Ok(ProblemGroup {
        identity: n.field("identity")?.index()?,
        first: n.field("first")?.boolean()?,
        direct: n.field("direct")?.boolean()?,
        spacing_x: n.field("spacing_x")?.nullable(|n| n.number())?,
        spacing_y: n.field("spacing_y")?.nullable(|n| n.number())?,
        label: n.field("label")?.string()?.into(),
        members: data::indices(n.field("members")?)?,
    })
}
pub(crate) fn usage_value(u: &UsageEntry) -> Value {
    json!({"definition":u.definition,"label":u.label,"reference":reference_value(&u.reference),"generated":u.generated,"rule_made":u.rule_made,"roles":u.roles.iter().map(|r|json!({"role":r.role,"members":r.members.iter().map(filler_value).collect::<Vec<_>>(),"written":r.written})).collect::<Vec<_>>(),"params":data::pairs_value(&u.params),"priority":u.priority.map(|p|p.as_str()),"parent":u.parent,"object":u.object,"written_numbers":u.written_numbers,"source":u.source.as_ref().map(data::loc_value)})
}
pub(crate) fn usage(n: Node<'_>) -> Result<UsageEntry> {
    Ok(UsageEntry {
        definition: n.field("definition")?.string()?.into(),
        label: n.field("label")?.string()?.into(),
        reference: reference(n.field("reference")?)?,
        generated: n.field("generated")?.boolean()?,
        rule_made: n.field("rule_made")?.boolean()?,
        roles: n.field("roles")?.list(|n| {
            Ok(RoleFill {
                role: n.field("role")?.string()?.into(),
                members: n.field("members")?.list(filler)?,
                written: data::optional_string(n.field("written")?)?,
            })
        })?,
        params: data::pairs(n.field("params")?)?,
        priority: n.field("priority")?.nullable(data::priority)?,
        parent: n.field("parent")?.nullable(|n| n.index())?,
        object: n.field("object")?.nullable(|n| n.index())?,
        written_numbers: n.field("written_numbers")?.index()?,
        source: n.field("source")?.nullable(data::loc)?,
    })
}
pub(crate) fn net_value(n: &ProblemNet) -> Value {
    json!({"name":n.name,"reference":reference_value(&n.reference),"pins":n.pins.iter().map(point_value).collect::<Vec<_>>(),"wires":n.wires,"routes":n.routes,"port":n.port})
}
pub(crate) fn net(n: Node<'_>) -> Result<ProblemNet> {
    Ok(ProblemNet {
        name: n.field("name")?.string()?.into(),
        reference: reference(n.field("reference")?)?,
        pins: n.field("pins")?.list(|n| point(n, 0))?,
        wires: data::indices(n.field("wires")?)?,
        routes: data::indices(n.field("routes")?)?,
        port: n.field("port")?.nullable(|n| n.index())?,
    })
}
