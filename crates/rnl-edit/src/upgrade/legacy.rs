//! Standard-series spellings that changed between earlier series and 0.4.
use rnl_runtime::host::LevelKind;
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Ref};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BindingKind {
    Define,
    Symbol,
    Other,
}

pub(super) fn renamed(name: &str) -> Option<&'static str> {
    Some(match name {
        "RES" => "RESISTOR",
        "CAP" => "CAPACITOR",
        "IND" => "INDUCTOR",
        "VSRC" => "VOLTAGE_SOURCE",
        "ISRC" => "CURRENT_SOURCE",
        "NMOS4" => "NMOS_4TERM",
        "PMOS4" => "PMOS_4TERM",
        "VCVS" => "VCVS_4TERM",
        "VCCS" => "VCCS_4TERM",
        "GROUND" => "GROUND_SYMBOL",
        "SUPPLY_BAR" => "SUPPLY_SYMBOL",
        "PORT_TAG" => "IO_PORT_SYMBOL",
        "NC_CROSS" => "NO_CONNECT_SYMBOL",
        "JUNCTION_DOT" => "JUNCTION_SYMBOL",
        "CURRENT_ARROW_SYM" => "ARROW_SYMBOL",
        "ARROW_HEAD" => "ARROW_SYMBOL",
        "GND" => "GROUND",
        "IOPORT" => "IO_PORT",
        "NC" => "NO_CONNECT",
        "GND_STUB" => "GROUND_STUB",
        "GND_FILL_SET" => "GROUND_FILL_SET",
        "LABEL_TERM_SET" => "LABEL_FILL_SET",
        "MIRROR_PAIR" => "MIRRORED_LAYOUT",
        _ => return None,
    })
}

pub(super) fn renamed_03_symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        "VCVS" | "CCVS" => "CONTROLLED_VOLTAGE_SOURCE",
        "VCCS" | "CCCS" => "CONTROLLED_CURRENT_SOURCE",
        "CURRENT_ARROW_SYMBOL" => "ARROW_SYMBOL",
        "OPAMP_5TERM" => "OPAMP",
        "COMPARATOR_5TERM" => "COMPARATOR",
        "ADDER" => "ADDER_SYMBOL",
        "MULTIPLIER" => "MULTIPLIER_SYMBOL",
        "DELAY_CELL_SYMBOL" => "DELAY_CELL",
        _ => return None,
    })
}

/// The general definitions of the series 0.3 that the series 0.4 renames: a definition that only
/// places is named with `_LAYOUT`.
pub(super) fn renamed_03_definition(name: &str) -> Option<&'static str> {
    Some(match name {
        "MIRRORED" => "MIRRORED_LAYOUT",
        _ => return None,
    })
}

/// The name of the series 0.4 for a name of the series 0.3 of this kind.
fn renamed_03(name: &str, kind: BindingKind) -> Option<&'static str> {
    match kind {
        BindingKind::Symbol => renamed_03_symbol(name),
        BindingKind::Define => renamed_03_definition(name),
        BindingKind::Other => None,
    }
}

pub(super) fn merged_package(name: &str) -> Option<&'static str> {
    Some(match name {
        "rnl-std-symbols" | "rnl-std-core" => "rnl-std-all",
        "rnl-std-virtual" => "rnl-std-basic",
        "rnl-std-layout" => "rnl-std-semantics",
        _ => return None,
    })
}

fn bindings(version: &str) -> BTreeMap<String, (String, BindingKind)> {
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
            if let Some(body) = parse_record_features(&record.text, true, false).body {
                for member in &body.members {
                    let kind = if member.segments.iter().filter_map(|s| s.call.as_ref()).any(|c| c.kw == "DEFINE") {
                        BindingKind::Define
                    } else if member.segments.iter().filter_map(|s| s.call.as_ref()).any(|c| c.kw == "DEFINE-SYMBOL") {
                        BindingKind::Symbol
                    } else {
                        BindingKind::Other
                    };
                    for bind in member.binds.iter().filter(|b| b.prefix == '$') {
                        out.insert(bind.name.clone(), (package.clone(), kind));
                    }
                }
            }
        }
    }
    out
}

#[derive(Default)]
pub(super) struct Legacy {
    names: BTreeMap<(usize, String), (String, Option<String>)>,
    packages: BTreeMap<(usize, String), String>,
    removed: BTreeSet<(usize, String)>,
}
impl Legacy {
    pub(super) fn new(input: &Interpretation) -> Self {
        let mut out = Self::default();
        let (shadowed, aliases) = super::standard::shadows(input);
        if aliases {
            return out;
        }
        let target = bindings("0.4");
        let bundled = rnl_view::bundled();
        let known = |version: &str, package: &str| {
            let sources: Vec<_> = input
                .sources
                .iter()
                .filter(|(_, s)| scan_package_decl(s).is_some_and(|(n, v)| n == package && v == version))
                .collect();
            !sources.is_empty() && sources.iter().all(|(_, s)| bundled.files.values().any(|b| b == s))
        };
        for (old, (package, kind)) in bindings("0.2") {
            let new = renamed(&old).unwrap_or(&old);
            let Some((destination, target_kind)) = target.get(new) else { continue };
            if *target_kind != kind
                || shadowed.contains(&format!("${old}"))
                || shadowed.contains(&format!("${new}"))
                || !known("0.2", &package)
            {
                continue;
            }
            // Names that did not move need no explicit dependency.
            if old == new && package == *destination {
                continue;
            }
            for level in &input.model.levels {
                if level.packages.iter().any(|p| p.name == package && p.version == "0.2") {
                    out.names.insert((level.level, old.clone()), (new.into(), Some(destination.clone())));
                }
            }
        }
        for (old, (package, kind)) in bindings("0.3") {
            let Some(new) = renamed_03(&old, kind) else { continue };
            let Some((destination, target_kind)) = target.get(new) else { continue };
            if *target_kind != kind
                || shadowed.contains(&format!("${old}"))
                || shadowed.contains(&format!("${new}"))
                || !known("0.3", &package)
            {
                continue;
            }
            let explicit_pkg = (package != *destination).then(|| destination.clone());
            for level in &input.model.levels {
                if level.packages.iter().any(|p| p.name == package && p.version == "0.3") {
                    out.names.insert((level.level, old.clone()), (new.into(), explicit_pkg.clone()));
                }
            }
        }
        for version in ["0.2", "0.3"] {
            for level in &input.model.levels {
                for package in &level.packages {
                    if package.version == version && known(version, &package.name) {
                        if let Some(new) = merged_package(&package.name) {
                            out.packages.insert((level.level, package.name.clone()), new.into());
                        }
                    }
                }
            }
        }
        if input.model.levels.is_empty() {
            for (idx, level) in input.doc.levels.iter().enumerate() {
                if level.kind == LevelKind::Package || idx == 0 {
                    for pkg in ["rnl-std-symbols", "rnl-std-core", "rnl-std-virtual", "rnl-std-layout"] {
                        if known("0.2", pkg) || known("0.3", pkg) {
                            if let Some(new) = merged_package(pkg) {
                                out.packages.insert((idx, pkg.into()), new.into());
                            }
                        }
                    }
                }
            }
        }
        for version in ["0.2", "0.3"] {
            for (old, (package, kind)) in bindings(version) {
                let new = if version == "0.2" {
                    renamed(&old).unwrap_or(&old)
                } else {
                    renamed_03(&old, kind).unwrap_or(&old)
                };
                if target.get(new).is_some_and(|(_, tk)| *tk == kind) || shadowed.contains(&format!("${old}")) {
                    continue;
                }
                if !known(version, &package) {
                    continue;
                }
                for level in &input.model.levels {
                    if level.packages.iter().any(|p| p.name == package && p.version == version) {
                        out.removed.insert((level.level, old.clone()));
                    }
                }
            }
        }
        out
    }
    pub(super) fn removed(&self, readings: &[usize], reference: &Ref) -> bool {
        reference.prefix == '$'
            && reference
                .name
                .as_ref()
                .is_some_and(|name| readings.iter().any(|level| self.removed.contains(&(*level, name.clone()))))
    }
    pub(super) fn reference(&self, readings: &[usize], r: &Ref) -> Option<&(String, Option<String>)> {
        if r.prefix != '$' || !r.path.is_empty() || r.sub.is_some() || r.places.is_some() {
            return None;
        }
        let name = r.name.as_ref()?;
        let mut values = readings.iter().map(|level| self.names.get(&(*level, name.clone())));
        let first = values.next()??;
        values.all(|v| v == Some(first)).then_some(first)
    }
    pub(super) fn package(&self, readings: &[usize], name: &str) -> Option<&str> {
        let mut values = readings.iter().map(|level| self.packages.get(&(*level, name.into())));
        let first = values.next()??;
        values.all(|v| v == Some(first)).then_some(first.as_str())
    }
}
