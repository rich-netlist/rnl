//! Kind conversions for a particular input, without changing the object passed elsewhere (5.2.4).

use std::rc::Rc;

use crate::diag::codes;

use super::ctx::Ctx;
use super::eval::R;
use super::obj::{InputMember, ObjData, ObjId, SceneId, SetData};
use super::Interp;

#[derive(Clone)]
pub(crate) struct RoleInput {
    pub source: ObjId,
    pub resolved: Option<ObjId>,
    pub scene: SceneId,
    pub kinds: Vec<String>,
    pub collection: bool,
    pub allow_set: bool,
    pub site: Rc<Ctx>,
    pub off: usize,
    pub role: String,
}

impl Interp<'_> {
    /// A later failure must still reach a strict consumer; it was a member when creation ran.
    pub(crate) fn late_input(&self, object: ObjId) -> bool {
        self.role_inputs.contains_key(&object) || matches!(self.o(object), ObjData::NamedSubobject { .. })
    }

    /// Internal input views have no additional language-level identity.
    pub(crate) fn written_input(&self, mut object: ObjId) -> ObjId {
        while let Some(view) = self.role_inputs.get(&object) {
            object = view.source;
        }
        object
    }

    pub(crate) fn input_value(&self, mut object: ObjId) -> ObjId {
        while let Some(view) = self.role_inputs.get(&object) {
            object = view.resolved.unwrap_or(view.source);
        }
        object
    }

    pub(super) fn role_kind_matches(&self, object: ObjId, wanted: &str) -> bool {
        let actual = self.kind_of(object);
        actual == wanted
            || (wanted == "net" && self.is_empty_node(object))
            || self.obj_kind(object) == Some(wanted)
            || (wanted == "instance" && actual == "bundle")
            || (wanted == "point" && actual == "position")
    }

    pub(super) fn conversion_kind(&self, object: ObjId, kinds: &[String]) -> Option<String> {
        if kinds.iter().any(|kind| self.role_kind_matches(object, kind)) {
            return Some(self.kind_of(object).into());
        }
        let actual = self.kind_of(object);
        kinds
            .iter()
            .find(|kind| {
                matches!(
                    (actual, kind.as_str()),
                    ("terminal", "pin" | "point" | "net") | ("pin", "point" | "terminal" | "net")
                )
            })
            .cloned()
    }

    /// Recognize a point role without performing the checks deferred until after step 7.
    pub(super) fn role_input_converts_to_point(&self, object: ObjId) -> bool {
        self.role_inputs.get(&object).is_some_and(|input| {
            self.conversion_kind(self.scene_value(input.source), &input.kinds).as_deref() == Some("point")
        })
    }

    pub(crate) fn role_input(
        &mut self,
        object: ObjId,
        kinds: &Option<Vec<String>>,
        role: &str,
        collection: bool,
        ctx: &Ctx,
        off: usize,
    ) -> R<ObjId> {
        let Some(kinds) = kinds else { return Ok(object) };
        let Some(scene) = ctx.scene else { return Ok(object) };
        if !collection && kinds.iter().any(|kind| self.role_kind_matches(object, kind)) {
            return Ok(object);
        }
        // Terminal-to-net needs no symbol, so body creation already sees that net (5.2.4).
        if !collection
            && !self.role_inputs.contains_key(&object)
            && matches!(self.o(object), ObjData::Term { .. })
            && self.conversion_kind(object, kinds).as_deref() == Some("net")
        {
            return self.term_net(scene, object, ctx, off);
        }
        if !collection
            && !matches!(self.o(object), ObjData::NamedSubobject { .. })
            && !self.role_inputs.contains_key(&object)
            && self.conversion_kind(object, kinds).is_none()
        {
            return Ok(object); // The existing role check reports this mismatch.
        }
        let input = self.new_obj(ObjData::Marker);
        self.role_inputs.insert(
            input,
            RoleInput {
                source: object,
                resolved: None,
                scene,
                kinds: kinds.clone(),
                collection,
                allow_set: collection,
                site: Rc::new(ctx.clone()),
                off,
                role: role.to_string(),
            },
        );
        if self.scenes[scene].phase >= 7 {
            self.resolve_role_input(input);
        }
        Ok(input)
    }

    pub(crate) fn resolve_role_inputs(&mut self, scene: SceneId) {
        let mut inputs: Vec<_> = self
            .role_inputs
            .iter()
            .filter(|(_, view)| view.scene == scene && view.resolved.is_none())
            .map(|(&input, _)| input)
            .collect();
        inputs.sort();
        for input in inputs {
            self.resolve_role_input(input);
        }
    }

    pub(super) fn resolve_role_input(&mut self, input: ObjId) {
        let Some(view) = self.role_inputs.get(&input).cloned() else { return };
        if view.resolved.is_some() {
            return;
        }
        if self.role_inputs.contains_key(&view.source) {
            self.resolve_role_input(view.source);
        }
        self.resolve_position_role_inputs(view.source);
        let source = self.scene_value(view.source);
        let spread_source = view.collection || (view.allow_set && self.is_set(source));
        let members = if spread_source {
            self.scene_input_members(view.scene, source, Some(&view.site))
        } else {
            vec![InputMember::plain(source)]
        };
        let mut converted = Vec::new();
        let mut dependencies = Vec::new();
        let mut failure = None;
        for member in members {
            let value = self.scene_value(member.object);
            let result = if let Some(cause) = self
                .failed_obj(value)
                .or_else(|| member.dependencies.iter().find_map(|dependency| self.failed_obj(*dependency)))
            {
                let diagnostic = self.err_cause(
                    &view.site,
                    codes::USAGE_ROLE_FAILED_OBJECT,
                    format!("the role {} received a failed object", view.role),
                    view.off,
                    cause,
                );
                Err(super::eval::Stop::Fail(Some(diagnostic)))
            } else {
                self.convert_role_member(value, &view)
            };
            match result {
                Ok(value) if spread_source && self.is_set(value) => {
                    for mut child in self.scene_input_members(view.scene, value, Some(&view.site)) {
                        child.depend_on(&member.dependencies);
                        converted.push((child.object, true));
                        dependencies.push(child.dependencies);
                    }
                }
                Ok(value) if view.allow_set || !self.is_set(value) => {
                    converted.push((value, true));
                    dependencies.push(member.dependencies);
                }
                Ok(_) => {
                    let diagnostic = self.err(
                        &view.site,
                        codes::USAGE_ROLE_WRONG_KIND,
                        format!(
                            "the role {} takes a single object, but its conversion gives a set (5.2.4, 9.4)",
                            view.role
                        ),
                        view.off,
                    );
                    failure = Some(self.new_obj(ObjData::Failed(Some(diagnostic))));
                    break;
                }
                Err(super::eval::Stop::Fail(cause)) => {
                    failure = Some(self.new_obj(ObjData::Failed(cause)));
                    break;
                }
                Err(_) => unreachable!("kind conversion only fails"),
            }
        }
        let value = failure.unwrap_or_else(|| {
            if spread_source {
                let mut data = SetData::plain(converted, false);
                data.item_dependencies = dependencies;
                self.new_obj(ObjData::Set(Box::new(data)))
            } else {
                converted[0].0
            }
        });
        self.role_inputs.get_mut(&input).unwrap().resolved = Some(value);
        self.subobject_inputs += 1;
    }

    fn convert_role_member(&mut self, object: ObjId, view: &RoleInput) -> R<ObjId> {
        if view.kinds.iter().any(|kind| self.role_kind_matches(object, kind)) {
            return Ok(object);
        }
        let target = self.conversion_kind(object, &view.kinds);
        let value = self.convert_member(object, target.as_deref(), view.scene, &view.site, view.off)?;
        match value {
            Some(value) => Ok(value),
            None => self.fail(
                &view.site,
                codes::USAGE_ROLE_WRONG_KIND,
                format!(
                    "the role {} takes {}, but {} ({}) cannot convert to it (5.2.4, 9.4)",
                    view.role,
                    view.kinds.join("/"),
                    self.label(object),
                    self.kind_of(object)
                ),
                view.off,
            ),
        }
    }

    /// Apply a conversion after its required symbols are known. The consumer reports a mismatch.
    pub(crate) fn convert_member(
        &mut self,
        object: ObjId,
        target: Option<&str>,
        scene: SceneId,
        site: &Ctx,
        off: usize,
    ) -> R<Option<ObjId>> {
        if target.is_some_and(|kind| self.role_kind_matches(object, kind)) {
            return Ok(Some(object));
        }
        let value = match (self.o(object).clone(), target) {
            (ObjData::Term { owner, .. }, Some("pin" | "point")) => self.pin_for(scene, owner, object).map(|index| {
                let pin = self.scene_pin_object(scene, owner, index);
                if target == Some("point") {
                    self.scene_pos_of(scene, pin)
                } else {
                    pin
                }
            }),
            (ObjData::Term { .. }, Some("net")) => {
                return self.term_net(scene, object, site, off).map(Some);
            }
            (ObjData::ScenePin { .. }, Some("point")) => Some(self.scene_pos_of(scene, object)),
            (ObjData::ScenePin { owner, index }, Some("terminal" | "net")) => {
                let values = if target == Some("net") {
                    self.scene_pin_nets(scene, owner, index)
                } else {
                    self.scene_pin_terminals(scene, owner, index)
                };
                if values.is_empty() {
                    (target == Some("net") && self.scene_pin_has_empty_node(scene, owner, index))
                        .then(|| self.new_obj(ObjData::EmptyNode { direct: false }))
                } else if values.len() == 1 && !self.scene_pin_is_bus(scene, owner, index) {
                    Some(values[0])
                } else {
                    Some(self.new_obj(ObjData::Set(Box::new(SetData::plain(
                        values.into_iter().map(|v| (v, true)).collect(),
                        false,
                    )))))
                }
            }
            _ => None,
        };
        Ok(value)
    }
}
