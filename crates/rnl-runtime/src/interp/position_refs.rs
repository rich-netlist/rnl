//! Explicit position sub-objects use the same position kinds as conversions (5.2.4, 6.5).

use crate::diag::codes;

use super::ctx::Ctx;
use super::eval::R;
use super::obj::{ObjData, ObjId, SceneId, Shape};
use super::Interp;

impl Interp<'_> {
    pub(crate) fn position_kind(&self, object: ObjId) -> bool {
        match self.o(self.scene_value(object)) {
            ObjData::Inst { .. }
            | ObjData::Bundle { .. }
            | ObjData::Usage(_)
            | ObjData::Term { .. }
            | ObjData::SymbolPin { .. }
            | ObjData::ScenePin { .. }
            | ObjData::Point { .. }
            | ObjData::Text { .. }
            | ObjData::KeyText { .. }
            | ObjData::SymLabel { .. }
            | ObjData::Annotation(_) => true,
            ObjData::Pos { parent } => self.position_kind(*parent),
            ObjData::Prim {
                shape: Shape::Rectangle | Shape::Circle | Shape::Ellipse | Shape::Text, points, ..
            } => points.len() == 1,
            _ => false,
        }
    }

    pub(super) fn position_subobject(&mut self, parent: ObjId, ctx: &Ctx, off: usize) -> R<ObjId> {
        if let Some(scene) = ctx.scene {
            if self.late_input(parent) || matches!(self.o(parent), ObjData::Term { .. }) {
                // A written reference keeps its parent and name. Its later value is a position,
                // never the physical pin itself, so it does not connect a wire to that pin.
                return Ok(self.scene_named_subobject(scene, parent, "position", ctx, off));
            }
        }
        if !self.position_kind(parent) {
            return self.fail(
                ctx,
                codes::REF_NO_SUBOBJECT,
                format!("{} ({}) has no position sub-object (5.2.4, 6.5)", self.label(parent), self.kind_of(parent)),
                off,
            );
        }
        Ok(match ctx.scene {
            Some(scene) => self.scene_pos_of(scene, parent),
            None => self.new_obj(ObjData::Pos { parent }),
        })
    }

    pub(super) fn position_waits_for_role(&self, object: ObjId) -> bool {
        if let Some(view) = self.role_inputs.get(&object) {
            if view.resolved.is_none() {
                return true;
            }
        }
        match self.o(object) {
            ObjData::NamedSubobject { parent, .. } | ObjData::Pos { parent } => self.position_waits_for_role(*parent),
            _ => false,
        }
    }

    /// Used by a dependent role check once role resolution is running. Do not force these
    /// conversions in the ordinary symbol queue: earlier creation is never repeated (8.2).
    pub(super) fn resolve_position_role_inputs(&mut self, object: ObjId) {
        if self.role_inputs.contains_key(&object) {
            self.resolve_role_input(object);
        }
        if let ObjData::NamedSubobject { parent, name, .. } = self.o(object).clone() {
            if name == "position" {
                self.resolve_position_role_inputs(parent);
                let scene = match self.o(object) {
                    ObjData::NamedSubobject { site, .. } => site.scene,
                    _ => None,
                };
                if let Some(scene) = scene {
                    self.resolve_position_reference(scene, object);
                }
            }
        }
    }

    pub(super) fn resolve_position_reference(&mut self, scene: SceneId, object: ObjId) {
        let ObjData::NamedSubobject { parent, .. } = self.o(object).clone() else { return };
        if self.position_waits_for_role(parent) {
            return;
        }
        let value = self.scene_value(parent);
        let valid = match *self.o(value) {
            ObjData::Term { owner, .. } => {
                self.pin_for(scene, owner, value).is_some()
                    || !self.scenes[scene].symbol.contains_key(&owner)
                        && matches!(self.o(owner), ObjData::Inst { .. } | ObjData::Bundle { .. })
            }
            _ => self.position_kind(value),
        };
        // The written parent matters: p::position and alias::position remain separate points
        // even when p and alias ultimately refer to one physical pin (5.7, 6.5).
        let position = valid.then(|| self.scene_pos_of(scene, parent));
        if let ObjData::NamedSubobject { resolved, .. } = self.om(object) {
            *resolved = position;
        }
    }
}
