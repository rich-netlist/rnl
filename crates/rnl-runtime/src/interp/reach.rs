//! The levels reached by selections and the sets that depend on them (10.1, 10.4).
use std::collections::HashSet;

use super::ctx::{Ctx, FrameKind};
use super::obj::{Lit, ObjData, ObjId, SceneId, SetKind, UnitId};
use super::unit::{Place, UnitKind};
use super::Interp;

/// Dependency and reach are separate: REMOVE-FROM can introduce a dependency without
/// extending the levels in which the edited set has members.
#[derive(Clone, Copy, Default)]
pub(super) struct SetReach {
    pub selected: bool,
    pub reaches: bool,
}

impl Interp<'_> {
    /// A set edited with a selection remains a rule input, including when the
    /// edit is reached through another set.
    pub(super) fn selected_by_selection_edit(&self, object: ObjId) -> bool {
        if !self.places.enabled {
            return false;
        }
        fn visit(it: &Interp<'_>, object: ObjId, edited: bool, seen: &mut HashSet<(ObjId, bool)>) -> bool {
            if !seen.insert((object, edited)) {
                return false;
            }
            let ObjData::Set(set) = it.o(object) else {
                return false;
            };
            if edited && matches!(set.kind, SetKind::Select { .. } | SetKind::Gen { .. }) {
                return true;
            }
            set.made_of().any(|child| visit(it, child, edited, seen))
                || set.mods.iter().any(|edit| visit(it, edit.source, true, seen))
        }
        visit(self, object, false, &mut HashSet::new())
    }

    pub(super) fn selection_at_home(&mut self, scene: SceneId, writer: UnitId, site: Option<&Ctx>) -> bool {
        if let Some(site) = site.filter(|site| site.frame_kind() == Some(FrameKind::Definition)) {
            return site.body_scene.or(site.scene) == Some(scene);
        }
        let level = self.scenes[scene].level;
        let writer = site.map_or(writer, |site| site.unit);
        if self.units[writer].kind == UnitKind::Level {
            return self.units[writer].level == Some(level);
        }
        let packages = self.effective_packages(level);
        let Some(package) = packages.entry(writer) else { return false };
        match package.place {
            Place::Level(at, _, _) => at == level,
            Place::Root(_, _) => {
                let at = package.decl.as_ref().and_then(|decl| self.units[self.use_writer(decl)].level);
                self.doc.file_level(at.unwrap_or(level)) == level
            }
        }
    }

    pub(super) fn selection_reaches(&mut self, scene: SceneId, object: ObjId) -> bool {
        let SetKind::Select { params, writer, site, .. } = self.set(object).kind.clone() else {
            return true;
        };
        self.selection_at_home(scene, writer, site.as_deref())
            || site.as_ref().is_some_and(|site| site.recursive_selection)
            || params.get("recursive").and_then(Lit::boolean) == Some(true)
    }

    pub(super) fn rule_reaches(&mut self, ctx: &Ctx, inputs: impl IntoIterator<Item = ObjId>) -> bool {
        let Some(scene) = ctx.scene else {
            return true;
        };
        if self.selection_at_home(scene, ctx.unit, Some(ctx)) {
            return true;
        }
        if ctx.written_selection {
            return ctx.recursive_selection;
        }
        let mut selected = false;
        for input in inputs {
            let reach = self.set_reach(scene, input, &mut HashSet::new());
            if reach.reaches {
                return true;
            }
            selected |= reach.selected;
        }
        !selected
    }

    pub(super) fn set_reach(&mut self, scene: SceneId, object: ObjId, seen: &mut HashSet<ObjId>) -> SetReach {
        if !seen.insert(object) {
            return SetReach::default();
        }
        let ObjData::Set(set) = self.o(object).clone() else {
            seen.remove(&object);
            return SetReach::default();
        };
        let mut result = match set.kind {
            SetKind::Select { .. } => SetReach { selected: true, reaches: self.selection_reaches(scene, object) },
            SetKind::Union { source } => self.set_reach(scene, source, seen),
            SetKind::Plain | SetKind::Constraints { .. } => {
                let mut result = SetReach::default();
                for (member, _) in set.items {
                    let child = self.set_reach(scene, member, seen);
                    result.selected |= child.selected;
                    result.reaches |= child.reaches;
                }
                result
            }
            SetKind::Gen { .. } => {
                let reaches = set.reach_sources.iter().any(|source| self.set_reach(scene, *source, seen).reaches);
                SetReach { selected: true, reaches }
            }
        };
        if let Some(site) = &set.reach_site {
            let home = self.selection_at_home(scene, site.unit, Some(site));
            if site.written_selection {
                result.reaches = home || site.recursive_selection;
            } else {
                result.reaches |= home;
            }
        }
        for edit in set.mods {
            let source = self.set_reach(scene, edit.source, seen);
            result.selected |= source.selected;
            if edit.add {
                result.reaches |= if edit.ctx.written_selection {
                    self.selection_at_home(scene, edit.ctx.unit, Some(&edit.ctx)) || edit.ctx.recursive_selection
                } else {
                    source.reaches
                };
            }
        }
        seen.remove(&object);
        result
    }
}
