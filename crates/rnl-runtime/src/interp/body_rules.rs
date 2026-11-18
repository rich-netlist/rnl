//! Rules in an ordinary definition follow its usage without creating it again (10.4).
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::ctx::{Ctx, FrameKind, FrameRef};
use super::obj::{MemberKey, ObjId, SceneId};
use super::Interp;
use crate::syntax::Member;

#[derive(Clone)]
struct BodyRule {
    home: SceneId,
    usage: ObjId,
    member: Rc<Member>,
    index: usize,
    context: Ctx,
}

#[derive(Default)]
pub(super) struct BodyRules {
    marked: HashSet<(ObjId, usize)>,
    seen: HashSet<(ObjId, usize)>,
    rules: Vec<BodyRule>,
    frames: HashMap<(ObjId, SceneId), FrameRef>,
}

impl Interp<'_> {
    /// Keep a failed nested member separate from its enclosing, tolerant member set.
    pub(super) fn mark_failed_body_members(&mut self, definition: ObjId, member: &Member, ctx: &Ctx) {
        use crate::syntax::{Coord, ObjExpr, Value};
        fn literal_error(interp: &Interp<'_>, definition: ObjId, value: &Value) -> Option<crate::diag::DiagId> {
            match value {
                Value::Hole(name) => interp.body_parameter_errors.get(&(definition, name.clone())).copied(),
                Value::Arr(values) => values.iter().find_map(|value| literal_error(interp, definition, value)),
                _ => None,
            }
        }
        let key = super::eval::ptr(member);
        let mut diagnostic = self.body_member_errors.get(&key).copied();
        if let Some(error) = &member.error {
            if diagnostic.is_none() {
                diagnostic = Some(self.err(ctx, error.code, error.message.clone(), member.error_pos));
            }
        }
        for segment in &member.segments {
            if let Some(call) = &segment.call {
                diagnostic = diagnostic.or_else(|| self.body_errors.get(&super::eval::ptr(call)).copied());
                if let Some(params) = &call.params {
                    for value in params.positional.iter().chain(params.named.iter().map(|named| &named.value)) {
                        diagnostic = diagnostic.or_else(|| literal_error(self, definition, value));
                    }
                }
                if matches!(
                    self.kw(&call.kw, ctx.unit).as_str(),
                    "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE"
                ) {
                    continue;
                }
            }
            for argument in &segment.args {
                match &argument.obj {
                    ObjExpr::Ref(reference) => {
                        diagnostic = diagnostic.or_else(|| self.body_errors.get(&super::eval::ptr(reference)).copied());
                    }
                    ObjExpr::Point(point) => {
                        for coord in [&point.x, &point.y] {
                            if let Coord::Hole(name) = coord {
                                diagnostic = diagnostic
                                    .or_else(|| self.body_parameter_errors.get(&(definition, name.clone())).copied());
                            }
                        }
                    }
                    ObjExpr::Set(set) => {
                        for member in &set.members {
                            self.mark_failed_body_members(definition, member, ctx);
                        }
                    }
                }
            }
        }
        if let Some(diagnostic) = diagnostic {
            self.body_member_errors.entry(key).or_insert(diagnostic);
        }
    }

    pub(super) fn check_body_level_symbols(&mut self, member: &Member, ctx: &Ctx) {
        use crate::syntax::ObjExpr;
        let mut level = false;
        for segment in &member.segments {
            if !segment.piped {
                level = matches!(segment.args.first().map(|a| &a.obj), Some(ObjExpr::Ref(reference)) if reference.is_bare_level());
            }
            if let Some(call) = &segment.call {
                let keyword = self.kw(&call.kw, ctx.unit);
                if level && keyword == "SET-SYMBOL" {
                    let diagnostic = self.err(
                        ctx,
                        crate::diag::codes::DEFINE_BODY_BLOCK_SYMBOL,
                        "a definition body cannot set the block symbol of its level (6.9, 9.2.6)",
                        call.start,
                    );
                    self.body_errors.insert(super::eval::ptr(call), diagnostic);
                    self.note_failed_at(ctx.rec, call.start, false);
                }
                if matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE") {
                    continue;
                }
                if crate::words::words().keyword(&keyword).is_some_and(|signature| signature.output != "pass") {
                    level = false;
                }
            }
            for argument in &segment.args {
                if let ObjExpr::Set(set) = &argument.obj {
                    for member in &set.members {
                        self.check_body_level_symbols(member, ctx);
                    }
                }
            }
        }
    }

    pub(super) fn foreign_body_object(&self, object: ObjId, ctx: &Ctx) -> bool {
        use super::obj::ObjData;
        let Some(scene) = ctx.scene else {
            return false;
        };
        if ctx.body_scene.is_none() || ctx.body_scene == ctx.scene {
            return false;
        }
        if let Some(input) = self.role_inputs.get(&object) {
            return self.foreign_body_object(input.source, ctx);
        }
        match self.o(object) {
            ObjData::Pos { parent } | ObjData::NamedSubobject { parent, .. } => self.foreign_body_object(*parent, ctx),
            ObjData::ScenePin { owner, .. } | ObjData::Term { owner, .. } => self.foreign_body_object(*owner, ctx),
            ObjData::KeyText { attach, .. } => self.foreign_body_object(*attach, ctx),
            ObjData::Set(_) => false,
            _ => self.scene_bound(object) && !self.belongs(scene, object),
        }
    }

    pub(super) fn note_rule(&mut self, ctx: &Ctx) {
        if ctx.frame_kind() == Some(FrameKind::Definition) {
            if let (Some(usage), Some(statement)) = (ctx.frame.as_ref().and_then(|f| f.borrow().usage), ctx.stmt) {
                self.body_rules.marked.insert((usage, statement.start));
            }
        } else {
            self.rules.insert(ctx.member_key.clone());
        }
    }

    pub(super) fn remember_body_rule(&mut self, member: &Rc<Member>, index: usize, ctx: &Ctx) {
        let Some(usage) = ctx.frame.as_ref().and_then(|frame| frame.borrow().usage) else {
            return;
        };
        let key = (usage, member.start);
        if !self.body_rules.marked.remove(&key) || !self.body_rules.seen.insert(key) {
            return;
        }
        let Some(home) = self.usage(usage).site.as_ref().and_then(|site| site.scene) else {
            return;
        };
        self.body_rules.rules.push(BodyRule { home, usage, member: member.clone(), index, context: ctx.clone() });
    }

    pub(super) fn inherited_body_rules(&mut self, scene: SceneId) {
        let ancestors = self.doc.ancestors(self.scenes[scene].level);
        let mut rules: Vec<_> = self
            .body_rules
            .rules
            .iter()
            .filter(|rule| ancestors.contains(&self.scenes[rule.home].level))
            .cloned()
            .collect();
        rules.sort_by(|a, b| (&a.context.pos, a.index).cmp(&(&b.context.pos, b.index)));
        for rule in rules {
            let key = (rule.usage, scene);
            let frame = match self.body_rules.frames.get(&key) {
                Some(frame) => frame.clone(),
                None => {
                    let original = self.usage(rule.usage).frame.as_ref().unwrap();
                    let mut frame = original.borrow().clone();
                    let names: HashSet<_> = self
                        .body_rules
                        .rules
                        .iter()
                        .filter(|r| r.usage == rule.usage)
                        .flat_map(|r| r.member.binds.iter().filter(|b| b.prefix == '%').map(|b| b.name.clone()))
                        .collect();
                    frame.locals.retain(|(name, _)| !names.contains(name));
                    frame.local_dependencies.retain(|name, _| !names.contains(name));
                    frame.local_bindings.retain(|name, _| !names.contains(name));
                    frame.local_binding_errors.clear();
                    for binding in self.body_rules.rules.iter().filter(|r| r.usage == rule.usage) {
                        frame.evaluated_members.remove(&binding.index);
                    }
                    let frame = Rc::new(RefCell::new(frame));
                    self.body_rules.frames.insert(key, frame.clone());
                    frame
                }
            };
            let mut ctx = rule.context.clone();
            ctx.scene = Some(scene);
            ctx.body_scene = Some(rule.home);
            ctx.frame = Some(frame);
            if let MemberKey::M { unit, rec, k, embedded } = ctx.member_key.clone() {
                let index = (!embedded).then_some(k);
                let before = self.position_of(rule.home, unit, Some(rec), index);
                let mut after = self.position_of(scene, unit, Some(rec), index);
                if ctx.pos.starts_with(&before) {
                    after.extend_from_slice(&ctx.pos[before.len()..]);
                    ctx.pos = after;
                }
            }
            let _ = self.definition_member(&rule.member, rule.index, &ctx);
        }
    }
}
