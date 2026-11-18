//! The packages in effect (standard 16.2, 16.3): what a USE-PACKAGE brings in through the packages
//! a package references, one package of each name on a chain of levels, and where the names of a
//! package are looked up.
//!
//! The header record of a package file may reference other packages with USE-PACKAGE (16.2).
//! Used, a package brings in what it references first, in the order written and each expanded the
//! same way, then itself; within one expansion each package is placed once, at its first place.
//! On a chain of levels one package of a name is in effect: the USE-PACKAGE written in a file
//! nearest to the level, else, for a package that only references bring in, the newest version
//! within the major version of the reference expanded first. The standard collection is placed
//! first at the root when referenced by a header or added by a tool setting (16.3).

use crate::host::language::Languages;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{codes, Extra, Loc, Severity};
use crate::host::{LevelId, LevelKind};

use super::obj::{SceneId, UnitId};
use super::unit::{
    is_alias_package, is_read_version, package_version_cmp, same_package_major, version_key, Binding, DeviceDecl,
    EffPackage, Place, UnitKind, UseDecl,
};
use super::Interp;

/// The standard collection (16.3): the package that the tool setting `include_std` references at
/// the root of the main file.
pub const STANDARD_PACKAGE: &str = "rnl-std-all";

/// The name of the standard collection (16.3), [`STANDARD_PACKAGE`].
pub fn standard_package_name() -> &'static str {
    STANDARD_PACKAGE
}

/// A reference, a USE-PACKAGE in the header record of a package file (16.2): the package and the
/// index of the reference among its references.
pub(crate) type RefKey = (UnitId, usize);

/// What a package brings in when used, by name, and the references that close a cycle in it.
pub(crate) type Expansion = Rc<(Vec<String>, Vec<RefKey>)>;

/// The chain of levels that a set of packages is in effect on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum EffKey {
    /// The chain from the root down to a host level.
    Level(LevelId),
    /// A package read without a level: checked on its own (2.7), or in effect nowhere. The
    /// standard package is the tool's (16.2).
    Alone(UnitId),
}

/// The packages in effect on one chain of levels, one of each name (16.2, 16.3).
#[derive(Debug, Default)]
pub(crate) struct Eff {
    /// In the order they first appear; their places order them in the queues (8.3).
    pub entries: Vec<EffPackage>,
    /// The entry of each package.
    index: HashMap<UnitId, usize>,
    /// By name, the package in effect on the chain, or none when the USE-PACKAGE or reference
    /// that takes effect finds none that can be read.
    pub unit: HashMap<String, Option<UnitId>>,
    /// The references that fail on the chain: those that close a cycle, and those of another
    /// major version than the reference expanded first.
    pub failed: HashSet<RefKey>,
}

impl Eff {
    /// The entry of a package in effect.
    pub(crate) fn entry(&self, u: UnitId) -> Option<&EffPackage> {
        self.index.get(&u).map(|&k| &self.entries[k])
    }
}

/// Where a use places what it brings in.
#[derive(Clone, Copy)]
enum At {
    Root,
    Level(LevelId),
}

/// A USE-PACKAGE written in a file, the standard package that the tool setting adds, or a package
/// checked on its own, in the order of nearness: a later one is nearer.
struct Use {
    decl: Option<UseDecl>,
    unit: Option<UnitId>,
    at: At,
    name: String,
}

/// A package brought in by a reference on a chain of levels.
#[derive(Clone, Copy)]
struct Cand {
    /// The package the reference found.
    unit: UnitId,
    /// The use that brings it in.
    by: usize,
    reference: RefKey,
    /// The first reference on the way from the package of that use.
    first: RefKey,
}

/// The candidates of each name, by the names in the order first met.
#[derive(Default)]
struct Cands {
    names: Vec<String>,
    by: HashMap<String, Vec<Cand>>,
}

impl Cands {
    fn push(&mut self, name: &str, c: Cand) {
        match self.by.get_mut(name) {
            Some(v) => v.push(c),
            None => {
                self.names.push(name.to_string());
                self.by.insert(name.to_string(), vec![c]);
            }
        }
    }
}

/// A `$` name as written, with the name it stands for when written through an alias (16.5).
pub(crate) fn shown(b: &Binding) -> String {
    let name = format!("${}", b.name);
    if b.written == name {
        name
    } else {
        format!("{} ({name})", b.written)
    }
}

impl Interp<'_> {
    fn standard_unit(&self, key: EffKey) -> UnitId {
        match key {
            EffKey::Level(level) => self.level_unit[level],
            EffKey::Alone(unit) => unit,
        }
    }

    fn standard_request_location(&self, unit: UnitId) -> Option<Loc> {
        let writer = self.file_unit(unit).unwrap_or(unit);
        self.units[writer].header_record.and_then(|record| self.rec_loc(Some(record), 0)).or_else(|| {
            let file = self.units[writer]
                .level
                .map(|level| self.doc.levels[level].file.clone())
                .unwrap_or_else(|| self.units[writer].name.clone());
            Some(Loc::line(file, 1))
        })
    }

    /// The standard package that the tool setting references for the files of the language
    /// version `language` (16.3): the newest version in the package directories of the standard
    /// series written in that version, as the collection header declares; or the version that the
    /// reading asks for ([`crate::Options::std_version`]). The reference checks the language of
    /// the package before making its names available. Reading it reads the packages it references.
    pub(crate) fn std_package(&mut self, language: &str) -> Option<UnitId> {
        if !self.std_enabled || !self.include_std {
            return None;
        }
        if let Some(&s) = self.std_units.get(language) {
            return s;
        }
        let standard = STANDARD_PACKAGE;
        self.scan_packages(standard);
        // of the files of one version, the one found is the first in the order of the package
        // directories (16.2): one of a frontend's own directories hides the bundled one
        let mut found: Vec<(String, String, String)> = Vec::new();
        for c in self.pkg_files[standard].iter().filter(|c| c.0 == standard) {
            if !found.iter().any(|f| f.1 == c.1) {
                found.push(c.clone());
            }
        }
        let chosen = if let Some(v) = self.std_version.clone() {
            // Select the requested package version; implicit_std checks its language separately.
            let c = found.iter().find(|c| c.1 == v).cloned();
            if c.is_none() && self.missing_std.is_none() {
                // said on the first chain that would use it ([`Self::tell_missing_std`]): a version
                // that a header record pins, or an inline package of its name, stands in its place
                self.missing_std = Some((v, false));
            }
            c
        } else {
            let written_in = |c: &(String, String, String)| version_key(&self.pkg_languages[&c.2]);
            let want = version_key(language);
            // the series written in the file's language version; without one, the one written in
            // the nearest older version, which the tool reads too, else in the nearest newer one
            let nearest = |older: bool| {
                let side = found.iter().map(written_in).filter(|v| if older { *v < want } else { *v > want });
                if older {
                    side.max()
                } else {
                    side.min()
                }
            };
            let series = [Some(want), nearest(true), nearest(false)].into_iter().flatten().find(|v| {
                // a language version with a standard series in it
                found.iter().any(|c| written_in(c) == *v)
            });
            // of that series the newest version
            let mut best: Option<&(String, String, String)> = None;
            for c in found.iter().filter(|c| Some(written_in(c)) == series) {
                if best.is_none_or(|b| package_version_cmp(&c.1, &b.1).is_gt()) {
                    best = Some(c);
                }
            }
            best.cloned()
        };
        let out = chosen.map(|(n, v, path)| {
            let key = super::unit::package_cache_key(n.clone(), v.clone());
            match self.packages.get(&key) {
                Some(&u) => u,
                None => self.load_package_file(&path, Some((n, v))),
            }
        });
        self.std_units.insert(language.to_string(), out);
        out
    }

    /// The standard package that the tool setting `include_std` references on a chain (16.3): one
    /// reference at the root of the main file, or of the package checked on its own, the one of
    /// the language version of the file of the chain's level; none when the header record of that
    /// root writes a USE-PACKAGE of it, which stands instead, none where the file's version is not
    /// read (16.1), and none for a package read alone that is not read, such as one of a version
    /// not read. A standard package of another language version, or an alias package, is not
    /// used.
    pub(crate) fn implicit_std(&mut self, key: EffKey) -> Option<UnitId> {
        let unit = match key {
            EffKey::Level(level) if !self.file_ok(level) => return None,
            EffKey::Level(level) => self.level_unit[level],
            EffKey::Alone(u) if !self.units[u].ok => return None,
            EffKey::Alone(u) => u,
        };
        // the setting adds one reference at the main file's root: not a separate one for each
        // included file, and none when the main file is not read
        let root = match key {
            EffKey::Level(_) if !self.file_ok(0) => return None,
            EffKey::Level(_) => self.level_unit[0],
            EffKey::Alone(u) if self.package_main == Some(u) => u,
            EffKey::Alone(_) => return None,
        };
        if !self.include_std || self.units[root].uses.iter().any(|decl| decl.in_header && decl.name == STANDARD_PACKAGE)
        {
            return None;
        }
        let language = self.version_of(Some(unit));
        self.std_package(&language)
            .filter(|&p| is_read_version(&self.version_of(Some(p))))
            .filter(|&p| !is_alias_package(&self.units[p]))
    }

    /// A package's version: what its PACKAGE says, or the `version=` of the USE-PACKAGE of an
    /// inline one (16.2).
    pub(crate) fn pkg_ver(&self, u: UnitId) -> String {
        let unit = &self.units[u];
        if unit.embedded {
            unit.version.clone()
        } else {
            unit.pkg_version.clone()
        }
    }

    /// The packages in effect at a host level (8.3, 16.2, 16.3).
    pub(crate) fn effective_packages(&mut self, level: LevelId) -> Rc<Eff> {
        self.eff(EffKey::Level(level))
    }

    /// The packages in effect on a chain, computed once.
    pub(crate) fn eff(&mut self, key: EffKey) -> Rc<Eff> {
        // the standard package of the chain first: reading it reads the packages it references
        self.implicit_std(key);
        if self.effs_units != self.units.len() {
            self.effs.clear();
            self.envs.clear();
            self.std_envs.clear();
            self.effs_units = self.units.len();
        }
        if let Some(e) = self.effs.get(&key) {
            return e.clone();
        }
        let uses = self.uses_on(key);
        let e = Rc::new(self.effective(&uses, Some(STANDARD_PACKAGE)));
        self.effs.insert(key, e.clone());
        e
    }

    /// The uses on a chain, in the order of nearness: the standard package at the root, then
    /// level by level the USE-PACKAGEs of the header record (at the root) and of the level (16.3).
    fn uses_on(&mut self, key: EffKey) -> Vec<Use> {
        let standard = STANDARD_PACKAGE;
        let mut uses = Vec::new();
        let std = self.implicit_std(key);
        if let Some(s) = std {
            uses.push(Use { decl: None, unit: Some(s), at: At::Root, name: standard.into() });
        }
        match key {
            EffKey::Level(level) => {
                let mut chain = self.doc.ancestors(level);
                chain.reverse();
                chain.push(level);
                // the level of a file whose version is not read has no packages in effect (16.1)
                if !self.file_ok(level) {
                    chain.clear();
                }
                for lv in chain {
                    let unit = &self.units[self.level_unit[lv]];
                    if !unit.ok {
                        continue;
                    }
                    for header in [true, false] {
                        for d in unit.uses.iter().filter(|d| d.in_header == header) {
                            let at = if header { At::Root } else { At::Level(lv) };
                            uses.push(Use { decl: Some(d.clone()), unit: d.pkg, at, name: d.name.clone() });
                        }
                    }
                }
                // a version of the standard package that a header record on the chain pins comes
                // first (16.3): it stands in the place of the one of the file's language version,
                // so that the packages at the root are in the order of the series in effect. An
                // inline package of its name, which drops or freezes it, leaves the place as it is
                let pinned =
                    uses.iter().rev().find(|u| u.decl.is_some() && u.name == standard && matches!(u.at, At::Root));
                if let (Some(_), Some(p)) = (std, pinned.and_then(|u| u.unit)) {
                    if !self.units[p].embedded && self.units[p].ok {
                        uses[0].unit = Some(p);
                    }
                }
            }
            EffKey::Alone(u) => {
                let name = self.units[u].pkg_name.clone();
                uses.push(Use { decl: None, unit: Some(u), at: At::Root, name });
            }
        }
        self.tell_missing_std(key, &uses);
        self.tell_std_alias(key, &uses);
        uses
    }

    fn tell_std_alias(&mut self, key: EffKey, uses: &[Use]) {
        let unit = self.standard_unit(key);
        let standard = STANDARD_PACKAGE;
        if uses.iter().any(|u| u.name == standard) {
            return;
        }
        let language = self.version_of(Some(unit));
        let Some(package) = self.std_units.get(&language).copied().flatten() else { return };
        if !self.units[package].ok
            || !is_read_version(&self.version_of(Some(package)))
            || !is_alias_package(&self.units[package])
        {
            return;
        }
        // The setting adds one root reference, so all included levels share its failure.
        let requester = match key {
            EffKey::Level(_) => self.level_unit[0],
            EffKey::Alone(u) if self.package_main == Some(u) => u,
            _ => return,
        };
        if !self.std_alias_reported.insert(requester) {
            return;
        }
        let version = self.pkg_ver(package);
        let loc = self.standard_request_location(requester);
        let context = self.units[requester].pkg().to_string();
        self.diag_in(Severity::Error, codes::USE_PACKAGE_ALIAS_PACKAGE,
            format!("the standard package {standard} {version} is an alias package; the standard-package setting loads ordinary packages with USE-PACKAGE (16.3, 16.5)"),
            loc, &context);
    }

    /// Say that the version of the standard package that the reading asks for cannot be found,
    /// once, on the first chain that would use it (16.3): one whose files are read, or the package
    /// checked on its own, where no use names the standard package but the one the setting adds. On a
    /// chain where a header record pins a version of it, or an inline package of its name drops or
    /// freezes it, that use stands in its place, and the version asked for is not needed; nor is it
    /// for a package in effect nowhere.
    fn tell_missing_std(&mut self, key: EffKey, uses: &[Use]) {
        let standard = STANDARD_PACKAGE;
        let Some((v, false)) = self.missing_std.clone() else { return };
        let read = match key {
            EffKey::Level(level) => self.file_ok(level),
            EffKey::Alone(u) => self.package_main == Some(u) && self.units[u].ok,
        };
        if !read || uses.iter().any(|u| u.name == standard) {
            return;
        }
        self.missing_std = Some((v.clone(), true));
        let message = format!("the standard package {standard} {v} that the reading asks for cannot be found, in the package directories or among the bundled packages: the files that do not pin a version of it are read without a standard package (16.3)");
        let unit = self.standard_unit(key);
        let loc = self.standard_request_location(unit);
        let package = self.units[unit].pkg().to_string();
        self.diag_in(Severity::Error, codes::STD_VERSION_NOT_FOUND, message, loc, &package);
    }

    /// What a package brings in when used, as its references are written: the packages it
    /// references first, in the order written, each expanded the same way, then the package
    /// itself; each name once, at its first place (16.2). A reference that closes a cycle is an
    /// error and fails, reported once for each package an expansion starts from.
    pub(crate) fn expansion(&mut self, name: &str, unit: Option<UnitId>) -> Expansion {
        let Some(u) = unit else { return Rc::new((vec![name.to_string()], Vec::new())) };
        if let Some(x) = self.expansions.get(&u) {
            return x.clone();
        }
        let (mut out, mut cycles) = (Vec::new(), Vec::new());
        if self.units[u].ok {
            self.expand_refs(u, &mut vec![name.to_string()], &mut out, &mut cycles);
        }
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
        let x = Rc::new((out, cycles));
        self.expansions.insert(u, x.clone());
        x
    }

    fn expand_refs(&mut self, u: UnitId, path: &mut Vec<String>, out: &mut Vec<String>, cycles: &mut Vec<RefKey>) {
        for k in 0..self.units[u].refs.len() {
            let r = self.units[u].refs[k].clone();
            if let Some(at) = path.iter().position(|n| *n == r.name) {
                let ring: Vec<&str> = path[at..].iter().map(String::as_str).chain([r.name.as_str()]).collect();
                let pkg = self.units[u].pkg_name.clone();
                let loc = self.rec_loc(r.rec, r.offset);
                self.diag_in(
                    Severity::Error,
                    codes::PACKAGE_REFERENCE_CYCLE,
                    format!(
                        "packages reference each other in a cycle, {}: this reference fails (16.2)",
                        ring.join(" -> ")
                    ),
                    loc,
                    &pkg,
                );
                cycles.push((u, k));
                continue;
            }
            let Some(p) = r.pkg else { continue }; // not found: reported where it is read
            if self.units[p].ok {
                path.push(r.name.clone());
                self.expand_refs(p, path, out, cycles);
                path.pop();
            }
            if !out.contains(&r.name) {
                out.push(r.name.clone());
            }
        }
    }

    /// One package of each name on a chain (16.2, "Which USE-PACKAGE takes effect" and "Packages
    /// that reference packages"). A name written in a file takes the nearest USE-PACKAGE; a name
    /// that only references bring in takes the newest version within the major version of the
    /// reference expanded first, and references of another major version fail. What a package
    /// brings in is expanded from the packages in effect only, so a package that is replaced or
    /// dropped brings nothing in.
    fn effective(&mut self, uses: &[Use], standard_first: Option<&str>) -> Eff {
        let mut eff = Eff::default();
        // the root keeps the first place a name gets there (16.3): (k, j), k counting the uses at
        // the root that bring in names not placed yet, j the place of the name in what one brings in
        let mut root_place: HashMap<String, (usize, usize)> = HashMap::new();
        let full: Vec<Expansion> = uses.iter().map(|u| self.expansion(&u.name, u.unit)).collect();
        let first =
            standard_first.and_then(|name| uses.iter().rposition(|u| u.name == name && matches!(u.at, At::Root)));
        // Give the standard its root position without reordering the references themselves:
        // the nearest file/level still decides which version or inline content is used (16.3).
        let roots = first.into_iter().chain((0..uses.len()).filter(|i| Some(*i) != first));
        let mut k = 0;
        for i in roots {
            let x = &full[i];
            if matches!(uses[i].at, At::Root) && x.0.iter().any(|n| !root_place.contains_key(n)) {
                for (j, n) in x.0.iter().enumerate() {
                    root_place.entry(n.clone()).or_insert((k, j));
                }
                k += 1;
            }
        }
        let mut direct: HashMap<&str, usize> = HashMap::new();
        for (i, u) in uses.iter().enumerate() {
            direct.insert(&u.name, i); // the nearest one
        }
        let tops: Vec<usize> = (0..uses.len())
            .filter(|&i| direct[uses[i].name.as_str()] == i && uses[i].unit.is_some_and(|u| self.units[u].ok))
            .collect();
        for &i in &tops {
            eff.failed.extend(full[i].1.iter().copied());
        }
        // the version of each name brought in by references alone, until it stays the same: only
        // the references of the version chosen are followed
        let mut chosen: HashMap<String, UnitId> = HashMap::new();
        let mut cands = Cands::default();
        let mut seen: HashMap<usize, Vec<String>> = HashMap::new();
        for _ in 0..8 {
            cands = Cands::default();
            seen.clear();
            for &i in &tops {
                let mut path = vec![uses[i].name.clone()];
                let start = uses[i].unit.unwrap();
                let walk = Walk { by: i, direct: &direct, chosen: &chosen, failed: &eff.failed };
                self.walk_cands(start, None, &mut path, &walk, &mut cands, &mut seen);
                seen.entry(i).or_default().push(uses[i].name.clone());
            }
            let mut next: HashMap<String, UnitId> = HashMap::new();
            for n in &cands.names {
                if !direct.contains_key(n.as_str()) {
                    next.insert(n.clone(), self.newest(&cands.by[n]));
                }
            }
            if next == chosen {
                break;
            }
            chosen = next;
        }
        self.version_diagnostics(uses, &direct, &chosen, &cands, &mut eff);
        // what is in effect, in the order first met
        let mut order: Vec<&String> = Vec::new();
        for (i, x) in full.iter().enumerate() {
            for n in x.0.iter().chain(seen.get(&i).into_iter().flatten()) {
                if !order.contains(&n) {
                    order.push(n);
                }
            }
        }
        for n in order {
            let (i, u) = if let Some(&i) = direct.get(n.as_str()) {
                (i, uses[i].unit)
            } else if let Some(&u) = chosen.get(n) {
                // placed at the nearest use that brings in a version of the major version chosen
                let cs = &cands.by[n];
                let first = self.pkg_ver(cs[0].unit);
                let i = cs
                    .iter()
                    .filter(|c| same_package_major(&self.pkg_ver(c.unit), &first))
                    .map(|c| c.by)
                    .max()
                    .unwrap();
                (i, Some(u))
            } else {
                continue; // brought in only by a package that is not in effect
            };
            let u = u.filter(|&u| self.units[u].ok);
            eff.unit.insert(n.clone(), u);
            let Some(u) = u else { continue };
            let place = match uses[i].at {
                At::Root => {
                    // a name only a newer version than the references name brings in comes last
                    let next = (k, root_place.len());
                    let (k0, j) = *root_place.entry(n.clone()).or_insert(next);
                    Place::Root(k0, j)
                }
                At::Level(lv) => {
                    let walked = seen.get(&i).map(Vec::as_slice).unwrap_or(&[]);
                    let j = walked.iter().position(|w| w == n).unwrap_or(walked.len());
                    Place::Level(lv, uses[i].decl.as_ref().map_or((0, 0), |d| d.pos), j)
                }
            };
            eff.index.insert(u, eff.entries.len());
            eff.entries.push(EffPackage {
                decl: uses[i].decl.clone(),
                unit: u,
                place,
                languages: Languages::NONE,
                solver_config: false,
            });
        }
        // Propagate each effective use's section mask through the effective reference graph.
        // A package still binds its names once; only its object rules and DEVICE are restricted.
        for &i in &tops {
            if let Some(unit) = uses[i].unit {
                if let Some(&index) = eff.index.get(&unit) {
                    let language = uses[i].decl.as_ref().map_or(Languages::ALL, |decl| decl.languages);
                    eff.entries[index].languages = eff.entries[index].languages.union(language);
                    eff.entries[index].solver_config |= uses[i].decl.as_ref().is_none_or(|decl| decl.solver_config);
                }
            }
        }
        loop {
            let mut changed = false;
            for index in 0..eff.entries.len() {
                let unit = eff.entries[index].unit;
                let language = eff.entries[index].languages;
                for (reference, decl) in self.units[unit].refs.iter().enumerate() {
                    if decl.pkg.is_none() || eff.failed.contains(&(unit, reference)) {
                        continue;
                    }
                    let Some(Some(target)) = eff.unit.get(&decl.name) else { continue };
                    let Some(&target) = eff.index.get(target) else { continue };
                    let config = eff.entries[index].solver_config && decl.solver_config;
                    if config && !eff.entries[target].solver_config {
                        eff.entries[target].solver_config = true;
                        changed = true;
                    }
                    let old = eff.entries[target].languages;
                    let new = old.union(language.intersect(decl.languages));
                    if old != new {
                        eff.entries[target].languages = new;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        eff
    }

    /// The newest version within the major version of the first candidate; of equal versions,
    /// the first.
    fn newest(&self, cs: &[Cand]) -> UnitId {
        let first = self.pkg_ver(cs[0].unit);
        let mut best = cs[0].unit;
        for c in cs.iter().filter(|c| same_package_major(&self.pkg_ver(c.unit), &first)) {
            if package_version_cmp(&self.pkg_ver(c.unit), &self.pkg_ver(best)).is_gt() {
                best = c.unit;
            }
        }
        best
    }

    /// The references a use brings in, depth first: each a candidate for its name, and the
    /// names in the order of the use's expansion (16.2).
    fn walk_cands(
        &self,
        u: UnitId,
        first: Option<RefKey>,
        path: &mut Vec<String>,
        walk: &Walk,
        cands: &mut Cands,
        seen: &mut HashMap<usize, Vec<String>>,
    ) {
        for (k, r) in self.units[u].refs.iter().enumerate() {
            if path.contains(&r.name) || walk.failed.contains(&(u, k)) {
                continue;
            }
            let Some(p) = r.pkg else { continue };
            let first = first.unwrap_or((u, k));
            cands.push(&r.name, Cand { unit: p, by: walk.by, reference: (u, k), first });
            if walk.direct.contains_key(r.name.as_str()) {
                continue; // written in a file: placed and expanded where it is written
            }
            if walk.chosen.get(&r.name).is_none_or(|&c| c == p) && self.units[p].ok {
                path.push(r.name.clone());
                self.walk_cands(p, Some(first), path, walk, cands, seen);
                path.pop();
            }
            let order = seen.entry(walk.by).or_default();
            if !order.contains(&r.name) {
                order.push(r.name.clone());
            }
        }
    }

    /// The diagnostics of the versions references ask for (16.2, "Where diagnostics are
    /// reported"): the hint about different versions and the warning about another major version
    /// at the USE-PACKAGE written in the file, the error of a failing reference at the reference.
    fn version_diagnostics(
        &mut self,
        uses: &[Use],
        direct: &HashMap<&str, usize>,
        chosen: &HashMap<String, UnitId>,
        cands: &Cands,
        eff: &mut Eff,
    ) {
        for n in &cands.names {
            let cs = &cands.by[n];
            if let Some(&i) = direct.get(n.as_str()) {
                let Some(w) = uses[i].unit else { continue };
                let wv = self.pkg_ver(w);
                for c in cs {
                    let cv = self.pkg_ver(c.unit);
                    if wv.is_empty() || cv.is_empty() || same_package_major(&cv, &wv) {
                        continue;
                    }
                    let who = self.units[c.reference.0].pkg_name.clone();
                    let (loc, pkg) = self.use_loc(uses[i].decl.as_ref(), c);
                    self.diag_in(
                        Severity::Warning,
                        codes::PACKAGE_REFERENCE_OTHER_MAJOR_AT_USE, format!("package {n} is {wv} here, and package {who} references {cv}: another major version, so {who} may use names that {n} {wv} does not have (16.2)"),
                        loc,
                        &pkg,
                    );
                }
                continue;
            }
            let first = cs[0];
            let (fv, fwho) = (self.pkg_ver(first.unit), self.units[first.reference.0].pkg_name.clone());
            let win = self.pkg_ver(chosen[n]);
            for c in cs {
                let cv = self.pkg_ver(c.unit);
                let who = self.units[c.reference.0].pkg_name.clone();
                if !same_package_major(&cv, &fv) {
                    eff.failed.insert(c.reference);
                    let r = &self.units[c.reference.0].refs[c.reference.1];
                    let loc = self.rec_loc(r.rec, r.offset);
                    let origin = self.use_origin(uses[c.by].decl.as_ref());
                    self.sink.add(
                        Severity::Error,
                        codes::PACKAGE_REFERENCE_OTHER_MAJOR_FAILS, format!("package {who} references package {n} {cv}, of another major version than the reference expanded first ({fwho} references {fv}): this reference fails, and the statements of {who} that use its names fail with it (16.2)"),
                        loc,
                        Extra { cause: None, origin, in_package: who },
                    );
                } else if c.unit != chosen[n] {
                    let (loc, pkg) = self.use_loc(uses[c.by].decl.as_ref(), c);
                    self.diag_in(
                        Severity::Hint,
                        codes::PACKAGE_REFERENCE_OLDER_VERSION, format!("package {who} references package {n} {cv}; of the references to it, the newest within that major version is {win}, which is used (16.2)"),
                        loc,
                        &pkg,
                    );
                }
            }
        }
    }

    /// Where a choice among references is reported: at the USE-PACKAGE written in a file that
    /// brings the package in; for the standard package that the tool setting adds and a package
    /// checked on its own, at the first reference on the way, in its package (16.2, 7.6).
    fn use_loc(&self, top: Option<&UseDecl>, c: &Cand) -> (Option<Loc>, String) {
        if let Some(d) = top.filter(|d| d.rec.is_some()) {
            return (self.rec_loc(d.rec, d.offset), self.units[d.unit].pkg().to_string());
        }
        let (u, k) = c.first;
        let r = &self.units[u].refs[k];
        (self.rec_loc(r.rec, r.offset), self.units[u].pkg_name.clone())
    }

    /// Which USE-PACKAGE in a file brought a reference in (16.2).
    fn use_origin(&self, top: Option<&UseDecl>) -> String {
        match top.and_then(|d| self.rec_loc(d.rec, d.offset).map(|l| (d, l))) {
            Some((d, l)) => format!("brought in by the USE-PACKAGE of {} ({l})", d.name),
            None => String::new(),
        }
    }

    /// What a package's references bring in, (name, package), the referenced packages' own
    /// first, each name once: the package of the name in effect on the chain, else the one the
    /// reference found; none for one that cannot be read (16.2). References that fail on the
    /// chain bring nothing.
    fn walk_refs(&self, u: UnitId, eff: &Eff, path: &mut Vec<String>, out: &mut Vec<(String, Option<UnitId>)>) {
        for (k, r) in self.units[u].refs.iter().enumerate() {
            if eff.failed.contains(&(u, k)) || path.contains(&r.name) || r.pkg.is_none() {
                continue;
            }
            let t = match eff.unit.get(&r.name) {
                Some(t) => *t,
                None => r.pkg,
            };
            if let Some(t) = t.filter(|&t| self.units[t].ok) {
                path.push(r.name.clone());
                self.walk_refs(t, eff, path, out);
                path.pop();
            }
            if !out.iter().any(|(n, _)| *n == r.name) {
                out.push((r.name.clone(), t));
            }
        }
    }

    /// The standard package in effect on a chain and every package it references, directly or
    /// not (16.2): their names, and the packages, the later first.
    pub(crate) fn std_env(&mut self, key: EffKey) -> Rc<(HashSet<String>, Vec<UnitId>)> {
        let standard = STANDARD_PACKAGE;
        let eff = self.eff(key);
        if let Some(x) = self.std_envs.get(&key) {
            return x.clone();
        }
        let mut items = Vec::new();
        if let Some(s) = eff.unit.get(standard).copied().flatten() {
            self.walk_refs(s, &eff, &mut vec![standard.to_string()], &mut items);
            items.push((standard.to_string(), Some(s)));
        }
        let names = items.iter().map(|(n, _)| n.clone()).collect();
        let units = items.iter().rev().filter_map(|(_, u)| *u).filter(|&u| self.units[u].ok).collect();
        let x = Rc::new((names, units));
        self.std_envs.insert(key, x.clone());
        x
    }

    /// Where the `$` names of a package are looked up after its own (16.2, "Names in a package"):
    /// the packages it references, the later first and each before what it references, then the
    /// standard package in effect on the chain. The standard package and the packages it
    /// references see only themselves and the packages they reference.
    fn pkg_env(&mut self, u: UnitId, key: EffKey) -> Rc<Vec<UnitId>> {
        let eff = self.eff(key);
        if let Some(x) = self.envs.get(&(key, u)) {
            return x.clone();
        }
        let name = self.units[u].pkg_name.clone();
        let mut items = Vec::new();
        self.walk_refs(u, &eff, &mut vec![name.clone()], &mut items);
        let mut out: Vec<UnitId> =
            items.iter().rev().filter_map(|(_, p)| *p).filter(|&p| p != u && self.units[p].ok).collect();
        let std = self.std_env(key);
        if !std.0.contains(&name) {
            for &s in &std.1 {
                if s != u && !out.contains(&s) {
                    out.push(s);
                }
            }
        }
        let out = Rc::new(out);
        self.envs.insert((key, u), out.clone());
        out
    }

    /// The chain a package's names are looked up on: that of the level drawn, else the first
    /// chain the package is in effect on, or the package alone (16.2). A package's definitions
    /// are read once, so a package in effect on chains with different packages reads them as on
    /// the first.
    pub(crate) fn name_chain(&self, u: UnitId, scene: Option<SceneId>) -> EffKey {
        match scene {
            Some(s) => EffKey::Level(self.scenes[s].level),
            None => self.home.get(&u).copied().unwrap_or(EffKey::Alone(u)),
        }
    }

    /// A `$` name of a package that it does not define itself (16.2).
    pub(crate) fn package_lookup(&mut self, name: &str, u: UnitId, scene: Option<SceneId>) -> Option<Binding> {
        let chain = self.name_chain(u, scene);
        let env = self.pkg_env(u, chain);
        let key = ('$', name.to_string());
        env.iter().find_map(|&p| self.units[p].names.get(&key).cloned())
    }

    /// Note the chain each package in effect on `key` is read on, unless it has one already.
    pub(crate) fn set_home(&mut self, key: EffKey) {
        let eff = self.eff(key);
        for e in &eff.entries {
            self.home.entry(e.unit).or_insert(key);
        }
    }

    /// A `$` name that a package does not find, which a package one of its references found
    /// has: that package is not in effect here, dropped, replaced, or its reference failed. Its
    /// name (16.2).
    pub(crate) fn missing_from_reference(&self, name: &str, u: UnitId) -> Option<String> {
        if self.units[u].kind != UnitKind::Package {
            return None;
        }
        let key = ('$', name.to_string());
        self.find_in_refs(u, &key, &mut vec![self.units[u].pkg_name.clone()], &mut HashSet::new())
    }

    fn find_in_refs(
        &self,
        u: UnitId,
        key: &(char, String),
        path: &mut Vec<String>,
        seen: &mut HashSet<UnitId>,
    ) -> Option<String> {
        for r in &self.units[u].refs {
            let Some(p) = r.pkg else { continue };
            if path.contains(&r.name) || !seen.insert(p) {
                continue;
            }
            if self.units[p].names.contains_key(key) {
                return Some(r.name.clone());
            }
            path.push(r.name.clone());
            let hit = self.find_in_refs(p, key, path, seen);
            path.pop();
            if hit.is_some() {
                return hit;
            }
        }
        None
    }

    /// In a package file, referenced packages that define the same `$X`: the later reference hides
    /// the earlier, and a definition of the package itself hides them all. A hint in the package,
    /// at the later reference or at its own definition (16.2), shown when the package is checked
    /// on its own (7.6).
    pub(crate) fn check_package_refs(&mut self, u: UnitId) {
        if !self.units[u].ok || self.units[u].refs.is_empty() || !self.refs_checked.insert(u) {
            return;
        }
        let chain = self.name_chain(u, None);
        let eff = self.eff(chain);
        let pkg = self.units[u].pkg_name.clone();
        // each name: the package that defines it, and the reference of this package that brings it
        let mut before: HashMap<String, (UnitId, usize)> = HashMap::new();
        for (k, r) in self.units[u].refs.clone().into_iter().enumerate() {
            if eff.failed.contains(&(u, k)) || r.name == pkg {
                continue;
            }
            let Some(found) = r.pkg else { continue };
            let t = match eff.unit.get(&r.name) {
                Some(t) => *t,
                None => Some(found),
            };
            let Some(t) = t.filter(|&t| self.units[t].ok) else { continue };
            let mut items = Vec::new();
            self.walk_refs(t, &eff, &mut vec![pkg.clone(), r.name.clone()], &mut items);
            items.push((r.name.clone(), Some(t)));
            let brought: Vec<UnitId> = items.into_iter().filter_map(|(_, p)| p).filter(|&p| self.units[p].ok).collect();
            for p in brought {
                for key in self.units[p].name_order.clone() {
                    if key.0 != '$' {
                        continue;
                    }
                    let Some((q, by)) = before.insert(key.1.clone(), (p, k)) else { continue };
                    if by == k || q == p {
                        continue; // what one reference brings in is the business of that package
                    }
                    let written = shown(&self.units[p].names[&key]);
                    let (pn, qn) = (self.units[p].pkg_name.clone(), self.units[q].pkg_name.clone());
                    let loc = self.rec_loc(r.rec, r.offset);
                    self.diag_in(
                        Severity::Hint,
                        codes::PACKAGE_REFERENCE_HIDES, format!("{written} in package {pn} hides the definition of the same name in package {qn}, referenced before it; package {pkg} uses the one of {pn} (16.2)"),
                        loc,
                        &pkg,
                    );
                }
            }
        }
        for key in self.units[u].name_order.clone() {
            if key.0 != '$' {
                continue;
            }
            let Some(&(q, _)) = before.get(&key.1) else { continue };
            let b = self.units[u].names[&key].clone();
            let qn = self.units[q].pkg_name.clone();
            let loc = self.rec_loc(b.rec, b.off);
            self.diag_in(
                Severity::Hint,
                codes::PACKAGE_OWN_HIDES_REFERENCE, format!("{} hides the definition of the same name in the referenced package {qn}; package {pkg} uses this one (16.2)", shown(&b)),
                loc,
                &pkg,
            );
        }
    }

    /// DEVICE entries in effect at a level, those of the packages in the order they are brought
    /// in; later ones win (6.12).
    pub(crate) fn devices_for(&mut self, level: LevelId) -> Vec<DeviceDecl> {
        let mut out = Vec::new();
        let eff = self.effective_packages(level);
        for e in &eff.entries {
            out.extend(self.units[e.unit].devices.clone());
        }
        let mut chain = self.doc.ancestors(level);
        chain.reverse();
        chain.push(level);
        for lv in chain {
            if self.doc.levels[lv].kind == LevelKind::File {
                out.extend(self.units[self.level_unit[lv]].devices.clone());
            }
        }
        out
    }
}

/// What a walk over the references of one use keeps fixed.
struct Walk<'a> {
    by: usize,
    direct: &'a HashMap<&'a str, usize>,
    chosen: &'a HashMap<String, UnitId>,
    failed: &'a HashSet<RefKey>,
}
