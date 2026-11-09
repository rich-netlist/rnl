//! Preserve removed standard-set policies as explicit file-owned rules.
use super::edit::{record_edit, Edit};
use rnl_runtime::host::{LevelKind, Record};
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Member, ObjExpr, Ref, Value};
use rnl_runtime::Interpretation;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Sets {
    entry: String,
    names: BTreeMap<String, String>,
    seed: String,
}

fn uses(member: &Member, name: &str) -> bool {
    member.segments.iter().flat_map(|s| &s.args).any(|a| match &a.obj {
        ObjExpr::Ref(r) => r.prefix == '$' && r.name.as_deref() == Some(name),
        ObjExpr::Set(set) => set.members.iter().any(|m| uses(m, name)),
        _ => false,
    })
}

fn overrides(member: &Member) -> bool {
    member.segments.iter().any(|segment| {
        let inline = segment.args.iter().any(|a| matches!(a.obj, ObjExpr::Set(_)));
        let package = segment.call.as_ref().filter(|c| c.kw == "USE-PACKAGE")
            .and_then(|c| c.params.as_ref()).and_then(|p| p.get("name"));
        (inline && matches!(package, Some(Value::Str(n)) if matches!(n.as_str(), "rnl-std" | "rnl-std-fill" | "rnl-std-basic")))
            || segment.args.iter().any(|a| matches!(&a.obj, ObjExpr::Set(s) if s.members.iter().any(|m| overrides(m))))
    })
}

impl Sets {
    pub(super) fn new(input: &Interpretation, entry: &str) -> Self {
        let mut result = Self::default();
        // One top-level seed must have the same package environment everywhere
        // it reaches. Section-specific and mixed package readings need separate rules.
        let roots: Vec<_> = input.doc.levels.iter().filter(|l| l.path == entry && l.kind == LevelKind::File).collect();
        if roots.len() != 1 || roots[0].section.is_some() {
            return result;
        }
        let (shadowed, aliases) = super::standard::shadows(input);
        if aliases || input.model.levels.is_empty() || input.problems.iter().any(|p| p.version != "0.3") {
            return result;
        }
        if input.model.levels.iter().any(|l| !l.packages.iter().any(|p| p.name == "rnl-std-fill" && p.version == "0.3"))
        {
            return result;
        }
        let bundled = rnl_view::bundled();
        let sources: Vec<_> = input
            .sources
            .iter()
            .filter(|(_, text)| scan_package_decl(text).is_some_and(|(n, _)| n == "rnl-std-fill"))
            .collect();
        if sources.is_empty()
            || sources.iter().any(|(_, text)| {
                !bundled.files.iter().any(|(path, original)| path.starts_with("<bundled>/0.3/") && text == original)
            })
        {
            return result;
        }
        for (path, text) in &input.sources {
            if bundled.files.values().any(|known| known == text) {
                continue;
            }
            let records = rnl_runtime::records::scan_run(
                &rnl_runtime::records::package_lines(text),
                path,
                &mut rnl_runtime::diag::Sink::default(),
            );
            if records.iter().any(|r| {
                parse_record_features(&r.text, true, false).body.is_some_and(|b| b.members.iter().any(|m| overrides(m)))
            }) {
                return result;
            }
        }
        let referenced = |name: &str| {
            input.doc.levels.iter().flat_map(|l| &l.records).any(|r| {
                parse_record_features(&r.text, true, false)
                    .body
                    .is_some_and(|body| body.members.iter().any(|m| uses(m, name)))
            })
        };
        let fresh = |base: &str| {
            let mut name = base.to_string();
            let mut index = 2;
            while input.sources.iter().any(|(_, text)| text.contains(&name)) {
                name = format!("{base}_{index}");
                index += 1;
            }
            name
        };
        let mut statements = Vec::new();
        if referenced("WIRE_NET_SET") && !shadowed.contains("$WIRE_NET_SET") {
            let name = fresh("$RNL_UPGRADE_WIRE_NET_SET");
            statements.push(format!("{name} = # @SELECT-NET(recursive=true)"));
            statements.push(format!("$GROUND_NET_SET @REMOVE-FROM {name}"));
            statements.push(format!("$SUPPLY_NET_SET @REMOVE-FROM {name}"));
            result.names.insert("WIRE_NET_SET".into(), name);
        }
        // 0.3 adds one free port tag per untagged port net. PORT_LIST would add
        // layout facts, so retain the former selection and generation explicitly.
        if !shadowed.contains("$PORT_FILL_SET") && !shadowed.contains("$IO_PORT") {
            let name = fresh("$RNL_UPGRADE_PORT_FILL_SET");
            statements.push(format!("{name} = # @SELECT-NET(port=true, recursive=true)"));
            statements.push(format!("$SUPPLY_NET_SET @REMOVE-FROM {name}"));
            statements.push(format!("$GROUND_NET_SET @REMOVE-FROM {name}"));
            statements.push(format!(
                "# @SELECT-USAGE(recursive=true) $IO_PORT => @SELECT-NET(recursive=true) => @REMOVE-FROM {name}"
            ));
            statements.push(format!("$IO_PORT @ATTACH {name}"));
            result.names.insert("PORT_FILL_SET".into(), name);
        }
        if !statements.is_empty() {
            result.entry = entry.into();
            result.seed = format!("@RNL {{ {} }}", statements.join("; "));
        }
        result
    }

    pub(super) fn seed(&self, path: &str) -> Option<&str> {
        (path == self.entry && !self.seed.is_empty()).then_some(&self.seed)
    }

    pub(super) fn reference(&self, source: &str, record: &Record, reference: &Ref) -> Result<Option<Edit>, String> {
        if reference.prefix != '$'
            || !reference.path.is_empty()
            || reference.sub.is_some()
            || reference.places.is_some()
        {
            return Ok(None);
        }
        let Some(replacement) = reference.name.as_ref().and_then(|n| self.names.get(n)) else { return Ok(None) };
        record_edit(source, record, reference.start..reference.end, replacement.clone(), "explicit-standard-set")
            .map(Some)
    }
}
