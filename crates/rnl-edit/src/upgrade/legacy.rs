//! Standard-series spellings that changed between 0.2 and the current series.
use rnl_runtime::interp::unit::scan_package_decl;
use rnl_runtime::syntax::{parse_record_features, Ref};
use rnl_runtime::Interpretation;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn renamed(name: &str) -> Option<&'static str> {
    Some(match name {
        "RES" => "RESISTOR",
        "CAP" => "CAPACITOR",
        "IND" => "INDUCTOR",
        "VSRC" => "VOLTAGE_SOURCE",
        "ISRC" => "CURRENT_SOURCE",
        "NMOS4" => "NMOS_4TERM",
        "PMOS4" => "PMOS_4TERM",
        "GROUND" => "GROUND_SYMBOL",
        "SUPPLY_BAR" => "SUPPLY_SYMBOL",
        "PORT_TAG" => "IO_PORT_SYMBOL",
        "NC_CROSS" => "NO_CONNECT_SYMBOL",
        "JUNCTION_DOT" => "JUNCTION_SYMBOL",
        "CURRENT_ARROW_SYM" => "CURRENT_ARROW_SYMBOL",
        "ARROW_HEAD" => "ARROW_SYMBOL",
        "GND" => "GROUND",
        "IOPORT" => "IO_PORT",
        "NC" => "NO_CONNECT",
        "GND_STUB" => "GROUND_STUB",
        "GND_FILL_SET" => "GROUND_FILL_SET",
        "LABEL_TERM_SET" => "LABEL_TERMINAL_SET",
        "MIRROR_PAIR" => "MIRRORED",
        _ => return None,
    })
}

pub(super) fn merged_package(name: &str) -> Option<&'static str> {
    Some(match name {
        "rnl-std-symbols" => "rnl-std-core",
        "rnl-std-virtual" => "rnl-std-basic",
        "rnl-std-layout" | "rnl-std-semantics" => "rnl-std-structures",
        _ => return None,
    })
}

fn bindings(version: &str) -> BTreeMap<String, String> {
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
                    for bind in member.binds.iter().filter(|b| b.prefix == '$') {
                        out.insert(bind.name.clone(), package.clone());
                    }
                }
            }
        }
    }
    out
}

#[derive(Default)]
pub(super) struct Legacy {
    names: BTreeMap<(usize, String), (String, String)>,
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
        let known = |package: &str| {
            let sources: Vec<_> = input
                .sources
                .iter()
                .filter(|(_, s)| scan_package_decl(s).is_some_and(|(n, v)| n == package && v == "0.2"))
                .collect();
            !sources.is_empty() && sources.iter().all(|(_, s)| bundled.files.values().any(|b| b == s))
        };
        for (old, package) in bindings("0.2") {
            let new = renamed(&old).unwrap_or(&old);
            let Some(destination) = target.get(new) else { continue };
            if shadowed.contains(&format!("${old}")) || shadowed.contains(&format!("${new}")) || !known(&package) {
                continue;
            }
            // Names that did not move need no explicit dependency.
            if old == new && package == *destination {
                continue;
            }
            for level in &input.model.levels {
                if level.packages.iter().any(|p| p.name == package && p.version == "0.2") {
                    out.names.insert((level.level, old.clone()), (new.into(), destination.clone()));
                }
            }
        }
        for level in &input.model.levels {
            for package in &level.packages {
                if package.version == "0.2" && known(&package.name) {
                    if let Some(new) = merged_package(&package.name) {
                        out.packages.insert((level.level, package.name.clone()), new.into());
                    }
                }
            }
        }
        for version in ["0.2", "0.3"] {
            for (old, package) in bindings(version) {
                let new = if version == "0.2" { renamed(&old).unwrap_or(&old) } else { &old };
                if target.contains_key(new) || shadowed.contains(&format!("${old}")) {
                    continue;
                }
                let sources: Vec<_> = input
                    .sources
                    .iter()
                    .filter(|(_, s)| scan_package_decl(s).is_some_and(|(n, v)| n == package && v == version))
                    .collect();
                if sources.is_empty() || sources.iter().any(|(_, s)| !bundled.files.values().any(|b| b == s)) {
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
    pub(super) fn reference(&self, readings: &[usize], r: &Ref) -> Option<&(String, String)> {
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
