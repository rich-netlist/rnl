//! Record language reach, based on host ownership rather than drawing coordinates (2.8).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::host::language::Languages;

use super::ctx::Ctx;
use super::obj::{ObjData, ObjId, SceneId};
use super::Interp;

/// Host sections are immutable during interpretation. Index each level's nets once,
/// instead of rescanning every instance for each selected net.
#[derive(Default)]
pub(crate) struct HostLanguages {
    levels: RefCell<HashMap<usize, Languages>>,
    nets: RefCell<HashMap<usize, HashMap<String, Languages>>>,
}

impl Interp<'_> {
    pub(super) fn level_languages(&self, level: usize) -> Languages {
        *self.host_languages.levels.borrow_mut().entry(level).or_insert_with(|| self.doc.level_languages(level))
    }

    fn net_languages(&self, level: usize, key: &str) -> Languages {
        self.host_languages
            .nets
            .borrow_mut()
            .entry(level)
            .or_insert_with(|| self.doc.net_language_index(level))
            .get(key)
            .copied()
            .unwrap_or(Languages::NONE)
    }
    pub(super) fn record_languages(&self, ctx: &Ctx) -> Languages {
        let mut language = ctx.header.languages;
        if let Some(scene) = ctx.scene.filter(|_| !ctx.in_definition() && !ctx.in_symbol()) {
            if let Some(effective) = self.effs.get(&super::packages::EffKey::Level(self.scenes[scene].level)) {
                if let Some(entry) = effective.entry(ctx.unit) {
                    language = language.intersect(entry.languages);
                }
            }
        }
        language
    }

    pub(super) fn record_has_language(&self, unit: usize, header: &super::unit::Header) -> bool {
        if header.languages == Languages::ALL && self.units[unit].level.is_some() {
            return true;
        }
        let Some(level) = self.units[unit].level else {
            // Standalone package checks have no host sections to restrict them. When a
            // host uses a package, its definitions are still published, but its records
            // are checked only if at least one effective chain reaches their language.
            if self.checking_path.is_some() {
                return true;
            }
            let mut in_scope = false;
            let active = self.effs.iter().any(|(key, effective)| {
                let super::packages::EffKey::Level(level) = key else { return false };
                effective.entry(unit).is_some_and(|entry| {
                    in_scope = true;
                    header.languages.intersect(entry.languages).overlaps(self.level_languages(*level))
                })
            });
            return active || (!in_scope && header.languages == Languages::ALL);
        };
        let levels = std::iter::once(level).chain(self.doc.descendants(level));
        levels
            .filter(|level| header.scope.as_ref().is_none_or(|name| self.doc.names_level(*level, name)))
            .any(|level| header.languages.overlaps(self.level_languages(level)))
    }

    pub(super) fn language_reaches_object(&mut self, object: ObjId, ctx: &Ctx) -> bool {
        if ctx.scene.is_some_and(|scene| self.inactive_objects.contains(&(scene, object))) {
            return false;
        }
        let language = self.record_languages(ctx);
        if language == Languages::ALL || ctx.in_symbol() {
            return true;
        }
        // A collection reference preserves its handle. Its final members, including later
        // edits, are checked together by the operation which consumes it (5.2, 2.8).
        if self.is_set(object) {
            return true;
        }
        let Some(scene) = ctx.scene else { return true };
        // Definitions and styles have their ordinary name scope, without section filtering.
        if matches!(self.o(object), ObjData::Definition(_) | ObjData::Symbol(_) | ObjData::Style(_)) {
            return true;
        }
        self.object_languages(scene, object, &mut HashSet::new(), ctx)
            .unwrap_or_else(|| self.level_languages(self.scenes[scene].level))
            .overlaps(language)
    }

    pub(super) fn language_inputs_reach(&mut self, ctx: &Ctx, phase: usize) -> bool {
        if self.record_languages(ctx) == Languages::ALL && self.inactive_objects.is_empty() {
            return true;
        }
        let Some(scene) = ctx.scene else { return true };
        for &object in &ctx.language_inputs {
            if !self.language_reaches_object(object, ctx) {
                return false;
            }
            if self.record_languages(ctx) != Languages::ALL || self.is_set(object) {
                let inputs = self.flatten_inputs(scene, object, phase, Some(ctx));
                if !self.language_reaches_object(object, ctx)
                    || inputs.iter().any(|input| !self.language_reaches_object(input.object, ctx))
                {
                    return false;
                }
            }
        }
        true
    }

    /// Evaluate a deferred statement's complete input before adding any of its effects.
    pub(super) fn queue_task(&mut self, scene: SceneId, phase: usize, ctx: &Ctx, task: super::scene::Task) {
        let mut context = ctx.clone();
        context.scene = Some(scene);
        let guarded: super::scene::Task = Rc::new(move |it, scene, phase| {
            if it.language_inputs_reach(&context, phase) {
                task(it, scene, phase);
            }
        });
        self.scenes[scene].tasks[phase].push((ctx.pos.clone(), guarded));
    }

    /// None means no host anchor; such an object belongs to every language of its level.
    /// An intersection of zero means that its host anchors straddle the language sections.
    fn object_languages(
        &mut self,
        scene: SceneId,
        object: ObjId,
        seen: &mut HashSet<ObjId>,
        ctx: &Ctx,
    ) -> Option<Languages> {
        if !seen.insert(object) {
            return None;
        }
        if let Some(input) = self.role_inputs.get(&object) {
            let value = self.object_languages(scene, input.source, seen, ctx);
            seen.remove(&object);
            return value;
        }
        let value = match self.o(object).clone() {
            ObjData::Inst { .. } => {
                let inst = self.inst_of(object);
                Some(self.doc.language_at(&inst.file, inst.line))
            }
            ObjData::Net { scene, key, .. } => Some(self.net_languages(self.scenes[scene].level, &key)),
            ObjData::Term { owner, .. } => match self.o(owner) {
                ObjData::LevelObj { scene } => {
                    let level = &self.doc.levels[self.scenes[*scene].level];
                    Some(self.doc.language_at(&level.file, level.line))
                }
                _ => self.object_languages(scene, owner, seen, ctx),
            },
            ObjData::ScenePin { owner, .. } => self.object_languages(scene, owner, seen, ctx),
            ObjData::NamedSubobject { parent, .. } | ObjData::Pos { parent } => {
                self.object_languages(scene, parent, seen, ctx)
            }
            ObjData::Text { attach, .. } | ObjData::KeyText { attach, .. } => {
                self.object_languages(scene, attach, seen, ctx)
            }
            ObjData::Usage(usage) => {
                let mut objects: Vec<_> = usage.roles.iter().map(|(_, object)| *object).collect();
                objects.extend(usage.parent);
                self.objects_languages(scene, objects, seen, ctx)
            }
            ObjData::Wire { points, .. } | ObjData::Prim { points, .. } => {
                self.objects_languages(scene, points.iter().copied(), seen, ctx)
            }
            ObjData::Route { net, .. } => self.object_languages(scene, net, seen, ctx),
            ObjData::Bundle { members, .. } => self.objects_languages(scene, members.iter().copied(), seen, ctx),
            ObjData::Set(_) => {
                // A usage can carry a collection role. Its section follows that role's
                // final membership, including edits written after this reference.
                if !self.is_lazy(object)
                    && self
                        .constraint_input(object, ctx.stmt.as_ref().map_or(0, |s| s.start), usize::MAX - 1, ctx)
                        .is_err()
                {
                    seen.remove(&object);
                    return Some(Languages::NONE);
                }
                let members = if self.is_lazy(object) {
                    self.input_items(scene, object, self.scenes[scene].phase, None)
                } else {
                    self.written_inputs(object, Some(scene), &mut HashSet::new())
                };
                self.objects_languages(scene, members.into_iter().map(|member| member.object), seen, ctx)
            }
            ObjData::LevelObj { scene } => Some(self.level_languages(self.scenes[scene].level)),
            _ => None,
        };
        let value = value.or_else(|| {
            let parent = self.scenes[scene].creation_sites.get(&object).and_then(|site| site.frame_usage());
            parent.and_then(|parent| self.object_languages(scene, parent, seen, ctx))
        });
        seen.remove(&object);
        value
    }

    fn objects_languages(
        &mut self,
        scene: SceneId,
        objects: impl IntoIterator<Item = ObjId>,
        seen: &mut HashSet<ObjId>,
        ctx: &Ctx,
    ) -> Option<Languages> {
        objects
            .into_iter()
            .filter_map(|object| self.object_languages(scene, object, seen, ctx))
            .reduce(Languages::intersect)
    }
}
