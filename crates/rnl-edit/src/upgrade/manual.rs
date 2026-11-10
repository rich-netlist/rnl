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
pub(super) fn definition_references(member: &Member) -> Option<Reason> {
    let set = body(member, "DEFINE")?;
    let mut refs = Vec::new();
    for statement in &set.members {
        rnl_runtime::syntax::walk_refs(statement, &mut refs);
    }
    refs.iter().any(|r| matches!(r, RefOrHole::Ref(r) if r.prefix == '%' && !r.path.is_empty())).then_some((
        "definition-references", "definition-local object paths require each usage's expansion and symbol binding; preserve the original references until their role and pin identities can be verified (9.4, 12.1)"))
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
            if matches!(value, Value::Str(name) if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit())) {
                return Some(("numeric-pin-name", "a digits-only PIN terminal name becomes a positional terminal in 0.4; resolve its old name separately for each symbol assignment before choosing a terminal map (6.4, 9.2.4, 11.3)"));
            }
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

pub(super) fn package_context(member: &Member) -> Option<Reason> {
    for segment in &member.segments {
        let Some(call) = &segment.call else { continue };
        if call.kw == "USE-ALIASES" || call.kw == "ALIAS" {
            return Some(("alias-context", "alias-expanded statements require migration in their original alias environment; verify the expanded calls and standard names before accepting the new series (16.5)"));
        }
        if call.kw == "USE-PACKAGE" && segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Set(_))) {
            if let Some(Value::Str(name)) = call.params.as_ref().and_then(|p| p.get("name")) {
                if super::legacy::merged_package(name).is_some() {
                    return Some(("legacy-package-override", "this inline override targets a package that was merged; choose which definitions or rules to suppress in its replacement package (16.2, 16.3)"));
                }
            }
        }
    }
    None
}
pub(super) fn terminal_traversal(member: &Member) -> Option<Reason> {
    for (index, segment) in member.segments.iter().enumerate() {
        let Some(call) = &segment.call else { continue };
        let previous = index.checked_sub(1).and_then(|i| member.segments[i].call.as_ref()).map(|c| c.kw.as_str());
        let affected = match call.kw.as_str() {
            "SELECT-INSTANCE" => {
                segment.args.iter().any(|a| matches!(&a.obj,ObjExpr::Ref(r) if r.prefix=='&'))
                    || matches!(previous, Some("SELECT-NET" | "SELECT-TERMINAL"))
            }
            "SELECT-NET" => {
                segment.args.iter().any(|a| matches!(&a.obj,ObjExpr::Ref(r) if r.prefix=='#'&&r.name.is_some()))
                    || matches!(previous, Some("SELECT-INSTANCE" | "SELECT-TERMINAL"))
            }
            _ => false,
        };
        if affected {
            return Some(("terminal-traversal", "this selection traverses electrical terminals that previously needed pins; check membership before retaining the rule in 0.4 (10.1, 10.2)"));
        }
    }
    None
}
pub(super) fn reasons(member: &Member) -> Vec<Reason> {
    [
        definition_coordinates(member),
        definition_references(member),
        collection_constraints(member),
        symbol_bindings(member),
        device_terminals(member),
        selected_creation(member),
        text_meaning(member),
        package_context(member),
        terminal_traversal(member),
    ]
    .into_iter()
    .flatten()
    .collect()
}
