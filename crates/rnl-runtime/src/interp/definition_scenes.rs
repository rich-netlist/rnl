//! Resolve free body names after creation without forcing forward object creation (6.9).
//!
//! As for member-place dependencies, only source sites survive a provisional interpretation.
//! An invalid reference is reported at its definition on the next reading, before any of
//! its statement's objects are created. Object handles and provisional diagnostics do not survive.
use std::collections::HashMap;
use std::rc::Rc;

use super::ctx::Ctx;
use super::eval::{ptr, Found};
use super::obj::{DefData, MemberKey, MemberRef, ObjData, RecId, SceneId, UnitId};
use super::unit::Binding;
use super::{Interp, Memo};
use crate::diag::codes;
use crate::syntax::{Member, ObjExpr, Ref, SetExpr};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Site {
    unit: UnitId,
    rec: Option<RecId>,
    definition: usize,
    reference: usize,
}

struct Query {
    site: Site,
    binding: Binding,
    written: String,
}

#[derive(Default)]
pub(super) struct DefinitionScenes {
    pub plans: HashMap<Site, String>,
    queries: Vec<Query>,
}

impl Interp<'_> {
    /// Nested declarations still have argument-independent free names. Keep their
    /// local scopes separate without evaluating their bodies or role arguments.
    pub(super) fn check_definition_scene_names(&mut self, d: &DefData, ctx: &Ctx) {
        fn body(interp: &mut Interp<'_>, d: &DefData, set: &SetExpr, ctx: &Ctx, outer: &[(char, String)]) {
            let mut locals = outer.to_vec();
            for member in &set.members {
                locals.extend(member.binds.iter().map(|b| (b.prefix, b.name.clone())));
            }
            for statement in &set.members {
                member(interp, d, statement, ctx, &locals);
            }
        }
        fn member(interp: &mut Interp<'_>, d: &DefData, statement: &Rc<Member>, ctx: &Ctx, locals: &[(char, String)]) {
            for (index, segment) in statement.segments.iter().enumerate() {
                for argument in &segment.args {
                    match &argument.obj {
                        ObjExpr::Ref(reference) if matches!(reference.prefix, '$' | '%') => {
                            if !locals.iter().any(|(prefix, name)| {
                                *prefix == reference.prefix && Some(name) == reference.name.as_ref()
                            }) {
                                interp.check_definition_scene_reference(d, reference, ctx);
                            }
                        }
                        ObjExpr::Set(set) => {
                            let owns_body = segment.call.as_ref().is_some_and(|call| {
                                matches!(
                                    interp.kw(&call.kw, ctx.unit).as_str(),
                                    "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE"
                                )
                            });
                            if owns_body {
                                // A declaration has its own locals and hides the names of
                                // its result, just as defining_for does when it is created.
                                let mut nested = d.clone();
                                let mut defining = (*d.defining).clone();
                                for binding in statement.binds.iter().filter(|b| b.after == index) {
                                    defining.insert(
                                        (binding.prefix, binding.name.clone()),
                                        (MemberRef { rec: d.rec, m: statement.clone() }, d.def_unit),
                                    );
                                }
                                nested.defining = Rc::new(defining);
                                nested.off = segment.start;
                                body(interp, &nested, set, ctx, &[]);
                            } else {
                                body(interp, d, set, ctx, locals);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Only this statement is omitted; enclosing tolerant sets survive.
            for segment in &statement.segments {
                for argument in &segment.args {
                    if let ObjExpr::Ref(reference) = &argument.obj {
                        if let Some(&diagnostic) = interp.body_errors.get(&ptr(reference)) {
                            interp.body_member_errors.entry(ptr(&**statement)).or_insert(diagnostic);
                        }
                    }
                }
            }
        }
        if let Some(set) = &d.body {
            body(self, d, set, ctx, &[]);
        }
    }

    pub(super) fn forbidden_body_scene_value(&self, prefix: char, value: super::obj::ObjId) -> bool {
        match self.o(value) {
            ObjData::Set(_) => prefix == '%',
            _ => self.scene_bound(value),
        }
    }

    pub(super) fn check_definition_scene_reference(&mut self, d: &DefData, r: &Ref, ctx: &Ctx) {
        if self.body_errors.contains_key(&ptr(r)) {
            return;
        }
        let site = Site { unit: d.def_unit, rec: d.rec, definition: d.off, reference: r.start };
        if let Some(message) = self.definition_scenes.plans.get(&site).cloned() {
            if !self.body_errors.contains_key(&ptr(r)) {
                let diagnostic = self.err(ctx, codes::DEFINE_BODY_SCENE_REF, message, r.start);
                self.body_errors.insert(ptr(r), diagnostic);
                self.note_failed_at(ctx.rec, r.start, false);
            }
            return;
        }
        let Some(name) = &r.name else { return };
        let binding = if let Some((member, unit)) = d.defining.get(&(r.prefix, name.clone())) {
            match self.outer_lookup(r.prefix, name, member, *unit, ctx.scene) {
                Found::Binding(binding) => Some(binding),
                _ => None,
            }
        } else {
            self.lookup_unit(r.prefix, name, d.def_unit, false, ctx.scene)
        };
        if let Some(binding) = binding {
            self.definition_scenes.queries.push(Query { site, binding, written: r.text() });
        }
    }

    pub(super) fn settle_definition_scenes(&mut self) -> bool {
        let mut changed = false;
        let mut contexts: HashMap<MemberKey, Vec<Option<SceneId>>> = HashMap::new();
        for ((member, scene), value) in &self.memo {
            if matches!(value, Memo::Val(_)) {
                contexts.entry(member.clone()).or_default().push(*scene);
            }
        }
        for query in std::mem::take(&mut self.definition_scenes.queries) {
            let scenes: Vec<_> = self
                .binding_candidates(&query.binding)
                .iter()
                .flat_map(|binding| contexts.get(&binding.key()).into_iter().flatten().copied())
                .collect();
            // No selection or member list is evaluated to decide whether the name is allowed.
            let forbidden = scenes.into_iter().any(|scene| {
                let Some(value) = self.evaluated_binding(&query.binding, scene) else { return false };
                self.forbidden_body_scene_value(query.binding.prefix, value)
            });
            if forbidden && !self.definition_scenes.plans.contains_key(&query.site) {
                self.definition_scenes.plans.insert(query.site,format!("{} refers to an object of the drawing level; scene objects enter a definition body through roles (6.9)",query.written));
                changed = true;
            }
        }
        changed
    }
}
