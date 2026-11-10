//! Source constructs that need a semantic choice rather than a spelling change.
use rnl_runtime::syntax::{Member, ObjExpr, RefOrHole, SetExpr, Value};
use std::collections::BTreeSet;

pub(super) type Reason = (&'static str, &'static str);
fn body<'a>(member: &'a Member, keyword: &str) -> Option<&'a SetExpr> {
    member.segments.iter().find(|s| s.call.as_ref().is_some_and(|c| c.kw == keyword))?.args.iter().find_map(
        |a| match &a.obj {
            ObjExpr::Set(s) => Some(s.as_ref()),
            _ => None,
        },
    )
}
fn absolute(member: &Member) -> bool {
    member.segments.iter().any(|s| {
        if s.call.as_ref().is_some_and(|c| c.kw == "DEFINE-SYMBOL") {
            return false;
        }
        s.call.as_ref().is_some_and(|c| c.kw == "PLACE")
            || s.args.iter().any(|a| match &a.obj {
                ObjExpr::Point(p) => {
                    !matches!((&p.x, &p.y), (rnl_runtime::syntax::Coord::Free, rnl_runtime::syntax::Coord::Free))
                }
                ObjExpr::Set(set) => set.members.iter().any(|m| absolute(m)),
                _ => false,
            })
    })
}
pub(super) fn definition_coordinates(member: &Member) -> Option<Reason> {
    body(member,"DEFINE").filter(|s|s.members.iter().any(|m|absolute(m))).map(|_|(
        "definition-coordinates", "absolute coordinates in a definition become usage-relative; lift the original facts with their priorities or choose an explicit origin (9.4)"))
}
fn nested_items(set: &SetExpr) -> bool {
    let items = set.members.iter().flat_map(|m| &m.segments).flat_map(|s| &s.args).collect::<Vec<_>>();
    items.len() > 1 && items.iter().any(|a| matches!(a.obj, ObjExpr::Set(_)))
}
pub(super) fn collection_constraints(member: &Member) -> Option<Reason> {
    for s in &member.segments {
        let Some(call) = &s.call else { continue };
        let sets = s
            .args
            .iter()
            .filter_map(|a| match &a.obj {
                ObjExpr::Set(set) => Some(set),
                _ => None,
            })
            .collect::<Vec<_>>();
        if matches!(call.kw.as_str(), "ALIGN" | "ORDER" | "DISTRIBUTE") && sets.iter().any(|s| nested_items(s)) {
            return Some(("constraint-items","held sets used as centres become combinations of members; preserve the original centre explicitly (5.2, 14.1)"));
        }
        if call.kw == "DISTANCE" && !sets.is_empty() {
            return Some(("distance-sets","distance between set extents becomes pairwise point distances; the old range is not a single point (14.5)"));
        }
        if call.kw == "ORIENT-RELATION" && sets.len() == 2 {
            return Some(("orientation-sets","two collection slots no longer form one orientation fact; splitting changes conflict resolution (14.9)"));
        }
    }
    None
}
fn terminal_keys(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Arr(values) => {
            for v in values {
                terminal_keys(v, out)
            }
        }
        Value::Num { value, .. } => out.push(value.to_string()),
        _ => out.push(super::rewrite::value(value)),
    }
}
pub(super) fn symbol_bindings(member: &Member) -> Option<Reason> {
    let set = body(member, "DEFINE-SYMBOL")?;
    let mut terminals = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut pins = false;
    let mut included = false;
    let mut text_names = BTreeSet::new();
    for m in &set.members {
        for bind in &m.binds {
            if !names.insert((bind.prefix, bind.name.clone())) {
                return Some(("symbol-local-names","reused symbol-local names no longer replace earlier bindings; rename distinct results and their references (9.2.4, 9.2.7)"));
            }
        }
        for segment in &m.segments {
            if segment.call.is_none() && segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Ref(_))) {
                included = true;
            }
            let Some(call) = &segment.call else { continue };
            if call.kw == "TEXT" {
                text_names.extend(m.binds.iter().filter(|b| b.prefix == '%').map(|b| b.name.clone()));
            }
            if call.kw != "PIN" {
                continue;
            }
            pins = true;
            let Some(value) = call.params.as_ref().and_then(|p| p.get("terminal")) else { continue };
            let mut keys = Vec::new();
            terminal_keys(value, &mut keys);
            for key in keys {
                if !terminals.insert(key) {
                    return Some(("symbol-pin-overrides","later PIN bindings no longer remove displaced pins; remove displaced bindings while preserving any referenced input points (9.2.4)"));
                }
            }
        }
    }
    if pins && included {
        return Some(("symbol-pin-inheritance","included symbol pins now accumulate and their names share the local scope; check displaced bindings and name conflicts before inclusion (9.2.7)"));
    }
    for m in &set.members {
        let mut refs = Vec::new();
        rnl_runtime::syntax::walk_refs(m, &mut refs);
        if refs.iter().any(|r|matches!(r,RefOrHole::Ref(r) if r.prefix=='%' && r.name.as_ref().is_some_and(|n|text_names.contains(n)))) {
            return Some(("symbol-text-result","a fixed TEXT result now names its graphic instead of its input point; name and reuse the input point separately (9.2.5)"));
        }
    }
    None
}
pub(super) fn device_terminals(member: &Member) -> Option<Reason> {
    for call in member.segments.iter().filter_map(|s| s.call.as_ref()).filter(|c| c.kw == "DEVICE") {
        if let Some(Value::Arr(names)) = call.params.as_ref().and_then(|p| p.get("terminals")) {
            let mut seen = BTreeSet::new();
            for value in names {
                if let Value::Str(name) = value {
                    if (!name.is_empty() && name.chars().all(|c| c.is_ascii_digit())) || !seen.insert(name) {
                        return Some(("device-terminal-names","numeric or repeated DEVICE names are invalid in 0.4; choose distinct nonnumeric names and update references (6.12)"));
                    }
                }
            }
        }
    }
    None
}
pub(super) fn selected_creation(member: &Member) -> Option<Reason> {
    fn selection(m: &Member) -> bool {
        m.segments.iter().any(|s| {
            s.call.as_ref().is_some_and(|c| c.kw.starts_with("SELECT-"))
                || s.args.iter().any(|a| matches!(&a.obj,ObjExpr::Set(set) if set.members.iter().any(|m|selection(m))))
        })
    }
    let selection = selection(member);
    if selection
        && member.segments.iter().filter_map(|s| s.call.as_ref()).any(|c| matches!(c.kw.as_str(), "WIRE" | "BUNDLE"))
    {
        return Some(("selection-dependent-creation","WIRE and BUNDLE cannot depend on a selection in 0.4; choose explicit members or a routing rule with equivalent topology (10.4, 11.8, 13.1)"));
    }
    None
}
pub(super) fn text_meaning(member: &Member) -> Option<Reason> {
    for call in
        member.segments.iter().filter_map(|s| s.call.as_ref()).filter(|c| matches!(c.kw.as_str(), "TEXT" | "PIN"))
    {
        if let Some(params) = &call.params {
            for value in
                params.positional.iter().chain(params.named.iter().filter(|n| n.name == "label").map(|n| &n.value))
            {
                if matches!(value,Value::Str(s) if [r"\subrest",r"\subdigits",r"\("].iter().any(|marker|s.contains(marker)))
                {
                    return Some(("text-meaning","literal text gains name-format or math meaning in 0.4; retain its intended rendered text explicitly (13.6)"));
                }
            }
        }
    }
    None
}
pub(super) fn reasons(member: &Member) -> Vec<Reason> {
    [
        definition_coordinates(member),
        collection_constraints(member),
        symbol_bindings(member),
        device_terminals(member),
        selected_creation(member),
        text_meaning(member),
    ]
    .into_iter()
    .flatten()
    .collect()
}
