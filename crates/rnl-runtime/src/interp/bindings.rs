//! Level names retain their first valid creation, including forward references (6.6).
use std::collections::HashMap;

use super::ctx::Ctx;
use super::eval::{Bound, Stop, R};
use super::obj::{MemberKey, ObjId, SceneId, UnitId};
use super::unit::Binding;
use super::{Interp, Memo};
use crate::diag::{codes, Severity};

pub(super) type BindingName = (UnitId, char, String, Option<SceneId>);
pub(super) type BindingChoice = (MemberKey, usize);

#[derive(Default)]
pub(super) struct Bindings {
    /// Validity when the member created its objects, before symbol-dependent checks.
    valid: HashMap<(MemberKey, Option<SceneId>), Vec<bool>>,
    /// Only source identities survive a repeated reading, never object or diagnostic ids.
    pub(super) plans: HashMap<BindingName, BindingChoice>,
    queries: HashMap<BindingName, Binding>,
}

impl Interp<'_> {
    pub(super) fn modern_binding(&self, binding: &Binding) -> bool {
        self.follows_0_4(binding.rec.map_or(binding.owner, |rec| self.rec_unit[rec]))
    }

    pub(super) fn remember_binding_creation(
        &mut self,
        key: &MemberKey,
        scene: Option<SceneId>,
        bound: &Bound,
        ctx: &Ctx,
    ) {
        if self.follows_0_4(self.source_unit(ctx)) {
            let valid = bound.iter().map(|(_, value, _)| !self.is_failed(*value)).collect();
            self.bindings.valid.insert((key.clone(), scene), valid);
        }
    }

    /// Bindings compete in written order; a leading name still names the whole pipe.
    pub(super) fn binding_candidates(&self, binding: &Binding) -> Vec<Binding> {
        let unit = &self.units[binding.owner];
        let key = (binding.prefix, binding.name.clone());
        let first = unit.names.get(&key).unwrap_or(binding);
        let mut candidates = vec![first.clone()];
        candidates.extend(unit.dups.iter().filter(|b| b.prefix == binding.prefix && b.name == binding.name).cloned());
        candidates
    }

    pub(super) fn binding_slot(&self, binding: &Binding, scene: Option<SceneId>) -> Option<usize> {
        let bound = self.bound_memo.get(&(binding.key(), scene))?;
        let mut written: Vec<_> = binding.member.binds.iter().enumerate().collect();
        written.sort_by_key(|(index, slot)| (slot.after, *index));
        let occurrence = written
            .iter()
            .filter(|(_, slot)| slot.prefix == binding.prefix && slot.name == binding.name)
            .position(|(_, slot)| slot.off == binding.off)?;
        bound
            .iter()
            .enumerate()
            .filter(|(_, ((prefix, name), _, _))| *prefix == binding.prefix && *name == binding.name)
            .nth(occurrence)
            .map(|(index, _)| index)
    }

    pub(super) fn binding_result(&self, binding: &Binding, scene: Option<SceneId>, default: ObjId) -> (ObjId, bool) {
        let key = (binding.key(), scene);
        if let Some(index) = self.binding_slot(binding, scene) {
            let value = self.bound_memo[&key][index].1;
            let valid = self
                .bindings
                .valid
                .get(&key)
                .and_then(|values| values.get(index))
                .copied()
                .unwrap_or_else(|| !self.is_failed(value));
            (value, valid)
        } else {
            (default, !self.is_failed(default))
        }
    }

    pub(super) fn first_valid_binding_value(&mut self, binding: &Binding, ctx: &Ctx) -> R<ObjId> {
        let mut first_failed = None;
        for candidate in self.binding_candidates(binding) {
            let candidate_context = ctx.with_dependencies(&[]);
            let value = match self.binding_value_one(&candidate, &candidate_context) {
                Err(Stop::Language | Stop::OwnLevel) => continue,
                other => other?,
            };
            let scene =
                if self.places.rules.contains(&candidate.key()) { ctx.scene } else { self.home_scene(candidate.owner) };
            let (_, valid) = self.binding_result(&candidate, scene, value);
            if valid {
                ctx.depend_on(&candidate_context.reference_dependencies.borrow());
                return Ok(value);
            }
            first_failed.get_or_insert(value);
        }
        first_failed.ok_or(Stop::Language)
    }

    /// Read an already evaluated name for exports and consumers. A later symbol failure
    /// does not make another declaration take over the name.
    pub(super) fn evaluated_binding(&self, binding: &Binding, scene: Option<SceneId>) -> Option<ObjId> {
        if !self.modern_binding(binding) {
            let key = binding.key();
            let Memo::Val(value) = self.memo.get(&(key.clone(), scene))? else { return None };
            return Some(self.bound_value(&key, scene, binding.prefix, &binding.name, *value));
        }
        let mut failed = None;
        for candidate in self.binding_candidates(binding) {
            let target =
                if self.places.rules.contains(&candidate.key()) { scene } else { self.home_scene(candidate.owner) };
            let Some(Memo::Val(default)) = self.memo.get(&(candidate.key(), target)) else { continue };
            let (value, valid) = self.binding_result(&candidate, target, *default);
            if valid {
                return Some(value);
            }
            failed.get_or_insert(value);
        }
        failed
    }

    /// Static body checks must use the declaration that survives creation. Wait for the
    /// normal reading to resolve competing declarations, rather than creating them early.
    pub(super) fn static_binding(&mut self, binding: Binding, scene: Option<SceneId>) -> Option<Binding> {
        if !self.modern_binding(&binding) {
            return Some(binding);
        }
        let candidates = self.binding_candidates(&binding);
        if candidates.len() == 1 {
            return Some(binding);
        }
        let name = (binding.owner, binding.prefix, binding.name.clone(), scene);
        self.bindings.queries.insert(name.clone(), binding);
        let choice = self.bindings.plans.get(&name)?;
        candidates.into_iter().find(|candidate| (candidate.key(), candidate.off) == *choice)
    }

    pub(super) fn settle_bindings(&mut self) -> bool {
        let mut changed = false;
        for (name, binding) in std::mem::take(&mut self.bindings.queries) {
            for candidate in self.binding_candidates(&binding) {
                let scene = if self.places.rules.contains(&candidate.key()) {
                    name.3
                } else {
                    self.home_scene(candidate.owner)
                };
                match self.memo.get(&(candidate.key(), scene)) {
                    Some(Memo::Val(default)) => {
                        if !self.binding_result(&candidate, scene, *default).1 {
                            continue;
                        }
                        let choice = (candidate.key(), candidate.off);
                        if self.bindings.plans.get(&name) != Some(&choice) {
                            self.bindings.plans.insert(name, choice);
                            changed = true;
                        }
                        break;
                    }
                    Some(Memo::Language | Memo::OwnLevel) => continue,
                    // No value means this reading cannot determine the declaration yet.
                    _ => break,
                }
            }
        }
        changed
    }

    pub(super) fn check_modern_binding_duplicates(&mut self, unit: UnitId, scene: Option<SceneId>) {
        let package = if scene.is_none() { self.units[unit].pkg().to_string() } else { String::new() };
        for key in self.units[unit].name_order.clone() {
            let binding = self.units[unit].names[&key].clone();
            if !self.modern_binding(&binding) {
                continue;
            }
            let mut first = None;
            for candidate in self.binding_candidates(&binding) {
                let Some(Memo::Val(default)) = self.memo.get(&(candidate.key(), scene)) else { continue };
                let (value, valid) = self.binding_result(&candidate, scene, *default);
                if !valid {
                    continue;
                }
                if let Some(first) = first {
                    if first != value {
                        self.diag_in(Severity::Error,codes::NAME_BOUND_TWICE,
                            format!("{}{} is already bound to another object in this level; the first binding is kept (6.6)",candidate.prefix,candidate.name),
                            self.rec_loc(candidate.rec,candidate.member.start),&package);
                    }
                } else {
                    first = Some(value);
                }
            }
        }
    }
}
