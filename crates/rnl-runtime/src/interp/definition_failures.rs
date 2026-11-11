//! Failures of fixed local names propagate once at their definition (6.9).
use std::collections::HashMap;

use super::ctx::Ctx;
use super::eval::ptr;
use super::obj::DefData;
use super::Interp;
use crate::diag::{codes, DiagId};
use crate::syntax::{Member, ObjExpr, SetExpr};

type Scope = HashMap<(char, String), Option<DiagId>>;

impl Interp<'_> {
    pub(super) fn check_failed_local_names(&mut self, definition: &DefData, ctx: &Ctx) {
        fn body(interp: &mut Interp<'_>, set: &SetExpr, ctx: &Ctx, scopes: &mut Vec<Scope>) {
            let mut scope = Scope::new();
            for member in &set.members {
                for binding in &member.binds {
                    let cause = interp.body_member_errors.get(&ptr(&**member)).copied();
                    scope
                        .entry((binding.prefix, binding.name.clone()))
                        .and_modify(|old| {
                            // Any possibly valid creation can supply this local name.
                            if cause.is_none() {
                                *old = None;
                            }
                        })
                        .or_insert(cause);
                }
            }
            scopes.push(scope);
            for member in &set.members {
                statement(interp, member, ctx, scopes);
            }
            scopes.pop();
        }
        fn statement(interp: &mut Interp<'_>, member: &Member, ctx: &Ctx, scopes: &mut Vec<Scope>) {
            if interp.body_member_errors.contains_key(&ptr(member)) {
                return;
            }
            for segment in &member.segments {
                let owns_body = segment.call.as_ref().is_some_and(|call| {
                    matches!(
                        interp.kw(&call.kw, ctx.unit).as_str(),
                        "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE"
                    )
                });
                for argument in &segment.args {
                    match &argument.obj {
                        ObjExpr::Ref(reference) if matches!(reference.prefix, '$' | '%') => {
                            let Some(name) = &reference.name else { continue };
                            let cause = scopes
                                .iter()
                                .rev()
                                .find_map(|scope| scope.get(&(reference.prefix, name.clone())))
                                .copied()
                                .flatten();
                            if let Some(cause) = cause {
                                let diagnostic = interp.err_cause(
                                    ctx,
                                    codes::REF_FAILED,
                                    format!("{}{name} failed (7.2, 6.9)", reference.prefix),
                                    reference.start,
                                    Some(cause),
                                );
                                interp.body_errors.insert(ptr(reference), diagnostic);
                                interp.body_member_errors.entry(ptr(member)).or_insert(diagnostic);
                                interp.note_failed_at(ctx.rec, reference.start, false);
                            }
                        }
                        ObjExpr::Set(set) => {
                            if owns_body {
                                body(interp, set, ctx, &mut Vec::new());
                            } else {
                                body(interp, set, ctx, scopes);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        if let Some(set) = &definition.body {
            loop {
                let before = self.body_member_errors.len();
                body(self, set, ctx, &mut Vec::new());
                if self.body_member_errors.len() == before {
                    break;
                }
            }
        }
    }
}
