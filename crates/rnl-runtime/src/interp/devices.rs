//! Effective DEVICE declarations for each host instance (6.12).

use std::collections::HashMap;
use std::rc::Rc;

use crate::diag::{codes, Severity};
use crate::host::language::Languages;
use crate::host::{glob, LevelId, LevelKind};

use super::obj::{ObjData, ObjId, SceneId};
use super::unit::DeviceDecl;
use super::Interp;

/// What the declarations that can act on an instance depend on besides its name: the file level
/// it is read in, its model, its host's case rule and the languages of its line.
type CandidateKey = (LevelId, Option<String>, bool, Languages);

#[derive(Clone, Debug)]
pub(crate) struct DeviceInfo {
    pub terminals: Vec<String>,
    pub mtype: Option<String>,
}

impl Interp<'_> {
    /// Resolve once before the scene's statements, so selections, attributes and assertions
    /// all read the same type, independently of when a terminal is first referenced.
    pub(super) fn prepare_devices(&mut self, s: SceneId) {
        let level = self.scenes[s].level;
        // The declarations of the packages in effect on the level, and those of the chain of
        // files down to an instance's file, are the same for every instance of the level, or of
        // the file: they are made once, when an instance first needs them.
        let mut packaged: Option<Rc<Vec<DeviceDecl>>> = None;
        let mut chains: HashMap<LevelId, Rc<Vec<DeviceDecl>>> = HashMap::new();
        // The declarations that can match an instance, by its file, model, case rule and language:
        // all it takes but its name, which each instance matches on its own. A level of thousands
        // of instances of a few models matches the model patterns once for each.
        let mut candidates: HashMap<CandidateKey, Rc<[usize]>> = HashMap::new();
        for object in self.scenes[s].insts.clone() {
            let inst = self.inst_of(object).clone();
            let file = self.doc.file_level_of(&inst).unwrap_or(level);
            // an instance of a file none of whose RNL is read takes no DEVICE declaration (16.1)
            if !self.file_ok(file) {
                continue;
            }
            let packaged = match &packaged {
                Some(declarations) => declarations.clone(),
                None => packaged.insert(Rc::new(self.package_device_declarations(level))).clone(),
            };
            let chain = match chains.get(&file) {
                Some(declarations) => declarations.clone(),
                None => {
                    let declarations = Rc::new(self.file_device_declarations(file));
                    chains.insert(file, declarations.clone());
                    declarations
                }
            };
            let language = if packaged.is_empty() && chain.is_empty() {
                Languages::ALL
            } else {
                self.doc.language_at(&inst.file, inst.line)
            };
            let ports = if inst.is_subckt {
                self.doc.subckt_of(&inst).map(|lv| self.doc.levels[lv].ports.clone())
            } else {
                None
            };
            let mut info = DeviceInfo {
                terminals: ports.clone().or_else(|| self.default_terminals(&inst)).unwrap_or_default(),
                mtype: inst.mtype.clone(),
            };
            let key = (file, inst.model.clone(), inst.case_sensitive, language);
            let matching = match candidates.get(&key) {
                Some(matching) => {
                    debug_assert_eq!(**matching, *device_candidates(&packaged, &chain, &inst, language));
                    matching.clone()
                }
                None => {
                    let matching: Rc<[usize]> = device_candidates(&packaged, &chain, &inst, language).into();
                    candidates.insert(key, matching.clone());
                    matching
                }
            };
            for &k in matching.iter() {
                let decl = if k < packaged.len() { &packaged[k] } else { &chain[k - packaged.len()] };
                // a pattern of `*` alone takes any name
                if !(any_name(&decl.name) || glob(&decl.name, &inst.name, inst.case_sensitive)) {
                    continue;
                }
                if let Some(terms) = &decl.terminals {
                    let names: Vec<String> = terms.iter().map(|t| t.str().unwrap_or_default().to_string()).collect();
                    if let Some(ports) = &ports {
                        let equal = names.len() == ports.len()
                            && names.iter().zip(ports).all(|(a, b)| {
                                if inst.case_sensitive {
                                    a == b
                                } else {
                                    a.eq_ignore_ascii_case(b)
                                }
                            });
                        if !equal {
                            self.diag_in(Severity::Warning, codes::DEVICE_SUBCIRCUIT_PORTS_CONFLICT,
                                format!("@DEVICE cannot change the port names of #{}: its subcircuit is defined by the host (6.12)", inst.name),
                                Some(decl.loc.clone()), &decl.package);
                        }
                    } else {
                        info.terminals = names;
                    }
                }
                if let Some(mtype) = &decl.mtype {
                    if let Some(host_type) = &inst.mtype {
                        if !host_type.eq_ignore_ascii_case(mtype) {
                            self.diag_in(Severity::Warning, codes::DEVICE_HOST_TYPE_CONFLICT,
                                format!("@DEVICE cannot change the type of #{} from {host_type} to {mtype}: its host supplies the type (6.12)", inst.name),
                                Some(decl.loc.clone()), &decl.package);
                        }
                    } else {
                        info.mtype = Some(mtype.clone());
                    }
                }
            }
            self.scenes[s].device_info.insert(object, info);
            self.scenes[s].pin_nets.clear();
        }
    }

    /// The DEVICE declarations of the packages in effect on `level`, each with the languages of
    /// the use that brings its package in, in the order of the packages.
    fn package_device_declarations(&mut self, level: LevelId) -> Vec<DeviceDecl> {
        #[cfg(test)]
        tests::PACKAGE_DECLARATIONS_MADE.with(|count| count.set(count.get() + 1));
        let effective = self.effective_packages(level);
        let mut declarations = Vec::new();
        for entry in &effective.entries {
            declarations.extend(self.units[entry.unit].devices.iter().cloned().map(|mut declaration| {
                declaration.languages = declaration.languages.intersect(entry.languages);
                declaration
            }));
        }
        declarations
    }

    /// The DEVICE declarations of the headers of the files from the outermost down to `file`.
    /// Includes may put their host lines into an outer circuit. Their header DEVICE declarations
    /// still act on those lines, without affecting the outer file's lines.
    fn file_device_declarations(&self, file: LevelId) -> Vec<DeviceDecl> {
        let mut chain = self.doc.ancestors(file);
        chain.reverse();
        chain.push(file);
        let mut declarations = Vec::new();
        for lv in chain {
            if self.doc.levels[lv].kind == LevelKind::File {
                declarations.extend(self.units[self.level_unit[lv]].devices.iter().cloned());
            }
        }
        declarations
    }

    /// Type conditions share one matcher in circuit selections, pin selections and assertions.
    pub(super) fn instance_type_matches(&self, object: ObjId, pattern: &str) -> bool {
        let pattern = pattern.to_lowercase();
        self.instance_type(object).is_some_and(|kind| glob(&pattern, kind, true))
    }

    pub(super) fn instance_type(&self, object: ObjId) -> Option<&str> {
        let ObjData::Inst { scene, .. } = self.o(object) else { return None };
        match self.scenes[*scene].device_info.get(&object) {
            Some(info) => info.mtype.as_deref(),
            None => self.inst_of(object).mtype.as_deref(),
        }
    }
}

/// The declarations of `packaged` then `chain`, by their index in that order, that act on an
/// instance like `inst` whatever its name: of a language `language` overlaps, and whose model
/// pattern, if any, takes its model (6.12).
fn device_candidates(
    packaged: &[DeviceDecl],
    chain: &[DeviceDecl],
    inst: &crate::host::Instance,
    language: Languages,
) -> Vec<usize> {
    packaged
        .iter()
        .chain(chain)
        .enumerate()
        .filter(|(_, decl)| {
            decl.languages.overlaps(language)
                && decl.model.as_ref().is_none_or(|pattern| {
                    inst.model.as_ref().is_some_and(|name| glob(pattern, name, inst.case_sensitive))
                })
        })
        .map(|(k, _)| k)
        .collect()
}

/// Whether a name pattern takes every name: one of `*` alone, as [`glob`] matches it.
fn any_name(pattern: &str) -> bool {
    !pattern.is_empty() && pattern.bytes().all(|b| b == b'*')
}

#[cfg(test)]
pub(super) mod tests {
    use std::cell::Cell;

    use crate::diag::{Severity, Sink};
    use crate::host::{Dialect, HostDocument, Instance, Level, LevelKind};
    use crate::records::{package_lines, scan_run};
    use crate::source::MemoryProvider;
    use crate::{interpret, Options};

    thread_local! {
        /// How many times the DEVICE declarations of the packages in effect on a level were made,
        /// on this thread.
        pub(in crate::interp) static PACKAGE_DECLARATIONS_MADE: Cell<usize> = const { Cell::new(0) };
    }

    /// The times the declarations of the packages in effect were made while a level of
    /// `instances` instances with a DEVICE declaration in its header is read.
    fn declarations_made(instances: usize) -> usize {
        let mut sink = Sink::new();
        let mut main = Level::new(LevelKind::File, "devices.sp", "devices.sp", "devices.sp");
        main.circuit = true;
        for k in 0..instances {
            main.instances.push(Instance {
                name: format!("X{}", k + 1),
                letter: Some('X'),
                nodes: vec![format!("n{k}"), format!("n{}", k + 1)],
                model: Some("cell".into()),
                file: "devices.sp".into(),
                line: k as u32 + 2,
                order: k,
                ..Instance::default()
            });
        }
        let records = "@RNL(type=\"header\") { @VERSION(\"0.4\"); \
                       @DEVICE(name=\"*\", model=\"cell\", terminals=[\"a\", \"b\"])}\n@RNL { }";
        main.records = scan_run(&package_lines(records), "devices.sp", &mut sink);
        let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec![] };
        PACKAGE_DECLARATIONS_MADE.with(|count| count.set(0));
        let read = interpret(
            HostDocument::new(main, dialect),
            &MemoryProvider::new(),
            &Options { std: false, ..Options::default() },
            sink,
        );
        let errors: Vec<_> = read.sink.all().iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "{errors:?}");
        PACKAGE_DECLARATIONS_MADE.with(Cell::get)
    }

    #[test]
    fn the_declarations_of_a_level_are_made_once_for_all_its_instances() {
        let (four, sixteen) = (declarations_made(4), declarations_made(16));
        // made once for the level in each reading, however many instances it has
        assert_eq!(four, sixteen, "{four} for 4 instances, {sixteen} for 16");
    }
}
