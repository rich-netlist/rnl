//! The host circuit as host adapters hand it to the runtime (standard 2.1, 2.6, 6.1, 6.3).
//!
//! The model is dialect-neutral: adapters for SPICE, Spectre or another host fill it in, and the
//! runtime never parses netlist syntax itself. Levels follow the text: the main file level is the
//! top circuit, every included file has a file level, every subcircuit definition is a level.
//!
//! Each file is read by the adapter of its format ([`format`]), so the files of one document can
//! be of several dialects; names compare by the rules of the file they are written in.

pub mod format;
pub mod package;
pub mod text;

use std::collections::{HashMap, HashSet};

use crate::diag::Loc;

pub type LevelId = usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelKind {
    /// The main file level or an included file's level.
    File,
    /// A `.subckt … .ends`.
    Subckt,
    /// A package file (2.7); not part of the document's level tree.
    Package,
}

/// One element of the netlist.
#[derive(Clone, Debug, Default)]
pub struct Instance {
    pub name: String,
    /// The SPICE element letter, upper case; None in dialects whose names carry no letter.
    pub letter: Option<char>,
    pub nodes: Vec<String>,
    /// The name in the model position: a `.model` name, a subcircuit name, or a Spectre master (10.1, TR1).
    pub model: Option<String>,
    /// The value written on the line, raw text (13.6); for Spectre the principal parameter.
    pub value: Option<String>,
    /// Parameters as written, `name=value`.
    pub params: Vec<(String, String)>,
    pub file: String,
    pub line: u32,
    /// The file level that read the line of the instance: the include, or the library section,
    /// through which it was read (2.8, 6.1). A file, or one section of it, included twice is read
    /// twice, by two file levels, each of its own version; None for an instance no adapter read.
    pub file_level: Option<LevelId>,
    /// Netlist order within the drawing level (11.6).
    pub order: usize,
    /// The model type, lower case, such as "nmos" (10.1).
    pub mtype: Option<String>,
    /// An instance of a subcircuit, defined in the netlist or not (a PDK device).
    pub is_subckt: bool,
    /// The adapter's default terminal names (its standard DEVICE table, 6.12).
    pub default_terminals: Option<Vec<String>>,
    /// A device whose model card tells its polarity (M, Q, J and Z of SPICE) and that names a model
    /// the netlist has no card of, or none: nothing can tell which symbol to draw it with, so unless
    /// a rule or a statement of the file gives it one, it is not drawn (9.2.6).
    pub model_missing: bool,
    /// Names compare by case (Spectre) or not (SPICE).
    pub case_sensitive: bool,
}

impl Instance {
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| if self.case_sensitive { k == name } else { k.eq_ignore_ascii_case(name) })
            .map(|(_, v)| v.as_str())
    }
}

/// Where a run of record text came from: from character `off` on, the text is in `file` at
/// `line`, starting after column `col` (0-based column of the text's first character).
#[derive(Clone, Debug, PartialEq)]
pub struct Seg {
    pub off: usize,
    pub file: String,
    pub line: u32,
    pub col: i64,
}

/// One `@RNL(…) {…}` record cut out of the carrier lines (2.2–2.4).
#[derive(Clone, Debug)]
pub struct Record {
    /// From `@RNL` to the matching `}`; carrier prefixes removed, lines joined by "\n".
    pub text: String,
    /// In the order of the text, each starting after the one before.
    pub segs: Vec<Seg>,
    /// Lines of the host file the record occupies (first, last), for writeback.
    pub lines: (u32, u32),
    pub file: String,
}

impl Record {
    /// The location of a character offset in the record text.
    pub fn loc(&self, offset: usize) -> Loc {
        // the last segment that starts at or before the offset, else the first
        let seg = &self.segs[self.segs.partition_point(|s| s.off <= offset).saturating_sub(1)];
        let col = seg.col + (offset as i64 - seg.off as i64) + 1;
        Loc::new(seg.file.clone(), seg.line, col.max(1) as u32)
    }
}

#[derive(Clone, Debug)]
pub struct Level {
    /// The explicit include directive that loaded this file level; absent for auto-includes.
    pub include_site: Option<Loc>,
    pub kind: LevelKind,
    pub name: String,
    /// The file the level's text is in.
    pub file: String,
    pub parent: Option<LevelId>,
    pub children: Vec<LevelId>,
    /// Only the main file level and the subcircuits have a circuit (6.1).
    pub circuit: bool,
    pub ports: Vec<String>,
    pub instances: Vec<Instance>,
    pub records: Vec<Record>,
    pub line: u32,
    /// The line before which text added to the end of this level goes: the `.ends` of a
    /// subcircuit, the `.end` of a main file, or one past the last line of a file (17).
    pub end_line: u32,
    /// The file path, for `path=` relative to the writing file (16.2, 16.5).
    pub path: String,
    pub case_sensitive: bool,
    /// For the level of an included file read through a library section (`.lib file sec`, or
    /// Spectre's `include "file" section=sec`), the section: the reading reads only the text of
    /// that section, and what lies outside it is not read (2.2).
    pub section: Option<String>,
    /// For the level of a library section, the lines of its file that open and close the section
    /// (`.lib sec` and `.endl`, Spectre's `section sec` and `endsection`), the second one past the
    /// last line of the file when nothing closes it. The section is read as a file of its own (2.8):
    /// two sections of one file are two file levels, each with its header record and its version.
    pub section_lines: Option<(u32, u32)>,
}

impl Level {
    pub fn new(kind: LevelKind, name: impl Into<String>, file: impl Into<String>, path: impl Into<String>) -> Level {
        Level {
            include_site: None,
            kind,
            name: name.into(),
            file: file.into(),
            parent: None,
            children: Vec::new(),
            circuit: false,
            ports: Vec::new(),
            instances: Vec::new(),
            records: Vec::new(),
            line: 0,
            end_line: 0,

            path: path.into(),
            case_sensitive: false,
            section: None,
            section_lines: None,
        }
    }

    pub fn title(&self) -> String {
        match self.kind {
            LevelKind::Subckt => self.name.clone(),
            _ => file_name(&self.file).to_string(),
        }
    }

    /// Close the library section the level reads at the line `line`, when it is open
    /// ([`Level::section_lines`]).
    pub fn close_section(&mut self, line: u32) {
        if let Some((_, end @ 0)) = self.section_lines.as_mut() {
            *end = line;
        }
    }

    pub fn same_name(&self, a: &str, b: &str) -> bool {
        if self.case_sensitive {
            a == b
        } else {
            a.eq_ignore_ascii_case(b)
        }
    }

    /// The index of the instance named `name`, the first if there are several. It looks through
    /// every instance; a reader that checks each new name keeps their [`Level::name_key`]s instead.
    pub fn inst(&self, name: &str) -> Option<usize> {
        self.instances.iter().position(|i| self.same_name(&i.name, name))
    }

    /// The key a name is compared by in this level: as written, or with its ASCII letters in lower
    /// case. Two names are the same by [`Level::same_name`] exactly when their keys are equal.
    pub fn name_key(&self, name: &str) -> String {
        if self.case_sensitive {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        }
    }
}

/// The dialect rules the runtime needs (3.6, 6.3, 6.12).
#[derive(Clone, Debug)]
pub struct Dialect {
    pub name: String,
    /// Names of instances, nets and subcircuits compare by case.
    pub case_sensitive: bool,
    /// Names merged into the global ground `0`, compared by the case rule (6.3).
    pub ground_aliases: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct HostDocument {
    /// levels[0] is the main file level.
    pub levels: Vec<Level>,
    /// The dialect of the main file's format.
    pub dialect: Dialect,
    /// The dialect of each file read, by its name in the document, when its format is not that of
    /// the main file: the names it writes compare by its rules ([`HostDocument::rules`]).
    pub file_dialects: HashMap<String, Dialect>,
    /// The format each file was read with, by its name in the document, in the order read
    /// ([`format::Reading`]); none in a document that no reading made.
    pub file_formats: Vec<FileFormat>,
    /// Subcircuit key -> level, for the subcircuits written in files of the main file's dialect.
    pub subckts: HashMap<String, LevelId>,
    /// The subcircuits written in files of another dialect, in the order read;
    /// [`HostDocument::add_subckt`] keeps the last of each name as their dialect compares names.
    pub foreign_subckts: Vec<LevelId>,
    /// Global net keys (`.global`, Spectre `global`), with `0` implied, of the files of the main
    /// file's dialect.
    pub globals: HashSet<String>,
    /// The global nets declared in files of another dialect: the key by that dialect, and whether
    /// it compares names by case.
    pub foreign_globals: Vec<(String, bool)>,
    /// File name -> text, for display and writeback.
    pub sources: Vec<(String, String)>,
    /// The regions written to be encrypted, SPICE's `.prot … .unprot` and Spectre's `protect …
    /// unprotect`: the file, and the lines of the statements that open and close each; one left
    /// open closes after the last line of the file. Records there are read, with a warning, but
    /// they are gone once the region is encrypted, so tools do not write there (2.2).
    pub protected: Vec<(String, u32, u32)>,
    /// Host regions where new records must not be written (2.2), classified by the adapters:
    /// control/foreign-language blocks, preprocessing regions and host title lines. Each span
    /// excludes insertion after its opening line through its closing line (inclusive); an
    /// unterminated region closes one past EOF. Existing RNL in known active conditional
    /// branches can be read, but new records go outside the conditional.
    pub excluded: Vec<(String, u32, u32)>,
    /// Unselected library sections for a particular reading of a file. A whole-file reading
    /// skips these spans, but another file level may explicitly select the same section.
    /// The level is the reading's file level, not the subcircuit currently being parsed.
    pub unselected_sections: Vec<(LevelId, u32, u32)>,
    /// The regions that a reading in Spectre's dialect reads in SPICE mode, where `//` is no
    /// comment and only `*` carries (2.2): the file, the line after which the mode holds, that of
    /// the `simulator lang=spice` or of the `.lib` section that switches to it, 0 in a file that
    /// starts in it, and the line of the `simulator lang=spectre` that ends it, one past the last
    /// line of the file when none does. The rest of a file of such a reading is in Spectre mode.
    pub spice_mode: Vec<(String, u32, u32)>,
}

impl HostDocument {
    pub fn new(main: Level, dialect: Dialect) -> HostDocument {
        HostDocument {
            levels: vec![main],
            dialect,
            file_dialects: HashMap::new(),
            file_formats: Vec::new(),
            subckts: HashMap::new(),
            foreign_subckts: Vec::new(),
            globals: HashSet::new(),
            foreign_globals: Vec::new(),
            sources: Vec::new(),
            protected: Vec::new(),
            excluded: Vec::new(),
            unselected_sections: Vec::new(),
            spice_mode: Vec::new(),
        }
    }

    pub fn main(&self) -> &Level {
        &self.levels[0]
    }

    /// The region written to be encrypted ([`HostDocument::protected`]) that holds the line `line`
    /// of `file`, or what is written in before that line, as the lines that open and close it.
    pub fn protected_at(&self, file: &str, line: u32) -> Option<(u32, u32)> {
        self.protected.iter().find(|(f, a, b)| f == file && *a < line && line <= *b).map(|(_, a, b)| (*a, *b))
    }

    /// A region where inserting RNL before `line` is unsafe: a plaintext protection region or
    /// one of the adapter's excluded host regions. Overlapping regions return the outer start.
    pub fn unwritable_at(&self, file: &str, line: u32) -> Option<(u32, u32)> {
        self.protected
            .iter()
            .chain(&self.excluded)
            .filter(|(f, a, b)| f == file && *a < line && line <= *b)
            .map(|(_, a, b)| (*a, *b))
            .min_by_key(|(a, _)| *a)
    }

    /// An unsafe insertion position for the reading containing `level`. Script, conditional
    /// and protection spans always apply; unselected library spans apply only to that reading.
    pub fn unwritable_in(&self, level: LevelId, line: u32) -> Option<(u32, u32)> {
        let file_level = self.file_level(level);
        self.unwritable_at(&self.levels[file_level].file, line)
            .into_iter()
            .chain(
                self.unselected_sections
                    .iter()
                    .filter(|(l, a, b)| *l == file_level && *a < line && line <= *b)
                    .map(|(_, a, b)| (*a, *b)),
            )
            .min_by_key(|(a, _)| *a)
    }

    /// Whether the line `line` of `file`, or what is written in before that line, is read in SPICE
    /// mode, where only `*` is a comment (2.2): in a file of Spectre's dialect, in the regions of
    /// [`HostDocument::spice_mode`]; in a file of another dialect, everywhere.
    pub fn in_spice_mode(&self, file: &str, line: u32) -> bool {
        self.rules(file).name != "spectre"
            || self.spice_mode.iter().any(|(f, a, b)| f == file && *a < line && line <= *b)
    }

    /// The dialect of the file `file`, by its name in the document: that of its format, the main
    /// file's unless the file is of another.
    pub fn rules(&self, file: &str) -> &Dialect {
        self.file_dialects.get(file).unwrap_or(&self.dialect)
    }

    /// The dialect of the file a level is written in, whose rules its names compare by: a
    /// subcircuit's nets compare by the rules of the file that writes its `.subckt`, the main file
    /// level's by the main file's.
    pub fn level_rules(&self, level: LevelId) -> &Dialect {
        self.rules(&self.levels[level].file)
    }

    /// Whether the files of the document are of more than one dialect.
    pub fn mixed(&self) -> bool {
        self.file_dialects.values().any(|d| d.name != self.dialect.name)
    }

    /// Whether the file `file`, by its name in the document, is a package file (2.7): one read in
    /// the RNL package format, as a package file checked on its own is ([`crate::check_package`]).
    pub fn is_package(&self, file: &str) -> bool {
        self.rules(file).name == package::NAME
    }

    /// The format a file was read with, by its name in the document.
    pub fn file_format(&self, file: &str) -> Option<&FileFormat> {
        self.file_formats.iter().find(|f| f.file == file)
    }

    /// The key a name is compared by in the level `level`, by the rules of its file.
    pub fn key_in(&self, level: LevelId, name: &str) -> String {
        dialect_key(self.level_rules(level), name)
    }

    /// The key of a net of the level `level` written as `name` in the file `written`: compared by
    /// the rules of the level's file, and merged into `0` when it is a ground alias by the rules of
    /// the file that writes it, so that `gnd` is ground in a statement of a SPICE file only (6.3).
    pub fn net_key_in(&self, level: LevelId, name: &str, written: &str) -> String {
        let at = self.rules(written);
        if at.ground_aliases.iter().any(|g| dialect_key(at, g) == dialect_key(at, name)) {
            return "0".to_string();
        }
        self.key_in(level, name)
    }

    /// Whether the net of the level `level` whose key is `key` is global: `0`, or a global net
    /// declared in a file of the main file's dialect or of another, compared without case when
    /// either the level's dialect or the declaring one compares names so.
    pub fn is_global(&self, level: LevelId, key: &str) -> bool {
        if key == "0" || self.globals.contains(key) {
            return true;
        }
        let cs = self.level_rules(level).case_sensitive;
        let main = self.dialect.case_sensitive;
        (cs != main && self.globals.iter().any(|g| same(g, key, cs && main)))
            || self.foreign_globals.iter().any(|(g, gcs)| same(g, key, cs && *gcs))
    }

    /// Declare the global net `name`, written in the file `file` (`.global`, Spectre `global`).
    pub fn add_global(&mut self, name: &str, file: &str) {
        let rules = self.rules(file);
        if rules.name == self.dialect.name {
            let k = self.net_key(name);
            self.globals.insert(k);
        } else {
            let entry = (dialect_key(rules, name), rules.case_sensitive);
            self.foreign_globals.push(entry);
        }
    }

    /// Register the subcircuit `level` by its name, as the dialect of its file compares names: a
    /// later definition of the same name in a file of that dialect stands for it.
    pub fn add_subckt(&mut self, level: LevelId) {
        let lv = &self.levels[level];
        let rules = self.rules(&lv.file);
        if rules.name == self.dialect.name {
            let key = self.key(&lv.name);
            self.subckts.insert(key, level);
            return;
        }
        let (name, dialect, cs) = (lv.name.clone(), rules.name.clone(), rules.case_sensitive);
        let levels = &self.levels;
        let rules_of = |l: LevelId| self.file_dialects.get(&levels[l].file).map_or(&self.dialect.name, |d| &d.name);
        let kept: Vec<LevelId> = self
            .foreign_subckts
            .iter()
            .copied()
            .filter(|&other| !(*rules_of(other) == dialect && same(&levels[other].name, &name, cs)))
            .collect();
        self.foreign_subckts = kept;
        self.foreign_subckts.push(level);
    }

    /// The subcircuits that a reference to `name`, from a file whose names compare by case or not,
    /// finds: those of the same name, compared by case only when both the reference and the
    /// definition compare names so. When one of them is spelt as the reference writes it, that one
    /// alone; else all, so that more than one is a name that is not unique.
    pub fn subckt_candidates(&self, name: &str, case_sensitive: bool) -> Vec<LevelId> {
        if self.foreign_subckts.is_empty() && case_sensitive == self.dialect.case_sensitive {
            return self.subckts.get(&self.key(name)).copied().into_iter().collect();
        }
        let mut found: Vec<LevelId> = if case_sensitive == self.dialect.case_sensitive {
            self.subckts.get(&self.key(name)).copied().into_iter().collect()
        } else {
            let mut v: Vec<LevelId> =
                self.subckts.values().copied().filter(|&l| same(&self.levels[l].name, name, false)).collect();
            v.sort_unstable();
            v
        };
        for &l in &self.foreign_subckts {
            if same(&self.levels[l].name, name, case_sensitive && self.level_rules(l).case_sensitive) {
                found.push(l);
            }
        }
        let exact: Vec<LevelId> = found.iter().copied().filter(|&l| self.levels[l].name == name).collect();
        if exact.len() == 1 {
            exact
        } else {
            found
        }
    }

    /// The subcircuit that a reference to `name`, from a file whose names compare by case or not,
    /// names: the one of [`HostDocument::subckt_candidates`]; None when there is none, or more.
    pub fn subckt_for(&self, name: &str, case_sensitive: bool) -> Option<LevelId> {
        match self.subckt_candidates(name, case_sensitive).as_slice() {
            [one] => Some(*one),
            _ => None,
        }
    }

    /// The subcircuit an instance names, by the rules of the file it is written in.
    pub fn subckt_of(&self, inst: &Instance) -> Option<LevelId> {
        let name = inst.model.as_deref()?;
        self.subckt_for(name, self.rules(&inst.file).case_sensitive)
    }

    /// The key a name is compared by in this dialect.
    pub fn key(&self, name: &str) -> String {
        if self.dialect.case_sensitive {
            name.to_string()
        } else {
            name.to_lowercase()
        }
    }

    /// A net's key: ground aliases are merged into `0` (6.3).
    pub fn net_key(&self, name: &str) -> String {
        let k = self.key(name);
        if self.dialect.ground_aliases.iter().any(|g| self.key(g) == k) {
            "0".to_string()
        } else {
            k
        }
    }

    /// The key of a level's name, by the rules of the file it is written in: that of
    /// [`HostDocument::key`] in a file of the main file's dialect, and qualified by the dialect in a
    /// file of another, so that two subcircuits that one dialect tells apart keep two keys.
    pub fn level_key(&self, level: LevelId) -> String {
        let rules = self.level_rules(level);
        let key = dialect_key(rules, &self.levels[level].name);
        if rules.name == self.dialect.name {
            self.key(&self.levels[level].name)
        } else {
            format!("{}\u{0}{key}", rules.name)
        }
    }

    /// Whether `name`, written in RNL, names the level `level`: compared by the rules of the file
    /// the level is written in, as a reference to an object is (3.6).
    pub fn names_level(&self, level: LevelId, name: &str) -> bool {
        same(&self.levels[level].name, name, self.level_rules(level).case_sensitive)
    }

    /// The subcircuit named `name`, compared as the main file's dialect compares names.
    pub fn subckt(&self, name: &str) -> Option<LevelId> {
        self.subckt_for(name, self.dialect.case_sensitive)
    }

    /// The file level whose text holds `level`: `level` itself when it is a file level, else the
    /// file level that its subcircuit is written in, a library section read as a file (2.8).
    pub fn file_level(&self, level: LevelId) -> LevelId {
        let mut at = level;
        while self.levels[at].kind == LevelKind::Subckt {
            match self.levels[at].parent {
                Some(p) => at = p,
                None => break,
            }
        }
        at
    }

    /// The file level that read an instance: the one the adapter read it through
    /// ([`Instance::file_level`]), else the one whose text holds its line ([`Self::file_level_at`]).
    pub fn file_level_of(&self, inst: &Instance) -> Option<LevelId> {
        inst.file_level.or_else(|| self.file_level_at(&inst.file, inst.line))
    }

    /// The file level whose text holds the line `line` of `file`: the one that reads the library
    /// section that holds it, else the first that reads the file whole. None when no file level
    /// reads that line. A file read twice has two; [`Self::file_level_of`] tells an instance's.
    pub fn file_level_at(&self, file: &str, line: u32) -> Option<LevelId> {
        let of_file = || {
            (0..self.levels.len()).filter(|&l| self.levels[l].kind == LevelKind::File && self.levels[l].file == file)
        };
        of_file()
            .find(|&l| self.levels[l].section_lines.is_some_and(|(a, b)| a < line && line < b))
            .or_else(|| of_file().find(|&l| self.levels[l].section.is_none()))
    }

    pub fn ancestors(&self, lv: LevelId) -> Vec<LevelId> {
        let mut out = Vec::new();
        let mut cur = self.levels[lv].parent;
        while let Some(p) = cur {
            out.push(p);
            cur = self.levels[p].parent;
        }
        out
    }

    pub fn descendants(&self, lv: LevelId) -> Vec<LevelId> {
        let mut out = Vec::new();
        for &c in &self.levels[lv].children {
            out.push(c);
            out.extend(self.descendants(c));
        }
        out
    }

    pub fn source(&self, file: &str) -> Option<&str> {
        self.sources.iter().find(|(f, _)| f == file).map(|(_, t)| t.as_str())
    }

    pub fn add_level(&mut self, mut lv: Level, parent: Option<LevelId>) -> LevelId {
        lv.parent = parent;
        let id = self.levels.len();
        self.levels.push(lv);
        if let Some(p) = parent {
            self.levels[p].children.push(id);
        }
        id
    }
}

/// The format a file was read with ([`HostDocument::file_formats`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFormat {
    /// The file's name in the document.
    pub file: String,
    /// The name of the format that read it ([`format::Format::name`]).
    pub format: String,
    /// The format it is read with when none is set for it.
    pub default: String,
}

/// The key a name is compared by in a dialect: as written, or in lower case.
pub fn dialect_key(dialect: &Dialect, name: &str) -> String {
    if dialect.case_sensitive {
        name.to_string()
    } else {
        name.to_lowercase()
    }
}

/// Whether two names are the same: as written, or in lower case.
fn same(a: &str, b: &str, case_sensitive: bool) -> bool {
    a == b || (!case_sensitive && a.to_lowercase() == b.to_lowercase())
}

/// The last component of a `/`-separated path.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// `*` matches any run of characters; the rest compares as written, by case or not (6.12, 10.1).
pub fn glob(pattern: &str, name: &str, case_sensitive: bool) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = if case_sensitive {
        (pattern.chars().collect(), name.chars().collect())
    } else {
        (pattern.to_lowercase().chars().collect(), name.to_lowercase().chars().collect())
    };
    // iterative wildcard match with backtracking on the last `*`
    let (mut i, mut j) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while j < n.len() {
        if i < p.len() && p[i] != '*' && p[i] == n[j] {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            star = Some(i);
            mark = j;
            i += 1;
        } else if let Some(s) = star {
            i = s + 1;
            mark += 1;
            j = mark;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == '*' {
        i += 1;
    }
    i == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(glob("R*", "r1", false));
        assert!(!glob("R*", "r1", true));
        assert!(glob("sky130_fd_pr__nfet_*", "sky130_fd_pr__nfet_01v8", false));
        assert!(glob("*", "", false));
        assert!(glob("a*b*c", "aXXbYc", true));
        assert!(!glob("a*b", "aXXbY", true));
    }

    #[test]
    fn instances_by_name() {
        let mut lv = Level::new(LevelKind::File, "t.sp", "t.sp", "t.sp");
        for name in ["R1", "RÄ", "X1", "x1"] {
            lv.instances.push(Instance { name: name.into(), ..Instance::default() });
        }
        // only ASCII letters ignore case, as same_name compares; the first of a name is found
        assert_eq!(lv.inst("r1"), Some(0));
        assert_eq!(lv.inst("rÄ"), Some(1));
        assert_eq!(lv.inst("rä"), None);
        assert_eq!(lv.inst("x1"), Some(2));
        for cs in [false, true] {
            lv.case_sensitive = cs;
            for (a, b) in [("R1", "r1"), ("RÄ", "rÄ"), ("RÄ", "rä"), ("X1", "X2"), ("x1", "x1")] {
                assert_eq!(lv.name_key(a) == lv.name_key(b), lv.same_name(a, b), "{a} {b} {cs}");
            }
        }
        // the answer follows the instances and the case rule as they are now
        assert_eq!(lv.inst("x1"), Some(3));
        lv.instances[0].name = "R2".into();
        assert_eq!((lv.inst("R1"), lv.inst("R2")), (None, Some(0)));
    }
}
