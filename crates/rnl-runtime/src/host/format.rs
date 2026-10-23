//! Formats: the host adapters that read the files of a netlist, each file with its own.
//!
//! A [`Format`] reads one file of its kind into a [`Reading`], the document shared by all the files
//! of the netlist, and hands each file that one includes back to the reading, which reads it with
//! the format chosen for that file. Once every file is read, each format that read one finishes
//! what needs them all, such as telling an element's nodes from its model name. What its files
//! define, subcircuits, model cards and global nets, is found from the files of other formats too:
//! a name compares by case only when both the file that refers to it and the one that defines it
//! compare names so, a definition spelt as written comes first, and more than one is an error.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

use crate::diag::{Loc, Sink};
use crate::host::{file_name, Dialect, FileFormat, HostDocument, Level, LevelId, LevelKind};
use crate::source::{dirname, normalize, relative_to, resolve_include, Provider};

/// A format of host files: an adapter, by the name that frontends and settings give it.
pub trait Format: Sync {
    /// Its name, such as `spice`, as settings and requests give it.
    fn name(&self) -> &'static str;
    /// Its name as shown, such as `SPICE`.
    fn title(&self) -> &'static str;
    /// The suffixes of the files it reads when none is set, lower case and without the dot.
    fn extensions(&self) -> &'static [&'static str];
    /// The rules its files' names follow (3.6, 6.3).
    fn dialect(&self) -> Dialect;
    /// Whether a file whose format is not set is of this format, by its path and its text.
    fn claims(&self, path: &str, text: &str) -> bool;
    /// Read one file into the reading ([`FileRead`]).
    fn read_file(&self, cx: &mut Reading, file: &FileRead);
    /// Once every file is read: what the format's files need all of them for.
    fn finish(&self, cx: &mut Reading);
}

/// One file to read: its path, the file level it is read into, the circuit its elements outside
/// subcircuits belong to (6.1), and the include chain that led to it, as `path#section` keys.
pub struct FileRead<'a> {
    pub path: &'a str,
    pub level: LevelId,
    pub circ: LevelId,
    /// The main file, whose first line is a title.
    pub main: bool,
    pub stack: &'a [String],
    /// Read only this library section of the file.
    pub section: Option<&'a str>,
}

/// A model card as the files of every format see it: its name, the type of its devices (10.1),
/// where it is written, and the format of that file with its case rule.
#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    pub mtype: Option<String>,
    pub format: &'static str,
    pub case_sensitive: bool,
    pub file: String,
    pub line: u32,
}

/// How each file of a reading is read: the formats there are, and the one of each file by its
/// path and text.
pub struct Formats<'a> {
    pub list: &'a [&'a dyn Format],
    /// The index in `list` of the format of a file, by its path as the reading resolved it, its
    /// text, and whether it is the main file; and the index of the one it would have when none is
    /// set for it.
    pub choose: &'a dyn Fn(&str, &str, bool) -> (usize, usize),
}

/// One reading of a netlist: the document its files are read into, and what the formats share
/// while they read.
pub struct Reading<'a> {
    pub provider: &'a dyn Provider,
    pub sink: &'a mut Sink,
    pub doc: HostDocument,
    /// The [`Level::name_key`]s of each level's instances, to find a name defined twice without
    /// looking through the level's instances.
    pub names: HashMap<LevelId, HashSet<String>>,
    formats: &'a Formats<'a>,
    main_dir: String,
    /// The formats that read a file, by index in `formats.list`.
    used: Vec<usize>,
    /// The model cards of every format by their names in lower case, each format's under the key
    /// its rules give, in the order written ([`Reading::add_card`]).
    cards: HashMap<String, Vec<Card>>,
    /// Each format's own state across its files.
    state: HashMap<TypeId, Box<dyn Any>>,
}

impl<'a> Reading<'a> {
    /// Read the netlist whose main file is `path`, then `includes` as includes of it at its line 1,
    /// without an include directive (a viewer's automatic includes). `main_file`: the main file's
    /// first line is a title, as it is when the file is opened, not when it is indexed as a
    /// library. None when the main file cannot be read.
    pub fn read(
        path: &str,
        provider: &'a dyn Provider,
        sink: &'a mut Sink,
        formats: &'a Formats<'a>,
        includes: &[String],
        main_file: bool,
    ) -> Option<HostDocument> {
        let path = normalize(path);
        let text = provider.read(&path)?;
        let (k, default) = (formats.choose)(&path, &text, true);
        let format = formats.list[k];
        let dialect = format.dialect();
        let fname = file_name(&path).to_string();
        let mut main = Level::new(LevelKind::File, fname.clone(), fname.clone(), path.clone());
        main.circuit = true;
        main.case_sensitive = dialect.case_sensitive;
        let doc = HostDocument::new(main, dialect);
        let mut cx = Reading {
            provider,
            sink,
            doc,
            names: HashMap::new(),
            formats,
            main_dir: dirname(&path),
            used: Vec::new(),
            cards: HashMap::new(),
            state: HashMap::new(),
        };
        cx.note_format(&fname, k, default);
        let stack = [format!("{path}#")];
        format.read_file(
            &mut cx,
            &FileRead { path: &path, level: 0, circ: 0, main: main_file, stack: &stack, section: None },
        );
        for extra in includes {
            cx.include(extra, None, 1, &fname, &path, 0, 0, &stack, false);
        }
        for k in cx.used.clone() {
            formats.list[k].finish(&mut cx);
        }
        cx.not_unique_subckts();
        Some(cx.doc)
    }

    /// A file's name in the document: relative to the main file's directory when it is in it or
    /// below, the root included, else its path.
    pub fn display(&self, path: &str) -> String {
        let p = normalize(path);
        match relative_to(&p, &self.main_dir) {
            Some(rest) => rest.to_string(),
            None => p,
        }
    }

    /// The state of a format across its files, made when first asked for.
    pub fn state<T: Default + 'static>(&mut self) -> &mut T {
        self.state
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut::<T>()
            .expect("the state of a type is of that type")
    }

    /// The files, by their names in the document, that a format other than `name` reads: what
    /// the format `name` leaves alone when it finishes.
    pub fn others(&self, name: &str) -> HashSet<String> {
        self.doc.file_formats.iter().filter(|f| f.format != name).map(|f| f.file.clone()).collect()
    }

    /// Whether the file `file`, by its name in the document, is read by the format `name`.
    pub fn is_of(&self, file: &str, name: &str) -> bool {
        self.doc.file_format(file).map_or(self.doc.dialect.name == name, |f| f.format == name)
    }

    fn note_format(&mut self, file: &str, k: usize, default: usize) {
        let format = self.formats.list[k];
        if !self.used.contains(&k) {
            self.used.push(k);
        }
        if !self.doc.file_formats.iter().any(|f| f.file == file) {
            self.doc.file_formats.push(FileFormat {
                file: file.to_string(),
                format: format.name().to_string(),
                default: self.formats.list[default].name().to_string(),
            });
        }
        if format.name() != self.doc.dialect.name {
            self.doc.file_dialects.entry(file.to_string()).or_insert_with(|| format.dialect());
        }
    }

    /// Read the file that `target` names, included from the line `no` of the file `fname` (path
    /// `path`) into the level `cur`, its elements outside subcircuits into `circ`, with the format
    /// chosen for it: a file level for it, below `cur`; `section`, only that library section of it.
    /// `explicit`: an include directive, not an automatic include. A file that includes itself, or
    /// that cannot be found, is an error.
    #[allow(clippy::too_many_arguments)]
    pub fn include(
        &mut self,
        target: &str,
        section: Option<&str>,
        no: u32,
        fname: &str,
        path: &str,
        cur: LevelId,
        circ: LevelId,
        stack: &[String],
        explicit: bool,
    ) {
        let full = resolve_include(self.provider, path, target, self.sink, Loc::line(fname, no));
        let key = format!("{}#{}", full, section.unwrap_or("").to_lowercase());
        if stack.contains(&key) {
            self.sink.error(format!("the include of {target} forms a cycle"), Some(Loc::line(fname, no)));
            return;
        }
        let Some(text) = self.provider.read(&full) else {
            self.sink.error(format!("the included file {target} cannot be found"), Some(Loc::line(fname, no)));
            return;
        };
        let display = self.display(&full);
        let name = match section {
            Some(s) => format!("{} ({s})", file_name(&display)),
            None => file_name(&display).to_string(),
        };
        let mut lv = Level::new(LevelKind::File, &name, &display, &full);
        lv.line = no;
        lv.section = section.map(String::from);
        lv.include_site = explicit.then(|| Loc::line(fname, no));
        let child = self.doc.add_level(lv, Some(cur));
        let (k, default) = (self.formats.choose)(&full, &text, false);
        self.note_format(&display, k, default);
        let mut st = stack.to_vec();
        st.push(key);
        let read = FileRead { path: &full, level: child, circ, main: false, stack: &st, section };
        self.formats.list[k].read_file(self, &read);
    }

    /// Register a model card, `replace` when a later card of the same name in a file of the same
    /// format stands for an earlier one, else the first stands.
    pub fn add_card(&mut self, card: Card, replace: bool) {
        let list = self.cards.entry(card.name.to_lowercase()).or_default();
        let same = |c: &Card| c.format == card.format && (!card.case_sensitive || c.name == card.name);
        match list.iter().position(same) {
            Some(k) if replace => list[k] = card,
            Some(_) => {}
            None => list.push(card),
        }
    }

    /// Whether model cards of a format other than `format` are written.
    pub fn foreign_cards(&self, format: &str) -> bool {
        self.cards.values().flatten().any(|c| c.format != format)
    }

    /// Whether some model card of the reading is the name `name`, compared without case.
    pub fn is_card(&self, name: &str) -> bool {
        self.cards.contains_key(&name.to_lowercase())
    }

    /// The model cards of every format that a reference to `name` from a file whose names compare
    /// by case, or not, finds, as [`HostDocument::subckt_candidates`] finds subcircuits: those of
    /// the same name, by case only when both files compare names so; when one of them is spelt as
    /// written, that one alone, else all.
    pub fn card_candidates(&self, name: &str, case_sensitive: bool) -> Vec<Card> {
        let found: Vec<Card> = self
            .cards
            .get(&name.to_lowercase())
            .into_iter()
            .flatten()
            .filter(|c| !(case_sensitive && c.case_sensitive) || c.name == name)
            .cloned()
            .collect();
        let exact: Vec<Card> = found.iter().filter(|c| c.name == name).cloned().collect();
        if exact.len() == 1 {
            exact
        } else {
            found
        }
    }

    /// The model card that a reference to `name`, written at `loc`, finds ([`Reading::card_candidates`]);
    /// None when none does, and an error when more than one does.
    pub fn card(&mut self, name: &str, case_sensitive: bool, loc: Loc) -> Option<Card> {
        let found = self.card_candidates(name, case_sensitive);
        if found.len() > 1 {
            let at: Vec<String> = found.iter().map(|c| format!("{}:{}", c.file, c.line)).collect();
            self.sink
                .error(format!("the model name {name} is not unique: model cards at {}", at.join(", ")), Some(loc));
            return None;
        }
        found.into_iter().next()
    }

    /// An instance naming a subcircuit that more than one definition, in files of different
    /// formats, is found for (3.4): an error, naming them.
    fn not_unique_subckts(&mut self) {
        if !self.doc.mixed() {
            return;
        }
        let mut errors = Vec::new();
        for lv in &self.doc.levels {
            for inst in lv.instances.iter().filter(|i| i.is_subckt) {
                let Some(name) = inst.model.as_deref() else { continue };
                let found = self.doc.subckt_candidates(name, self.doc.rules(&inst.file).case_sensitive);
                if found.len() > 1 {
                    let at: Vec<String> = found
                        .iter()
                        .map(|&l| format!("{}:{}", self.doc.levels[l].file, self.doc.levels[l].line))
                        .collect();
                    errors.push((
                        format!("the subcircuit name {name} is not unique: defined at {}", at.join(", ")),
                        Loc::line(&inst.file, inst.line),
                    ));
                }
            }
        }
        for (message, loc) in errors {
            self.sink.error(message, Some(loc));
        }
    }
}
