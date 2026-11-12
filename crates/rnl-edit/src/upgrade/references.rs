//! Resolve standard usage terminals to named pins through the source interpretation.
use std::collections::{BTreeMap, BTreeSet};

use rnl_runtime::host::Record;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::model::SymbolOrigin;
use rnl_runtime::source::normalize;
use rnl_runtime::syntax::{parse_record_features, ObjExpr, Ref, Segment, Sep, Value};
use rnl_runtime::Interpretation;

use super::edit::{record_edit, Edit};

type SymbolKey = (String, String);
type Pins = Vec<(String, String)>;

#[derive(Default)]
pub(super) struct References {
    aliases: bool,
    shadowed: BTreeSet<String>,
    scopes: BTreeMap<(String, u32, u32), Vec<std::ops::Range<usize>>>,
    collections: super::collections::Collections,
    readings: BTreeMap<(String, u32, u32), Vec<usize>>,
    pins: BTreeMap<(usize, String, String), String>,
    usages: BTreeSet<(usize, String)>,
    netname_key_objects: BTreeSet<(usize, String)>,
    transconductor_objects: BTreeSet<(usize, String)>,
    opamp02_levels: BTreeSet<usize>,
    opamp02_objects: BTreeSet<(usize, String)>,
    opamp02_oriented: BTreeSet<(usize, String)>,
    legacy: super::legacy::Legacy,
    host_terminals: super::host_terminals::HostTerminals,
    roles: BTreeMap<(usize, String, String), String>,
    definitions: BTreeMap<(usize, String), super::standard::Changes>,
    terminals: super::terminals::Terminals,
    pub local_symbols: super::local_symbols::LocalSymbols,
    usage_changes: super::usages::Usages,
    approximate: super::approximate::Approximate,
    usage_reviews: BTreeMap<(usize, String, u32, u32), Vec<super::manual::Reason>>,
}

fn target_pins() -> BTreeMap<SymbolKey, Pins> {
    let mut direct = BTreeMap::new();
    let mut inherited: BTreeMap<SymbolKey, Vec<String>> = BTreeMap::new();
    for (path, source) in &rnl_view::bundled().files {
        if !path.starts_with("<bundled>/0.4/") {
            continue;
        }
        let Some((package, _)) = scan_package_decl(source) else { continue };
        let records = rnl_runtime::records::scan_run(
            &rnl_runtime::records::package_lines(source),
            path,
            &mut rnl_runtime::diag::Sink::default(),
        );
        for record in records {
            let ast = parse_record_features(&record.text, true, true);
            let Some(body) = ast.body else { continue };
            for member in &body.members {
                let Some(binding) = member.binds.iter().find(|b| b.prefix == '$' || b.prefix == '%') else {
                    continue;
                };
                for segment in &member.segments {
                    if segment.call.as_ref().is_none_or(|c| c.kw != "DEFINE-SYMBOL") {
                        continue;
                    }
                    let key = (package.clone(), format!("{}{}", binding.prefix, binding.name));
                    let pins = direct.entry(key.clone()).or_insert_with(Vec::new);
                    for arg in &segment.args {
                        let ObjExpr::Set(body) = &arg.obj else { continue };
                        for statement in &body.members {
                            for segment in &statement.segments {
                                if let Some(call) = &segment.call {
                                    if call.kw == "PIN" {
                                        let terminal = call.params.as_ref().and_then(|p| p.get("terminal"));
                                        let name = statement.binds.iter().find(|b| b.prefix == '%');
                                        if let (Some(Value::Str(terminal)), Some(name)) = (terminal, name) {
                                            pins.push((terminal.clone(), name.name.clone()));
                                        }
                                    }
                                } else {
                                    for arg in &segment.args {
                                        if let ObjExpr::Ref(r) = &arg.obj {
                                            if (r.prefix == '$' || r.prefix == '%') && r.path.is_empty() {
                                                if let Some(name) = &r.name {
                                                    inherited
                                                        .entry(key.clone())
                                                        .or_default()
                                                        .push(format!("{}{name}", r.prefix));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // The packaged symbol graph is finite. Iterate to propagate inherited named pins.
    for _ in 0..direct.len() {
        let mut changed = false;
        for (key, parents) in &inherited {
            for parent in parents {
                let inherited = direct.get(&(key.0.clone(), parent.clone())).cloned().unwrap_or_default();
                for pin in inherited {
                    let pins = direct.entry(key.clone()).or_default();
                    if !pins.contains(&pin) {
                        pins.push(pin);
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    direct
}

impl References {
    pub(super) fn new(input: &Interpretation, approximate: bool) -> Self {
        let (shadowed, aliases) = super::standard::shadows(input);
        let mut result = Self {
            aliases,
            shadowed,
            definitions: super::standard::role_changes(input),
            terminals: super::terminals::Terminals::new(input),
            local_symbols: super::local_symbols::LocalSymbols::new(input),
            legacy: super::legacy::Legacy::new(input),
            host_terminals: super::host_terminals::HostTerminals::new(input),
            ..Self::default()
        };
        if approximate {
            result.definitions.retain(|(_, name), _| name != "$MIRRORED");
        }
        let policies = super::standard::definition_policies(input);
        result.usage_changes = super::usages::Usages::new(input, &policies);
        result.approximate = super::approximate::Approximate::new(input, &policies);
        for problem in &input.problems {
            for usage in &problem.usages {
                let Some(loc) = &usage.source else { continue };
                let reasons =
                    result.usage_reviews.entry((problem.level, normalize(&loc.file), loc.line, loc.col)).or_default();
                let policy = policies.get(&(problem.level, usage.definition.clone()));
                if let Some(reason) = policy.and_then(|p| p.review) {
                    reasons.push(reason);
                }
                if usage
                    .object
                    .and_then(|i| problem.objects.get(i))
                    .is_some_and(|o| matches!(o.position, rnl_runtime::problem::Position::At(_)))
                    && !policy.is_some_and(|p| p.tied)
                {
                    reasons.push(("usage-main-point", "this usage followed its main role's point; 0.4 gives it an independent point, so preserve that relation explicitly (12.1)"));
                }
            }
        }
        let targets = target_pins();
        let bundled = rnl_view::bundled();
        for (index, level) in input.doc.levels.iter().enumerate() {
            for record in &level.records {
                let mut drawing = index;
                while !input.doc.levels[drawing].circuit {
                    let Some(parent) = input.doc.levels[drawing].parent else { break };
                    drawing = parent;
                }
                let ast = parse_record_features(&record.text, true, false);
                fn scopes(member: &rnl_runtime::syntax::Member, out: &mut Vec<std::ops::Range<usize>>) {
                    for segment in &member.segments {
                        for arg in &segment.args {
                            if let ObjExpr::Set(body) = &arg.obj {
                                if segment
                                    .call
                                    .as_ref()
                                    .is_some_and(|c| matches!(c.kw.as_str(), "DEFINE" | "DEFINE-SYMBOL"))
                                {
                                    out.push(body.start..body.end);
                                } else {
                                    for member in &body.members {
                                        scopes(member, out);
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(body) = &ast.body {
                    let ranges =
                        result.scopes.entry((normalize(&level.path), record.lines.0, record.lines.1)).or_default();
                    for member in &body.members {
                        scopes(member, ranges);
                    }
                }
                let scope = ast.header.as_ref().and_then(|p| p.get("scope"));
                let readings =
                    result.readings.entry((normalize(&level.path), record.lines.0, record.lines.1)).or_default();
                if let Some(Value::Str(scope)) = scope {
                    readings.extend(
                        input
                            .doc
                            .levels
                            .iter()
                            .enumerate()
                            .filter(|(_, l)| l.circuit && l.same_name(&l.name, scope))
                            .map(|(i, _)| i),
                    );
                } else {
                    readings.push(drawing);
                }
            }
        }
        result.collections = super::collections::Collections::new(input, &result.readings);
        if !result.aliases && !result.shadowed.contains("$OPAMP") {
            let sym02_sources: Vec<_> = input
                .sources
                .iter()
                .filter(|(_, text)| scan_package_decl(text).is_some_and(|(n, v)| n == "rnl-std-symbols" && v == "0.2"))
                .collect();
            let known_sym02 = !sym02_sources.is_empty()
                && sym02_sources.iter().all(|(_, text)| bundled.files.values().any(|orig| orig == text));
            if known_sym02 {
                for level in &input.model.levels {
                    if level.packages.iter().any(|p| p.name == "rnl-std-symbols" && p.version == "0.2") {
                        result.opamp02_levels.insert(level.level);
                        for object in &level.objects {
                            if object.symbol.as_ref().is_some_and(|s| {
                                s.name == "$OPAMP"
                                    && matches!(&s.origin, SymbolOrigin::Package(p) if p == "rnl-std-symbols")
                            }) {
                                result.opamp02_objects.insert((level.level, object.id.clone()));
                            }
                        }
                    }
                }
                for problem in &input.problems {
                    if !result.opamp02_levels.contains(&problem.level) {
                        continue;
                    }
                    for object in &problem.objects {
                        if matches!(object.orient, Some(rnl_runtime::problem::OrientSlot::Fixed { .. })) {
                            if let rnl_runtime::problem::Reference::Name(id) = &object.reference {
                                if result.opamp02_objects.contains(&(problem.level, id.clone())) {
                                    result.opamp02_oriented.insert((problem.level, id.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
        for level in &input.model.levels {
            for object in &level.objects {
                if let Some(symbol) = &object.symbol {
                    if let SymbolOrigin::Package(package) = &symbol.origin {
                        let sources: Vec<_> = input
                            .sources
                            .iter()
                            .filter(|(_, text)| scan_package_decl(text).is_some_and(|(name, _)| &name == package))
                            .collect();
                        let known = !sources.is_empty()
                            && sources.iter().all(|(_, text)| bundled.files.values().any(|original| original == text));
                        if known {
                            if matches!(
                                symbol.name.as_str(),
                                "$GROUND_SYMBOL"
                                    | "$NAMED_GROUND_SYMBOL"
                                    | "$GROUND"
                                    | "$SUPPLY_SYMBOL"
                                    | "$SUPPLY_BAR"
                                    | "$IO_PORT_SYMBOL"
                                    | "$PORT_TAG"
                            ) {
                                result.netname_key_objects.insert((level.level, object.id.clone()));
                            }
                            if symbol.name == "$TRANSCONDUCTOR" {
                                result.transconductor_objects.insert((level.level, object.id.clone()));
                            }
                        }
                    }
                }
                if object.kind != "usage" {
                    continue;
                }
                result.usages.insert((level.level, object.id.clone()));
                if let Some(def) = &object.definition {
                    if !result.shadowed.contains(def)
                        && matches!(def.as_str(), "$GROUND" | "$GND" | "$SUPPLY" | "$IO_PORT" | "$IOPORT")
                    {
                        result.netname_key_objects.insert((level.level, object.id.clone()));
                    }
                }
                if let Some(changes) =
                    object.definition.as_ref().and_then(|d| result.definitions.get(&(level.level, d.clone())))
                {
                    for (old, new) in changes {
                        result.roles.insert((level.level, object.id.clone(), old.clone()), new.clone());
                    }
                }
                let Some(symbol) = &object.symbol else { continue };
                if let SymbolOrigin::Level(owner) = &symbol.origin {
                    for (number, (role, _)) in object.attach.iter().enumerate() {
                        if let Some(pin) = result.local_symbols.pins.get(&(*owner, symbol.name.clone(), role.clone())) {
                            result.pins.insert((level.level, object.id.clone(), role.clone()), pin.clone());
                            result.pins.insert((level.level, object.id.clone(), (number + 1).to_string()), pin.clone());
                        }
                    }
                    continue;
                }
                let SymbolOrigin::Package(package) = &symbol.origin else { continue };
                let raw_sym = symbol.name.trim_start_matches('$');
                let target_sym = super::legacy::renamed_03_symbol(raw_sym)
                    .or_else(|| super::legacy::renamed(raw_sym))
                    .map(|s| format!("${s}"))
                    .unwrap_or_else(|| symbol.name.clone());
                let target_pkg = super::legacy::merged_package(package).unwrap_or(package);
                let Some(pins) = targets
                    .get(&(package.clone(), target_sym.clone()))
                    .or_else(|| targets.get(&(target_pkg.to_string(), target_sym)))
                else {
                    continue;
                };
                // Resolve provenance, not just a familiar definition or symbol name. A custom
                // package or a local override must never inherit a standard-package rewrite.
                let sources: Vec<_> = input
                    .sources
                    .iter()
                    .filter(|(_, text)| scan_package_decl(text).is_some_and(|(name, _)| &name == package))
                    .collect();
                let known = !sources.is_empty()
                    && sources.iter().all(|(_, text)| bundled.files.values().any(|original| original == text));
                if !known {
                    continue;
                }
                for (number, (role, _)) in object.attach.iter().enumerate() {
                    let matching: Vec<_> = pins.iter().filter(|(terminal, _)| terminal == role).collect();
                    if let [(_, pin)] = matching.as_slice() {
                        result.pins.insert((level.level, object.id.clone(), role.clone()), pin.clone());
                        result.pins.insert((level.level, object.id.clone(), (number + 1).to_string()), pin.clone());
                    }
                }
            }
        }
        result
    }

    pub(super) fn readings_at(&self, path: &str, record: &Record, at: usize) -> Option<&Vec<usize>> {
        let key = (normalize(path), record.lines.0, record.lines.1);
        if self.scopes.get(&key).is_some_and(|scopes| scopes.iter().any(|s| s.contains(&at))) {
            return None;
        }
        self.readings.get(&key)
    }
    pub(super) fn usage_changes(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
        approximate: bool,
        report: &mut super::FileReport,
    ) -> Result<(Vec<Edit>, bool), String> {
        let Some(readings) = self.readings_at(path, record, member.start) else { return Ok((Vec::new(), false)) };
        self.usage_changes.edits((source, record, member), readings, approximate, report, |r| {
            self.host_terminal(path, source, record, r)
        })
    }
    pub(super) fn approximate(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
        enabled: bool,
        report: &mut super::FileReport,
    ) -> Result<super::approximate::Outcome, String> {
        // Definition-local choices use only their own syntax/witnesses, never
        // the scene objects that happen to have the same spelling.
        let readings = self.readings_at(path, record, member.start).map(Vec::as_slice).unwrap_or_default();
        self.approximate.edits(source, record, member, readings, enabled, report)
    }
    pub(super) fn approximate_role(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        r: &Ref,
    ) -> Result<Option<Edit>, String> {
        let Some(readings) = self.readings_at(path, record, r.start) else { return Ok(None) };
        self.approximate.role_reference(source, record, r, readings)
    }
    pub(super) fn collection_reviews(
        &self,
        path: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
    ) -> Vec<super::manual::Reason> {
        let Some(readings) = self.readings_at(path, record, member.start) else { return Vec::new() };
        self.collections.reasons(readings, member)
    }

    pub(super) fn usage_reviews(
        &self,
        path: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
    ) -> Vec<super::manual::Reason> {
        if !member.segments.iter().any(|s| s.call.as_ref().is_some_and(|c| matches!(c.kw.as_str(), "AS" | "ATTACH"))) {
            return Vec::new();
        }
        let start = record.loc(member.text_start());
        let end = record.loc(member.end);
        let Some(readings) = self.readings_at(path, record, member.start) else {
            return Vec::new();
        };
        let mut reasons = Vec::new();
        for ((level, file, line, col), found) in &self.usage_reviews {
            // Diagnostic locations use the adapter's display filename, which
            // need not be the provider path. The reading level disambiguates it.
            if readings.contains(level)
                && *file == normalize(&start.file)
                && (*line, *col) >= (start.line, start.col)
                && (*line, *col) <= (end.line, end.col)
            {
                for reason in found {
                    if !reasons.contains(reason) {
                        reasons.push(*reason);
                    }
                }
            }
        }
        reasons
    }

    pub(super) fn text_key(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
        index: usize,
    ) -> Result<Option<Edit>, String> {
        if self.aliases {
            return Ok(None);
        }
        let segment = &member.segments[index];
        let Some(call) = segment.call.as_ref().filter(|c| c.kw == "TEXT") else { return Ok(None) };
        let Some(params) = &call.params else { return Ok(None) };
        let Some(Value::Str(old_key)) = params.get("key") else { return Ok(None) };
        if !matches!(old_key.as_str(), "name" | "formula") {
            return Ok(None);
        }
        let Some(readings) = self.readings_at(path, record, segment.start).filter(|r| !r.is_empty()) else {
            return Ok(None);
        };
        fn matches_set(o: &ObjExpr, readings: &[usize], set: &BTreeSet<(usize, String)>) -> bool {
            match o {
                ObjExpr::Ref(r)
                    if (r.prefix == '#' || r.prefix == '%')
                        && r.path.is_empty()
                        && r.sub.is_none()
                        && r.places.is_none()
                        && r.number.is_none() =>
                {
                    let Some(name) = &r.name else { return false };
                    let id = format!("{}{}", r.prefix, rnl_runtime::problem::written_name(name));
                    readings.iter().all(|l| set.contains(&(*l, id.clone())))
                }
                ObjExpr::Set(s) => {
                    !s.members.is_empty()
                        && s.members.iter().all(|m| {
                            m.binds.is_empty()
                                && m.segments.len() == 1
                                && m.segments[0].call.is_none()
                                && !m.segments[0].piped
                                && !m.segments[0].args.is_empty()
                                && m.segments[0].args.iter().all(|a| matches_set(&a.obj, readings, set))
                        })
                }
                _ => false,
            }
        }
        let matched = if segment.piped && segment.args.is_empty() {
            let mut ok = false;
            for prev in member.segments[..index].iter().rev() {
                let Some(pcall) = &prev.call else { continue };
                match pcall.kw.as_str() {
                    "SELECT-NET" | "SELECT-PIN" | "SELECT-TERMINAL" | "TEXT" | "WIRE" | "BUNDLE" => break,
                    "ATTACH" | "AS" | "SELECT-USAGE" | "SELECT-ANCESTOR-USAGE" if old_key == "name" => {
                        ok = prev.args.iter().any(|a| match &a.obj {
                            ObjExpr::Ref(r) if r.prefix == '$' && r.path.is_empty() => {
                                r.name.as_deref().is_some_and(|n| {
                                    let d = format!("${n}");
                                    !self.shadowed.contains(&d)
                                        && matches!(n, "GROUND" | "GND" | "SUPPLY" | "IO_PORT" | "IOPORT")
                                })
                            }
                            _ => false,
                        });
                        break;
                    }
                    "SET-SYMBOL" => {
                        ok = prev.args.iter().any(|a| match &a.obj {
                            ObjExpr::Ref(r) if r.prefix == '$' && r.path.is_empty() => {
                                r.name.as_deref().is_some_and(|n| {
                                    let s = format!("${n}");
                                    !self.shadowed.contains(&s)
                                        && if old_key == "name" {
                                            matches!(
                                                n,
                                                "NAMED_GROUND_SYMBOL"
                                                    | "GROUND_SYMBOL"
                                                    | "SUPPLY_SYMBOL"
                                                    | "IO_PORT_SYMBOL"
                                            )
                                        } else {
                                            n == "TRANSCONDUCTOR"
                                        }
                                })
                            }
                            _ => false,
                        });
                        break;
                    }
                    _ => {}
                }
            }
            if !ok && !member.segments[0].args.is_empty() {
                let target_set =
                    if old_key == "name" { &self.netname_key_objects } else { &self.transconductor_objects };
                let direct: Vec<_> = member.segments[0]
                    .args
                    .iter()
                    .filter(|a| !matches!(&a.obj, ObjExpr::Ref(r) if r.prefix == '$'))
                    .collect();
                if !direct.is_empty()
                    && !member.segments[..index].iter().any(|s| {
                        s.call.as_ref().is_some_and(|c| {
                            matches!(c.kw.as_str(), "SELECT-NET" | "SELECT-PIN" | "SELECT-TERMINAL" | "TEXT" | "WIRE")
                        })
                    })
                {
                    ok = direct.iter().all(|a| matches_set(&a.obj, readings, target_set));
                }
            }
            ok
        } else if !segment.piped && !segment.args.is_empty() {
            let target_set = if old_key == "name" { &self.netname_key_objects } else { &self.transconductor_objects };
            segment.args.iter().all(|a| matches_set(&a.obj, readings, target_set))
        } else {
            false
        };
        if !matched {
            return Ok(None);
        }
        let new_key = if old_key == "name" { "netname" } else { "label" };
        let mut parts = params.positional.iter().map(super::rewrite::value).collect::<Vec<_>>();
        for named in &params.named {
            let k = if named.quoted { rnl_runtime::problem::written_string(&named.name) } else { named.name.clone() };
            let v = if named.name == "key" {
                rnl_runtime::problem::written_string(new_key)
            } else {
                super::rewrite::value(&named.value)
            };
            parts.push(format!("{k}={v}"));
        }
        record_edit(source, record, call.start..call.end, format!("@TEXT({})", parts.join(", ")), "standard-text-key")
            .map(Some)
    }

    pub(super) fn terminal_map(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
        index: usize,
    ) -> Result<Option<Edit>, String> {
        let readings =
            self.readings_at(path, record, member.segments[index].start).map(Vec::as_slice).unwrap_or_default();
        self.terminals.edit(source, record, member, index, readings)
    }

    pub(super) fn opamp02_orient(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
    ) -> Result<Vec<Edit>, String> {
        if self.aliases || self.opamp02_levels.is_empty() {
            return Ok(Vec::new());
        }
        let Some(readings) = self.readings_at(path, record, member.start).filter(|r| !r.is_empty()) else {
            return Ok(Vec::new());
        };
        if !readings.iter().all(|l| self.opamp02_levels.contains(l)) {
            return Ok(Vec::new());
        }
        fn matches_set(o: &ObjExpr, readings: &[usize], set: &BTreeSet<(usize, String)>) -> bool {
            match o {
                ObjExpr::Ref(r)
                    if (r.prefix == '#' || r.prefix == '%')
                        && r.path.is_empty()
                        && r.sub.is_none()
                        && r.places.is_none()
                        && r.number.is_none() =>
                {
                    let Some(name) = &r.name else { return false };
                    let id = format!("{}{}", r.prefix, rnl_runtime::problem::written_name(name));
                    readings.iter().all(|l| set.contains(&(*l, id.clone())))
                }
                ObjExpr::Set(s) => {
                    !s.members.is_empty()
                        && s.members.iter().all(|m| {
                            m.binds.is_empty()
                                && m.segments.len() == 1
                                && m.segments[0].call.is_none()
                                && !m.segments[0].piped
                                && !m.segments[0].args.is_empty()
                                && m.segments[0].args.iter().all(|a| matches_set(&a.obj, readings, set))
                        })
                }
                _ => false,
            }
        }
        fn is_bare_level(o: &ObjExpr) -> bool {
            match o {
                ObjExpr::Ref(r) => r.prefix == '#' && r.name.is_none() && r.number.is_none() && r.path.is_empty(),
                ObjExpr::Set(s) => {
                    s.members.iter().flat_map(|m| &m.segments).flat_map(|seg| &seg.args).any(|a| is_bare_level(&a.obj))
                }
                _ => false,
            }
        }
        fn set_opamp_end(segment: &Segment) -> Option<usize> {
            let call = segment.call.as_ref().filter(|c| c.kw == "SET-SYMBOL")?;
            let syms: Vec<_> = segment
                .args
                .iter()
                .filter_map(|a| match &a.obj {
                    ObjExpr::Ref(r) if r.prefix == '$' && r.path.is_empty() && r.sub.is_none() => {
                        Some((r.name.as_deref()?, r.end))
                    }
                    _ => None,
                })
                .collect();
            match syms.as_slice() {
                [("OPAMP", end)] => Some((*end).max(call.end)),
                _ => None,
            }
        }
        fn parse_orient_code(s: &str) -> Option<rnl_runtime::geom::Orient> {
            match s {
                "left" => Some(rnl_runtime::geom::Orient::R0),
                "down" => Some(rnl_runtime::geom::Orient::R90),
                "right" => Some(rnl_runtime::geom::Orient::R180),
                "up" => Some(rnl_runtime::geom::Orient::R270),
                _ => rnl_runtime::geom::Orient::parse(s),
            }
        }
        fn flip_orient_value(v: &Value) -> Option<String> {
            match v {
                Value::Str(s) => {
                    let c = parse_orient_code(s)?;
                    Some(rnl_runtime::problem::written_string(rnl_runtime::geom::Orient::MX.then(c).as_str()))
                }
                Value::Arr(items) if !items.is_empty() => {
                    let flipped = items.iter().map(flip_orient_value).collect::<Option<Vec<_>>>()?;
                    Some(format!("[{}]", flipped.join(", ")))
                }
                _ => None,
            }
        }
        fn flip_orient_call(call: &rnl_runtime::syntax::Call) -> Option<String> {
            if call.kw != "ORIENT" {
                return None;
            }
            let params = call.params.as_ref()?;
            if params.positional.is_empty() && params.get("code").is_none() {
                return None;
            }
            let mut parts = Vec::new();
            if let Some((first, rest)) = params.positional.split_first() {
                parts.push(flip_orient_value(first)?);
                parts.extend(rest.iter().map(super::rewrite::value));
            }
            for named in &params.named {
                let k =
                    if named.quoted { rnl_runtime::problem::written_string(&named.name) } else { named.name.clone() };
                let v = if named.name == "code" {
                    flip_orient_value(&named.value)?
                } else {
                    super::rewrite::value(&named.value)
                };
                parts.push(format!("{k}={v}"));
            }
            Some(format!("@ORIENT({})", parts.join(", ")))
        }
        let mut edits = Vec::new();
        let mut pipeline_has_orient = false;
        for (idx, seg) in member.segments.iter().enumerate() {
            let Some(call) = seg.call.as_ref().filter(|c| c.kw == "ORIENT") else { continue };
            let matched = if seg.piped && seg.args.is_empty() {
                let mut by_sym = None;
                let mut broken = false;
                for prev in member.segments[..idx].iter().rev() {
                    let Some(pcall) = &prev.call else { continue };
                    if matches!(
                        pcall.kw.as_str(),
                        "SELECT-NET" | "SELECT-PIN" | "SELECT-TERMINAL" | "TEXT" | "WIRE" | "BUNDLE"
                    ) {
                        broken = true;
                        break;
                    }
                    if pcall.kw == "SET-SYMBOL" {
                        by_sym = Some(set_opamp_end(prev).is_some());
                        break;
                    }
                }
                if broken {
                    false
                } else if let Some(ok) = by_sym {
                    ok
                } else {
                    let direct: Vec<_> = member.segments[0]
                        .args
                        .iter()
                        .filter(|a| !matches!(&a.obj, ObjExpr::Ref(r) if r.prefix == '$'))
                        .collect();
                    !direct.is_empty() && direct.iter().all(|a| matches_set(&a.obj, readings, &self.opamp02_objects))
                }
            } else if !seg.piped && !seg.args.is_empty() {
                seg.args.iter().all(|a| matches_set(&a.obj, readings, &self.opamp02_objects))
            } else {
                false
            };
            if matched {
                pipeline_has_orient = true;
                if let Some(replacement) = flip_orient_call(call) {
                    edits.push(record_edit(
                        source,
                        record,
                        call.start..call.end,
                        replacement,
                        "legacy-symbol-orientation",
                    )?);
                }
            }
        }
        if pipeline_has_orient {
            return Ok(edits);
        }
        for (idx, seg) in member.segments.iter().enumerate() {
            let Some(opamp_end) = set_opamp_end(seg) else {
                continue;
            };
            let direct: Vec<_> =
                seg.args.iter().filter(|a| !matches!(&a.obj, ObjExpr::Ref(r) if r.prefix == '$')).collect();
            if !seg.piped {
                if direct.is_empty() || direct.iter().any(|a| is_bare_level(&a.obj)) {
                    continue;
                }
                if direct.iter().all(|a| matches_set(&a.obj, readings, &self.opamp02_oriented)) {
                    continue;
                }
                edits.push(record_edit(
                    source,
                    record,
                    opamp_end..opamp_end,
                    " => @ORIENT(\"MX\")".into(),
                    "legacy-symbol-orientation",
                )?);
            } else if direct.is_empty() {
                let first_direct: Vec<_> = member.segments[0]
                    .args
                    .iter()
                    .filter(|a| !matches!(&a.obj, ObjExpr::Ref(r) if r.prefix == '$'))
                    .collect();
                let from_select = member.segments[..idx].iter().any(|s| {
                    s.call.as_ref().is_some_and(|c| {
                        matches!(c.kw.as_str(), "SELECT-INSTANCE" | "SELECT-USAGE" | "SELECT-ANCESTOR-USAGE")
                    })
                });
                if !from_select && first_direct.iter().any(|a| is_bare_level(&a.obj)) {
                    continue;
                }
                if !from_select
                    && !first_direct.is_empty()
                    && !member.segments[..idx]
                        .iter()
                        .any(|s| s.call.as_ref().is_some_and(|c| matches!(c.kw.as_str(), "AS" | "ATTACH")))
                    && first_direct.iter().all(|a| matches_set(&a.obj, readings, &self.opamp02_oriented))
                {
                    continue;
                }
                let bound_usage_oriented = member.binds.iter().any(|b| {
                    b.prefix == '%' && {
                        let id = format!("%{}", rnl_runtime::problem::written_name(&b.name));
                        readings.iter().all(|l| self.opamp02_oriented.contains(&(*l, id.clone())))
                    }
                });
                if bound_usage_oriented {
                    continue;
                }
                let any_oriented = readings.iter().any(|l| self.opamp02_oriented.iter().any(|(lvl, _)| lvl == l));
                let suffix = if from_select && any_oriented {
                    " => @ORIENT(\"MX\", priority=\"low\")"
                } else {
                    " => @ORIENT(\"MX\")"
                };
                edits.push(record_edit(
                    source,
                    record,
                    opamp_end..opamp_end,
                    suffix.into(),
                    "legacy-symbol-orientation",
                )?);
            }
        }
        Ok(edits)
    }

    pub(super) fn unresolved_pins(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        member: &rnl_runtime::syntax::Member,
    ) -> Vec<super::manual::Reason> {
        let refs = member.segments.iter().flat_map(|s| &s.args).filter_map(|a| match &a.obj {
            ObjExpr::Ref(r) => Some(r),
            _ => None,
        });
        let readings = self.readings_at(path, record, member.start).map(Vec::as_slice).unwrap_or_default();
        if refs.into_iter().any(|r| {
            if r.prefix != '%' || r.path.last().is_none_or(|p| p.sep != Sep::Term) {
                return false;
            }
            let Some(name) = &r.name else { return false };
            let mut object = format!("%{}", rnl_runtime::problem::written_name(name));
            for part in &r.path[..r.path.len() - 1] {
                object.push_str(&format!("::{}", rnl_runtime::problem::written_name(&part.name)));
            }
            readings.iter().any(|level| self.usages.contains(&(*level, object.clone())))
                && !matches!(self.usage_pin(path, source, record, r), Ok(Some(_)))
        }) {
            vec![("usage-pin", "the usage terminal has no uniquely established target pin name; preserve its electrical binding and name the corresponding symbol pin before replacing the reference (9.2.4, 12.1)")]
        } else {
            Vec::new()
        }
    }

    pub(super) fn legacy_reference(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        reference: &Ref,
    ) -> Result<Option<Edit>, String> {
        let readings = self.readings_at(path, record, reference.start).map(Vec::as_slice).unwrap_or_default();
        let Some((new, _)) = self.legacy.reference(readings, reference) else { return Ok(None) };
        if reference.name.as_ref() == Some(new) {
            return Ok(None);
        }
        record_edit(
            source,
            record,
            reference.start..reference.end,
            format!("${}", rnl_runtime::problem::written_name(new)),
            "legacy-standard-name",
        )
        .map(Some)
    }
    pub(super) fn host_terminal(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        reference: &Ref,
    ) -> Result<Option<Edit>, String> {
        let readings = self.readings_at(path, record, reference.start).map(Vec::as_slice).unwrap_or_default();
        let Some(replacement) = self.host_terminals.replacement(readings, reference) else { return Ok(None) };
        record_edit(source, record, reference.start..reference.end, replacement, "host-terminal-position").map(Some)
    }
    pub(super) fn removed_standard_name(&self, path: &str, record: &Record, reference: &Ref) -> bool {
        let readings = self.readings_at(path, record, reference.start).map(Vec::as_slice).unwrap_or_default();
        self.legacy.removed(readings, reference)
    }
    pub(super) fn legacy_packages(&self, path: &str, records: &[Record]) -> BTreeSet<String> {
        let mut packages = BTreeSet::new();
        for record in records {
            let readings = self
                .readings
                .get(&(normalize(path), record.lines.0, record.lines.1))
                .map(Vec::as_slice)
                .unwrap_or_default();
            let Some(body) = parse_record_features(&record.text, true, false).body else { continue };
            for member in &body.members {
                let mut refs = Vec::new();
                rnl_runtime::syntax::walk_refs(member, &mut refs);
                for r in refs {
                    if let rnl_runtime::syntax::RefOrHole::Ref(r) = r {
                        if let Some((_, Some(package))) = self.legacy.reference(readings, r) {
                            packages.insert(package.clone());
                        }
                    }
                }
            }
        }
        packages
    }
    pub(super) fn legacy_package(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        segment: &Segment,
    ) -> Result<Option<Edit>, String> {
        if segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Set(_))) {
            return Ok(None);
        }
        let Some(call) = segment.call.as_ref().filter(|c| c.kw == "USE-PACKAGE") else { return Ok(None) };
        let Some(params) = &call.params else { return Ok(None) };
        let Some(Value::Str(name)) = params.get("name") else { return Ok(None) };
        let readings = self.readings_at(path, record, segment.start).map(Vec::as_slice).unwrap_or_default();
        let Some(new) = self.legacy.package(readings, name) else { return Ok(None) };
        let mut values = params.positional.iter().map(super::rewrite::value).collect::<Vec<_>>();
        values.push(format!("name={}", rnl_runtime::problem::written_string(new)));
        values.push("version=\"0.4\"".into());
        for n in &params.named {
            if !matches!(n.name.as_str(), "name" | "version" | "path") {
                values.push(format!("{}={}", n.name, super::rewrite::value(&n.value)));
            }
        }
        record_edit(
            source,
            record,
            call.start..call.end,
            format!("@USE-PACKAGE({})", values.join(", ")),
            "legacy-standard-package",
        )
        .map(Some)
    }

    pub(super) fn role_arguments(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        segment: &Segment,
    ) -> Result<Vec<Edit>, String> {
        if segment.call.as_ref().is_none_or(|c| !matches!(c.kw.as_str(), "AS" | "ATTACH")) {
            return Ok(Vec::new());
        }
        let Some(definition) = segment.args.iter().find_map(|a| match &a.obj {
            ObjExpr::Ref(r) if r.prefix == '$' && r.path.is_empty() => r.name.as_ref().map(|n| format!("${n}")),
            _ => None,
        }) else {
            return Ok(Vec::new());
        };
        let Some(readings) = self.readings_at(path, record, segment.start) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for arg in &segment.args {
            let Some(role) = &arg.role else { continue };
            let mut replacements = readings
                .iter()
                .map(|level| self.definitions.get(&(*level, definition.clone())).and_then(|changes| changes.get(role)));
            let Some(Some(new)) = replacements.next() else { continue };
            if replacements.any(|other| other != Some(new)) {
                continue;
            }
            let chars: Vec<_> = record.text.chars().collect();
            let Some(offset) = chars[arg.start..arg.end].iter().rposition(|c| *c == '(') else { continue };
            out.push(record_edit(
                source,
                record,
                arg.start + offset..arg.end,
                format!("({new})"),
                "standard-role-name",
            )?);
        }
        Ok(out)
    }

    pub(super) fn role_reference(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        reference: &Ref,
    ) -> Result<Option<Edit>, String> {
        if reference.prefix != '%'
            || reference.sub.is_some()
            || reference.places.is_some()
            || reference.number.is_some()
        {
            return Ok(None);
        }
        let Some(name) = &reference.name else { return Ok(None) };
        let Some(first) = reference.path.first() else { return Ok(None) };
        if first.sep != Sep::Sub || first.sub.is_some() || reference.path.iter().any(|p| p.sub.is_some()) {
            return Ok(None);
        }
        let object = format!("%{}", rnl_runtime::problem::written_name(name));
        let Some(readings) = self.readings_at(path, record, reference.start) else {
            return Ok(None);
        };
        let mut replacements =
            readings.iter().map(|level| self.roles.get(&(*level, object.clone(), first.name.clone())));
        let Some(Some(new)) = replacements.next() else { return Ok(None) };
        if replacements.any(|other| other != Some(new)) {
            return Ok(None);
        }
        let mut text = object;
        for (i, part) in reference.path.iter().enumerate() {
            text.push_str(if part.sep == Sep::Sub { "::" } else { ":" });
            text.push_str(&rnl_runtime::problem::written_name(if i == 0 { new } else { &part.name }));
        }
        record_edit(source, record, reference.start..reference.end, text, "standard-role-reference").map(Some)
    }

    pub(super) fn usage_pin(
        &self,
        path: &str,
        source: &str,
        record: &Record,
        reference: &Ref,
    ) -> Result<Option<Edit>, String> {
        if reference.prefix != '%'
            || reference.sub.is_some()
            || reference.places.is_some()
            || reference.number.is_some()
        {
            return Ok(None);
        }
        let Some(name) = &reference.name else { return Ok(None) };
        let Some((terminal, parents)) = reference.path.split_last() else { return Ok(None) };
        if terminal.sep != Sep::Term
            || terminal.sub.is_some()
            || parents.iter().any(|p| p.sep != Sep::Sub || p.sub.is_some())
        {
            return Ok(None);
        }
        let mut object = format!("%{}", rnl_runtime::problem::written_name(name));
        for part in parents {
            object.push_str("::");
            object.push_str(&rnl_runtime::problem::written_name(&part.name));
        }
        let Some(readings) = self.readings_at(path, record, reference.start) else {
            return Ok(None);
        };
        let mut resolved = None;
        for level in readings {
            let Some(pin) = self.pins.get(&(*level, object.clone(), terminal.name.clone())) else { return Ok(None) };
            if resolved.as_ref().is_some_and(|old| old != pin) {
                return Ok(None);
            }
            resolved = Some(pin.clone());
        }
        let Some(pin) = resolved else { return Ok(None) };
        record_edit(
            source,
            record,
            reference.start..reference.end,
            format!("{object}::{}", rnl_runtime::problem::written_name(&pin)),
            "usage-named-pin",
        )
        .map(Some)
    }
}
