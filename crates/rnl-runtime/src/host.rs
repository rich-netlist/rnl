//! The host circuit as host adapters hand it to the runtime (spec-draft 2.1, 2.6, 6.1, 6.3).
//!
//! The model is dialect-neutral: adapters for SPICE, Spectre or another host fill it in, and the
//! runtime never parses netlist syntax itself. Levels follow the text: the main file level is the
//! top circuit, every included file has a file level, every subcircuit definition is a level.

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
    /// Netlist order within the drawing level (11.6).
    pub order: usize,
    /// The model type, lower case, such as "nmos" (10.1).
    pub mtype: Option<String>,
    /// An instance of a subcircuit, defined in the netlist or not (a PDK device).
    pub is_subckt: bool,
    /// The adapter's default terminal names (its standard DEVICE table, 6.12).
    pub default_terminals: Option<Vec<String>>,
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
}

impl Level {
    pub fn new(kind: LevelKind, name: impl Into<String>, file: impl Into<String>, path: impl Into<String>) -> Level {
        Level {
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
        }
    }

    pub fn title(&self) -> String {
        match self.kind {
            LevelKind::Subckt => self.name.clone(),
            _ => file_name(&self.file).to_string(),
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
    pub dialect: Dialect,
    /// Subcircuit key -> level.
    pub subckts: HashMap<String, LevelId>,
    /// Global net keys (`.global`, Spectre `global`), with `0` implied.
    pub globals: HashSet<String>,
    /// File name -> text, for display and writeback.
    pub sources: Vec<(String, String)>,
}

impl HostDocument {
    pub fn new(main: Level, dialect: Dialect) -> HostDocument {
        HostDocument {
            levels: vec![main],
            dialect,
            subckts: HashMap::new(),
            globals: HashSet::new(),
            sources: Vec::new(),
        }
    }

    pub fn main(&self) -> &Level {
        &self.levels[0]
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

    pub fn subckt(&self, name: &str) -> Option<LevelId> {
        self.subckts.get(&self.key(name)).copied()
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
