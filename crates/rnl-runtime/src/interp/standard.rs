//! Suggestions for missing names, discovered from available standard collections (16.3).
//! Inspect declarations without interpreting unused packages or reporting their body errors.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::diag::{codes, Sink};
use crate::records::{package_lines, scan_run};
use crate::source::{dirname, join, Provider};
use crate::syntax::{parse_record_features, ObjExpr, Value};

use super::ctx::Ctx;
use super::packages::standard_package_name;
use super::unit::{
    package_version_cmp, params_by_name, scan_language_decl, scan_package_decl, valid_package_version, version_ok,
};
use super::Interp;

#[derive(Default)]
pub(super) struct StandardHints {
    names: Option<BTreeMap<String, Vec<String>>>,
    reported: HashSet<String>,
}

#[derive(Clone)]
struct Package {
    name: String,
    version: String,
    path: String,
    text: String,
}

fn package(files: &dyn Provider, path: &str) -> Option<Package> {
    let text = files.read(path)?;
    if scan_language_decl(&text).as_deref() != Some("0.4") {
        return None;
    }
    let (name, version) = scan_package_decl(&text)?;
    Some(Package { name, version, path: path.into(), text })
}

fn index(files: &dyn Provider) -> BTreeMap<String, Vec<String>> {
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut candidates: HashMap<String, Vec<Package>> = HashMap::new();
    let mut seen = HashSet::new();
    // Discover names by following the formal collection, without recognizing definition names.
    let mut pending = vec![(standard_package_name("0.4").to_string(), None::<String>, None::<String>)];
    while let Some((name, version, path)) = pending.pop() {
        if version.as_ref().is_some_and(|v| !valid_package_version(v)) {
            continue;
        }
        // A missing path or an unsuitable declared name/version falls back to lookup;
        // an existing file without a valid PACKAGE does not (16.2). Keep this inspection
        // quiet, since these packages have not been requested by the document.
        let by_name = path.as_ref().and_then(|path| files.read(path)).is_none_or(|text| {
            scan_package_decl(&text)
                .is_some_and(|(declared, have)| declared != name || !version_ok(version.as_deref(), &have, true))
        });
        let found = if by_name {
            let choices = candidates.entry(name.clone()).or_insert_with(|| {
                let mut choices = Vec::<Package>::new();
                for path in files.package_candidates(&name) {
                    if let Some(p) = package(files, &path).filter(|p| p.name == name) {
                        // The first copy of a version wins, as in normal package lookup.
                        if !choices.iter().any(|before| package_version_cmp(&before.version, &p.version, true).is_eq())
                        {
                            choices.push(p);
                        }
                    }
                }
                choices
            });
            let compatible = choices.iter().filter(|p| version_ok(version.as_deref(), &p.version, true));
            if version.is_some() {
                compatible.min_by(|a, b| package_version_cmp(&a.version, &b.version, true)).cloned()
            } else {
                compatible.max_by(|a, b| package_version_cmp(&a.version, &b.version, true)).cloned()
            }
        } else {
            path.and_then(|path| package(files, &path))
                .filter(|p| p.name == name && version_ok(version.as_deref(), &p.version, true))
        };
        let Some(p) = found else { continue };
        if !seen.insert(p.path.clone()) {
            continue;
        }
        for record in scan_run(&package_lines(&p.text), &p.path, &mut Sink::new()) {
            let ast = parse_record_features(&record.text, true, true);
            if ast.error.is_some() {
                continue;
            }
            let kind = ast.header.as_ref().and_then(|h| h.get("type"));
            let Some(body) = ast.body else { continue };
            for member in body.members.iter().filter(|m| m.error.is_none()) {
                if kind == Some(&Value::Str("header".into())) {
                    let Some(segment) = member.segments.first() else { continue };
                    let Some(call) = segment.call.as_ref().filter(|c| c.kw == "USE-PACKAGE") else { continue };
                    let Ok(params) = params_by_name("USE-PACKAGE", call.params.as_ref(), true, str::to_string) else {
                        continue;
                    };
                    let text = |key: &str| match params.get(key) {
                        Some(Value::Str(s)) => Some(s.clone()),
                        _ => None,
                    };
                    let Some(name) = text("name") else { continue };
                    if let Some(ObjExpr::Set(inline)) = segment.args.first().map(|a| &a.obj) {
                        for binding in inline
                            .members
                            .iter()
                            .filter(|m| m.error.is_none())
                            .flat_map(|m| &m.binds)
                            .filter(|b| b.prefix == '$')
                        {
                            names.entry(binding.name.clone()).or_default().push(name.clone());
                        }
                    } else {
                        pending.push((name, text("version"), text("path").map(|path| join(&dirname(&p.path), &path))));
                    }
                } else if kind.is_none() || kind == Some(&Value::Str("authored".into())) {
                    for binding in member.binds.iter().filter(|b| b.prefix == '$') {
                        let owners = names.entry(binding.name.clone()).or_default();
                        if !owners.contains(&p.name) {
                            owners.push(p.name.clone());
                        }
                    }
                }
            }
        }
    }
    names
}

impl Interp<'_> {
    pub(super) fn hint_standard_name(&mut self, name: &str, ctx: &Ctx, offset: usize) {
        if !self.follows_0_4(self.source_unit(ctx)) {
            return;
        }
        let Some(loc) = self.rec_loc(ctx.rec, offset) else { return };
        if self.standard_hints.reported.contains(&loc.file) {
            return;
        }
        if self.standard_hints.names.is_none() {
            self.standard_hints.names = Some(index(self.provider));
        }
        let Some(packages) = self.standard_hints.names.as_ref().and_then(|names| names.get(name)).cloned() else {
            return;
        };
        let eff = self.eff(self.name_chain(self.name_unit(ctx), ctx.scene));
        let Some(package) = packages
            .into_iter()
            .find(|name| eff.unit.get(name).copied().flatten().is_none_or(|u| self.units[u].embedded))
        else {
            return;
        };
        self.standard_hints.reported.insert(loc.file);
        self.hint_at(ctx, codes::STANDARD_PACKAGE_NOT_USED,
            format!("${name} is defined by package {package}, which is not in use; add @USE-PACKAGE for it to the file header, or enable the tool standard-package setting such as --include-std (16.3)"), offset);
    }
}
