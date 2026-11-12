//! Application signatures known before a general definition is applied (6.9).

use std::collections::HashSet;
use std::rc::Rc;

use crate::diag::codes;
use crate::syntax::{Arg, Member, ObjExpr, Segment, SetExpr, Value};

use super::ctx::Ctx;
use super::eval::{ptr, Found, R};
use super::kw::parse_definition_fields;
use super::obj::{DefData, Lit, MemberRef, ObjData, Role, WStr};
use super::unit::definition_header_params;
use super::Interp;

struct StaticDefinition {
    data: DefData,
    label: String,
    known_roles: bool,
    known_params: bool,
}

fn literal(value: &Value) -> Option<Lit> {
    Some(match value {
        Value::Num { value, .. } => Lit::Num(*value),
        Value::Str(value) => Lit::Str(WStr::new(value, None)),
        Value::Bool(value) => Lit::Bool(*value),
        Value::Arr(values) => Lit::Arr(values.iter().map(literal).collect::<Option<_>>()?),
        Value::Hole(_) | Value::Word(_) => return None,
    })
}

fn header_holes_are_parameters(value: &Value, owner: &DefData) -> bool {
    match value {
        Value::Hole(name) => owner.params.iter().any(|(parameter, _)| parameter == name),
        Value::Arr(values) => values.iter().all(|value| header_holes_are_parameters(value, owner)),
        _ => true,
    }
}

impl Interp<'_> {
    /// Check literal nested declarations in their own role and name environment.
    /// The template is never bound or applied; actual creations reuse only its diagnostics.
    pub(super) fn check_nested_definitions(&mut self, owner: &DefData, ctx: &Ctx) {
        fn walk(interp: &mut Interp<'_>, set: &SetExpr, owner: &DefData, ctx: &Ctx) {
            for member in &set.members {
                for (index, segment) in member.segments.iter().enumerate() {
                    let keyword = segment.call.as_ref().map(|call| interp.kw(&call.kw, owner.def_unit));
                    if keyword.as_deref() == Some("DEFINE") {
                        let mut declaration = (**member).clone();
                        declaration.segments = vec![segment.clone()];
                        declaration.binds.retain(|binding| binding.after == index);
                        for binding in &mut declaration.binds {
                            binding.after = 0;
                        }
                        let Some(mut nested) = interp.static_callee_member(
                            &declaration,
                            owner,
                            ctx,
                            &[],
                            String::new(),
                            &mut HashSet::new(),
                        ) else {
                            continue;
                        };
                        let body = nested.data.body.as_ref().unwrap();
                        let key = (owner.def_unit, ptr(&**body));
                        if interp.static_nested_definitions.contains_key(&key) {
                            continue;
                        }
                        let mut defining = (*owner.defining).clone();
                        for binding in &declaration.binds {
                            defining.insert(
                                (binding.prefix, binding.name.clone()),
                                (MemberRef { rec: owner.rec, m: member.clone() }, owner.def_unit),
                            );
                        }
                        nested.data.defining = Rc::new(defining);
                        let id = interp.new_obj(ObjData::Definition(Box::new(nested.data)));
                        interp.static_nested_definitions.insert(key, (id, nested.known_roles && nested.known_params));
                        interp.check_definition_with_signature(id, ctx, nested.known_roles, nested.known_params);
                    } else if !matches!(keyword.as_deref(), Some("DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE")) {
                        for argument in &segment.args {
                            if let ObjExpr::Set(set) = &argument.obj {
                                walk(interp, set, owner, ctx);
                            }
                        }
                    }
                }
            }
        }
        if let Some(body) = &owner.body {
            walk(self, body, owner, ctx);
        }
    }

    /// A declaration whose header depends on its enclosing application needs its own
    /// body nodes. Preserve source diagnostics, but never share argument-dependent errors
    /// with another creation from the same text. Source node IDs and locations stay intact.
    pub(super) fn independent_definition_body(&mut self, body: &Rc<SetExpr>, unit: super::obj::UnitId) -> Rc<SetExpr> {
        let mut cloned = (**body).clone();
        cloned.members = body
            .members
            .iter()
            .map(|original| {
                let mut member = (**original).clone();
                for segment in &mut member.segments {
                    for argument in &mut segment.args {
                        if let ObjExpr::Set(set) = &mut argument.obj {
                            *set = self.independent_definition_body(set, unit);
                        }
                    }
                }
                let member = Rc::new(member);
                if let Some(&cause) = self.body_member_errors.get(&ptr(&**original)) {
                    self.body_member_errors.insert(ptr(&*member), cause);
                }
                for (old, new) in original.segments.iter().zip(&member.segments) {
                    if let (Some(old), Some(new)) = (&old.call, &new.call) {
                        if let Some(&cause) = self.body_errors.get(&ptr(old)) {
                            self.body_errors.insert(ptr(new), cause);
                        }
                    }
                    for (old, new) in old.args.iter().zip(&new.args) {
                        if let (ObjExpr::Ref(old), ObjExpr::Ref(new)) = (&old.obj, &new.obj) {
                            if let Some(&cause) = self.body_errors.get(&ptr(old)) {
                                self.body_errors.insert(ptr(new), cause);
                            }
                        }
                    }
                }
                member
            })
            .collect();
        let cloned = Rc::new(cloned);
        if let Some(&checked) = self.static_nested_definitions.get(&(unit, ptr(&**body))) {
            self.static_nested_definitions.insert((unit, ptr(&*cloned)), checked);
        }
        cloned
    }

    /// Inspect declarations without evaluating bodies, creating objects, or forcing selections.
    /// Ambiguous local creation results and argument-dependent headers stay deferred.
    fn static_callee(
        &mut self,
        object: &ObjExpr,
        owner: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
        seen: &mut HashSet<(usize, usize)>,
    ) -> Option<StaticDefinition> {
        let ObjExpr::Ref(reference) = object else { return None };
        if !matches!(reference.prefix, '$' | '%')
            || reference.number.is_some()
            || !reference.path.is_empty()
            || reference.sub.is_some()
            || reference.places.is_some()
        {
            return None;
        }
        let name = reference.name.as_ref()?;
        let local_scopes = scopes
            .iter()
            .enumerate()
            .rev()
            .map(|(index, set)| (*set, index + 1))
            .chain(owner.body.as_deref().filter(|_| reference.prefix == '%').map(|set| (set, 0)));
        for (set, depth) in local_scopes {
            let candidates: Vec<_> = set
                .members
                .iter()
                .flat_map(|member| {
                    member.binds.iter().filter_map(move |binding| {
                        (binding.prefix == reference.prefix && binding.name == *name).then_some((member, binding))
                    })
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            // A later valid creation can win after a failed one. Do not guess its identity.
            if candidates.len() != 1 {
                return None;
            }
            let (member, binding) = candidates[0];
            if binding.after != 0 || self.body_member_errors.contains_key(&ptr(&**member)) {
                return None;
            }
            return self.static_callee_member(
                member,
                owner,
                ctx,
                &scopes[..depth],
                format!("{}{name}", reference.prefix),
                seen,
            );
        }
        let binding = if let Some((member, unit)) = owner.defining.get(&(reference.prefix, name.clone())) {
            match self.outer_lookup(reference.prefix, name, member, *unit, ctx.scene) {
                Found::Binding(binding) => binding,
                _ => return None,
            }
        } else {
            self.lookup_unit(reference.prefix, name, owner.def_unit, false, ctx.scene)?
        };
        let binding = self.static_binding(binding, ctx.scene)?;
        let binding_name = format!("{}{}", binding.prefix, binding.name);
        let environment = DefData {
            roles: Vec::new(),
            params: Vec::new(),
            body: None,
            def_unit: binding.owner,
            header: binding.header.clone(),
            rec: binding.rec,
            off: binding.off,
            defining: Rc::new(Default::default()),
        };
        self.static_callee_member(&binding.member, &environment, ctx, &[], binding_name, seen)
    }

    #[allow(clippy::too_many_arguments)]
    fn static_callee_member(
        &mut self,
        member: &Member,
        owner: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
        label: String,
        seen: &mut HashSet<(usize, usize)>,
    ) -> Option<StaticDefinition> {
        if member.error.is_some() || member.segments.len() != 1 || !seen.insert((owner.def_unit, ptr(member))) {
            return None;
        }
        let segment = &member.segments[0];
        if segment.piped || segment.args.len() != 1 || segment.args[0].role.is_some() {
            return None;
        }
        let Some(call) = &segment.call else {
            return self.static_callee(&segment.args[0].obj, owner, ctx, scopes, seen);
        };
        if self.kw(&call.kw, owner.def_unit) != "DEFINE" {
            return None;
        }
        if call.params.as_ref().is_some_and(|params| {
            params
                .positional
                .iter()
                .chain(params.named.iter().map(|named| &named.value))
                .any(|value| !header_holes_are_parameters(value, owner))
        }) {
            return None;
        }
        let ObjExpr::Set(body) = &segment.args[0].obj else { return None };
        let mut declaration_context = ctx.clone();
        declaration_context.unit = owner.def_unit;
        declaration_context.rec = owner.rec;
        let version = self.version_of(Some(self.source_unit(&declaration_context)));
        let params =
            definition_header_params(call.params.as_ref(), |name| self.word(name, owner.def_unit), &version).ok()?;
        let mut fields = Vec::new();
        let mut known_roles = true;
        let mut known_params = true;
        for (name, value) in params {
            // Defaults are irrelevant to a signature. Keep the written parameter names without
            // substituting guessed values for defaults that depend on an enclosing application.
            let value = if name == "parameters" {
                let Value::Arr(items) = value else {
                    known_params = false;
                    continue;
                };
                let mut names = Vec::new();
                for item in items {
                    let name = match item {
                        Value::Str(name) => name,
                        Value::Arr(values) if values.len() == 2 => {
                            let Value::Str(name) = &values[0] else {
                                if matches!(values[0], Value::Hole(_)) {
                                    known_params = false;
                                    break;
                                }
                                return None;
                            };
                            name.clone()
                        }
                        Value::Hole(_) => {
                            known_params = false;
                            break;
                        }
                        _ => return None,
                    };
                    names.push(Lit::Str(WStr::new(&name, None)));
                }
                if !known_params {
                    continue;
                }
                Lit::Arr(names)
            } else {
                let Some(value) = literal(&value) else {
                    known_roles = false;
                    continue;
                };
                value
            };
            fields.push((name, value));
        }
        let fields = parse_definition_fields(&fields, &version).ok()?;
        // The first binding names the created object; later bindings do not change its
        // definition-scoped alias namespace (16.5).
        let label = member
            .binds
            .iter()
            .find(|binding| binding.after == 0)
            .map(|binding| format!("{}{}", binding.prefix, binding.name))
            .unwrap_or(label);
        Some(StaticDefinition {
            data: DefData {
                roles: fields.roles,
                params: fields.params,
                body: Some(body.clone()),
                def_unit: owner.def_unit,
                header: owner.header.clone(),
                rec: owner.rec,
                off: segment.start,
                defining: owner.defining.clone(),
            },
            label,
            known_roles,
            known_params,
        })
    }

    pub(super) fn check_definition_callee(
        &mut self,
        kw: &str,
        segment: &Segment,
        owner: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
    ) -> R<()> {
        let first = kw == "ATTACH";
        let definition = if first { segment.args.first() } else { segment.args.last() };
        let Some(definition) = definition else { return Ok(()) };
        let Some(callee) = self.static_callee(&definition.obj, owner, ctx, scopes, &mut HashSet::new()) else {
            return Ok(());
        };
        let data = &callee.data;
        let label = &callee.label;
        let taken: Vec<_> = data
            .roles
            .iter()
            .map(|role| role.name.clone())
            .chain(data.params.iter().map(|(name, _)| name.clone()))
            .chain(data.local_names())
            .collect();
        let scoped: Vec<_> = self
            .file_unit(ctx.unit)
            .and_then(|unit| self.units[unit].scoped.get(label))
            .filter(|_| label.starts_with('$'))
            .map(|table| {
                table
                    .iter()
                    .filter(|(alias, _)| !taken.contains(alias))
                    .map(|(alias, (original, _))| (alias.clone(), original.clone()))
                    .collect()
            })
            .unwrap_or_default();
        // A dynamic role list does not hide fixed parameter names, and a dynamic parameter
        // list does not hide a fixed role count. A scoped alias may be shadowed by either list.
        let param_alias_uncertain = !callee.known_roles
            && segment.call.as_ref().and_then(|call| call.params.as_ref()).is_some_and(|params| {
                params.named.iter().any(|named| !named.quoted && scoped.iter().any(|(alias, _)| alias == &named.name))
            });
        if callee.known_params && !param_alias_uncertain {
            self.application_params(kw, segment, ctx, data, &scoped, label, true)?;
        } else {
            // The reserved priority parameter is fixed even when all own parameters vary.
            let mut fixed = segment.clone();
            if let Some(params) = fixed.call.as_mut().and_then(|call| call.params.as_mut()) {
                params.positional.clear();
                params.named.retain(|named| named.name == "priority");
            }
            self.params_at_definition(kw, &fixed, ctx, &crate::words::words().usage_params(), false)?;
        }
        let written = if first { &segment.args[1..] } else { &segment.args[..segment.args.len() - 1] };
        // A pipe contributes one unlabelled argument, even when its value is a set.
        let args: Vec<Option<&Arg>> =
            segment.piped.then_some(None).into_iter().chain(written.iter().map(Some)).collect();
        let labelled = args.iter().filter(|arg| arg.is_some_and(|arg| arg.role.is_some())).count();
        if labelled > 0 && labelled != args.len() {
            return self.fail(
                ctx,
                codes::ROLE_LABELS_MIXED,
                "label the roles of all objects or of none (4.2 rule 6)",
                segment.start,
            );
        }
        if !callee.known_roles {
            return Ok(());
        }
        if !callee.known_params
            && written.iter().any(|argument| {
                !argument.role_quoted
                    && argument.role.as_ref().is_some_and(|name| scoped.iter().any(|(alias, _)| alias == name))
            })
        {
            return Ok(());
        }
        let mut filled = Vec::new();
        for (index, argument) in args.iter().enumerate() {
            let role = if labelled > 0 {
                let argument = argument.unwrap();
                let written = argument.role.as_ref().unwrap();
                let name = if argument.role_quoted {
                    written
                } else {
                    scoped.iter().find(|(alias, _)| alias == written).map(|(_, original)| original).unwrap_or(written)
                };
                let Some(role) = data.roles.iter().position(|role| &role.name == name) else {
                    return self.fail(
                        ctx,
                        codes::USAGE_UNKNOWN_ROLE,
                        format!("the ordinary definition {label} has no role {written} (9.4)"),
                        argument.start,
                    );
                };
                if filled.contains(&role) {
                    return self.fail(
                        ctx,
                        codes::USAGE_ROLE_LABELLED_TWICE,
                        format!("the role {name} of the ordinary definition {label} is labelled twice (4.2 rule 6)"),
                        argument.start,
                    );
                }
                role
            } else {
                if index >= data.roles.len() {
                    return self.fail(
                        ctx,
                        codes::USAGE_TOO_MANY_OBJECTS,
                        self.usage_role_count_message(label, data.roles.len(), args.len(), ctx),
                        segment.start,
                    );
                }
                index
            };
            filled.push(role);
            if let Some(argument) = argument {
                self.check_static_role_kind(argument, &data.roles[role], label, owner, ctx)?;
            }
        }
        for (index, role) in data.roles.iter().enumerate() {
            if !role.is_set && !filled.contains(&index) {
                return self.fail(
                    ctx,
                    codes::USAGE_ROLE_NOT_FILLED,
                    format!("the single role {} of the ordinary definition {label} is not filled (9.4)", role.name),
                    segment.start,
                );
            }
        }
        Ok(())
    }

    fn check_static_role_kind(
        &mut self,
        argument: &Arg,
        role: &Role,
        label: &str,
        owner: &DefData,
        ctx: &Ctx,
    ) -> R<()> {
        let Some(wanted) = &role.kinds else { return Ok(()) };
        if let ObjExpr::Set(set) = &argument.obj {
            for member in &set.members {
                if member.error.is_none() && member.segments.len() == 1 && member.segments[0].call.is_none() {
                    for argument in &member.segments[0].args {
                        self.check_static_role_kind(argument, role, label, owner, ctx)?;
                    }
                }
            }
            return Ok(());
        }
        let Some(actual) = self.definition_object_kinds(&argument.obj, owner, ctx) else { return Ok(()) };
        let fits = actual.iter().any(|actual| {
            wanted.iter().any(|wanted| {
                actual == wanted
                    || matches!(
                        (actual.as_str(), wanted.as_str()),
                        ("bundle", "instance")
                            | ("position", "point")
                            | ("text", "label" | "annotation")
                            | ("terminal", "pin" | "point" | "net")
                            | ("pin", "terminal" | "point" | "net")
                    )
            })
        });
        if !actual.is_empty() && !fits {
            return self.fail(
                ctx,
                codes::USAGE_ROLE_WRONG_KIND,
                format!(
                    "the role {} of the ordinary definition {label} takes {}, not {} (9.4, 6.9)",
                    role.name,
                    wanted.join("/"),
                    actual.join("/")
                ),
                argument.start,
            );
        }
        Ok(())
    }
}
