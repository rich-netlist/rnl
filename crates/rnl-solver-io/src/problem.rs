use crate::{
    data, fact,
    json::{Error, Node, Result},
    model,
    point::{orientation, point, point_value},
};
use rnl_runtime::problem::*;
use serde_json::{json, Value};

/// Encode the complete problem, including markup, literal source units, exact pin references,
/// nested TABLE structure and writeback metadata. No display-text projection is used.
pub fn encode_problem(p: &LevelProblem) -> Result<Value> {
    crate::validate::problem(p)?;
    let value = json!({
        "junction":data::junction_value(&p.junction),"solver_config":p.solver_config.iter().map(data::setting_value).collect::<Vec<_>>(),
        "level":p.level,"title":p.title,"kind":p.kind,"name":p.name,"version":p.version,"authored_numbers":p.authored_numbers,
        "file":p.file,"line":p.line,"end_line":p.end_line,"packages":p.packages,"names":p.names,
        "objects":p.objects.iter().map(model::object_value).collect::<Vec<_>>(),"symbols":p.symbols.iter().map(model::symbol_value).collect::<Vec<_>>(),
        "facts":p.facts.iter().map(fact::value).collect::<Vec<_>>(),"wires":p.wires.iter().map(model::wire_value).collect::<Vec<_>>(),
        "routes":p.routes.iter().map(model::route_value).collect::<Vec<_>>(),"currents":p.currents.iter().map(|c|json!({"pin":point_value(&c.pin),"direction":c.direction})).collect::<Vec<_>>(),
        "groups":p.groups.iter().map(model::group_value).collect::<Vec<_>>(),"usages":p.usages.iter().map(model::usage_value).collect::<Vec<_>>(),
        "nets":p.nets.iter().map(model::net_value).collect::<Vec<_>>(),"orient_lists":p.orient_lists.iter().map(|(fact,codes)|json!({"fact":fact,"codes":codes.iter().map(|c|c.as_str()).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "ids":{"objects":p.ids.objects,"usages":p.ids.usages},
    });
    // In particular, nullable numbers must not silently lose a NaN or infinity as JSON null.
    if decode_problem(&value)? != *p {
        return Err(Error {
            path: "$".into(),
            message: "problem cannot be represented losslessly (check nonfinite numbers)".into(),
        });
    }
    Ok(value)
}

/// Decode a complete problem and check its indices and coordinate/usage dependencies before
/// calling solver routines that assume the runtime has already established these invariants.
pub fn decode_problem(value: &Value) -> Result<LevelProblem> {
    let n = Node::root(value);
    let kind = match n.field("kind")?.string()? {
        "file" => "file",
        "subckt" => "subckt",
        _ => return Err(n.field("kind")?.error("unknown drawing level kind")),
    };
    let ids = n.field("ids")?;
    let p = LevelProblem {
        junction: data::junction(n.field("junction")?, 0)?,
        solver_config: n.field("solver_config")?.list(data::setting)?,
        level: n.field("level")?.index()?,
        title: n.field("title")?.string()?.into(),
        kind,
        name: n.field("name")?.string()?.into(),
        version: n.field("version")?.string()?.into(),
        authored_numbers: n.field("authored_numbers")?.index()?,
        file: n.field("file")?.string()?.into(),
        line: n.field("line")?.u32()?,
        end_line: n.field("end_line")?.u32()?,
        packages: n.field("packages")?.list(|n| {
            let items = data::strings(n.clone())?;
            match items.as_slice() {
                [name, version] => Ok((name.clone(), version.clone())),
                _ => Err(n.error("a package requires name and version")),
            }
        })?,
        names: data::strings(n.field("names")?)?,
        objects: n.field("objects")?.list(model::object)?,
        symbols: n.field("symbols")?.list(model::symbol)?,
        facts: n.field("facts")?.list(fact::decode)?,
        wires: n.field("wires")?.list(model::wire)?,
        routes: n.field("routes")?.list(model::route)?,
        currents: n.field("currents")?.list(|n| {
            Ok(TerminalCurrent { pin: point(n.field("pin")?, 0)?, direction: n.field("direction")?.string()?.into() })
        })?,
        groups: n.field("groups")?.list(model::group)?,
        usages: n.field("usages")?.list(model::usage)?,
        nets: n.field("nets")?.list(model::net)?,
        orient_lists: n
            .field("orient_lists")?
            .list(|n| Ok((n.field("fact")?.index()?, n.field("codes")?.list(orientation)?)))?,
        ids: ProblemIds {
            objects: data::strings(ids.field("objects")?)?,
            usages: data::strings(ids.field("usages")?)?,
        },
    };
    crate::validate::problem(&p)?;
    Ok(p)
}
