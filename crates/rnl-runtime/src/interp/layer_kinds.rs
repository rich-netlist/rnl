//! Fixed drawing and net-routing inputs are checked without expanding a definition (6.9).
use super::{
    ctx::Ctx,
    eval::{ptr, Found, R},
    obj::DefData,
    Interp,
};
use crate::{
    diag::codes,
    syntax::{Member, ObjExpr, SetExpr},
    words::words,
};
use std::collections::HashSet;
use std::rc::Rc;

fn drawable(kind: &str) -> bool {
    matches!(kind, "instance" | "bundle" | "usage" | "wire" | "graphic" | "annotation" | "text" | "label")
}

impl Interp<'_> {
    pub(super) fn check_definition_input_kind(
        &mut self,
        keyword: &str,
        member: &Member,
        index: usize,
        owner: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
    ) -> R<()> {
        let (code, section, accepts): (_, _, fn(&str) -> bool) = match keyword {
            "LAYER" => (codes::LAYER_NOT_TAKEN, "11.6", drawable),
            "PLACE" => (codes::PLACE_NOT_TAKEN, "11.1, 13.5", |kind| kind != "annotation"),
            "ROUTE-NET" | "UNROUTE-NET" => {
                (codes::ROUTE_TARGET_NOT_NET, "13.7", |kind| matches!(kind, "net" | "terminal" | "pin"))
            }
            _ => return Ok(()),
        };
        let segment = &member.segments[index];
        let mut known = Vec::new();
        if segment.piped && index > 0 {
            if let Some(kinds) = self.layer_member_kinds(member, index - 1, owner, ctx, scopes, &mut HashSet::new()) {
                known.push(kinds);
            }
        }
        for arg in &segment.args {
            if let Some(kinds) = self.layer_object_kinds(&arg.obj, owner, ctx, scopes, &mut HashSet::new()) {
                known.push(kinds);
            }
        }
        if let Some(kinds) = known.iter().find(|kinds| !kinds.is_empty() && !kinds.iter().any(|kind| accepts(kind))) {
            return self.fail(
                ctx,
                code,
                format!("{keyword} does not take {} (6.9, {section})", kinds.join("/")),
                segment.start,
            );
        }
        Ok(())
    }

    fn layer_object_kinds(
        &mut self,
        object: &ObjExpr,
        owner: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
        seen: &mut HashSet<(usize, usize, usize)>,
    ) -> Option<Vec<String>> {
        if let Some(kinds) = self.definition_object_kinds(object, owner, ctx) {
            return Some(kinds);
        }
        let ObjExpr::Ref(reference) = object else {
            return None;
        };
        if !matches!(reference.prefix, '%' | '$')
            || reference.number.is_some()
            || reference.sub.is_some()
            || reference.places.is_some()
            || !reference.path.is_empty()
        {
            return None;
        }
        let name = reference.name.as_ref()?;
        let local_scopes = scopes
            .iter()
            .enumerate()
            .rev()
            .map(|(i, set)| (*set, i + 1))
            .chain(owner.body.as_deref().filter(|_| reference.prefix == '%').map(|set| (set, 0)));
        for (set, depth) in local_scopes {
            let candidates: Vec<_> = set
                .members
                .iter()
                .flat_map(|member| {
                    member.binds.iter().filter_map(move |b| {
                        (b.prefix == reference.prefix && b.name == *name).then_some((member, b.after))
                    })
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            if candidates.len() != 1 {
                return None;
            }
            let (member, after) = candidates[0];
            return self.layer_member_kinds(member, after, owner, ctx, &scopes[..depth], seen);
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
        let after = binding.member.binds.iter().find(|b| b.off == binding.off)?.after;
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
        self.layer_member_kinds(&binding.member, after, &environment, ctx, &[], seen)
    }

    #[allow(clippy::too_many_arguments)]
    fn layer_member_kinds(
        &mut self,
        member: &Member,
        after: usize,
        owner: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
        seen: &mut HashSet<(usize, usize, usize)>,
    ) -> Option<Vec<String>> {
        if member.error.is_some()
            || self.body_member_errors.contains_key(&ptr(member))
            || !seen.insert((owner.def_unit, ptr(member), after))
        {
            return None;
        }
        let mut kinds = None;
        for segment in member.segments.iter().take(after + 1) {
            if !segment.piped {
                kinds = if segment.args.len() == 1 {
                    self.layer_object_kinds(&segment.args[0].obj, owner, ctx, scopes, seen)
                } else {
                    None
                };
            } else if !segment.args.is_empty() {
                kinds = None;
            }
            if let Some(call) = &segment.call {
                let keyword = self.kw(&call.kw, owner.def_unit);
                let output = &words().keyword(&keyword)?.output;
                match output.as_str() {
                    "pass" => {}
                    "new:instance" | "new:bundle" | "new:usage" | "new:wire" | "new:annotation" | "new:text"
                    | "new:constraint" | "new:route" | "def:definition" | "def:symbol" | "def:style" => {
                        kinds = Some(vec![output[4..].to_owned()]);
                    }
                    _ => kinds = None,
                }
            }
        }
        kinds
    }
}
