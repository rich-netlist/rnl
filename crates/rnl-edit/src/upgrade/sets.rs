//! Preserve removed standard-set policies as explicit file-owned rules.
use super::edit::{record_edit, Edit};
use rnl_runtime::host::{LevelKind, Record};
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::model::SymbolOrigin;
use rnl_runtime::problem::written_name;
use rnl_runtime::syntax::{parse_record_features, Member, ObjExpr, Ref, Value};
use rnl_runtime::Interpretation;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Sets {
    entry: String,
    names: BTreeMap<String, String>,
    seed: String,
    routing: BTreeMap<usize, (bool, BTreeMap<String, bool>)>,
}

fn literal_nets(obj: &ObjExpr, names: &mut Vec<String>) -> bool {
    match obj {
        ObjExpr::Ref(r) if r.prefix == '&' && r.path.is_empty() && r.sub.is_none() && r.places.is_none() => {
            if let Some(name) = &r.name {
                names.push(name.clone());
                return true;
            }
            false
        }
        ObjExpr::Set(set) => set.members.iter().all(|m| {
            m.binds.is_empty()
                && matches!(m.segments.as_slice(), [s] if s.call.is_none()
                && s.args.iter().all(|arg| literal_nets(&arg.obj, names)))
        }),
        _ => false,
    }
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
        (inline && matches!(package, Some(Value::Str(n)) if matches!(n.as_str(), "rnl-std" | "rnl-std-all" | "rnl-std-core" | "rnl-std-full" | "rnl-std-fill" | "rnl-std-basic" | "rnl-std-spice" | "rnl-std-spectre")))
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
            for p in &input.problems {
                let case_sensitive = input.doc.levels[p.level].case_sensitive;
                let nets = p
                    .nets
                    .iter()
                    .map(|n| {
                        let name = if case_sensitive { n.name.clone() } else { n.name.to_ascii_lowercase() };
                        (name, !n.routes.is_empty())
                    })
                    .collect();
                result.routing.insert(p.level, (case_sensitive, nets));
            }
        }
        // 0.3 defines a candidate set but does not attach labels from it by default.
        // Empty only the bundled 0.4 fill's set, retaining a file-owned set for old consumers.
        if !shadowed.contains("$LABEL_TERMINAL_SET") && referenced("LABEL_TERMINAL_SET") {
            let name = fresh("$RNL_UPGRADE_LABEL_TERMINAL_SET");
            statements.push(format!(
                "{name} = # @SELECT-TERMINAL(recursive=true) => @SELECT-PIN(wired=false, recursive=true)"
            ));
            for set in ["$GROUND_NET_SET", "$SUPPLY_NET_SET"] {
                statements.push(format!(
                    "{set} @SELECT-TERMINAL(recursive=true) => @SELECT-PIN(recursive=true) => @REMOVE-FROM {name}"
                ));
            }
            statements.push(format!("# @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-TERMINAL(recursive=true) => @SELECT-PIN(recursive=true) => @REMOVE-FROM {name}"));
            result.names.insert("LABEL_TERMINAL_SET".into(), name);
        }
        if !shadowed.contains("$LABEL_FILL_SET") {
            statements.push("# @SELECT-PIN(recursive=true) => @REMOVE-FROM $LABEL_FILL_SET".into());
        }
        // Preserve 0.3 four-terminal MOS and voltage-controlled switch defaults and bulk-pin
        // ground/supply stub fill whenever an instance actually resolved to those 0.3 symbols.
        let bundled_03 = |pkg_name: &str| {
            let s: Vec<_> = input
                .sources
                .iter()
                .filter(|(_, t)| scan_package_decl(t).is_some_and(|(n, _)| n == pkg_name))
                .collect();
            !s.is_empty()
                && s.iter()
                    .all(|(_, t)| bundled.files.iter().any(|(p, orig)| p.starts_with("<bundled>/0.3/") && t == orig))
                && input
                    .model
                    .levels
                    .iter()
                    .all(|l| l.packages.iter().any(|p| p.name == pkg_name && p.version == "0.3"))
        };
        let has_symbol = |pkg_name: &str, sym_name: &str| {
            bundled_03(pkg_name)
                && input.model.levels.iter().flat_map(|l| &l.objects).any(|o| {
                    o.kind == "instance"
                        && o.symbol.as_ref().is_some_and(|s| {
                            s.name == sym_name && matches!(&s.origin, SymbolOrigin::Package(p) if p == pkg_name)
                        })
                })
        };
        let has_spice = bundled_03("rnl-std-spice");
        let has_spectre = bundled_03("rnl-std-spectre");
        let has_nmos_4term = has_symbol("rnl-std-semiconductors", "$NMOS_4TERM");
        let has_pmos_4term = has_symbol("rnl-std-semiconductors", "$PMOS_4TERM");
        let has_vc_switch = has_symbol("rnl-std-basic", "$VOLTAGE_CONTROLLED_SWITCH");
        if has_spice || has_spectre {
            if has_nmos_4term && !shadowed.contains("$NMOS_4TERM") {
                statements.push("# @SELECT-INSTANCE(type=\"nmos\", recursive=true) => @SET-SYMBOL $NMOS_4TERM".into());
            }
            if has_pmos_4term && !shadowed.contains("$PMOS_4TERM") {
                statements.push("# @SELECT-INSTANCE(type=\"pmos\", recursive=true) => @SET-SYMBOL $PMOS_4TERM".into());
            }
            if has_vc_switch && !shadowed.contains("$VOLTAGE_CONTROLLED_SWITCH") {
                if has_spice {
                    statements.push(
                        "# @SELECT-INSTANCE(name=\"S*\", recursive=true) => @SET-SYMBOL $VOLTAGE_CONTROLLED_SWITCH"
                            .into(),
                    );
                }
                if has_spectre {
                    statements.push(
                        "# @SELECT-INSTANCE(model=\"relay\", recursive=true) => @SET-SYMBOL $VOLTAGE_CONTROLLED_SWITCH"
                            .into(),
                    );
                }
                statements.push(
                    "# @SELECT-INSTANCE(type=\"sw\", recursive=true) => @SET-SYMBOL $VOLTAGE_CONTROLLED_SWITCH".into(),
                );
            }
        }
        if has_nmos_4term || has_pmos_4term {
            if !shadowed.contains("$GROUND_FILL_SET")
                && !shadowed.contains("$GROUND_NET_SET")
                && !shadowed.contains("$NO_CONNECT")
            {
                let bg = fresh("%rnl_upgrade_bulk_ground");
                statements.push(format!(
                    "{bg} = $GROUND_NET_SET @SELECT-TERMINAL(type=\"nmos\", terminal=\"b\", recursive=true) => @SELECT-PIN(wired=false, recursive=true)"
                ));
                statements.push(format!(
                    "$GROUND_NET_SET @SELECT-TERMINAL(type=\"pmos\", terminal=\"b\", recursive=true) => @SELECT-PIN(wired=false, recursive=true) => @ADD-TO {bg}"
                ));
                statements.push(format!(
                    "# @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-PIN(role=\"t\", recursive=true) => @REMOVE-FROM {bg}"
                ));
                statements.push(format!("{bg} @ADD-TO $GROUND_FILL_SET"));
            }
            if !shadowed.contains("$SUPPLY_FILL_SET")
                && !shadowed.contains("$SUPPLY_NET_SET")
                && !shadowed.contains("$NO_CONNECT")
            {
                let bs = fresh("%rnl_upgrade_bulk_supply");
                statements.push(format!(
                    "{bs} = $SUPPLY_NET_SET @SELECT-TERMINAL(type=\"nmos\", terminal=\"b\", recursive=true) => @SELECT-PIN(wired=false, recursive=true)"
                ));
                statements.push(format!(
                    "$SUPPLY_NET_SET @SELECT-TERMINAL(type=\"pmos\", terminal=\"b\", recursive=true) => @SELECT-PIN(wired=false, recursive=true) => @ADD-TO {bs}"
                ));
                statements.push(format!(
                    "# @SELECT-USAGE(recursive=true) $NO_CONNECT => @SELECT-PIN(role=\"t\", recursive=true) => @REMOVE-FROM {bs}"
                ));
                statements.push(format!("{bs} @ADD-TO $SUPPLY_FILL_SET"));
            }
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

    /// Block new structure routing on explicitly removed nets only when the
    /// old interpreter found no whole-net route. An explicit WIRE is unaffected.
    pub(super) fn unrouting(
        &self,
        source: &str,
        record: &Record,
        member: &Member,
        readings: &[usize],
    ) -> Result<Result<Option<Edit>, super::manual::Reason>, String> {
        if !self.names.contains_key("WIRE_NET_SET") {
            return Ok(Ok(None));
        }
        let Some(segment) = member.segments.iter().find(|s| {
            s.call.as_ref().is_some_and(|c| c.kw == "REMOVE-FROM")
                && matches!(s.args.last().map(|a| &a.obj), Some(ObjExpr::Ref(r))
                    if r.prefix == '$' && r.name.as_deref() == Some("WIRE_NET_SET") && r.path.is_empty())
        }) else {
            return Ok(Ok(None));
        };
        let review = ("wire-set-exclusion", "the removed nets or their old routing intent cannot be identified consistently; specify the required UNROUTE-NET in each drawing (13.7)");
        let mut names = Vec::new();
        if segment.piped
            || readings.is_empty()
            || !segment.args[..segment.args.len() - 1].iter().all(|a| literal_nets(&a.obj, &mut names))
        {
            return Ok(Err(review));
        }
        let mut excluded = Vec::new();
        for name in names {
            let mut routes = readings.iter().map(|level| {
                let (case_sensitive, nets) = self.routing.get(level)?;
                if *case_sensitive {
                    nets.get(&name)
                } else {
                    nets.get(&name.to_ascii_lowercase())
                }
            });
            let Some(Some(routed)) = routes.next() else { return Ok(Err(review)) };
            if routes.any(|other| other != Some(routed)) {
                return Ok(Err(review));
            }
            if !routed {
                excluded.push(format!("&{}", written_name(&name)));
            }
        }
        if excluded.is_empty() {
            return Ok(Ok(None));
        }
        let after = format!("; {{{}}} @UNROUTE-NET(priority=\"high\")", excluded.join(" "));
        record_edit(source, record, member.end..member.end, after, "preserve-wire-set-exclusions").map(|e| Ok(Some(e)))
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
