//! Role spellings derived from the two bundled package series.
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Member, ObjExpr, Value};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

type Roles = BTreeMap<String, (String, Vec<String>)>;
pub(super) type Changes = BTreeMap<String, String>;

fn roles(version: &str) -> Roles {
    let mut out = Roles::new();
    for (path, text) in &rnl_view::bundled().files {
        if !path.starts_with(&format!("<bundled>/{version}/")) {
            continue;
        }
        let Some((package, _)) = scan_package_decl(text) else { continue };
        for record in rnl_runtime::records::scan_run(
            &rnl_runtime::records::package_lines(text),
            path,
            &mut rnl_runtime::diag::Sink::default(),
        ) {
            let Some(body) = parse_record_features(&record.text, true, false).body else { continue };
            for member in &body.members {
                let Some(binding) = member.binds.iter().find(|b| b.prefix == '$') else { continue };
                for call in member.segments.iter().filter_map(|s| s.call.as_ref()).filter(|c| c.kw == "DEFINE") {
                    let Some(Value::Arr(values)) = call.params.as_ref().and_then(|p| p.get("roles")) else { continue };
                    let names = values
                        .iter()
                        .filter_map(|v| match v {
                            Value::Str(s) => Some(s),
                            Value::Arr(a) => match a.first() {
                                Some(Value::Str(s)) => Some(s),
                                _ => None,
                            },
                            _ => None,
                        })
                        .map(|s| s.trim_matches(['{', '}']).to_string())
                        .collect::<Vec<_>>();
                    if names.len() == values.len() {
                        out.insert(format!("${}", binding.name), (package.clone(), names));
                    }
                }
            }
        }
    }
    out
}

fn bound(member: &Member, names: &mut BTreeSet<String>) -> bool {
    let mut aliases = member
        .segments
        .iter()
        .filter_map(|s| s.call.as_ref())
        .any(|c| matches!(c.kw.as_str(), "ALIAS" | "USE-ALIASES"));
    names.extend(member.binds.iter().filter(|b| b.prefix == '$').map(|b| format!("${}", b.name)));
    for segment in &member.segments {
        for arg in &segment.args {
            if let ObjExpr::Set(set) = &arg.obj {
                for member in &set.members {
                    aliases |= bound(member, names);
                }
            }
        }
    }
    aliases
}

pub(super) fn shadows(input: &Interpretation) -> (BTreeSet<String>, bool) {
    let bundled = rnl_view::bundled();
    let mut shadowed = BTreeSet::new();
    let mut aliases = !input.imported_aliases.is_empty();
    for (path, text) in &input.sources {
        if bundled.files.values().any(|known| known == text) {
            continue;
        }
        for record in rnl_runtime::records::scan_run(
            &rnl_runtime::records::package_lines(text),
            path,
            &mut rnl_runtime::diag::Sink::default(),
        ) {
            if let Some(body) = parse_record_features(&record.text, true, false).body {
                for member in &body.members {
                    aliases |= bound(member, &mut shadowed);
                }
            }
        }
    }
    (shadowed, aliases)
}

pub(super) fn role_changes(input: &Interpretation) -> BTreeMap<(usize, String), Changes> {
    let bundled = rnl_view::bundled();
    let (shadowed, aliases) = shadows(input);
    // Aliases can change what a familiar spelling resolves to. Such sources need
    // alias-aware migration before applying name-based role changes.
    if aliases {
        return BTreeMap::new();
    }
    let target = roles("0.4");
    let mut out = BTreeMap::new();
    for version in ["0.2", "0.3"] {
        for (definition, (package, before)) in roles(version) {
            if shadowed.contains(&definition) {
                continue;
            }
            let Some((_, after)) = target.get(&definition) else { continue };
            if before.len() != after.len() {
                continue;
            }
            let mut changes = Changes::new();
            let mut known = true;
            for (old, new) in before.iter().zip(after) {
                if old == new {
                    continue;
                }
                if old.replace('_', "") == *new
                    || matches!((old.as_str(), new.as_str()), ("a", "left") | ("b", "right") | ("p", "at"))
                {
                    changes.insert(old.clone(), new.clone());
                } else {
                    known = false;
                }
            }
            if !known || changes.is_empty() {
                continue;
            }
            let sources: Vec<_> =
                input.sources.iter().filter(|(_, s)| scan_package_decl(s).is_some_and(|(n, _)| n == package)).collect();
            if sources.is_empty() || sources.iter().any(|(_, s)| !bundled.files.values().any(|b| b == s)) {
                continue;
            }
            for level in &input.model.levels {
                if level.packages.iter().any(|p| p.name == package && p.version == version) {
                    out.insert((level.level, definition.clone()), changes.clone());
                }
            }
        }
    }
    out
}

type DefinitionShapes = BTreeMap<String, (String, Vec<String>, usize, String)>;

fn definition_shapes(version: &str) -> DefinitionShapes {
    let mut out = BTreeMap::new();
    for (path, text) in &rnl_view::bundled().files {
        if !path.starts_with(&format!("<bundled>/{version}/")) {
            continue;
        }
        let Some((package, _)) = scan_package_decl(text) else { continue };
        for record in rnl_runtime::records::scan_run(
            &rnl_runtime::records::package_lines(text),
            path,
            &mut rnl_runtime::diag::Sink::default(),
        ) {
            let Some(body) = parse_record_features(&record.text, true, false).body else { continue };
            for member in &body.members {
                let Some(binding) = member.binds.iter().find(|b| b.prefix == '$') else { continue };
                for segment in &member.segments {
                    let Some(call) = segment.call.as_ref().filter(|c| c.kw == "DEFINE") else { continue };
                    let Some(params) = &call.params else { continue };
                    let roles = match params.get("roles") {
                        Some(Value::Arr(a)) => a.len(),
                        _ => 0,
                    };
                    let defaults = params.get("parameters").map(super::rewrite::value).unwrap_or_default();
                    if let Some(set) = segment.args.iter().find_map(|a| match &a.obj {
                        ObjExpr::Set(s) => Some(s),
                        _ => None,
                    }) {
                        let tokens = rnl_runtime::syntax::statement_tokens(
                            &record.text.chars().collect::<Vec<_>>(),
                            set.start,
                            set.end,
                        );
                        out.insert(format!("${}", binding.name), (package.clone(), tokens, roles, defaults.clone()));
                    }
                }
            }
        }
    }
    out
}

/// Changed package bodies are decisions about layout/routing/assertions, not
/// ordinary renames. Report them on the actual source applications.
pub(super) fn definition_reviews(input: &Interpretation) -> BTreeMap<(usize, String), super::manual::Reason> {
    let (shadowed, aliases) = shadows(input);
    if aliases {
        return BTreeMap::new();
    }
    let bundled = rnl_view::bundled();
    let target = definition_shapes("0.4");
    let mut out = BTreeMap::new();
    for version in ["0.2", "0.3"] {
        for (name, (package, before, roles, defaults)) in definition_shapes(version) {
            if shadowed.contains(&name) {
                continue;
            }
            let sources = input
                .sources
                .iter()
                .filter(|(_, s)| scan_package_decl(s).is_some_and(|(n, _)| n == package))
                .collect::<Vec<_>>();
            if sources.is_empty() || sources.iter().any(|(_, s)| !bundled.files.values().any(|b| b == s)) {
                continue;
            }
            let reason=match target.get(&name) {
                None=>Some(("standard-definition-removed","the standard definition was removed or split; its replacement needs an explicit semantic choice (16.3)")),
                Some((_,_,after_roles,_)) if *after_roles!=roles=>Some(("standard-definition-roles","the target standard definition has a different role list; preserving the original members requires an explicit choice (9.4, 16.3)")),
                Some((_,after,_,after_defaults)) if before!=*after || defaults!=*after_defaults=>Some(("standard-definition-body","the target standard definition changes body facts, assertions or parameter defaults; review those changes before replacing the original application (9.4, 16.3)")),
                _=>None,
            };
            let Some(reason) = reason else { continue };
            for level in &input.model.levels {
                if level.packages.iter().any(|p| p.name == package && p.version == version) {
                    out.insert((level.level, name.clone()), reason);
                }
            }
        }
    }
    out
}
