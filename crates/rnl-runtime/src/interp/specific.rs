//! Specific definitions keep their identity while each inclusion is expanded from its root (9.1).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::diag::{codes, DiagId};
use crate::syntax::SetExpr;

use super::ctx::{Ctx, Defining};
use super::eval::{Stop, R};
use super::kw::MAX_SYMBOL_LEVELS;
use super::obj::{ObjData, ObjId, StyleMap};
use super::Interp;

#[derive(Clone)]
struct Recipe {
    seed: ObjData,
    body: Option<Rc<SetExpr>>,
    ctx: Ctx,
    defining: Rc<Defining>,
    off: usize,
}

struct Active {
    object: ObjId,
    dependencies: HashSet<ObjId>,
    limit: Option<DiagId>,
    closed_cycle: bool,
}

struct Content {
    value: R<ObjId>,
    dependencies: HashSet<ObjId>,
}

#[derive(Default)]
pub(super) struct SpecificDefinitions {
    recipes: HashMap<ObjId, Recipe>,
    pending: Vec<ObjId>,
    checked: usize,
    active: Vec<Active>,
    roots: HashMap<ObjId, Content>,
    inclusions: HashMap<ObjId, Vec<Content>>,
    finishing: bool,
}

impl Interp<'_> {
    pub(super) fn specific_pending(&self, object: ObjId) -> bool {
        self.specific.recipes.contains_key(&object) && !self.specific.roots.contains_key(&object)
    }

    /// Publish the handle before reading its body. A reference can then find the
    /// definition even when an inclusion eventually comes back to it.
    pub(super) fn defer_specific(
        &mut self,
        object: ObjId,
        body: Option<Rc<SetExpr>>,
        ctx: &Ctx,
        defining: Rc<Defining>,
        off: usize,
    ) {
        let recipe = Recipe { seed: self.o(object).clone(), body, ctx: ctx.clone(), defining, off };
        self.specific.recipes.insert(object, recipe);
        self.specific.pending.push(object);
    }

    /// Check unused definitions too, after their names have been committed. A
    /// dependency encountered inside another body is checked independently later.
    pub(super) fn finish_specific_definitions(&mut self) {
        if self.specific.finishing || !self.specific.active.is_empty() || !self.in_progress.is_empty() {
            return;
        }
        self.specific.finishing = true;
        while self.specific.checked < self.specific.pending.len() {
            let object = self.specific.pending[self.specific.checked];
            self.specific.checked += 1;
            let recipe = self.specific.recipes[&object].clone();
            let _ = self.specific_content(object, &recipe.ctx, recipe.off);
        }
        self.specific.finishing = false;
    }

    /// Return the content for this inclusion path, never the truncated content
    /// cached while a different root was being made. A completed acyclic portion
    /// can be shared when none of its dependencies is an ancestor of this use.
    pub(super) fn specific_content(&mut self, object: ObjId, ctx: &Ctx, off: usize) -> R<ObjId> {
        let Some(recipe) = self.specific.recipes.get(&object).cloned() else { return Ok(object) };
        if let Some(parent) = self.specific.active.last_mut() {
            parent.dependencies.insert(object);
        }
        if self.specific.active.iter().any(|entry| entry.object == object) {
            let chain = self
                .specific
                .active
                .iter()
                .map(|entry| self.label(entry.object))
                .chain(std::iter::once(self.label(object)))
                .collect::<Vec<_>>()
                .join(" -> ");
            for entry in &mut self.specific.active {
                entry.closed_cycle = true;
            }
            return self.fail(ctx, codes::DEFINE_INCLUSION_CYCLE,
                format!("this inclusion closes a cycle: {chain}; the inclusion fails and the rest of the body takes effect (9.1)"), off);
        }
        if let Some(content) = self.specific.roots.get(&object) {
            if self.specific.active.iter().all(|entry| !content.dependencies.contains(&entry.object)) {
                let value = content.value.clone();
                let dependencies = content.dependencies.clone();
                if let Some(parent) = self.specific.active.last_mut() {
                    parent.dependencies.extend(dependencies);
                }
                return value;
            }
        }
        if !self.specific.active.is_empty() {
            if let Some(content) = self.specific.inclusions.get(&object).and_then(|contents| {
                contents.iter().find(|content| {
                    self.specific.active.iter().all(|entry| !content.dependencies.contains(&entry.object))
                })
            }) {
                let value = content.value.clone();
                let dependencies = content.dependencies.clone();
                self.specific.active.last_mut().unwrap().dependencies.extend(dependencies);
                return value;
            }
        }
        if self.specific.active.len() >= MAX_SYMBOL_LEVELS {
            let (code, message) = if matches!(recipe.seed, ObjData::Symbol(_)) {
                (codes::DEFINE_SYMBOL_EXPANSION_LIMIT,
                    format!("symbol expansion exceeds this implementation's limit of {MAX_SYMBOL_LEVELS} active symbol bodies (9.2.7)"))
            } else {
                (codes::DEFINE_STYLE_EXPANSION_LIMIT,
                    format!("style expansion exceeds this implementation's limit of {MAX_SYMBOL_LEVELS} active style bodies (9.1, 9.3)"))
            };
            let diagnostic = self.err(ctx, code, message, off);
            for entry in &mut self.specific.active {
                entry.limit.get_or_insert(diagnostic);
            }
            return Err(Stop::Fail(Some(diagnostic)));
        }
        let root = self.specific.active.is_empty();
        let target = if root {
            *self.om(object) = recipe.seed.clone();
            object
        } else {
            let target = self.new_obj(recipe.seed.clone());
            self.objs[target.0 as usize].name = self.objs[object.0 as usize].name.clone();
            target
        };
        self.specific.active.push(Active { object, dependencies: HashSet::new(), limit: None, closed_cycle: false });
        let evaluated = match &recipe.seed {
            ObjData::Symbol(_) => self.specific_symbol_body(target, &recipe),
            ObjData::Style(own) => self.specific_style_body(target, own, &recipe),
            _ => unreachable!("only symbols and styles have specific bodies"),
        };
        let active = self.specific.active.pop().unwrap();
        let result = active.limit.map_or(evaluated, |id| Err(Stop::Fail(Some(id)))).map(|()| target);
        if let Some(parent) = self.specific.active.last_mut() {
            parent.dependencies.extend(active.dependencies.iter().copied());
        }
        if root {
            if let Err(Stop::Fail(diagnostic)) = &result {
                *self.om(object) = ObjData::Failed(*diagnostic);
                self.failed_specific_creation(object);
            }
            self.specific.roots.insert(object, Content { value: result.clone(), dependencies: active.dependencies });
        } else if result.is_ok() && !active.closed_cycle {
            self.specific
                .inclusions
                .entry(object)
                .or_default()
                .push(Content { value: result.clone(), dependencies: active.dependencies });
        }
        result
    }

    fn specific_symbol_body(&mut self, symbol: ObjId, recipe: &Recipe) -> R<()> {
        if let Some(body) = &recipe.body {
            self.eval_symbol_body(symbol, body, &recipe.ctx, recipe.defining.clone())?;
            self.check_symbol_graph(symbol, &recipe.ctx, recipe.off)?;
            for usage in self.symbol(symbol).usages.clone() {
                let usage = self.usage(usage).clone();
                if usage.own_of == Some(symbol) && usage.sym_symbol.is_none() {
                    let label = self.label(usage.definition);
                    self.warn(usage.site.as_ref().unwrap(), codes::USAGE_IN_SYMBOL_NOT_DRAWN,
                        format!("the usage of {label} in a symbol body is given no symbol with SET-SYMBOL and is not drawn (9.2.7)"), usage.off);
                }
            }
        }
        Ok(())
    }

    fn specific_style_body(&mut self, target: ObjId, own: &StyleMap, recipe: &Recipe) -> R<()> {
        let mut props = StyleMap::new();
        let mut ctx = recipe.ctx.clone();
        ctx.defining = Some(recipe.defining.clone());
        if let Some(body) = &recipe.body {
            for member in &body.members {
                if let Some(error) = &member.error {
                    self.err(&ctx, error.code, error.message.clone(), member.error_pos);
                    continue;
                }
                for segment in &member.segments {
                    if segment.call.is_some() || member.bind().is_some() || member.segments.len() != 1 {
                        self.err(
                            &ctx,
                            codes::STYLE_BODY_NOT_STYLE,
                            "the set of DEFINE-STYLE only references other styles (9.3)",
                            member.start,
                        );
                        break;
                    }
                    for arg in &segment.args {
                        let Ok((object, _)) = self.eval_obj(&arg.obj, &ctx) else { continue };
                        if matches!(self.o(object), ObjData::Style(_)) {
                            let Ok(object) = self.specific_content(object, &ctx, arg.start) else { continue };
                            if let ObjData::Style(style) = self.o(object) {
                                props.extend(style.clone());
                            }
                        } else {
                            let label = self.label(object);
                            self.err(
                                &ctx,
                                codes::STYLE_BODY_NOT_STYLE,
                                format!(
                                    "{label} is not a style; the body of DEFINE-STYLE only references styles (9.3)"
                                ),
                                arg.start,
                            );
                        }
                    }
                }
            }
        }
        props.extend(own.clone());
        *self.om(target) = ObjData::Style(props);
        Ok(())
    }
}
