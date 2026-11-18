//! Effective DEVICE declarations for each host instance (6.12).

use crate::diag::{codes, Severity};
use crate::host::{glob, LevelKind};

use super::obj::{ObjData, ObjId, SceneId};
use super::Interp;

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
        for object in self.scenes[s].insts.clone() {
            let inst = self.inst_of(object).clone();
            let file = self.doc.file_level_of(&inst).unwrap_or(level);
            // an instance of a file none of whose RNL is read takes no DEVICE declaration (16.1)
            if !self.file_ok(file) {
                continue;
            }
            let effective = self.effective_packages(level);
            let mut declarations = Vec::new();
            for entry in &effective.entries {
                declarations.extend(self.units[entry.unit].devices.iter().cloned().map(|mut declaration| {
                    declaration.languages = declaration.languages.intersect(entry.languages);
                    declaration
                }));
            }
            // Includes may put their host lines into an outer circuit. Their header DEVICE
            // declarations still act on those lines, without affecting the outer file's lines.
            let mut chain = self.doc.ancestors(file);
            chain.reverse();
            chain.push(file);
            for lv in chain {
                if self.doc.levels[lv].kind == LevelKind::File {
                    declarations.extend(self.units[self.level_unit[lv]].devices.clone());
                }
            }
            let ports = if inst.is_subckt {
                self.doc.subckt_of(&inst).map(|lv| self.doc.levels[lv].ports.clone())
            } else {
                None
            };
            let mut info = DeviceInfo {
                terminals: ports.clone().or_else(|| self.default_terminals(&inst)).unwrap_or_default(),
                mtype: inst.mtype.clone(),
            };
            for decl in declarations {
                if !decl.languages.overlaps(self.doc.language_at(&inst.file, inst.line)) {
                    continue;
                }
                if !glob(&decl.name, &inst.name, inst.case_sensitive)
                    || decl.model.as_ref().is_some_and(|pattern| {
                        !inst.model.as_ref().is_some_and(|name| glob(pattern, name, inst.case_sensitive))
                    })
                {
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
        }
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
