//! The keywords (standard chapters 9–15): what each statement creates or adds.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::diag::codes;
use crate::geom::Orient;
use crate::syntax::{Member, ObjExpr, Segment, SetExpr, Value};
use crate::words::{words, ParamSpec, ParamType, ParamsSpec, PositionSpec, Takes};

use super::ctx::{Ctx, Frame, FrameKind, FrameRef, Nested, StmtSpan};
use super::eval::{get, ArgV, Stop, R};
use super::obj::*;
use super::scene::{FailedApplication, GroupFact, TargetOp, Task};
use super::Interp;

const MAX_SYMBOL_LEVELS: usize = 64;

pub(super) struct DefinitionFields {
    pub(super) roles: Vec<Role>,
    pub(super) params: Vec<(String, Option<Lit>)>,
}

type DefinitionReference<'a> = (crate::syntax::RefOrHole<'a>, Vec<(char, String)>);

pub(super) fn parse_definition_fields(
    p: &[(String, Lit)],
    version: &str,
) -> Result<DefinitionFields, (crate::diag::Code, String)> {
    let mut roles = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    if let Some(Lit::Arr(rs)) = get(p, "roles") {
        for r in rs.clone() {
            let (name, kinds) = match &r {
                Lit::Str(s) => (s.as_str().to_string(), None),
                Lit::Arr(v) if v.len() == 2 && matches!(v[0], Lit::Str(_)) => {
                    let kinds: Vec<String> = match &v[1] {
                        Lit::Arr(ks) => {
                            if super::unit::keeps_0_4(version) {
                                if let Some(index) = ks.iter().position(|k| k.str().is_none()) {
                                    return Err((
                                        codes::DEFINE_ROLE_MALFORMED,
                                        format!(
                                            "role {} kind list item {} is not a string (9.4)",
                                            v[0].str().unwrap(),
                                            index + 1
                                        ),
                                    ));
                                }
                            }
                            ks.iter().filter_map(|k| k.str().map(String::from)).collect()
                        }
                        Lit::Str(k) => vec![k.as_str().to_string()],
                        _ => {
                            return Err((
                                codes::DEFINE_ROLE_MALFORMED,
                                ("a role is a name or [name, kind] (9.4)").into(),
                            ))
                        }
                    };
                    let role_kinds = words().kinds_in(version);
                    for k in &kinds {
                        if !role_kinds.contains(k) {
                            return Err((codes::DEFINE_ROLE_UNKNOWN_KIND, format!("unknown kind name {k} (9.4)")));
                        }
                    }
                    (v[0].str().unwrap().to_string(), Some(kinds))
                }
                _ => return Err((codes::DEFINE_ROLE_MALFORMED, ("a role is a name or [name, kind] (9.4)").into())),
            };
            let is_set = name.starts_with('{') && name.ends_with('}') && name.len() >= 2;
            let name = if is_set { name[1..name.len() - 1].to_string() } else { name };
            if seen.contains(&name) || name == "position" || name == "net" {
                return Err((
                    codes::DEFINE_ROLE_NAME_INVALID,
                    format!("the role name {name} is used twice or is a standard sub-object name (9.4)"),
                ));
            }
            seen.push(name.clone());
            roles.push(Role { name, kinds, is_set, written: r });
        }
    }
    let mut params: Vec<(String, Option<Lit>)> = Vec::new();
    if let Some(Lit::Arr(qs)) = get(p, "parameters") {
        for q in qs.clone() {
            match &q {
                Lit::Arr(v) if v.len() == 2 && matches!(v[0], Lit::Str(_)) => {
                    params.push((v[0].str().unwrap().to_string(), Some(v[1].clone())))
                }
                Lit::Str(s) => params.push((s.as_str().to_string(), None)),
                _ => {
                    return Err((
                        codes::DEFINE_PARAM_MALFORMED,
                        ("a parameter is a name or [name, default] (9.4)").into(),
                    ))
                }
            }
            let n = params.last().unwrap().0.clone();
            if seen.contains(&n) || n == "priority" || n == "position" || n == "net" {
                return Err((codes::DEFINE_PARAM_NAME_INVALID, format!("the parameter name {n} cannot be used (9.4)")));
            }
            seen.push(n);
        }
    }
    Ok(DefinitionFields { roles, params })
}

fn graphic_point_count_is_valid(shape: Shape, count: usize) -> bool {
    match shape {
        Shape::Line => count >= 2,
        Shape::Rectangle | Shape::Circle | Shape::Ellipse => count == 2,
        Shape::Arc => count == 3,
        Shape::Bezier => count >= 4 && (count - 1) % 3 == 0,
        Shape::Text => false,
    }
}

/// Only literal, unbound points whose membership cannot be changed by another statement.
fn definition_point_count(segment: &Segment) -> Option<usize> {
    fn point(arg: &crate::syntax::Arg) -> bool {
        arg.role.is_none()
            && matches!(&arg.obj, ObjExpr::Point(p)
            if !matches!(p.x, crate::syntax::Coord::Hole(_)) && !matches!(p.y, crate::syntax::Coord::Hole(_)))
    }
    if segment.piped {
        return None;
    }
    if segment.args.len() == 1 && segment.args[0].role.is_none() {
        if let ObjExpr::Set(set) = &segment.args[0].obj {
            return set
                .members
                .iter()
                .all(|m| {
                    m.error.is_none()
                        && m.binds.is_empty()
                        && m.segments.len() == 1
                        && m.segments[0].call.is_none()
                        && m.segments[0].args.iter().all(point)
                })
                .then(|| set.members.iter().map(|m| m.segments[0].args.len()).sum());
        }
    }
    segment.args.iter().all(point).then_some(segment.args.len())
}

/// ORDER, DISTANCE: the one `direction` parameter as (axis, sense) (14.3, 14.5).
pub fn direction(d: &str) -> Option<(char, &'static str)> {
    Some(match d {
        "left-to-right" => ('x', "increasing"),
        "right-to-left" => ('x', "decreasing"),
        "bottom-to-top" => ('y', "increasing"),
        "top-to-bottom" => ('y', "decreasing"),
        "x" => ('x', "either"),
        "y" => ('y', "either"),
        _ => return None,
    })
}

/// An orientation code of ORIENT, or a word for one of the four turns, which is another spelling of
/// it for an object: `"left"` is `"R0"`, `"down"` `"R90"`, `"right"` `"R180"` and `"up"` `"R270"` (11.2).
fn orient_code(s: &str) -> Option<Orient> {
    match s {
        "left" => Some(Orient::R0),
        "down" => Some(Orient::R90),
        "right" => Some(Orient::R180),
        "up" => Some(Orient::R270),
        _ => Orient::parse(s),
    }
}

fn style_specs(group: &str) -> Vec<ParamSpec> {
    words()
        .group(group)
        .iter()
        .filter_map(|n| words().style_prop(n))
        .map(|s| ParamSpec::new(s.name.clone(), s.ty.clone()))
        .collect()
}

fn split_style(p: &[(String, Lit)]) -> (StyleMap, Vec<(String, Lit)>) {
    let mut style = StyleMap::new();
    let mut rest = Vec::new();
    for (k, v) in p {
        if words().is_style_prop(k) {
            style.insert(k.clone(), v.clone());
        } else {
            rest.push((k.clone(), v.clone()));
        }
    }
    (style, rest)
}

fn written_in(v: &Lit, unit: UnitId) -> Lit {
    match v {
        Lit::Str(s) if s.unit.is_none() => Lit::Str(WStr { s: s.s.clone(), unit: Some(unit) }),
        other => other.clone(),
    }
}

impl Interp<'_> {
    /// A scoped record can write into a level of another version (2.6, 13.6). Keep the old
    /// behavior for released files, and the source file across a 0.4 boundary.
    pub(super) fn source_unit(&self, ctx: &Ctx) -> UnitId {
        let source = ctx.rec.map_or(ctx.unit, |r| self.rec_unit[r]);
        if self.follows_0_4(source) || self.follows_0_4(ctx.unit) {
            source
        } else {
            ctx.unit
        }
    }

    pub(crate) fn keyword(
        &mut self,
        kw: &str,
        args: Vec<ArgV>,
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        // A signature with no parameters still rejects parameters written on the call (16.4).
        // Style and general definitions have open signatures and validate their own parameters.
        if words()
            .keyword(kw)
            .is_some_and(|sig| matches!(&sig.params, ParamsSpec::List(p) if p.is_empty()) && sig.style.is_none())
        {
            self.params(kw, seg, ctx, &[], None, false, None)?;
        }
        // Symbol bodies can resolve forward definitions recursively. Keep the locals of
        // unrelated keyword handlers out of every active symbol-expansion frame.
        if kw == "DEFINE-SYMBOL" {
            self.kw_define_symbol(kw, &args, seg, ctx, member)
        } else {
            self.keyword_other(kw, args, seg, ctx, member)
        }
    }

    fn keyword_other(
        &mut self,
        kw: &str,
        args: Vec<ArgV>,
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        match kw {
            "DEFINE-STYLE" => self.kw_define_style(kw, &args, seg, ctx, member),
            "DEFINE" => self.kw_define(kw, &args, seg, ctx, member),
            "AS" => self.kw_usage_noted(kw, args, seg, ctx, false, member),
            "ATTACH" => self.kw_usage_noted(kw, args, seg, ctx, true, member),
            "PIN" => self.kw_pin(kw, &args, seg, ctx),
            "LINE" | "RECTANGLE" | "CIRCLE" | "ELLIPSE" | "ARC" | "BEZIER" => self.kw_prim(kw, &args, seg, ctx),
            "TEXT" => self.kw_text(kw, &args, seg, ctx),
            "PLACE" => {
                let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
                if get(&p, "x").is_none() && get(&p, "y").is_none() {
                    let at = seg.call.as_ref().unwrap().start;
                    return self.fail(ctx, codes::PARAM_PLACE_NEEDS_X_OR_Y, "PLACE takes x, y or both (11.1)", at);
                }
                let priority = priority_of(&p, ctx);
                let op = TargetOp::Place {
                    x: get(&p, "x").and_then(Lit::num),
                    y: get(&p, "y").and_then(Lit::num),
                    priority,
                    off: seg.start,
                };
                self.target_each(kw, &args, seg, ctx, 7, op, "target")
            }
            "ORIENT" => {
                let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
                // a code, or in a file of 0.4 a list of them of which one is to hold, where a word for
                // a turn is another spelling of its code (11.2)
                let codes: Vec<Orient> = match get(&p, "code") {
                    Some(Lit::Arr(items)) => items.iter().filter_map(|v| v.str().and_then(orient_code)).collect(),
                    Some(v) => v.str().and_then(orient_code).into_iter().collect(),
                    None => Vec::new(),
                };
                let code = codes.first().copied().unwrap_or_default();
                let codes = if codes.len() > 1 { codes } else { Vec::new() };
                let op = TargetOp::Orient { code, codes, priority: priority_of(&p, ctx), off: seg.start };
                self.target_each(kw, &args, seg, ctx, 7, op, "target")
            }
            "LAYER" => {
                let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
                let layer = get(&p, "layer").and_then(Lit::str).unwrap_or("middle").to_string();
                self.target_each(kw, &args, seg, ctx, 8, TargetOp::Layer { layer, off: seg.start }, "target")
            }
            "SET-SYMBOL" => self.kw_set_symbol(kw, &args, seg, ctx),
            "SET-STYLE" => {
                let mapped = self.map_args(kw, &args, seg, ctx, sig_positions(kw))?;
                let sty = self.anon_def(mapped["style"].obj);
                if !matches!(self.o(sty), ObjData::Style(_)) {
                    let l = self.label(sty);
                    return self.fail(
                        ctx,
                        codes::STYLE_SET_STYLE_NOT_A_STYLE,
                        format!("the last object of SET-STYLE must be a style, not {l} (11.3)"),
                        mapped["style"].off,
                    );
                }
                self.target_each_mapped(
                    kw,
                    mapped,
                    seg,
                    ctx,
                    8,
                    TargetOp::Style { style: sty, off: seg.start },
                    "target",
                )
            }
            "CURRENT" => {
                let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
                let d = get(&p, "direction").and_then(Lit::str).unwrap_or("unspecified").to_string();
                let phase = if self.follows_0_4(self.source_unit(ctx)) { 5 } else { 3 };
                self.target_each(
                    kw,
                    &args,
                    seg,
                    ctx,
                    phase,
                    TargetOp::Current { direction: d, off: seg.start },
                    "terminal",
                )
            }
            "PRIORITY" => {
                let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
                let pr = Priority::parse(get(&p, "priority").and_then(Lit::str).unwrap_or("normal"))
                    .unwrap_or(Priority::Normal);
                // several objects make one set in which they are spread (5.2.1), so a set with GROUP
                // among them is not taken as the group: the error on its members says so
                let spread = if args.len() > 1 {
                    args.iter().map(|a| a.obj).filter(|o| self.is_set(*o)).collect()
                } else {
                    Vec::new()
                };
                let op = TargetOp::Priority { priority: pr, off: seg.call.as_ref().unwrap().start, spread };
                self.target_each(kw, &args, seg, ctx, 7, op, "target")
            }
            "TABLE" => self.kw_table(&args, seg, ctx),
            "SOLVER-CONFIG" => self.kw_solver_config(&args, seg, ctx),
            "GROUP" => {
                let params = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
                let spacing_x = get(&params, "spacing_x").and_then(Lit::num);
                let spacing_y = get(&params, "spacing_y").and_then(Lit::num);
                for (name, value) in [("spacing_x", spacing_x), ("spacing_y", spacing_y)] {
                    if value.is_some_and(|n| !n.is_finite() || n.fract() != 0.0) {
                        return self.fail(
                            ctx,
                            codes::PARAM_SPACING_NOT_WHOLE,
                            format!("the {name} of @GROUP must be a finite whole number of grid units (11.5)"),
                            seg.start,
                        );
                    }
                }
                let mapped = self.map_args(kw, &args, seg, ctx, sig_positions(kw))?;
                let mut obj = mapped["members"].obj;
                if !self.is_set(obj) {
                    if !self.follows_0_4(self.source_unit(ctx)) {
                        return self.fail(ctx, codes::KEYWORD_GROUP_NOT_A_SET, "GROUP takes a set (11.5)", seg.start);
                    }
                    self.check_closing(ctx, seg.start)?;
                    let mut data = SetData::plain(vec![(obj, true)], false);
                    data.item_dependencies = vec![mapped["members"].dependencies.clone()];
                    obj = self.new_obj(ObjData::Set(Box::new(data)));
                    self.note_made(obj, ctx, seg.start);
                }
                let Some(s) = ctx.scene else {
                    if self.is_lazy(obj) {
                        self.note_rule(ctx);
                        return Ok((obj, true));
                    }
                    return self.scene_only(ctx, seg.start);
                };
                let chain = self.apply_chain(ctx);
                let site = Rc::new(ctx.clone());
                self.scenes[s].groups.push(GroupFact {
                    pos: ctx.pos.clone(),
                    set: obj,
                    chain,
                    site,
                    off: seg.start,
                    table: None,
                    spacing_x,
                    spacing_y,
                });
                Ok((obj, self.is_lazy(obj)))
            }
            "ADD-TO" => self.kw_mod(kw, &args, seg, ctx, true),
            "REMOVE-FROM" => self.kw_mod(kw, &args, seg, ctx, false),
            "UNION" => {
                let mapped = self.map_args(kw, &args, seg, ctx, sig_positions(kw))?;
                let src = mapped["sets"].obj;
                self.check_closing(ctx, seg.start)?;
                let lazy = self.is_lazy(src);
                if lazy {
                    let key = ("union", ptr_seg(seg), self.frame_key(ctx), 0usize);
                    if let Some(&u) = self.node_memo.get(&key) {
                        return Ok((u, true));
                    }
                    let u = self.new_obj(ObjData::Set(Box::new(SetData {
                        kind: SetKind::Union { source: src },
                        reach_site: Some(Rc::new(ctx.clone())),
                        ..SetData::plain(Vec::new(), false)
                    })));
                    self.note_made(u, ctx, seg.start);
                    self.node_memo.insert(key, u);
                    return Ok((u, true));
                }
                let u = self.new_obj(ObjData::Set(Box::new(SetData {
                    kind: SetKind::Union { source: src },
                    reach_site: Some(Rc::new(ctx.clone())),
                    ..SetData::plain(Vec::new(), false)
                })));
                self.note_made(u, ctx, seg.start);
                Ok((u, false))
            }
            k if k.starts_with("SELECT-") => self.kw_select(kw, &args, seg, ctx),
            "WIRE" => self.kw_wire(kw, &args, seg, ctx),
            "ROUTE" => {
                let mut spec = self.sig_params(kw, ctx);
                spec.extend(style_specs("wire_style"));
                let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
                let (style, rest) = split_style(&p);
                let mode = get(&rest, "mode").and_then(Lit::str).unwrap_or("auto").to_string();
                let before = self.made_routes.len();
                let (obj, sel) =
                    self.target_each(kw, &args, seg, ctx, 7, TargetOp::Route { mode, style, off: seg.start }, "net")?;
                let made: Vec<ObjId> = self.made_routes[before..].to_vec();
                if made.len() == 1 && !sel {
                    return Ok((made[0], false));
                }
                Ok((obj, sel))
            }
            "BUNDLE" => {
                let mapped = self.map_args(kw, &args, seg, ctx, sig_positions(kw))?;
                let obj = mapped["members"].obj;
                let Some(s) = ctx.scene else { return self.scene_only(ctx, seg.start) };
                let members = if self.is_set(obj) { self.flatten(s, obj, 3, Some(ctx)) } else { vec![obj] };
                if members.is_empty() {
                    return self.fail(
                        ctx,
                        codes::KEYWORD_BUNDLE_EMPTY_SET,
                        "BUNDLE received an empty set and fails (5.2.1, 11.8)",
                        seg.start,
                    );
                }
                for m in &members {
                    if let Some(d) = self.failed_obj(*m) {
                        let id = self.err_cause(
                            ctx,
                            codes::KEYWORD_BUNDLE_MEMBER_FAILED,
                            "a member of the bundle cannot be found: the whole bundle fails (11.8)",
                            seg.start,
                            d,
                        );
                        return Err(Stop::Fail(Some(id)));
                    }
                    if !matches!(self.o(*m), ObjData::Inst { .. }) {
                        let l = self.label(*m);
                        return self.fail(
                            ctx,
                            codes::KEYWORD_BUNDLE_MEMBER_NOT_INSTANCE,
                            format!("the members of a bundle are host instances, not {l} (11.8)"),
                            seg.start,
                        );
                    }
                }
                let b = self.new_obj(ObjData::Bundle { members, empty: false, site: Some(Rc::new(ctx.clone())) });
                self.scene_add_bundle(s, b, ctx.pos.clone(), ctx);
                self.note_birth(s, b, ctx, seg.call.as_ref().unwrap().start, None);
                Ok((b, false))
            }
            "ALIGN" | "ORDER" | "DISTRIBUTE" | "DISTANCE" | "ORIENT-RELATION" => {
                self.kw_constraint(kw, &args, seg, ctx)
            }
            "ASSERT-INSTANCE" | "ASSERT-TERMINAL" | "ASSERT-NET" | "ASSERT-USAGE" | "ASSERT-IN" => {
                self.kw_assert(kw, &args, seg, ctx)
            }
            "ASSERT-TOPOLOGY" => self.kw_assert_topology(kw, &args, seg, ctx),
            _ => {
                let at = seg.call.as_ref().unwrap().start;
                self.fail(ctx, codes::KEYWORD_NOT_USABLE_HERE, format!("@{kw} cannot be used here"), at)
            }
        }
    }

    pub(crate) fn failed_obj(&self, id: ObjId) -> Option<Option<crate::diag::DiagId>> {
        if let Some(diagnostic) = self.late_failures.get(&id) {
            return Some(Some(*diagnostic));
        }
        match self.o(id) {
            ObjData::Failed(d) => Some(*d),
            ObjData::NamedSubobject { parent, resolved, .. } => {
                self.failed_obj(*parent).or_else(|| resolved.and_then(|value| self.failed_obj(value)))
            }
            ObjData::ScenePin { owner, .. } => self.failed_obj(*owner),
            _ => None,
        }
    }

    pub(crate) fn frame_key(&self, ctx: &Ctx) -> usize {
        match &ctx.frame {
            Some(f) => f.borrow().usage.map(|u| u.0 as usize + 1).unwrap_or(0),
            None => 0,
        }
    }

    // ------------------------------------------------------------------ definitions (9.1–9.4)

    fn unnamed_definition_warning(&mut self, ctx: &Ctx, seg: &Segment, member: &Rc<Member>) {
        if member.bind().is_none() && ctx.frame.is_none() && ctx.nested.is_none() && member.segments.len() == 1 {
            let at = seg.call.as_ref().unwrap().start;
            self.warn(
                ctx,
                codes::DEFINE_UNNAMED_UNUSED,
                "a definition at the top of a record without a name is never used (9.1)",
                at,
            );
        }
    }

    fn body_arg(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<Option<Rc<SetExpr>>> {
        if seg.args.is_empty() {
            if !args.is_empty() || self.follows_0_4(self.source_unit(ctx)) {
                return self.fail(
                    ctx,
                    codes::DEFINE_BODY_NOT_ONE_SET,
                    format!("@{kw} takes one set as its body (9.1)"),
                    seg.start,
                );
            }
            return Ok(None);
        }
        match (&seg.args[0].obj, seg.args.len(), args.len()) {
            (ObjExpr::Set(s), 1, 1) => Ok(Some(s.clone())),
            _ => self.fail(
                ctx,
                codes::DEFINE_BODY_NOT_ONE_SET,
                format!("@{kw} takes one set as its body (9.1)"),
                seg.start,
            ),
        }
    }

    fn kw_define_symbol(
        &mut self,
        kw: &str,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        let key = ("sym", ptr_seg(seg), self.frame_key(ctx), 0usize);
        if let Some(&s) = self.node_memo.get(&key) {
            return Ok((s, false));
        }
        let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
        let body = self.body_arg(kw, args, seg, ctx)?;
        // A graph check after evaluation cannot protect the call stack while uncached,
        // forward references are still expanding their definitions.
        if body.is_some() && self.symbol_expansions.len() >= MAX_SYMBOL_LEVELS {
            let id = self.err(
                ctx,
                codes::DEFINE_SYMBOL_EXPANSION_LIMIT, format!("symbol expansion exceeds this implementation's limit of {MAX_SYMBOL_LEVELS} active symbol bodies (9.2.7)"),
                seg.start,
            );
            for failure in &mut self.symbol_expansions {
                failure.get_or_insert(id);
            }
            return Err(Stop::Fail(Some(id)));
        }
        let sym = self.new_obj(ObjData::Symbol(Box::default()));
        self.symbol_mut(sym).unit = Some(ctx.unit);
        self.symbol_mut(sym).independent_pins = self.follows_0_4(self.source_unit(ctx));
        let start = member
            .binds
            .iter()
            .map(|b| b.off)
            .chain(member.segments.iter().flat_map(|s| s.call.iter().map(|c| c.start)))
            .min()
            .unwrap_or(member.start);
        self.symbol_mut(sym).source =
            self.rec_loc(ctx.rec, start).zip(self.rec_loc(ctx.rec, member.end.saturating_sub(1)));
        self.node_memo.insert(key, sym);
        if let Some(b) = get(&p, "box") {
            let ok = match b {
                Lit::Arr(v) if v.len() == 4 && v.iter().all(|x| matches!(x, Lit::Num(_))) => {
                    let n: Vec<f64> = v.iter().filter_map(Lit::num).collect();
                    self.symbol_mut(sym).bbox = Some([n[0], n[1], n[2], n[3]]);
                    true
                }
                _ => false,
            };
            if !ok {
                let at = seg.call.as_ref().unwrap().start;
                return self.fail(
                    ctx,
                    codes::PARAM_SYMBOL_BOX_MALFORMED,
                    "box= is written [x1, y1, x2, y2] (9.2.3)",
                    at,
                );
            }
        }
        if let Some(body) = body {
            let defining = self.defining_for(ctx, seg, member);
            self.symbol_expansions.push(None);
            let evaluated = self.eval_symbol_body(sym, &body, ctx, defining);
            // Restore the active stack before propagating any Result exit. Tolerant member
            // recovery must not turn the truncated active chain into successful symbols.
            let limit_error = self.symbol_expansions.pop().unwrap();
            let evaluated = limit_error.map_or(evaluated, |id| Err(Stop::Fail(Some(id))));
            if let Err(error) = evaluated {
                self.node_memo.remove(&key);
                return Err(error);
            }
            if let Err(error) = self.check_symbol_graph(sym, ctx, seg.start) {
                self.node_memo.remove(&key);
                return Err(error);
            }
            let usages = self.symbol(sym).usages.clone();
            for a in usages {
                let u = self.usage(a).clone();
                if u.own_of == Some(sym) && u.sym_symbol.is_none() {
                    let site = u.site.clone().unwrap();
                    let l = self.label(u.definition);
                    self.warn(&site, codes::USAGE_IN_SYMBOL_NOT_DRAWN, format!("the usage of {l} in a symbol body is given no symbol with SET-SYMBOL and is not drawn (9.2.7)"), u.off);
                }
            }
        }
        self.unnamed_definition_warning(ctx, seg, member);
        Ok((sym, false))
    }

    /// Reject unsupported graphs before any renderer or solver can consume them. The limit
    /// includes the outer symbol and is shared by graphics, fixed text and inferred extents.
    fn check_symbol_graph(&mut self, root: ObjId, ctx: &Ctx, off: usize) -> R<()> {
        let mut pending = vec![(root, false)];
        let mut active = HashSet::new();
        let mut depths: HashMap<ObjId, usize> = HashMap::new();
        while let Some((sym, finish)) = pending.pop() {
            if depths.contains_key(&sym) {
                continue;
            }
            let children = self.symbol_children(sym);
            if finish {
                active.remove(&sym);
                let depth = 1 + children.iter().map(|(_, child, _, _)| depths[child]).max().unwrap_or(0);
                if depth > MAX_SYMBOL_LEVELS {
                    return self.fail(
                        ctx,
                        codes::DEFINE_SYMBOL_DEPTH_LIMIT, format!("nested symbols exceed this implementation's limit of {MAX_SYMBOL_LEVELS} symbol levels (9.2.7)"),
                        off,
                    );
                }
                depths.insert(sym, depth);
            } else {
                if !active.insert(sym) {
                    return self.fail(ctx, codes::DEFINE_SYMBOL_CYCLE, "nested symbols form a cycle (9.2.7)", off);
                }
                pending.push((sym, true));
                pending.extend(children.into_iter().rev().map(|(_, child, _, _)| (child, false)));
            }
        }
        Ok(())
    }

    pub(crate) fn symbol(&self, id: ObjId) -> &SymbolDef {
        match self.o(id) {
            ObjData::Symbol(s) => s,
            _ => panic!("not a symbol"),
        }
    }

    pub(crate) fn symbol_mut(&mut self, id: ObjId) -> &mut SymbolDef {
        match self.om(id) {
            ObjData::Symbol(s) => s,
            _ => panic!("not a symbol"),
        }
    }

    fn kw_define_style(
        &mut self,
        kw: &str,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        let key = ("sty", ptr_seg(seg), self.frame_key(ctx), 0usize);
        if let Some(&s) = self.node_memo.get(&key) {
            return Ok((s, false));
        }
        let own = self.params(kw, seg, ctx, &[], Some(&[]), true, None)?;
        let body = self.body_arg(kw, args, seg, ctx)?;
        let mut props = StyleMap::new();
        let mut bctx = ctx.clone();
        bctx.defining = Some(self.defining_for(ctx, seg, member)); // the body cannot see this name (6.9)
        if let Some(body) = body {
            for m in body.members.iter() {
                if let Some(err) = &m.error {
                    self.err(ctx, err.code, err.message.clone(), m.error_pos);
                    continue;
                }
                for sg in &m.segments {
                    if sg.call.is_some() || m.bind().is_some() || m.segments.len() != 1 {
                        self.err(
                            ctx,
                            codes::STYLE_BODY_NOT_STYLE,
                            "the set of DEFINE-STYLE only references other styles (9.3)",
                            m.start,
                        );
                        break;
                    }
                    for a in &sg.args {
                        let Ok((obj, _)) = self.eval_obj(&a.obj, &bctx) else { continue };
                        match self.o(obj) {
                            ObjData::Style(p) => {
                                let p = p.clone();
                                props.extend(p);
                            }
                            _ => {
                                self.err(
                                    ctx,
                                    codes::STYLE_BODY_NOT_STYLE,
                                    "the set of DEFINE-STYLE only references other styles (9.3)",
                                    a.start,
                                );
                            }
                        }
                    }
                }
            }
        }
        for (k, v) in own {
            props.insert(k, v);
        }
        let sty = self.new_obj(ObjData::Style(props));
        self.node_memo.insert(key, sty);
        self.unnamed_definition_warning(ctx, seg, member);
        Ok((sty, false))
    }

    /// Parse declaration fields identically during static checking and actual creation.
    fn definition_fields(&mut self, p: &[(String, Lit)], ctx: &Ctx, at: usize) -> R<DefinitionFields> {
        match parse_definition_fields(p, &self.version_of(Some(self.source_unit(ctx)))) {
            Ok(fields) => Ok(fields),
            Err((code, message)) => self.fail(ctx, code, message, at),
        }
    }

    fn kw_define(
        &mut self,
        kw: &str,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        let key = ("dfn", ptr_seg(seg), self.frame_key(ctx), 0usize);
        if let Some(&d) = self.node_memo.get(&key) {
            return Ok((d, false));
        }
        let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
        let body = self.body_arg(kw, args, seg, ctx)?;
        let at = seg.call.as_ref().unwrap().start;
        let DefinitionFields { roles, params } = self.definition_fields(&p, ctx, at)?;
        let defining = self.defining_for(ctx, seg, member);
        let checked = body
            .as_ref()
            .and_then(|body| self.static_nested_definitions.get(&(ctx.unit, super::eval::ptr(&**body))))
            .copied();
        let body = if matches!(checked, Some((_, false))) {
            body.map(|body| self.independent_definition_body(&body, ctx.unit))
        } else {
            body
        };
        let d = DefData {
            roles,
            params,
            body: body.clone(),
            def_unit: ctx.unit,
            header: ctx.header.clone(),
            rec: ctx.rec,
            off: seg.start,
            defining,
        };
        let id = self.new_obj(ObjData::Definition(Box::new(d)));
        self.node_memo.insert(key, id);
        if body.is_some() {
            if let Some((checked, complete)) = checked {
                let parameters: Vec<_> = self
                    .body_parameter_errors
                    .iter()
                    .filter(|((definition, _), _)| *definition == checked)
                    .map(|((_, name), cause)| ((id, name.clone()), *cause))
                    .collect();
                self.body_parameter_errors.extend(parameters);
                if !complete {
                    self.check_definition(id, ctx);
                }
            } else {
                self.check_definition(id, ctx);
            }
        }
        self.unnamed_definition_warning(ctx, seg, member);
        Ok((id, false))
    }

    /// A static error remains the cause when the same body node fails during an expansion.
    fn definition_error<T>(
        &mut self,
        node: &T,
        ctx: &Ctx,
        code: crate::diag::Code,
        message: impl Into<String>,
        off: usize,
    ) {
        let diagnostic = self.err(ctx, code, message, off);
        if self.follows_0_4(self.source_unit(ctx)) {
            self.body_errors.insert(super::eval::ptr(node), diagnostic);
        }
    }

    /// Standard sub-object paths can contradict the kinds declared for a role (6.9).
    /// User-defined names and collection cardinalities still require an application.
    fn check_definition_role_path(&mut self, role: &Role, reference: &crate::syntax::Ref, ctx: &Ctx) {
        use crate::syntax::Sep;
        let Some(mut kinds) = role.kinds.clone() else { return };
        if role.is_set || reference.places.is_some() || kinds.is_empty() {
            return;
        }
        for part in &reference.path {
            let (allowed, next, code, subject): (&[&str], &str, _, _) = if part.sep == Sep::Term {
                (&["instance", "bundle"], "terminal", codes::REF_NO_TERMINALS, "terminals")
            } else {
                let name = if words().is_subobject(&part.name) {
                    part.name.clone()
                } else if kinds.iter().any(|kind| kind == "usage") {
                    // A usage's own names and scoped aliases precede global sub-object aliases.
                    return;
                } else {
                    self.word(&part.name, ctx.unit)
                };
                match name.as_str() {
                    "net" => (&["terminal", "pin"], "net", codes::REF_NO_SUBOBJECT, "a net sub-object"),
                    "position" => (
                        &["instance", "bundle", "usage", "terminal", "pin", "point", "text", "label", "annotation"],
                        "point",
                        codes::REF_NO_SUBOBJECT,
                        "a position sub-object",
                    ),
                    _ => return,
                }
            };
            if !kinds.iter().any(|kind| allowed.contains(&kind.as_str())) {
                self.definition_error(
                    reference,
                    ctx,
                    code,
                    format!(
                        "{} cannot have {subject}: role ?{} is declared as {} (6.5, 6.9)",
                        reference.text(),
                        role.name,
                        role.kinds.as_ref().unwrap().join("/")
                    ),
                    reference.start,
                );
                return;
            }
            kinds = vec![next.to_string()];
        }
    }

    /// Nested ordinary sets have local names; new definitions own their body placeholders.
    fn definition_refs<'a>(
        &self,
        member: &'a Member,
        ctx: &Ctx,
        scope: &[(char, String)],
        out: &mut Vec<DefinitionReference<'a>>,
    ) {
        fn value<'a>(literal: &'a Value, out: &mut Vec<DefinitionReference<'a>>) {
            match literal {
                Value::Hole(name) => out.push((crate::syntax::RefOrHole::Hole(name), Vec::new())),
                Value::Arr(items) => {
                    for item in items {
                        value(item, out);
                    }
                }
                _ => {}
            }
        }
        for segment in &member.segments {
            let owns_body = segment.call.as_ref().is_some_and(|call| {
                matches!(
                    self.kw(&call.kw, ctx.unit).as_str(),
                    "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE"
                )
            });
            for argument in &segment.args {
                match &argument.obj {
                    ObjExpr::Ref(reference) => out.push((crate::syntax::RefOrHole::Ref(reference), scope.to_vec())),
                    ObjExpr::Point(point) => {
                        for coordinate in [&point.x, &point.y] {
                            if let crate::syntax::Coord::Hole(name) = coordinate {
                                out.push((crate::syntax::RefOrHole::Hole(name), Vec::new()));
                            }
                        }
                    }
                    ObjExpr::Set(set) if !owns_body => {
                        let mut nested = scope.to_vec();
                        for member in &set.members {
                            nested.extend(member.binds.iter().map(|b| (b.prefix, b.name.clone())));
                        }
                        for member in &set.members {
                            self.definition_refs(member, ctx, &nested, out);
                        }
                    }
                    ObjExpr::Set(_) => {}
                }
            }
            if let Some(params) = segment.call.as_ref().and_then(|call| call.params.as_ref()) {
                for item in &params.positional {
                    value(item, out);
                }
                for named in &params.named {
                    value(&named.value, out);
                }
            }
        }
    }

    /// Checks that do not depend on the arguments, once per definition (6.9, M5).
    pub(super) fn check_definition(&mut self, id: ObjId, ctx: &Ctx) {
        self.check_definition_with_signature(id, ctx, true, true);
    }

    pub(super) fn check_definition_with_signature(
        &mut self,
        id: ObjId,
        ctx: &Ctx,
        known_roles: bool,
        known_params: bool,
    ) {
        let d = self.def(id).clone();
        let reused = d
            .body
            .as_ref()
            .and_then(|body| self.static_nested_definitions.get(&(d.def_unit, super::eval::ptr(&**body))))
            .is_some_and(|(checked, _)| *checked != id);
        if self.follows_0_4(self.source_unit(ctx)) {
            self.check_definition_scene_names(&d, ctx);
        }
        let roles: Vec<String> = d.roles.iter().map(|r| r.name.clone()).collect();
        let params: Vec<String> = d.params.iter().map(|(n, _)| n.clone()).collect();
        let locals = d.local_names();
        let mut names: Vec<String> = Vec::new();
        for m in d.body.as_ref().unwrap().members.iter() {
            if reused && self.body_member_errors.contains_key(&super::eval::ptr(&**m)) {
                continue;
            }
            if let Some(err) = &m.error {
                let diagnostic = self.err(ctx, err.code, err.message.clone(), m.error_pos);
                if self.follows_0_4(self.source_unit(ctx)) {
                    self.body_member_errors.insert(super::eval::ptr(&**m), diagnostic);
                }
                self.note_failed(ctx.rec, m.start, m.end, false);
                continue;
            }
            for b in m.binds.iter().filter(|b| b.prefix == '%') {
                if roles.contains(&b.name) || b.name == "position" || b.name == "net" {
                    let diagnostic = self.err(
                        ctx,
                        codes::DEFINE_LOCAL_NAME_CLASH,
                        format!(
                            "the local name {} is spelled like a role name or a standard sub-object name (9.4)",
                            b.name
                        ),
                        b.off,
                    );
                    if self.follows_0_4(self.source_unit(ctx)) {
                        self.body_member_errors.entry(super::eval::ptr(&**m)).or_insert(diagnostic);
                    }
                }
                if names.contains(&b.name) && !self.follows_0_4(self.source_unit(ctx)) {
                    let diagnostic = self.err(
                        ctx,
                        codes::DEFINE_LOCAL_NAME_TWICE,
                        format!("the local name {} is bound twice (6.6)", b.name),
                        b.off,
                    );
                    if self.follows_0_4(self.source_unit(ctx)) {
                        self.body_member_errors.entry(super::eval::ptr(&**m)).or_insert(diagnostic);
                    }
                }
                names.push(b.name.clone());
            }
            if self.follows_0_4(self.source_unit(ctx)) {
                self.check_body_level_symbols(m, ctx);
            }
            let mut refs = Vec::new();
            if self.follows_0_4(self.source_unit(ctx)) {
                self.definition_refs(m, ctx, &[], &mut refs);
            } else {
                let mut legacy = Vec::new();
                crate::syntax::walk_refs(m, &mut legacy);
                refs.extend(legacy.into_iter().map(|reference| (reference, Vec::new())));
            }
            for (r, nested_names) in refs {
                match r {
                    crate::syntax::RefOrHole::Ref(r) => {
                        if self.follows_0_4(self.source_unit(ctx))
                            && self.body_errors.contains_key(&super::eval::ptr(r))
                        {
                            continue;
                        }
                        if r.number.is_some() {
                            // reported once here, and each expansion leaves the statement out (5.9)
                            let diag = self.err(
                                ctx,
                                codes::NUMBER_IN_DEFINITION_BODY,
                                super::numbers::in_body_message(r),
                                r.start,
                            );
                            self.body_errors.insert(super::eval::ptr(r), diag);
                            self.note_failed_at(ctx.rec, r.start, false);
                        }
                        if (r.prefix == '#' || r.prefix == '&')
                            && !(self.follows_0_4(self.source_unit(ctx))
                                && (r.is_bare_level() || (r.prefix == '&' && r.name.is_none())))
                        {
                            self.definition_error(r, ctx, codes::DEFINE_BODY_HOST_REF, "the body of an ordinary definition cannot reference host objects; they come in through roles (6.9)", r.start);
                            self.note_failed_at(ctx.rec, r.start, false);
                        }
                        if r.prefix == '$' || r.prefix == '%' {
                            let n = r.name.clone().unwrap_or_default();
                            let locally_bound = (r.prefix == '%' && locals.contains(&n))
                                || nested_names.iter().any(|(prefix, name)| *prefix == r.prefix && name == &n);
                            if self.follows_0_4(self.source_unit(ctx))
                                && !locally_bound
                                && !d.defining.contains_key(&(r.prefix, n.clone()))
                                && self.lookup_unit(r.prefix, &n, d.def_unit, false, ctx.scene).is_none()
                            {
                                if let Err(Stop::Fail(Some(diagnostic))) = self.missing_reference::<()>(r, ctx) {
                                    self.body_errors.insert(super::eval::ptr(r), diagnostic);
                                    self.note_failed_at(ctx.rec, r.start, false);
                                }
                            }

                            if let Some((member, unit)) = d.defining.get(&(r.prefix, n.clone())).cloned() {
                                if !locally_bound {
                                    if let super::eval::Found::NoOuter =
                                        self.outer_lookup(r.prefix, &n, &member, unit, ctx.scene)
                                    {
                                        let (code, msg) = super::eval::no_outer_message(r);
                                        let diag = self.err(ctx, code, msg, r.start);
                                        self.body_errors.insert(super::eval::ptr(r), diag);
                                        self.note_failed_at(ctx.rec, r.start, false);
                                    }
                                }
                            }
                        }
                        if r.prefix == '?' {
                            if let Some(n) = &r.name {
                                if self.follows_0_4(self.source_unit(ctx)) {
                                    if let Some(role) = d.roles.iter().find(|role| &role.name == n) {
                                        self.check_definition_role_path(role, r, ctx);
                                    }
                                }
                                if !roles.contains(n) {
                                    if params.contains(n) {
                                        self.definition_error(
                                            r,
                                            ctx,
                                            codes::REF_HOLE_PARAM_AS_OBJECT,
                                            format!(
                                                "?{n} is a parameter and is written only where a literal goes (6.10)"
                                            ),
                                            r.start,
                                        );
                                    } else if known_roles && known_params {
                                        self.definition_error(
                                            r,
                                            ctx,
                                            codes::REF_HOLE_UNKNOWN,
                                            format!("?{n} is not in the role list or the parameter list (6.10)"),
                                            r.start,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    crate::syntax::RefOrHole::Hole(n) => {
                        if !known_params || (reused && self.body_parameter_errors.contains_key(&(id, n.to_string()))) {
                            continue;
                        }
                        let modern = self.follows_0_4(self.source_unit(ctx));
                        let role = roles.iter().any(|p| p == n);
                        if !params.iter().any(|p| p == n) && (modern || !role) {
                            let diagnostic = self.err(
                                ctx,
                                codes::REF_HOLE_NOT_A_PARAM,
                                if modern && role {
                                    format!("the role ?{n} is used where a literal parameter is required (6.10)")
                                } else {
                                    format!("?{n} is not in the parameter list (6.10)")
                                },
                                if modern { m.start } else { 0 },
                            );
                            if modern {
                                self.body_parameter_errors.entry((id, n.to_string())).or_insert(diagnostic);
                            }
                        }
                    }
                }
            }
            let modern = self.follows_0_4(self.source_unit(ctx));
            let mut calls = Vec::new();
            if modern {
                self.definition_calls(m, ctx, &mut calls);
            } else {
                calls.extend(m.segments.iter().filter_map(|segment| segment.call.as_ref()));
            }
            for call in calls {
                if reused && self.body_errors.contains_key(&super::eval::ptr(call)) {
                    continue;
                }
                let k = self.kw(&call.kw, ctx.unit);
                if modern {
                    if let Err(Stop::Fail(Some(diagnostic))) = self.check_segment_keyword(&k, call, ctx) {
                        self.body_errors.insert(super::eval::ptr(call), diagnostic);
                        continue;
                    }
                }
                {
                    if !words().is_keyword(&k) {
                        self.definition_error(
                            call,
                            ctx,
                            codes::KEYWORD_UNKNOWN,
                            format!("unknown keyword @{} (16.4)", call.kw),
                            call.start,
                        );
                    } else if k == "PIN" {
                        self.definition_error(
                            call,
                            ctx,
                            codes::DEFINE_BODY_PIN,
                            "PIN is written only in a DEFINE-SYMBOL body (9.2.4)",
                            call.start,
                        );
                    }
                }
            }
        }
        if self.follows_0_4(self.source_unit(ctx)) {
            for member in &d.body.as_ref().unwrap().members {
                self.check_definition_parameters(member, &d, ctx, &[]);
                self.mark_failed_body_members(id, member, ctx);
            }
            self.check_constant_local_bindings(&d, ctx);
            self.check_failed_local_names(&d, ctx);
            self.check_nested_definitions(&d, ctx);
        }
    }

    /// Constant creations have independent handles even at equal coordinates or members.
    /// Unknown application-dependent results must be compared when the body is expanded.
    fn check_constant_local_bindings(&mut self, d: &DefData, ctx: &Ctx) {
        fn has_parameter(value: &Value) -> bool {
            match value {
                Value::Hole(_) => true,
                Value::Arr(values) => values.iter().any(has_parameter),
                _ => false,
            }
        }
        // A unique, unconditional alias can be resolved before its written position.
        // Multiple candidates still need the first-valid-binding walk below.
        fn unique_alias(
            it: &Interp<'_>,
            d: &DefData,
            name: &str,
            seen: &mut HashSet<String>,
        ) -> Option<(usize, bool, bool)> {
            if !seen.insert(name.to_owned()) {
                return None;
            }
            let mut candidates = d.body.as_ref()?.members.iter().filter(|member| {
                !it.body_member_errors.contains_key(&super::eval::ptr(&***member))
                    && member.binds.iter().any(|b| b.prefix == '%' && b.name == name)
            });
            let member = candidates.next()?;
            if candidates.next().is_some() || member.binds.len() != 1 || member.segments.len() != 1 {
                return None;
            }
            let segment = &member.segments[0];
            if segment.call.is_some() || segment.args.len() != 1 || segment.args[0].role.is_some() {
                return None;
            }
            match &segment.args[0].obj {
                ObjExpr::Point(p)
                    if !matches!(p.x, crate::syntax::Coord::Hole(_))
                        && !matches!(p.y, crate::syntax::Coord::Hole(_)) =>
                {
                    Some((super::eval::ptr(p), true, false))
                }
                ObjExpr::Set(set) => Some((super::eval::ptr(&**set), false, true)),
                ObjExpr::Ref(r) if r.prefix == '%' && r.path.is_empty() && r.sub.is_none() && r.places.is_none() => {
                    unique_alias(it, d, r.name.as_deref()?, seen)
                }
                _ => None,
            }
        }
        let mut names: HashMap<String, Option<(usize, bool, bool)>> = HashMap::new();
        for member in &d.body.as_ref().unwrap().members {
            let id = super::eval::ptr(&**member);
            if self.body_member_errors.contains_key(&id) {
                continue;
            }
            let before = names.clone();
            let mut bindings = Vec::new();
            let mut fresh = None;
            let mut point = false;
            let mut set = false;
            for (index, segment) in member.segments.iter().enumerate() {
                if index == 0 && segment.args.len() == 1 && segment.args[0].role.is_none() {
                    if let ObjExpr::Point(p) = &segment.args[0].obj {
                        if !matches!(p.x, crate::syntax::Coord::Hole(_))
                            && !matches!(p.y, crate::syntax::Coord::Hole(_))
                        {
                            fresh = Some(super::eval::ptr(p));
                            point = true;
                        }
                    } else if let ObjExpr::Set(s) = &segment.args[0].obj {
                        fresh = Some(super::eval::ptr(&**s));
                        set = true;
                    } else if let ObjExpr::Ref(r) = &segment.args[0].obj {
                        if r.prefix == '%' && r.path.is_empty() && r.sub.is_none() && r.places.is_none() {
                            let known = r.name.as_ref().and_then(|name| {
                                names
                                    .get(name)
                                    .copied()
                                    .unwrap_or_else(|| unique_alias(self, d, name, &mut HashSet::new()))
                            });
                            if let Some((handle, is_point, is_set)) = known {
                                fresh = Some(handle);
                                point = is_point;
                                set = is_set;
                            }
                        }
                    }
                } else if !segment.args.is_empty() {
                    fresh = None;
                    point = false;
                    set = false;
                }
                if let Some(call) = &segment.call {
                    let keyword = self.kw(&call.kw, ctx.unit);
                    if keyword == "WIRE" || !self.modifier_keeps_input(&keyword, ctx) {
                        let dynamic = call.params.as_ref().is_some_and(|p| {
                            p.positional.iter().any(has_parameter) || p.named.iter().any(|n| has_parameter(&n.value))
                        });
                        let constant_size = call.params.as_ref().is_some_and(|p| {
                            let number = |name: &str| {
                                p.named.iter().any(|n| n.name == name && matches!(n.value, Value::Num { .. }))
                            };
                            match keyword.as_str() {
                                "CIRCLE" => number("radius"),
                                "RECTANGLE" | "ELLIPSE" => number("width") && number("height"),
                                _ => false,
                            }
                        });
                        let union = keyword == "UNION" && set;
                        let text = keyword == "TEXT"
                            && point
                            && call.params.as_ref().is_none_or(|p| p.named.iter().all(|n| n.name != "key"));
                        fresh = fresh
                            .filter(|_| !dynamic && ((point && constant_size) || union || text))
                            .map(|_| super::eval::ptr(call));
                        if call.params.as_ref().is_none_or(|p| p.named.is_empty() && p.positional.is_empty()) {
                            if let Some(shape) = Shape::parse(&keyword) {
                                if definition_point_count(segment)
                                    .is_some_and(|count| graphic_point_count_is_valid(shape, count))
                                {
                                    fresh = Some(super::eval::ptr(call));
                                }
                            }
                        }
                        if !dynamic && !segment.piped && segment.args.len() == 1 && segment.args[0].role.is_none() {
                            if let ObjExpr::Set(body) = &segment.args[0].obj {
                                let ordinary = keyword == "DEFINE";
                                let empty_symbol = keyword == "DEFINE-SYMBOL"
                                    && body.members.is_empty()
                                    && call
                                        .params
                                        .as_ref()
                                        .is_none_or(|p| p.named.is_empty() && p.positional.is_empty());
                                if ordinary || empty_symbol {
                                    fresh = Some(super::eval::ptr(call));
                                }
                            }
                        }
                        point = false;
                        set = union;
                    }
                }
                for (binding_index, b) in
                    member.binds.iter().enumerate().filter(|(_, b)| b.prefix == '%' && b.after == index)
                {
                    let known = fresh.map(|handle| (handle, point, set));
                    bindings.push((binding_index, b, known));
                    names.entry(b.name.clone()).or_insert(known);
                }
            }
            // A prefix is written first, although it receives the last segment's value.
            names = before.clone();
            bindings.sort_by_key(|(index, _, _)| *index);
            for (_, b, known) in bindings {
                match names.get(&b.name) {
                    Some(Some((old, _, _))) if known.is_some_and(|(new, _, _)| new != *old) => {
                        let diagnostic = self.err(
                            ctx,
                            codes::DEFINE_LOCAL_NAME_TWICE,
                            format!(
                                "the local name {} is bound to another object; the first valid binding is kept (6.6)",
                                b.name
                            ),
                            b.off,
                        );
                        self.body_member_errors.entry(id).or_insert(diagnostic);
                    }
                    None => {
                        names.insert(b.name.clone(), known);
                    }
                    _ => {}
                }
            }
            if self.body_member_errors.contains_key(&id) {
                names = before;
            }
        }
    }

    /// Names, counts and literal parameter values do not depend on an application's objects.
    fn check_definition_parameters(&mut self, member: &Member, definition: &DefData, ctx: &Ctx, scopes: &[&SetExpr]) {
        for (index, segment) in member.segments.iter().enumerate() {
            if let Some(call) = &segment.call {
                let keyword = self.kw(&call.kw, ctx.unit);
                if !self.body_errors.contains_key(&super::eval::ptr(call)) {
                    if let Some(signature) = words().keyword(&keyword) {
                        // Application parameters come from the named definition; open signatures
                        // have their own checks. Neither is described by this parameter list.
                        if matches!(
                            keyword.as_str(),
                            "ASSERT-INSTANCE" | "ASSERT-TERMINAL" | "ASSERT-NET" | "ASSERT-USAGE" | "ASSERT-IN"
                        ) {
                            if let Err(Stop::Fail(Some(diagnostic))) =
                                self.check_assertion_at_definition(&keyword, segment, ctx)
                            {
                                self.body_errors.insert(super::eval::ptr(call), diagnostic);
                            }
                        } else if keyword.starts_with("SELECT-") {
                            if let Err(Stop::Fail(Some(diagnostic))) =
                                self.selection_params(&keyword, segment, ctx, true)
                            {
                                self.body_errors.insert(super::eval::ptr(call), diagnostic);
                            }
                        } else if signature.open_params && keyword == "SOLVER-CONFIG" {
                            if let Err(Stop::Fail(Some(diagnostic))) =
                                self.check_solver_config_at_definition(segment, ctx)
                            {
                                self.body_errors.insert(super::eval::ptr(call), diagnostic);
                            }
                        } else if !signature.open_params && !matches!(signature.params, ParamsSpec::Definition) {
                            let mut spec = self.sig_params(&keyword, ctx);
                            if let Some(style) = &signature.style {
                                spec.extend(style_specs(style));
                            }
                            let open_style = matches!(signature.params, ParamsSpec::Style);
                            let checked = self
                                .params_at_definition(&keyword, segment, ctx, &spec, open_style)
                                .and_then(|params| {
                                    if keyword == "DEFINE" {
                                        self.definition_fields(&params, ctx, call.start)?;
                                    }
                                    Ok(())
                                });
                            if let Err(Stop::Fail(Some(diagnostic))) = checked {
                                self.body_errors.insert(super::eval::ptr(call), diagnostic);
                            }
                        }
                        if keyword == "LAYER" && !self.body_errors.contains_key(&super::eval::ptr(call)) {
                            if let Err(Stop::Fail(Some(diagnostic))) =
                                self.check_definition_layer(member, index, definition, ctx, scopes)
                            {
                                self.body_errors.insert(super::eval::ptr(call), diagnostic);
                            }
                        }
                        if !self.body_errors.contains_key(&super::eval::ptr(call))
                            && matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE")
                            && (segment.piped
                                || segment.args.len() != 1
                                || !matches!(segment.args[0].obj, ObjExpr::Set(_)))
                        {
                            let diagnostic = self.err(
                                ctx,
                                codes::DEFINE_BODY_NOT_ONE_SET,
                                format!("@{keyword} takes one set as its body (9.1)"),
                                segment.start,
                            );
                            self.body_errors.insert(super::eval::ptr(call), diagnostic);
                        }
                        if !self.body_errors.contains_key(&super::eval::ptr(call)) {
                            let checked = match &signature.positions {
                                crate::words::Positions::List(positions) => {
                                    self.check_definition_signature(&keyword, segment, definition, ctx, positions)
                                }
                                crate::words::Positions::Definition => {
                                    self.check_definition_application(&keyword, segment, definition, ctx, scopes)
                                }
                            };
                            if let Err(Stop::Fail(Some(diagnostic))) = checked {
                                self.body_errors.insert(super::eval::ptr(call), diagnostic);
                            }
                        }
                    }
                }
                if matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE") {
                    continue;
                }
            }
            for argument in &segment.args {
                if let ObjExpr::Set(set) = &argument.obj {
                    let mut nested = scopes.to_vec();
                    nested.push(set);
                    for member in &set.members {
                        self.check_definition_parameters(member, definition, ctx, &nested);
                    }
                }
            }
        }
    }

    /// The definition slot of AS/ATTACH is structural, before the definition's roles are known.
    fn check_definition_application(
        &mut self,
        kw: &str,
        segment: &Segment,
        definition: &DefData,
        ctx: &Ctx,
        scopes: &[&SetExpr],
    ) -> R<()> {
        let first = kw == "ATTACH";
        let where_ = if first { "first" } else { "last" };
        let argument = if first { segment.args.first() } else { segment.args.last() };
        let Some(argument) = argument else {
            return self.fail(
                ctx,
                codes::USAGE_DEFINITION_PIPED,
                format!("the definition of @{kw} is written {where_} and cannot come down a pipe (5.6)"),
                segment.start,
            );
        };
        if argument.role.is_some() {
            return self.fail(
                ctx,
                codes::USAGE_DEFINITION_HAS_ROLE,
                format!("the definition of @{kw} is written {where_} and carries no role (4.2 rule 7)"),
                argument.start,
            );
        }
        let wrong_kind = self
            .definition_object_kinds(&argument.obj, definition, ctx)
            .is_some_and(|kinds| !kinds.is_empty() && !kinds.iter().any(|kind| kind == "definition"));
        if wrong_kind {
            return self.fail(
                ctx,
                codes::USAGE_NOT_A_DEFINITION,
                format!("the object written {where_} for @{kw} must be an ordinary definition (5.6, 6.9)"),
                argument.start,
            );
        }
        self.check_definition_callee(kw, segment, definition, ctx, scopes)
    }

    /// Infer only kinds fixed by the written object or a declared role and standard path.
    /// Custom sub-objects, collection indexing and aliases scoped to usages remain unknown.
    pub(super) fn definition_object_kinds(
        &self,
        object: &ObjExpr,
        definition: &DefData,
        ctx: &Ctx,
    ) -> Option<Vec<String>> {
        let reference = match object {
            ObjExpr::Point(_) => return Some(vec!["point".into()]),
            ObjExpr::Ref(reference)
                if reference.prefix == '?' && reference.sub.is_none() && reference.places.is_none() =>
            {
                reference
            }
            _ => return None,
        };
        let mut kinds = if let Some(name) = &reference.name {
            let role = definition.roles.iter().find(|role| &role.name == name)?;
            if role.is_set {
                return None;
            }
            role.kinds.clone()?
        } else {
            vec!["usage".into()]
        };
        if kinds.is_empty() {
            return None;
        }
        for part in &reference.path {
            if part.sub.is_some() {
                return None;
            }
            let (allowed, next): (&[&str], &str) = if part.sep == crate::syntax::Sep::Term {
                (&["instance", "bundle"], "terminal")
            } else {
                let name = if words().is_subobject(&part.name) || part.quoted {
                    part.name.clone()
                } else if kinds.iter().any(|kind| kind == "usage") {
                    return None;
                } else {
                    self.word(&part.name, ctx.unit)
                };
                match name.as_str() {
                    "net" => (&["terminal", "pin"], "net"),
                    "position" => (
                        &["instance", "bundle", "usage", "terminal", "pin", "point", "text", "label", "annotation"],
                        "point",
                    ),
                    _ => return None,
                }
            };
            if !kinds.iter().any(|kind| allowed.contains(&kind.as_str())) {
                return None;
            }
            kinds = vec![next.into()];
        }
        Some(kinds)
    }

    /// Reject a slot only when every declared alternative has an incompatible kind (6.9).
    /// Unknown local values and application-dependent collection sizes stay deferred.
    fn check_definition_slot_kind(
        &mut self,
        kw: &str,
        argument: &crate::syntax::Arg,
        slot: &PositionSpec,
        definition: &DefData,
        ctx: &Ctx,
    ) -> R<()> {
        let (code, allowed): (_, &[&str]) = match (kw, slot.name.as_str()) {
            ("PLACE", _) => (
                codes::PLACE_NOT_TAKEN,
                &["instance", "bundle", "usage", "point", "position", "text", "label", "annotation"],
            ),
            ("ORIENT", _) => (codes::PLACE_ORIENT_NOT_TAKEN, &["instance", "bundle", "usage", "pin"]),
            ("LAYER", _) => (
                codes::LAYER_NOT_TAKEN,
                &["instance", "bundle", "usage", "wire", "graphic", "text", "label", "annotation"],
            ),
            ("ADD-TO" | "REMOVE-FROM", "target") => (codes::SET_EDIT_TARGET_NOT_SET, &["set"]),
            ("ASSERT-IN", "set") => (codes::SET_EDIT_TARGET_NOT_SET, &["set"]),
            ("ASSERT-INSTANCE", "members") => (codes::ASSERT_MEMBER_WRONG_KIND, &["instance", "bundle"]),
            ("ASSERT-TERMINAL", "members") => (codes::ASSERT_MEMBER_WRONG_KIND, &["terminal", "pin"]),
            ("ASSERT-NET", "members") => (codes::ASSERT_MEMBER_WRONG_KIND, &["net", "terminal", "pin"]),
            ("ASSERT-USAGE", "members") => (codes::ASSERT_MEMBER_WRONG_KIND, &["usage"]),
            ("ASSERT-USAGE", "definition") => (codes::USAGE_NOT_A_DEFINITION, &["definition"]),
            ("SELECT-USAGE" | "SELECT-ANCESTOR-USAGE", "definition") => {
                (codes::SELECT_DEFINITION_NOT_DEFINITION, &["definition"])
            }
            ("WIRE", "net") => (codes::WIRE_NET_WRONG_KIND, &["net", "pin", "terminal", "wire"]),
            ("WIRE", "points") => (codes::WIRE_POINT_WRONG_KIND, &["point", "position", "pin", "terminal"]),
            ("SET-SYMBOL", "symbol") => (codes::KEYWORD_SET_SYMBOL_NOT_A_SYMBOL, &["symbol"]),
            ("SET-STYLE", "style") => (codes::STYLE_SET_STYLE_NOT_A_STYLE, &["style"]),
            ("ALIGN" | "ORDER" | "DISTRIBUTE" | "DISTANCE", _) => (
                codes::CONSTRAINT_MEMBER_NOT_LOCATABLE,
                &[
                    "instance",
                    "bundle",
                    "usage",
                    "point",
                    "position",
                    "pin",
                    "terminal",
                    "text",
                    "label",
                    "annotation",
                    "graphic",
                ],
            ),
            _ if Shape::parse(kw).is_some_and(|shape| shape != Shape::Text) => {
                (codes::KEYWORD_GRAPHIC_NOT_POINTS, &["point", "position", "pin", "terminal"])
            }
            _ => return Ok(()),
        };
        let kinds = match &argument.obj {
            ObjExpr::Set(set) => {
                if allowed.contains(&"set") {
                    return Ok(());
                }
                // A plain written set feeds each member to the consuming slot. Calls may
                // change a member's kind, so those members require separate inference.
                for member in &set.members {
                    if member.error.is_none() && member.segments.len() == 1 && member.segments[0].call.is_none() {
                        for argument in &member.segments[0].args {
                            self.check_definition_slot_kind(kw, argument, slot, definition, ctx)?;
                        }
                    }
                }
                return Ok(());
            }
            _ => {
                let Some(kinds) = self.definition_object_kinds(&argument.obj, definition, ctx) else { return Ok(()) };
                kinds
            }
        };
        if !kinds.is_empty() && !kinds.iter().any(|kind| allowed.contains(&kind.as_str())) {
            return self.fail(
                ctx,
                code,
                format!("the {} of @{kw} cannot take {} (5.2.4, 6.9)", slot.name, kinds.join("/")),
                argument.start,
            );
        }
        Ok(())
    }

    /// Check the written signature where no argument value can change the result (6.9).
    fn check_definition_signature(
        &mut self,
        kw: &str,
        segment: &Segment,
        definition: &DefData,
        ctx: &Ctx,
        positions: &[PositionSpec],
    ) -> R<()> {
        // Optional definitions and application roles need definition lookup first.
        if let Some(position) = positions.iter().find(|p| matches!(p.takes, Takes::Def(_))) {
            // An optional definition depends on the last object's kind. Required definitions
            // always occupy the last written argument, even when their kind is not yet known.
            if position.optional {
                // A literal point or a declared scene object cannot be the optional
                // definition. It remains in the scope/member list, as at application time.
                let no_definition = segment.args.last().is_none_or(|argument| {
                    self.definition_object_kinds(&argument.obj, definition, ctx).is_some_and(|kinds| {
                        !kinds.is_empty()
                            && kinds.iter().all(|kind| !matches!(kind.as_str(), "definition" | "symbol" | "style"))
                    })
                });
                if no_definition && (!segment.piped || !segment.args.is_empty()) {
                    let positions: Vec<_> =
                        positions.iter().filter(|p| !matches!(p.takes, Takes::Def(_))).cloned().collect();
                    return self.check_definition_signature(kw, segment, definition, ctx, &positions);
                }
                return Ok(());
            }
            let Some(last) = segment.args.last() else {
                let (code, message) = if segment.piped {
                    (codes::KEYWORD_DEFINITION_PIPED,
                     "a definition cannot come down a pipe: the piped object fills the first position, and the definition is written after it (5.6)".to_string())
                } else {
                    (codes::KEYWORD_DEFINITION_MISSING, format!("the definition of @{kw} is written last (5.6)"))
                };
                return self.fail(ctx, code, message, segment.start);
            };
            if last.role.is_some() {
                return self.fail(
                    ctx,
                    codes::KEYWORD_DEFINITION_HAS_ROLE,
                    "a definition is written last and carries no role (4.2 rule 7)",
                    last.start,
                );
            }
            let mut remaining = segment.clone();
            remaining.args.pop();
            let positions: Vec<_> = positions.iter().filter(|p| !matches!(p.takes, Takes::Def(_))).cloned().collect();
            self.check_definition_signature(kw, &remaining, definition, ctx, &positions)?;
            let count = remaining.args.len() + usize::from(remaining.piped);
            if positions.len() == 1 && count > 1 {
                return self.fail(ctx, codes::KEYWORD_SLOT_COUNT_WITH_DEFINITION,
                    format!("@{kw} has 2 slots, and {} objects are written (5.2.1); the objects of one slot are written in {{…}}", count + 1),
                    segment.start);
            }
            self.check_definition_slot_kind(kw, last, position, definition, ctx)?;
            return Ok(());
        }
        let count = segment.args.len() + usize::from(segment.piped);
        let labelled = segment.args.iter().filter(|a| a.role.is_some()).count();
        if labelled > 0 {
            if labelled != count {
                return self.fail(
                    ctx,
                    codes::ROLE_LABELS_MIXED,
                    "label the roles of all objects or of none (4.2 rule 6)",
                    segment.start,
                );
            }
            let mut filled = HashSet::new();
            for arg in &segment.args {
                let written = arg.role.as_ref().unwrap();
                let role = if arg.role_quoted { written.clone() } else { self.word(written, ctx.unit) };
                if !positions.iter().any(|p| p.name == role) {
                    let more =
                        if !arg.role_quoted && !written.is_ascii() { "; is an alias package not loaded?" } else { "" };
                    return self.fail(
                        ctx,
                        codes::KEYWORD_UNKNOWN_POSITION,
                        format!("@{kw} has no position {written}{more} (5.3)"),
                        arg.start,
                    );
                }
                if !filled.insert(role) {
                    return self.fail(
                        ctx,
                        codes::KEYWORD_POSITION_LABELLED_TWICE,
                        "a role is labelled only once (4.2 rule 6)",
                        arg.start,
                    );
                }
            }
            for position in positions {
                if !position.optional && !filled.contains(&position.name) {
                    return self.fail(
                        ctx,
                        codes::KEYWORD_POSITION_NOT_FILLED,
                        format!("the {} of @{kw} is not filled (5.3)", position.name),
                        segment.start,
                    );
                }
            }
        } else if positions.len() == 1 {
            if count == 0 && !positions[0].optional {
                return self.fail(
                    ctx,
                    codes::KEYWORD_NO_OBJECT,
                    format!("@{kw} has no object: the signature does not match (5.2.1)"),
                    segment.start,
                );
            }
        } else {
            // A sole reference or piped value may be a set which is unpacked. Its count
            // is not known here; a literal point cannot become a set under substitution.
            let known_single = segment
                .args
                .first()
                .is_some_and(|arg| self.definition_object_kinds(&arg.obj, definition, ctx).is_some());
            let fixed_count = count != 1 || (!segment.piped && known_single);
            if fixed_count && count != positions.len() {
                let slots = positions.len();
                let objects = if count == 1 { "1 object is".to_owned() } else { format!("{count} objects are") };
                let hint = if count > slots { "; the objects of one slot are written in {…}" } else { "" };
                return self.fail(
                    ctx,
                    codes::KEYWORD_SLOT_COUNT,
                    format!("@{kw} has {slots} slots, and {objects} written (5.2.1){hint}"),
                    segment.start,
                );
            }
        }
        for (index, argument) in segment.args.iter().enumerate() {
            let slot = if let Some(written) = &argument.role {
                let name = if argument.role_quoted { written.clone() } else { self.word(written, ctx.unit) };
                positions.iter().find(|slot| slot.name == name)
            } else if positions.len() == 1 {
                positions.first()
            } else if count == positions.len() {
                positions.get(index + usize::from(segment.piped))
            } else {
                None
            };
            if let Some(slot) = slot {
                self.check_definition_slot_kind(kw, argument, slot, definition, ctx)?;
            }
        }
        if segment.call.as_ref().unwrap().params.as_ref().is_none_or(|p| p.named.is_empty() && p.positional.is_empty())
        {
            if let Some(shape) = Shape::parse(kw).filter(|shape| *shape != Shape::Text) {
                if definition_point_count(segment).is_some_and(|count| !graphic_point_count_is_valid(shape, count)) {
                    return self.fail(
                        ctx,
                        codes::KEYWORD_GRAPHIC_POINT_COUNT,
                        format!("@{kw} has the wrong number of points (9.2.5)"),
                        segment.start,
                    );
                }
            }
        }
        Ok(())
    }

    /// Calls in ordinary member sets share the enclosing definition's static checks.
    /// A new definition or package body owns its own names and is checked separately.
    fn definition_calls<'a>(&self, member: &'a Member, ctx: &Ctx, out: &mut Vec<&'a crate::syntax::Call>) {
        for segment in &member.segments {
            if let Some(call) = &segment.call {
                out.push(call);
                let keyword = self.kw(&call.kw, ctx.unit);
                if matches!(keyword.as_str(), "DEFINE" | "DEFINE-SYMBOL" | "DEFINE-STYLE" | "USE-PACKAGE") {
                    continue;
                }
            }
            for argument in &segment.args {
                if let ObjExpr::Set(set) = &argument.obj {
                    for member in &set.members {
                        self.definition_calls(member, ctx, out);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ usages: AS, ATTACH (9.4, 10.5, 12)

    /// [`Interp::kw_usage`], noting an application written in a record of a level that fails, with
    /// the name it binds, its definition and the objects written for its roles, so that a tool can
    /// list it with them ([`crate::model::FailedUsage`]).
    fn kw_usage_noted(
        &mut self,
        kw: &str,
        args: Vec<ArgV>,
        seg: &Segment,
        ctx: &Ctx,
        first: bool,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        let result = self.kw_usage(kw, args.clone(), seg, ctx, first);
        let (Err(Stop::Fail(diag)), Some(s)) = (&result, ctx.scene) else { return result };
        if ctx.frame.is_some() || ctx.selected {
            return result;
        }
        let npiped = usize::from(seg.piped).min(args.len());
        let written = &args[npiped..];
        let dfn_at = if first { 0 } else { written.len().saturating_sub(1) };
        let definition =
            written.get(dfn_at).map(|a| self.anon_def(a.obj)).filter(|d| matches!(self.o(*d), ObjData::Definition(_)));
        let roles_of: Vec<String> =
            definition.map(|d| self.def(d).roles.iter().map(|r| r.name.clone()).collect()).unwrap_or_default();
        let objs =
            args[..npiped].iter().chain(written.iter().enumerate().filter(|(k, _)| *k != dfn_at).map(|(_, a)| a));
        let roles = objs
            .enumerate()
            .filter(|(_, a)| self.failed_obj(a.obj).is_none())
            .map(|(k, a)| (a.role.clone().or_else(|| roles_of.get(k).cloned()), a.obj))
            .collect();
        let k = member.segments.iter().position(|x| x.id == seg.id);
        let name = member.binds.iter().find(|b| b.prefix == '%' && Some(b.after) == k).map(|b| b.name.clone());
        let stmt = ctx.stmt.as_ref().map_or((ctx.rec, seg.start, seg.end), |st| (st.rec, st.start, st.end));
        let app = FailedApplication { name, definition, roles, stmt, diag: *diag };
        if !self.scenes[s].failed_usages.iter().any(|f| f.stmt == app.stmt) {
            self.scenes[s].failed_usages.push(app);
        }
        result
    }

    /// Both static checking and applications use the callee's parameter namespace.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn application_params(
        &mut self,
        kw: &str,
        seg: &Segment,
        ctx: &Ctx,
        d: &DefData,
        scoped: &[(String, String)],
        dlabel: &str,
        static_check: bool,
    ) -> R<Vec<(String, Lit)>> {
        let own: Vec<String> = d.params.iter().map(|(n, _)| n.clone()).collect();
        let prio: Vec<String> = match self.file_unit(ctx.unit) {
            Some(f) => self.units[f].aliases.iter().filter(|(_, o)| *o == "priority").map(|(a, _)| a.clone()).collect(),
            None => Vec::new(),
        };
        let resolve = |s: &mut Self, name: &str, quoted: bool, off: usize| -> R<String> {
            if quoted {
                return Ok(name.to_string());
            }
            if let Some((_, o)) = scoped.iter().find(|(a, _)| a == name) {
                return Ok(o.clone());
            }
            if prio.iter().any(|p| p == name) {
                if own.iter().any(|p| p == name) {
                    return s.fail(ctx, codes::USAGE_PARAM_ALIAS_AMBIGUOUS, format!("{name} is both a parameter of the definition {dlabel} and an alias of priority; its meaning is not unique (16.5)"), off);
                }
                return Ok("priority".to_string());
            }
            Ok(name.to_string())
        };
        // parameters: the definition's parameter list plus `priority` (9.4, 14.8)
        let mut pspec: Vec<ParamSpec> =
            d.params.iter().map(|(n, _)| ParamSpec::new(n.clone(), ParamType::Any)).collect();
        pspec.extend(words().usage_params());
        let order: Vec<String> = d.params.iter().map(|(n, _)| n.clone()).collect();
        self.params_mode(kw, seg, ctx, &pspec, Some(&order), false, Some(&resolve), static_check)
    }

    /// AS takes the definition as its last object, ATTACH as the first written one; objects that
    /// come down the pipe fill roles and never take the definition position (5.5, 5.6).
    fn kw_usage(&mut self, kw: &str, args: Vec<ArgV>, seg: &Segment, ctx: &Ctx, first: bool) -> R<(ObjId, bool)> {
        let npiped = usize::from(seg.piped);
        let piped: Vec<ArgV> = args[..npiped.min(args.len())].to_vec();
        let written: Vec<ArgV> = args[npiped.min(args.len())..].to_vec();
        let wher = if first { "first" } else { "last" };
        if written.is_empty() {
            return self.fail(
                ctx,
                codes::USAGE_DEFINITION_PIPED,
                format!("the definition of @{kw} is written {wher} and cannot come down a pipe (5.6)"),
                seg.start,
            );
        }
        let dfn_arg = if first { written[0].clone() } else { written[written.len() - 1].clone() };
        if dfn_arg.role.is_some() {
            return self.fail(
                ctx,
                codes::USAGE_DEFINITION_HAS_ROLE,
                format!("the definition of @{kw} is written {wher} and carries no role (4.2 rule 7)"),
                dfn_arg.off,
            );
        }
        let dfn = self.anon_def(dfn_arg.obj);
        if !matches!(self.o(dfn), ObjData::Definition(_)) {
            let l = self.label(dfn);
            return self.fail(
                ctx,
                codes::USAGE_NOT_A_DEFINITION,
                format!("the object written {wher} for @{kw} must be an ordinary definition, not {l} (5.6)"),
                dfn_arg.off,
            );
        }
        let mut objs = piped;
        if first {
            objs.extend(written[1..].iter().cloned());
        } else {
            objs.extend(written[..written.len() - 1].iter().cloned());
        }
        let d = self.def(dfn).clone();
        // names in the definition's namespace may be definition-scoped aliases (16.5, D17)
        let scoped = self.scoped_for(ctx.unit, dfn);
        let dlabel = self.label(dfn);
        let p = self.application_params(kw, seg, ctx, &d, &scoped, &dlabel, false)?;
        // its own priority=; in a file of 0.3 the default of the record head gives an AS or ATTACH
        // one only when it is "disabled", which turns off everything in the record, so that a usage
        // without one keeps the priorities of the facts of its body, as a usage that a rule makes
        // does; in a file of 0.2 the record's default is the usage's (14.8)
        let own = get(&p, "priority").and_then(Lit::str).and_then(Priority::parse);
        let follows_0_3 = self.statement_follows_0_3(ctx);
        let head =
            if follows_0_3 { ctx.header.priority.filter(|p| *p == Priority::Disabled) } else { ctx.header.priority };
        let priority = own.or(head);
        // a string passed in remembers the file it is written in (16.5)
        let params: Vec<(String, Option<Lit>)> = d
            .params
            .iter()
            .map(|(n, def)| {
                (
                    n.clone(),
                    match get(&p, n) {
                        Some(v) => Some(written_in(v, self.source_unit(ctx))),
                        None => def.clone(),
                    },
                )
            })
            .collect();
        let given: Vec<(String, Lit)> = d
            .params
            .iter()
            .filter_map(|(n, _)| get(&p, n).map(|v| (n.clone(), written_in(v, self.source_unit(ctx)))))
            .collect();
        // roles (5.2.1, 9.4)
        let mut filled: Vec<(String, ArgV)> = Vec::new();
        let labeled = objs.iter().filter(|a| a.role.is_some()).count();
        if labeled > 0 && labeled != objs.len() {
            return self.fail(
                ctx,
                codes::ROLE_LABELS_MIXED,
                "label the roles of all objects or of none (4.2 rule 6)",
                seg.start,
            );
        }
        if labeled > 0 {
            for a in &objs {
                let r = a.role.clone().unwrap();
                let role = if a.role_quoted {
                    r.clone()
                } else {
                    scoped.iter().find(|(x, _)| *x == r).map(|(_, o)| o.clone()).unwrap_or(r.clone())
                };
                if !d.roles.iter().any(|x| x.name == role) {
                    let more = if !a.role_quoted && !r.is_ascii() { "; is an alias package not loaded?" } else { "" };
                    let then = if a.role_quoted && scoped.iter().any(|(x, _)| *x == r) {
                        let o = &scoped.iter().find(|(x, _)| *x == r).unwrap().1;
                        format!("; a quoted name is the name itself, not an alias; unquoted it would be {o} (16.5)")
                    } else {
                        String::new()
                    };
                    // a role of the standard series 0.2 that the series 0.3 renamed
                    let names: Vec<&str> = d.roles.iter().map(|x| x.name.as_str()).collect();
                    let renamed = self.role_renamed_in_0_3(&dlabel, &role, &names, ctx).unwrap_or_default();
                    return self.fail(
                        ctx,
                        codes::USAGE_UNKNOWN_ROLE,
                        format!("the ordinary definition {dlabel} has no role {r}{more} (9.4){then}{renamed}"),
                        a.off,
                    );
                }
                if filled.iter().any(|(n, _)| *n == role) {
                    return self.fail(
                        ctx,
                        codes::USAGE_ROLE_LABELLED_TWICE,
                        format!("the role {role} of the ordinary definition {dlabel} is labelled twice (4.2 rule 6)"),
                        a.off,
                    );
                }
                filled.push((role, a.clone()));
            }
        } else {
            if objs.len() > d.roles.len() {
                return self.fail(
                    ctx,
                    codes::USAGE_TOO_MANY_OBJECTS,
                    format!(
                        "the ordinary definition {dlabel} has {} roles, {} objects are written (9.4)",
                        d.roles.len(),
                        objs.len()
                    ),
                    seg.start,
                );
            }
            for (r, a) in d.roles.iter().zip(objs.iter()) {
                filled.push((r.name.clone(), a.clone()));
            }
        }
        for r in &d.roles {
            if !r.is_set && !filled.iter().any(|(n, _)| *n == r.name) {
                return self.fail(
                    ctx,
                    codes::USAGE_ROLE_NOT_FILLED,
                    format!("the single role {} of the ordinary definition {dlabel} is not filled (9.4)", r.name),
                    seg.start,
                );
            }
        }
        // single roles that receive a set are applied once per member (5.2.1)
        let mut multi = Vec::new();
        let mut lazy_roles = Vec::new();
        for r in &d.roles {
            if r.is_set {
                continue;
            }
            if let Some((_, a)) = filled.iter().find(|(n, _)| *n == r.name) {
                if self.is_lazy(a.obj) {
                    lazy_roles.push(r.name.clone());
                } else if self.is_set(a.obj) {
                    multi.push(r.name.clone());
                }
            }
        }
        if multi.len() + lazy_roles.len() > 1 {
            return self.fail(
                ctx,
                codes::USAGE_SEVERAL_SET_ROLES,
                "at most one single role of an application receives a set of several members (5.2.1)",
                seg.start,
            );
        }
        let app = Rc::new(Application { dfn, filled, params, given, priority, seg_start: seg.start, seg: seg.clone() });
        let collection_rule =
            self.follows_0_4(self.source_unit(ctx)) && app.filled.iter().any(|(_, arg)| self.is_lazy(arg.obj));
        let rname = lazy_roles.first().or_else(|| collection_rule.then(|| multi.first()).flatten()).cloned();
        if rname.is_some() || collection_rule {
            let key = ("gen", ptr_seg(seg), self.frame_key(ctx), 0usize);
            let gen = match self.node_memo.get(&key) {
                Some(&g) => g,
                None => {
                    let g = self.new_obj(ObjData::Set(Box::new(SetData {
                        kind: SetKind::Gen { by_scene: HashMap::new() },
                        reach_site: Some(Rc::new(ctx.clone())),
                        reach_sources: app.filled.iter().map(|(_, arg)| arg.obj).collect(),
                        ..SetData::plain(Vec::new(), false)
                    })));
                    self.node_memo.insert(key, g);
                    g
                }
            };
            if ctx.scene.is_none() {
                self.note_rule(ctx);
                return Ok((gen, true));
            }
            let lazy = rname.as_ref().map(|role| app.filled.iter().find(|(n, _)| n == role).unwrap().1.obj);
            let c = Rc::new(ctx.clone());
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, phase: usize| {
                let members = match lazy {
                    Some(lazy) => it.flatten_inputs(s, lazy, phase, Some(&c)),
                    None => vec![InputMember::plain(NOTHING)],
                };
                let mut out = Vec::new();
                for (k, member) in members.into_iter().enumerate() {
                    let mobj = member.object;
                    let mut f2 = app.filled.clone();
                    for e in f2.iter_mut() {
                        if rname.as_ref() == Some(&e.0) {
                            e.1 = ArgV {
                                obj: mobj,
                                dependencies: member.dependencies.clone(),
                                role: None,
                                role_quoted: false,
                                sel: true,
                                off: e.1.off,
                                explicit: false,
                            };
                        }
                    }
                    let mut sub = c.with_dependencies(&member.dependencies);
                    sub.pos = pos_plus(&c.pos, "g", k as i64);
                    sub.origin = Rc::from(if c.origin.is_empty() {
                        "generated by a rule".to_string()
                    } else {
                        format!("{}; generated by a rule", c.origin)
                    });
                    sub.selected = true;
                    if let Ok(a) = it.usage_once(&app, &f2, &sub, true, None) {
                        out.push(a);
                    }
                }
                if let ObjData::Set(sd) = it.om(gen) {
                    if let SetKind::Gen { by_scene } = &mut sd.kind {
                        by_scene.insert(s, out);
                    }
                }
            });
            self.defer(ctx, 5, task)?;
            return Ok((gen, true));
        }
        let Some(s) = ctx.scene else {
            if ctx.in_symbol() {
                return self.symbol_usage(&app, ctx, seg);
            }
            return self.scene_only(ctx, seg.start);
        };
        if let Some(rname) = multi.first().cloned() {
            let src = app.filled.iter().find(|(n, _)| *n == rname).unwrap().1.clone();
            let members = self.flatten_inputs(s, src.obj, 3, Some(ctx));
            let mut out = Vec::new();
            for (k, member) in members.into_iter().enumerate() {
                let mobj = member.object;
                let mut f2 = app.filled.clone();
                for e in f2.iter_mut() {
                    if e.0 == rname {
                        e.1 = ArgV {
                            obj: mobj,
                            dependencies: member.dependencies.clone(),
                            role: None,
                            role_quoted: false,
                            sel: src.sel,
                            off: src.off,
                            explicit: false,
                        };
                    }
                }
                let sub = ctx.with_dependencies(&member.dependencies).with_pos(pos_plus(&ctx.pos, "e", k as i64));
                out.push(self.usage_once(&app, &f2, &sub, false, Some((k, mobj)))?);
            }
            let set = self
                .new_obj(ObjData::Set(Box::new(SetData::plain(out.into_iter().map(|a| (a, true)).collect(), false))));
            return Ok((set, false));
        }
        let filled = app.filled.clone();
        Ok((self.usage_once(&app, &filled, ctx, false, None)?, false))
    }

    /// One application of a definition; `each` is the member of a set that a single role receives,
    /// with its place in the set, when the statement applies it once for each member (5.2.1).
    pub(crate) fn usage_once(
        &mut self,
        app: &Application,
        filled: &[(String, ArgV)],
        ctx: &Ctx,
        generated: bool,
        each: Option<(usize, ObjId)>,
    ) -> R<ObjId> {
        // what a generated usage expands to is generated as well (10.5)
        let generated = generated || ctx.frame_generated();
        let dfn = app.dfn;
        self.check_cycle(dfn, ctx, app.seg_start)?;
        let s = ctx.scene.unwrap();
        let d = self.def(dfn).clone();
        let mut rmap = Vec::new();
        let mut rsets = Vec::new();
        for r in &d.roles {
            let mut obj = match filled.iter().find(|(n, _)| *n == r.name) {
                Some((_, a)) => {
                    if let Some(dg) = self.failed_obj(a.obj) {
                        let id = self.err_cause(
                            ctx,
                            codes::USAGE_ROLE_FAILED_OBJECT,
                            format!("the role {} received a failed object", r.name),
                            app.seg_start,
                            dg,
                        );
                        return Err(Stop::Fail(Some(id)));
                    }
                    a.obj
                }
                None => self.new_obj(ObjData::Set(Box::new(SetData::plain(Vec::new(), true)))),
            };
            if r.is_set {
                rsets.push(r.name.clone());
                if self.is_empty_node(obj) {
                    if matches!(self.o(obj), ObjData::EmptyNode { direct: true }) {
                        self.warn(
                            ctx,
                            codes::EMPTY_NODE_IGNORED,
                            "the empty node written directly fills a set role with no members (6.3, 9.4)",
                            app.seg_start,
                        );
                    }
                    obj = self.new_obj(ObjData::Set(Box::new(SetData::plain(Vec::new(), true))));
                }
                if !self.is_set(obj) {
                    obj = self.new_obj(ObjData::Set(Box::new(SetData::plain(vec![(obj, true)], true))));
                }
                obj = self.role_input(obj, &r.kinds, &r.name, true, ctx, app.seg_start)?;
                let members = if self.is_lazy(obj) { Vec::new() } else { self.flatten(s, obj, 3, Some(ctx)) };
                for m in members {
                    self.check_kind(dfn, &r.name, &r.kinds, m, ctx, app.seg_start)?;
                }
            } else {
                obj = self.role_input(obj, &r.kinds, &r.name, false, ctx, app.seg_start)?;
                self.check_kind(dfn, &r.name, &r.kinds, obj, ctx, app.seg_start)?;
            }
            rmap.push((r.name.clone(), obj));
        }
        let parent = ctx.frame_usage();
        let u = UsageData {
            definition: dfn,
            roles: rmap,
            role_sets: rsets,
            params: app.params.clone(),
            given: app.given.clone(),
            locals: Vec::new(),
            parent,
            generated,
            priority: app.priority,
            in_symbol: false,
            sym_symbol: None,
            sym_orient: Orient::R0,
            own_of: None,
            site: Some(Rc::new(ctx.clone())),
            off: app.seg_start,
            frame: None,
            ctx_origin: ctx.origin.clone(),
        };
        let a = self.new_obj(ObjData::Usage(Box::new(u)));
        self.scene_add_usage(s, a, ctx.pos.clone(), ctx);
        let at = app.seg.call.as_ref().map_or(app.seg_start, |c| c.start);
        self.note_birth(s, a, ctx, at, each);
        self.expand(a, ctx, generated, app.seg_start)?;
        Ok(a)
    }

    /// An application inside a body that applies a definition still being expanded around it
    /// closes a cycle: that application fails, the rest of the bodies stay (9.4).
    fn check_cycle(&mut self, dfn: ObjId, ctx: &Ctx, off: usize) -> R<()> {
        let mut chain = Vec::new();
        let mut u = ctx.frame_usage();
        while let Some(a) = u {
            let ud = self.usage(a);
            chain.push(ud.definition);
            if ud.definition == dfn {
                let mut path: Vec<String> = chain.iter().rev().map(|d| self.label(*d)).collect();
                path.push(self.label(dfn));
                return self.fail(
                    ctx,
                    codes::USAGE_CYCLIC_APPLICATION,
                    format!("cyclic application: {}; this application fails (9.4)", path.join(" -> ")),
                    off,
                );
            }
            u = ud.parent;
        }
        Ok(())
    }

    pub(super) fn check_kind(
        &mut self,
        dfn: ObjId,
        role: &str,
        kinds: &Option<Vec<String>>,
        obj: ObjId,
        ctx: &Ctx,
        off: usize,
    ) -> R<()> {
        if self.is_empty_node(obj) {
            if kinds.as_ref().is_some_and(|kinds| kinds.iter().any(|kind| kind == "net")) {
                return Ok(());
            }
            return self.fail(
                ctx,
                codes::USAGE_ROLE_WRONG_KIND,
                format!(
                    "the role {role} of {} does not declare net; it cannot take the empty node (6.3, 9.4)",
                    self.label(dfn)
                ),
                off,
            );
        }
        let Some(kinds) = kinds else { return Ok(()) };
        if self.role_inputs.get(&obj).is_some_and(|view| view.resolved.is_none()) {
            return Ok(());
        }
        if self.follows_0_4(self.source_unit(ctx))
            && matches!(self.o(obj), ObjData::NamedSubobject { .. })
            && ctx.scene.is_some_and(|s| self.scenes[s].phase < 7)
        {
            return Ok(());
        }
        let any = self.kind_of(obj);
        // a TEXT is also a label or an annotation, by what it is attached to (5.1)
        let k = match any {
            "text" => self.obj_kind(obj).unwrap_or(any),
            _ => any,
        };
        // "instance" also takes bundles, and "terminal" their terminals, which are terminals (11.8)
        if !kinds.iter().any(|x| {
            x == k
                || x == any
                || (x == "instance" && k == "bundle")
                || (self.follows_0_4(self.source_unit(ctx)) && x == "point" && k == "position")
        }) {
            let dl = self.label(dfn);
            let ol = self.label(obj);
            return self.fail(
                ctx,
                codes::USAGE_ROLE_WRONG_KIND,
                format!(
                    "the role {role} of the ordinary definition {dl} takes {}, not {ol} ({k}) (9.4)",
                    kinds.join("/")
                ),
                off,
            );
        }
        Ok(())
    }

    fn expand(&mut self, a: ObjId, ctx: &Ctx, generated: bool, off: usize) -> R<()> {
        let ud = self.usage(a).clone();
        let d = self.def(ud.definition).clone();
        let Some(body) = &d.body else { return Ok(()) };
        let mut fr = Frame::new(FrameKind::Definition);
        fr.usage = Some(a);
        fr.roles = ud.roles.clone();
        fr.params = ud.params.clone();
        fr.definition = Some(ud.definition);
        fr.generated = generated;
        fr.selected = ctx.selected;
        for (k, m) in body.members.iter().enumerate() {
            for b in m.binds.iter().filter(|b| b.prefix == '%') {
                fr.local_members.push((b.name.clone(), MemberRef { rec: d.rec, m: m.clone() }, k));
            }
        }
        let fr: FrameRef = Rc::new(RefCell::new(fr));
        let dl = self.label(ud.definition);
        let here = match self.rec_loc(ctx.rec, off) {
            Some(l) => format!("applying {dl} ({l})"),
            None => format!("applying {dl}"),
        };
        let origin: Rc<str> = Rc::from(if ctx.origin.is_empty() { here } else { format!("{} -> {here}", ctx.origin) });
        {
            let u = self.usage_mut(a);
            u.frame = Some(fr.clone());
            u.ctx_origin = origin.clone();
        }
        let mut body_ctx = ctx.clone();
        body_ctx.frame = Some(fr.clone());
        body_ctx.origin = origin.clone();
        body_ctx.nested = None;
        self.register_place_body(body, &body_ctx);
        for (k, m) in body.members.iter().enumerate() {
            if m.error.is_some() {
                continue;
            }
            let mut sub = ctx.clone();
            sub.frame = Some(fr.clone());
            sub.origin = origin.clone();
            sub.nested = None;
            match self.definition_member(m, k, &sub) {
                Err(Stop::Language) => continue,
                result => {
                    result?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn definition_member(&mut self, m: &Rc<Member>, k: usize, ctx: &Ctx) -> R<ObjId> {
        let fr = ctx.frame.clone().unwrap();
        let usage = fr.borrow().usage.unwrap();
        let definition = fr.borrow().definition.unwrap();
        let data = self.def(definition);
        let modern = self.follows_0_4(data.rec.map_or(data.def_unit, |rec| self.rec_unit[rec]));
        let key = (super::eval::ptr(&**m), usage, if modern { ctx.scene } else { None });
        if let Some(v) = self.tm_memo.get(&key) {
            return Ok(*v);
        }
        if self.tm_in_progress.contains(&key) {
            return self.fail(ctx, codes::REF_NAME_CYCLE, "the name refers to itself in a cycle (6.6)", m.text_start());
        }
        self.tm_in_progress.insert(key);
        let dfn = fr.borrow().definition.unwrap();
        let d = self.def(dfn).clone();
        let origin = self.usage(usage).ctx_origin.clone();
        let mut sub = ctx.clone();
        sub.unit = d.def_unit;
        sub.rec = d.rec;
        sub.header = d.header.clone();
        sub.stmt = Some(StmtSpan { rec: d.rec, start: m.start, end: m.end });
        sub.recursive_selection = false;
        sub.written_selection = false;
        sub.body_scene = self.usage(usage).site.as_ref().and_then(|site| site.scene);
        sub.pos = pos_plus(&ctx.pos, "b", k as i64);
        sub.origin = if origin.is_empty() { ctx.origin.clone() } else { origin };
        sub.nested = None;
        sub.defining = Some(d.defining.clone());
        sub.def_unit = Some(d.def_unit);
        let (r, bound) = self.eval_member(m, &sub);
        self.tm_in_progress.remove(&key);
        self.remember_body_rule(m, k, ctx);
        let val = match r {
            Ok(v) => v,
            Err(Stop::Fail(dg)) => self.new_obj(ObjData::Failed(dg)),
            Err(Stop::LocalRef) if modern && sub.body_scene != ctx.scene => {
                if self.generation_rule(m, sub.unit) {
                    self.warn(
                        &sub,
                        codes::RULE_USES_OWN_LEVEL_OBJECTS,
                        "this rule uses objects of the level it is written in, so it takes effect only there (10.5)",
                        m.text_start(),
                    );
                }
                return Err(Stop::LocalRef);
            }
            Err(Stop::LocalRef) | Err(Stop::OwnLevel) => self.new_obj(ObjData::Failed(None)),
            Err(Stop::NoCircuit) => return Err(Stop::NoCircuit),
            Err(Stop::Language) => return Err(Stop::Language),
        };
        self.tm_memo.insert(key, val);
        if modern {
            fr.borrow_mut().evaluated_members.insert(k);
        }
        let uname = self.objs[usage.0 as usize].name.clone();
        let mut bindings: Vec<_> = m.binds.iter().enumerate().filter(|(_, b)| b.prefix == '%').collect();
        if modern {
            bindings.sort_by_key(|(index, b)| (b.after, *index));
        }
        let mut occurrences: HashMap<String, usize> = HashMap::new();
        for (binding_index, b) in bindings {
            let occurrence = occurrences.entry(b.name.clone()).or_default();
            let binding = bound.iter().filter(|(kk, _, _)| kk.0 == '%' && kk.1 == b.name).nth(if modern {
                *occurrence
            } else {
                0
            });
            *occurrence += 1;
            let v = binding.map(|(_, v, _)| *v).unwrap_or(val);
            if modern {
                let valid = !self.is_failed(v);
                let old = {
                    let frame = fr.borrow();
                    frame.local(&b.name).zip(frame.local_bindings.get(&b.name).copied())
                };
                if let Some((old_value, (old_member, old_binding, old_valid))) = old {
                    let old_first = (old_member, old_binding) <= (k, binding_index);
                    if old_valid && valid && old_value != v {
                        let duplicate = if old_first { (k, binding_index) } else { (old_member, old_binding) };
                        if fr.borrow_mut().local_binding_errors.insert(duplicate) {
                            let off = d.body.as_ref().unwrap().members[duplicate.0].binds[duplicate.1].off;
                            self.err(&sub, codes::DEFINE_LOCAL_NAME_TWICE,
                                format!("the local name {} is already bound to another object; the first valid binding is kept (6.6)", b.name),off);
                        }
                    }
                    if (old_valid && (!valid || old_first)) || (!old_valid && !valid && old_first) {
                        continue;
                    }
                }
                fr.borrow_mut().local_bindings.insert(b.name.clone(), (k, binding_index, valid));
            }
            fr.borrow_mut().set_local(&b.name, v);
            if let Some((_, _, dependencies)) = binding {
                fr.borrow_mut().local_dependencies.insert(b.name.clone(), dependencies.clone());
            }
            if modern && sub.body_scene != ctx.scene {
                continue;
            }
            let u = self.usage_mut(usage);
            if let Some(e) = u.locals.iter_mut().find(|(n, _)| *n == b.name) {
                e.1 = v;
            } else {
                u.locals.push((b.name.clone(), v));
            }
            let nameable = matches!(self.o(v), ObjData::Route { .. });
            if nameable && self.objs[v.0 as usize].name.is_empty() {
                self.objs[v.0 as usize].name =
                    if uname.is_empty() { b.name.clone() } else { format!("{uname}::{}", b.name) };
            }
        }
        Ok(val)
    }

    /// A usage inside a symbol body: drawn in the symbol only (9.2.7).
    fn symbol_usage(&mut self, app: &Application, ctx: &Ctx, seg: &Segment) -> R<(ObjId, bool)> {
        let d = self.def(app.dfn).clone();
        let first = d.roles.first().map(|r| r.name.clone());
        let pt = first.as_ref().and_then(|f| app.filled.iter().find(|(n, _)| n == f).map(|(_, a)| a.obj));
        let Some(pt) = pt.filter(|p| self.symbol_point(*p, self.follows_0_4(self.source_unit(ctx))).is_some()) else {
            return self.fail(
                ctx,
                codes::USAGE_IN_SYMBOL_FIRST_ROLE_NOT_POINT,
                "a usage in a symbol body has its first role filled with a point of the body (9.2.7)",
                seg.start,
            );
        };
        let symbol = ctx.frame.as_ref().unwrap().borrow().symbol.unwrap();
        let u = UsageData {
            definition: app.dfn,
            roles: vec![(first.unwrap(), pt)],
            role_sets: Vec::new(),
            params: app.params.clone(),
            given: app.given.clone(),
            locals: Vec::new(),
            parent: None,
            generated: false,
            priority: None,
            in_symbol: true,
            sym_symbol: None,
            sym_orient: Orient::R0,
            own_of: Some(symbol),
            site: Some(Rc::new(ctx.clone())),
            off: seg.start,
            frame: None,
            ctx_origin: ctx.origin.clone(),
        };
        let a = self.new_obj(ObjData::Usage(Box::new(u)));
        self.symbol_mut(symbol).push_usage(a);
        Ok((a, false))
    }

    // ------------------------------------------------------------------ symbol bodies (9.2)

    fn eval_symbol_body(
        &mut self,
        sym: ObjId,
        body: &Rc<SetExpr>,
        ctx: &Ctx,
        defining: Rc<super::ctx::Defining>,
    ) -> R<()> {
        let mut fr = Frame::new(FrameKind::Symbol);
        fr.symbol = Some(sym);
        let nested: super::ctx::NestedRef = Rc::new(RefCell::new(HashMap::new()));
        let sctx = Ctx {
            scene: None,
            unit: ctx.unit,
            rec: ctx.rec,
            header: ctx.header.clone(),
            pos: ctx.pos.clone(),
            frame: Some(Rc::new(RefCell::new(fr))),
            origin: ctx.origin.clone(),
            nested: Some(nested.clone()),
            member_key: ctx.member_key.clone(),
            selected: false,
            recursive_selection: false,
            written_selection: false,
            body_scene: None,
            defining: Some(defining),
            def_unit: Some(ctx.def_unit.unwrap_or(ctx.unit)),
            scope_in: false,
            stmt: ctx.stmt,
            segments: ctx.segments.clone(),
            modifier_input: Default::default(),
            reference_dependencies: Default::default(),
            language_inputs: Vec::new(),
        };
        for m in body.members.iter() {
            if self.follows_0_4(self.source_unit(&sctx))
                && self.body_member_errors.contains_key(&super::eval::ptr(&**m))
            {
                continue;
            }
            if let Some(err) = &m.error {
                self.err(ctx, err.code, err.message.clone(), m.error_pos);
                continue;
            }
            if m.segments.len() == 1 && m.segments[0].call.is_none() && m.bind().is_none() {
                for a in &m.segments[0].args {
                    let (obj, _) = match self.eval_obj(&a.obj, &sctx) {
                        Ok(v) => v,
                        Err(Stop::Fail(_)) => break,
                        Err(o) => return Err(o),
                    };
                    match self.o(obj) {
                        ObjData::Symbol(other) => {
                            let other = (**other).clone();
                            if self.symbol(sym).independent_pins && !self.include_pin_names(sym, &other, &sctx, a.start)
                            {
                                continue;
                            }
                            merge_symbol(self.symbol_mut(sym), &other);
                        }
                        ObjData::Point { .. } | ObjData::SymbolPin { .. } => {}
                        _ => {
                            self.err(
                                &sctx,
                                codes::DEFINE_SYMBOL_BODY_BAD_OBJECT, "a symbol body holds only graphics, pins, points, merged symbols, usages that only draw and texts on `?` (9.2.2)",
                                a.start,
                            );
                            break;
                        }
                    }
                }
                continue;
            }
            let (r, bound) = self.eval_member(m, &sctx);
            for (key, v, dependencies) in bound {
                let off = m
                    .binds
                    .iter()
                    .rev()
                    .find(|b| (b.prefix, &b.name) == (key.0, &key.1))
                    .map_or(m.text_start(), |b| b.off);
                if self.symbol(sym).independent_pins && !self.bind_symbol_name(sym, &key, v, &sctx, off) {
                    continue;
                }
                nested.borrow_mut().insert(key, Nested::Val(v, dependencies));
            }
            match r {
                Ok(_) | Err(Stop::Fail(_)) => {}
                Err(Stop::NoCircuit) => {
                    self.err(
                        &sctx,
                        codes::DEFINE_SYMBOL_BODY_BAD_STATEMENT,
                        "this statement cannot be written in a symbol body (9.2.2)",
                        m.start,
                    );
                }
                Err(Stop::LocalRef) => return Err(Stop::LocalRef),
                Err(Stop::OwnLevel) => return Err(Stop::OwnLevel),
                Err(Stop::Language) => return Err(Stop::Language),
            }
        }
        Ok(())
    }

    fn kw_pin(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        if !ctx.in_symbol() {
            let call = seg.call.as_ref().unwrap();
            if let Some(diagnostic) = self.reported_body_error(call, ctx) {
                return Err(Stop::Fail(Some(diagnostic)));
            }
            let at = call.start;
            return self.fail(
                ctx,
                codes::KEYWORD_PIN_OUTSIDE_SYMBOL,
                "pins cannot be written in the scene; PIN is written only in a DEFINE-SYMBOL body (13.4)",
                at,
            );
        }
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
        let pt = mapped["position"].obj;
        let Some((x, y)) = self.symbol_point(pt, self.follows_0_4(self.source_unit(ctx))) else {
            return self.fail(ctx, codes::KEYWORD_PIN_NOT_A_POINT, "PIN takes one point (9.2.4)", seg.start);
        };
        let raw = match get(&p, "terminal").cloned() {
            Some(Lit::Arr(v)) => v,
            Some(other) => vec![other],
            None => Vec::new(),
        };
        if raw.is_empty() && get(&p, "terminal").is_some() {
            let at = seg.call.as_ref().unwrap().start;
            return self.fail(
                ctx,
                codes::PARAM_PIN_TERMINAL_EMPTY,
                "the terminal= array of a bus pin has at least one element (9.2.4)",
                at,
            );
        }
        let raw = if self.follows_0_4(self.source_unit(ctx)) {
            let mut normalized = Vec::with_capacity(raw.len());
            for value in raw {
                if let Lit::Str(name) = &value {
                    if !name.as_str().is_empty() && name.as_str().bytes().all(|b| b.is_ascii_digit()) {
                        let Some(TermKey::Num(number)) = super::terminal_maps::terminal_key(&value) else {
                            return self.fail(ctx, codes::PARAM_PIN_TERMINAL_NUMBER,
                                "a digits-only terminal name is a number starting at 1; this implementation supports terminal numbers through 4294967295 (6.4)", seg.start);
                        };
                        normalized.push(Lit::Num(number as f64));
                        continue;
                    }
                }
                normalized.push(value);
            }
            normalized
        } else {
            raw
        };
        let terms: Vec<TermKey> = raw
            .iter()
            .filter_map(|t| match t {
                Lit::Num(n) => Some(TermKey::Num(*n as u32)),
                Lit::Str(s) => Some(TermKey::Name(s.as_str().to_string())),
                _ => None,
            })
            .collect();
        let label = get(&p, "label").and_then(|v| match v {
            Lit::Str(s) => Some(s),
            _ => None,
        });
        if let Some(l) = label {
            self.check_markup(l, ctx, seg);
        }
        let label_name_formats = label.is_some_and(|l| self.follows_0_4(l.unit.unwrap_or(self.source_unit(ctx))));
        let label = label.map(|l| l.as_str().to_string());
        let dir = get(&p, "direction").and_then(Lit::str).map(|d| match d {
            "right" => 0,
            "up" => 90,
            "left" => 180,
            _ => 270,
        });
        let bus = self.follows_0_4(self.source_unit(ctx)) && matches!(get(&p, "terminal"), Some(Lit::Arr(_)));
        let pin = Pin { names: Vec::new(), terms: terms.clone(), bus, pos: (x, y), label, label_name_formats, dir };
        let symbol = ctx.frame.as_ref().unwrap().borrow().symbol.unwrap();
        let independent = self.symbol(symbol).independent_pins;
        let index = self.symbol(symbol).pins.len();
        add_pin(self.symbol_mut(symbol), pin);
        if x != x.round() || y != y.round() {
            self.hint_at(ctx, codes::KEYWORD_PIN_OFF_GRID, "pins are best placed on integer points (9.2.4)", seg.start);
        }
        let result = if independent { self.new_obj(ObjData::SymbolPin { symbol, index }) } else { pt };
        Ok((result, false))
    }

    fn kw_prim(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let sig = words().keyword(kw).unwrap();
        let mut spec = sig.param_list().to_vec();
        spec.extend(style_specs(sig.style.as_deref().unwrap_or("line_style")));
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let positions = [PositionSpec {
            name: "points".into(),
            takes: Takes::Set,
            takes_versions: Vec::new(),
            kinds: Vec::new(),
            kind_versions: Vec::new(),
            kind_orders: Vec::new(),
            optional: false,
        }];
        let mapped = self.map_args(kw, args, seg, ctx, &positions)?;
        let obj = mapped["points"].obj;
        let (style, rest) = split_style(&p);
        let params = PrimParams {
            width: get(&rest, "width").and_then(Lit::num),
            height: get(&rest, "height").and_then(Lit::num),
            radius: get(&rest, "radius").and_then(Lit::num),
            fill: get(&rest, "fill").and_then(Lit::str).map(String::from),
            arrow: get(&rest, "arrow").and_then(Lit::str).map(String::from),
        };
        let shape = Shape::parse(kw).unwrap();
        let centred = (matches!(shape, Shape::Rectangle | Shape::Ellipse)
            && (params.width.is_some() || params.height.is_some()))
            || (shape == Shape::Circle && params.radius.is_some());
        if self.is_lazy(obj) {
            if !self.follows_0_4(self.source_unit(ctx)) {
                return self.fail(
                    ctx,
                    codes::KEYWORD_GRAPHIC_POINTS_SELECTION,
                    format!("the points of @{kw} take no selection (9.2.5)"),
                    seg.start,
                );
            }
            if !centred && !matches!(shape, Shape::Line | Shape::Arc | Shape::Bezier) {
                return self.fail(
                    ctx,
                    codes::KEYWORD_GRAPHIC_POINTS_SELECTION,
                    format!("the points of @{kw} take no selection in this form (9.2.5, 10.4)"),
                    seg.start,
                );
            }
            if matches!(shape, Shape::Rectangle | Shape::Ellipse)
                && !(params.width.is_some() && params.height.is_some())
            {
                return self.fail(
                    ctx,
                    codes::PARAM_GRAPHIC_WIDTH_HEIGHT,
                    format!("write both width and height of @{kw} (9.2.5)"),
                    seg.call.as_ref().unwrap().start,
                );
            }
            let key = ("graphic-gen", ptr_seg(seg), self.frame_key(ctx), 0usize);
            let gen = match self.node_memo.get(&key) {
                Some(&gen) => gen,
                None => {
                    let gen = self.new_obj(ObjData::Set(Box::new(SetData {
                        kind: SetKind::Gen { by_scene: HashMap::new() },
                        reach_site: Some(Rc::new(ctx.clone())),
                        reach_sources: vec![obj],
                        ..SetData::plain(Vec::new(), false)
                    })));
                    self.node_memo.insert(key, gen);
                    gen
                }
            };
            if ctx.scene.is_none() {
                self.note_rule(ctx);
                return Ok((gen, true));
            }
            let c = Rc::new(ctx.clone());
            let seg = seg.clone();
            let keyword = kw.to_string();
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, phase: usize| {
                let members: Vec<_> = it
                    .flatten_inputs(s, obj, phase, Some(&c))
                    .into_iter()
                    .filter(|member| it.in_generation_snapshot(s, member.object))
                    .collect();
                let mut out = Vec::new();
                if centred {
                    for (index, member) in members.into_iter().enumerate() {
                        let mut sub = c.with_dependencies(&member.dependencies);
                        sub.pos = pos_plus(&c.pos, "g", index as i64);
                        sub.selected = true;
                        if let Ok((object, _)) =
                            it.make_graphics(&keyword, shape, true, member.object, &params, &style, &sub, &seg)
                        {
                            out.push(object);
                        }
                    }
                } else if !members.is_empty() {
                    let mut sub = (*c).clone();
                    sub.pos = pos_plus(&c.pos, "g", 0);
                    sub.selected = true;
                    let mut set = SetData::plain(members.iter().map(|m| (m.object, true)).collect(), false);
                    set.item_dependencies = members.into_iter().map(|m| m.dependencies).collect();
                    let input = it.new_obj(ObjData::Set(Box::new(set)));
                    if let Ok((object, _)) =
                        it.make_graphics(&keyword, shape, false, input, &params, &style, &sub, &seg)
                    {
                        out.push(object);
                    }
                }
                if let ObjData::Set(data) = it.om(gen) {
                    if let SetKind::Gen { by_scene } = &mut data.kind {
                        by_scene.insert(s, out);
                    }
                }
            });
            self.defer(ctx, 5, task)?;
            return Ok((gen, true));
        }
        self.make_graphics(kw, shape, centred, obj, &params, &style, ctx, seg)
    }

    #[allow(clippy::too_many_arguments)]
    fn make_graphics(
        &mut self,
        kw: &str,
        shape: Shape,
        centred: bool,
        obj: ObjId,
        params: &PrimParams,
        style: &StyleMap,
        ctx: &Ctx,
        seg: &Segment,
    ) -> R<(ObjId, bool)> {
        let pts = self.shape_points(kw, obj, ctx, seg)?;
        if centred {
            if matches!(shape, Shape::Rectangle | Shape::Ellipse)
                && !(params.width.is_some() && params.height.is_some())
            {
                let at = seg.call.as_ref().unwrap().start;
                return self.fail(
                    ctx,
                    codes::PARAM_GRAPHIC_WIDTH_HEIGHT,
                    format!("write both width and height of @{kw} (9.2.5)"),
                    at,
                );
            }
            if self.is_set(obj) && pts.len() > 1 {
                let mut outs = Vec::new();
                for (k, member) in pts.into_iter().enumerate() {
                    let q = member.object;
                    let member_ctx = ctx.with_dependencies(&member.dependencies);
                    outs.push(self.make_prim(shape, vec![q], params, style, &member_ctx, seg, Some((k, q)))?);
                }
                let s = self.new_obj(ObjData::Set(Box::new(SetData::plain(
                    outs.into_iter().map(|o| (o, true)).collect(),
                    false,
                ))));
                return Ok((s, false));
            }
            for member in &pts {
                ctx.depend_on(&member.dependencies);
            }
            let points = pts.into_iter().map(|member| member.object).collect();
            return Ok((self.make_prim(shape, points, params, style, ctx, seg, None)?, false));
        }
        let n = pts.len();
        let ok = graphic_point_count_is_valid(shape, n);
        if !ok {
            return self.fail(
                ctx,
                codes::KEYWORD_GRAPHIC_POINT_COUNT,
                format!("@{kw} has the wrong number of points (9.2.5)"),
                seg.start,
            );
        }
        for member in &pts {
            ctx.depend_on(&member.dependencies);
        }
        let points = pts.into_iter().map(|member| member.object).collect();
        Ok((self.make_prim(shape, points, params, style, ctx, seg, None)?, false))
    }

    fn shape_points(&mut self, kw: &str, obj: ObjId, ctx: &Ctx, seg: &Segment) -> R<Vec<InputMember>> {
        let pts = if self.is_set(obj) {
            let data = self.set(obj);
            data.items
                .iter()
                .enumerate()
                .map(|(index, &(object, _))| InputMember {
                    object,
                    dependencies: data.item_dependencies.get(index).cloned().unwrap_or_default(),
                })
                .collect()
        } else {
            vec![InputMember::plain(obj)]
        };
        for p in &pts {
            if !matches!(
                self.o(p.object),
                ObjData::Point { .. }
                    | ObjData::SymbolPin { .. }
                    | ObjData::Term { .. }
                    | ObjData::Pos { .. }
                    | ObjData::ScenePin { .. }
                    | ObjData::NamedSubobject { .. }
            ) {
                return self.fail(
                    ctx,
                    codes::KEYWORD_GRAPHIC_NOT_POINTS,
                    format!("the points of @{kw} are points (9.2.5)"),
                    seg.start,
                );
            }
        }
        Ok(pts)
    }

    /// A graphic; `each` is the point it is centred on, with its place in the set, when the
    /// statement makes one for each point of a set (5.2.1).
    #[allow(clippy::too_many_arguments)]
    fn make_prim(
        &mut self,
        shape: Shape,
        pts: Vec<ObjId>,
        params: &PrimParams,
        style: &StyleMap,
        ctx: &Ctx,
        seg: &Segment,
        each: Option<(usize, ObjId)>,
    ) -> R<ObjId> {
        if ctx.in_symbol() {
            let mut local = Vec::new();
            for q in &pts {
                match self.symbol_point(*q, self.follows_0_4(self.source_unit(ctx))) {
                    Some(p) => local.push(p),
                    _ => {
                        return self.fail(
                            ctx,
                            codes::KEYWORD_GRAPHIC_SYMBOL_POINTS,
                            "graphics in a symbol body use points of the body (9.2.1)",
                            seg.start,
                        )
                    }
                }
            }
            let symbol = ctx.frame.as_ref().unwrap().borrow().symbol.unwrap();
            self.symbol_mut(symbol).push_prim(SymPrim {
                shape,
                points: local,
                params: params.clone(),
                style: style.clone(),
                text: None,
            });
            return Ok(self.new_obj(ObjData::Prim {
                shape,
                points: pts,
                params: params.clone(),
                style: style.clone(),
                site: Some(Rc::new(ctx.clone())),
                off: seg.start,
            }));
        }
        let Some(s) = ctx.scene else { return self.scene_only(ctx, seg.start) };
        let prim = self.new_obj(ObjData::Prim {
            shape,
            points: pts,
            params: params.clone(),
            style: style.clone(),
            site: Some(Rc::new(ctx.clone())),
            off: seg.start,
        });
        self.scene_add_prim(s, prim, ctx.pos.clone(), ctx);
        self.note_birth(s, prim, ctx, seg.call.as_ref().unwrap().start, each);
        Ok(prim)
    }

    // ------------------------------------------------------------------ TEXT (13.3)

    fn kw_text(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mut spec = self.sig_params(kw, ctx);
        spec.extend(style_specs("text_style"));
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let att = mapped["attach"].clone();
        let (style, rest) = split_style(&p);
        let key = get(&rest, "key").cloned();
        let mut items = TextItems {
            text: get(&rest, "text").and_then(|v| match v {
                Lit::Str(s) => Some(match written_in(&Lit::Str(s.clone()), self.source_unit(ctx)) {
                    Lit::Str(w) => w,
                    _ => unreachable!(),
                }),
                _ => None,
            }),
            halign: get(&rest, "halign").and_then(Lit::str).map(String::from),
            valign: get(&rest, "valign").and_then(Lit::str).map(String::from),
            rotate: get(&rest, "rotate").and_then(Lit::num),
            fix_rotate: get(&rest, "fix_rotate").and_then(Lit::boolean),
            dx: get(&rest, "dx").and_then(Lit::num),
            dy: get(&rest, "dy").and_then(Lit::num),
        };
        let call_start = seg.call.as_ref().unwrap().start;
        if let Some(t) = &items.text {
            self.check_markup(t, ctx, seg);
        }
        // the file the key is written in: its aliases decide what `::` finds first (13.3, 16.5)
        let key_unit = match &key {
            Some(Lit::Str(s)) => s.unit.unwrap_or(ctx.unit),
            _ => ctx.unit,
        };
        let key: Option<String> = key.and_then(|k| k.str().map(String::from));
        if let Some(k) = &key {
            if let Some((code, clash)) = self.key_clash(k, key_unit, None) {
                return self.fail(ctx, code, clash, call_start);
            }
        }
        if ctx.in_symbol() {
            // dx, dy in a symbol are local, like the point they offset (13.3, PR14)
            let off = if items.dx.is_some() || items.dy.is_some() {
                let o = (items.dx.take().unwrap_or(0.0), items.dy.take().unwrap_or(0.0));
                Some(o)
            } else {
                None
            };
            let symbol = ctx.frame.as_ref().unwrap().borrow().symbol.unwrap();
            if att.obj == OWNER {
                // a usage's role and local names are known when the symbol is given to it (phase 6)
                let lab = SymLabel {
                    key,
                    items,
                    position: off,
                    style,
                    unit: key_unit,
                    site: Some(Rc::new(ctx.clone())),
                    off: call_start,
                };
                let sm = self.symbol_mut(symbol);
                sm.labels.push(lab);
                let index = sm.labels.len() - 1;
                return Ok((self.new_obj(ObjData::SymLabel { symbol, index }), false));
            }
            if let Some((x, y)) = self.symbol_point(att.obj, self.follows_0_4(self.source_unit(ctx))) {
                if key.is_some() {
                    return self.fail(
                        ctx,
                        codes::TEXT_POINT_TAKES_NO_KEY,
                        "a text attached to a point takes no key= (13.3)",
                        call_start,
                    );
                }
                let d = off.unwrap_or((0.0, 0.0));
                self.symbol_mut(symbol).push_prim(SymPrim {
                    shape: Shape::Text,
                    points: vec![(x + d.0, y + d.1)],
                    params: PrimParams::default(),
                    style: style.clone(),
                    text: Some(items),
                });
                if self.follows_0_4(self.source_unit(ctx)) {
                    // Naming this graphic must not add a name to the pin it uses (5.1).
                    let text_point = self.new_obj(ObjData::Point { x: Some(x + d.0), y: Some(y + d.1), local: true });
                    return Ok((
                        self.new_obj(ObjData::Prim {
                            shape: Shape::Text,
                            points: vec![text_point],
                            params: PrimParams::default(),
                            style,
                            site: Some(Rc::new(ctx.clone())),
                            off: seg.start,
                        }),
                        false,
                    ));
                }
                return Ok((att.obj, false));
            }
            return self.fail(
                ctx,
                codes::KEYWORD_TEXT_SYMBOL_ATTACH,
                "a TEXT in a symbol body is attached to a point or to `?` (9.2.5)",
                seg.start,
            );
        }
        if self.is_lazy(att.obj) {
            if ctx.scene.is_none() {
                self.note_rule(ctx);
                return Ok((NOTHING, true));
            }
            let c = Rc::new(ctx.clone());
            let lazy = att.obj;
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, phase: usize| {
                let members = it.flatten_inputs(s, lazy, phase, Some(&c));
                for (k, member) in members.into_iter().enumerate() {
                    let member_ctx = c.with_dependencies(&member.dependencies);
                    let _ = it.scene_add_text(
                        s,
                        member.object,
                        key.clone(),
                        items.clone(),
                        style.clone(),
                        pos_plus(&c.pos, "g", k as i64),
                        &member_ctx,
                        true,
                        key_unit,
                        call_start,
                    );
                }
            });
            self.defer(ctx, 5, task)?;
            return Ok((NOTHING, true));
        }
        let Some(s) = ctx.scene else { return self.scene_only(ctx, seg.start) };
        if self.is_set(att.obj) {
            let members = self.flatten_inputs(s, att.obj, 3, Some(ctx));
            let mut outs = Vec::new();
            for (k, member) in members.into_iter().enumerate() {
                let obj = member.object;
                let member_ctx = ctx.with_dependencies(&member.dependencies);
                let t = self.scene_add_text(
                    s,
                    obj,
                    key.clone(),
                    items.clone(),
                    style.clone(),
                    pos_plus(&ctx.pos, "e", k as i64),
                    &member_ctx,
                    false,
                    key_unit,
                    call_start,
                )?;
                self.set_text_off(t, seg.start);
                self.note_birth(s, t, ctx, call_start, Some((k, obj)));
                outs.push(t);
            }
            let set = self
                .new_obj(ObjData::Set(Box::new(SetData::plain(outs.into_iter().map(|t| (t, true)).collect(), false))));
            return Ok((set, att.sel));
        }
        let t =
            self.scene_add_text(s, att.obj, key, items, style, ctx.pos.clone(), ctx, false, key_unit, call_start)?;
        self.set_text_off(t, seg.start);
        self.note_birth(s, t, ctx, call_start, None);
        Ok((t, att.sel))
    }

    fn set_text_off(&mut self, t: ObjId, off: usize) {
        match self.om(t) {
            ObjData::Text { off: o, .. } => *o = off,
            ObjData::KeyText { off: o, direct, .. } => {
                *o = off;
                // a TEXT written for the object itself, as this one is, is the last direct one
                if let Some((_, d)) = direct {
                    *d = off;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn check_markup(&mut self, text: &WStr, ctx: &Ctx, seg: &Segment) {
        let at = seg.call.as_ref().map(|c| c.start).unwrap_or(seg.start);
        let name_formats = self.follows_0_4(text.unit.unwrap_or(self.source_unit(ctx)));
        for (code, msg) in crate::markup::problems_with_name_formats(text.as_str(), name_formats) {
            self.warn(ctx, code, format!("{msg} (13.6)"), at);
        }
    }

    // ------------------------------------------------------------------ placement (11.1, 11.2, 11.6)

    #[allow(clippy::too_many_arguments)]
    fn target_each(
        &mut self,
        kw: &str,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        phase: usize,
        op: TargetOp,
        position: &str,
    ) -> R<(ObjId, bool)> {
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        self.target_each_mapped(kw, mapped, seg, ctx, phase, op, position)
    }

    /// Run an operation for each member of a single-object position (5.2.1).
    #[allow(clippy::too_many_arguments)]
    fn target_each_mapped(
        &mut self,
        kw: &str,
        mapped: HashMap<String, ArgV>,
        seg: &Segment,
        ctx: &Ctx,
        phase: usize,
        op: TargetOp,
        position: &str,
    ) -> R<(ObjId, bool)> {
        let a = mapped[position].clone();
        if matches!(op, TargetOp::Symbol { .. }) {
            if let Some(scene) = ctx.scene {
                self.check_symbol_selection(scene, a.obj, ctx, seg.start)?;
            }
        }
        if let Some(d) = self.failed_obj(a.obj) {
            let id = self.err_cause(ctx, codes::KEYWORD_OBJECT_FAILED, format!("the object of @{kw} failed"), a.off, d);
            return Err(Stop::Fail(Some(id)));
        }
        let symbol_set =
            matches!(op, TargetOp::Symbol { .. }) && self.is_set(a.obj) && self.follows_0_4(self.source_unit(ctx));
        if self.is_lazy(a.obj) || symbol_set {
            if ctx.scene.is_none() {
                self.note_rule(ctx);
                return Ok((a.obj, true));
            }
            let c = Rc::new(ctx.clone());
            let lazy = a.obj;
            let op2 = op.clone();
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, ph: usize| {
                if let TargetOp::Symbol { off, .. } = &op2 {
                    if it.check_symbol_selection(s, lazy, &c, *off).is_err() {
                        return;
                    }
                }
                // PRIORITY runs at the geometry phase, when every GROUP of the level is in
                let members = if matches!(op2, TargetOp::Priority { .. }) && it.scenes[s].grouped(lazy) {
                    vec![InputMember::plain(lazy)]
                } else {
                    it.flatten_inputs(s, lazy, ph, Some(&c))
                };
                let direct = symbol_set && !it.is_lazy(lazy);
                for (k, m) in members.into_iter().enumerate() {
                    let member_ctx = c.with_dependencies(&m.dependencies);
                    let _ = it.apply_op(s, &op2, m.object, pos_plus(&c.pos, "g", k as i64), direct, &member_ctx);
                }
            });
            if matches!(op, TargetOp::Symbol { .. }) && ctx.frame_generated() && self.follows_0_4(self.source_unit(ctx))
            {
                self.queue_task(ctx.scene.unwrap(), 6, ctx, task);
            } else if phase == 7
                && matches!(op, TargetOp::Priority { .. })
                && ctx.frame_generated()
                && self.follows_0_4(self.source_unit(ctx))
            {
                // This runs after generation and makes no objects, so it does not recurse (10.5).
                self.queue_task(ctx.scene.unwrap(), phase, ctx, task);
            } else {
                self.defer(ctx, phase, task)?;
            }
            if matches!(op, TargetOp::Symbol { .. }) && !ctx.frame_generated() {
                // the same symbol queue runs again over generated objects (8.2 step 6)
                let c = Rc::new(ctx.clone());
                let task6: Task = Rc::new(move |it: &mut Interp, s: SceneId, ph: usize| {
                    if let TargetOp::Symbol { off, .. } = &op {
                        if it.check_symbol_selection(s, lazy, &c, *off).is_err() {
                            return;
                        }
                    }
                    let members = it.flatten_inputs(s, lazy, ph, Some(&c));
                    for (k, m) in members.into_iter().enumerate() {
                        if it.is_generated(m.object) {
                            let member_ctx = c.with_dependencies(&m.dependencies);
                            let _ = it.apply_op(s, &op, m.object, pos_plus(&c.pos, "g", k as i64), false, &member_ctx);
                        }
                    }
                });
                if let Some(s) = ctx.scene {
                    self.queue_task(s, 6, ctx, task6);
                }
            }
            return Ok((a.obj, a.sel || self.is_lazy(a.obj)));
        }
        let Some(s) = ctx.scene else {
            if ctx.in_symbol() {
                return Ok((self.symbol_modifier(kw, a.obj, seg, ctx, &op)?, false));
            }
            return self.scene_only(ctx, seg.start);
        };
        if self.is_set(a.obj) {
            let members = self.flatten_inputs(s, a.obj, 3, Some(ctx));
            let sd = self.set(a.obj);
            if members.is_empty() && sd.explicit && sd.items.is_empty() {
                self.warn(
                    ctx,
                    codes::SET_EMPTY_LITERAL,
                    "an empty set written directly: nothing happens (5.2.1)",
                    a.off,
                );
            }
            if let TargetOp::Priority { off, .. } = op {
                // PRIORITY takes a set with GROUP as the group itself, any other set member by
                // member (5.2.1, 14.8). GROUP may be written after this statement, or in a body
                // that generation expands (10.5), so which it is is known only at the geometry
                // phase, the first that reads priorities (8.2); the members are those of now.
                let (set, direct, c) = (a.obj, !a.sel, Rc::new(ctx.clone()));
                let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, _| {
                    let targets =
                        if it.scenes[s].grouped(set) { vec![InputMember::plain(set)] } else { members.clone() };
                    for (k, m) in targets.into_iter().enumerate() {
                        let member_ctx = c.with_dependencies(&m.dependencies);
                        if it.apply_op(s, &op, m.object, pos_plus(&c.pos, "e", k as i64), direct, &member_ctx).is_err()
                        {
                            it.note_failed_at(c.rec, off, false);
                            if !it.follows_0_4(it.source_unit(&c)) {
                                break;
                            }
                        }
                    }
                });
                if self.scenes[s].phase >= phase {
                    task(self, s, phase);
                } else {
                    self.queue_task(s, phase, ctx, task);
                }
                return Ok((a.obj, a.sel));
            }
            let independent_members = self.follows_0_4(self.source_unit(ctx));
            let mut failed = false;
            for (k, m) in members.into_iter().enumerate() {
                let member_ctx = ctx.with_dependencies(&m.dependencies);
                match self.apply_op(s, &op, m.object, pos_plus(&ctx.pos, "e", k as i64), !a.sel, &member_ctx) {
                    Ok(()) => {}
                    Err(Stop::Fail(_)) if independent_members => {
                        // Each member is its own description (5.2.1, 7.3).
                        failed = true;
                    }
                    Err(error) => return Err(error),
                }
            }
            if failed {
                self.note_failed_at(ctx.rec, seg.start, false);
            }
            // Modifiers add facts; failure does not replace their input handles (5.5, 6.6).
            return Ok((a.obj, a.sel));
        }
        // what a generation expands counts as reached through a selection (8.3), but a PRIORITY
        // there that names its object, such as `%in::row` in a body, still comes from outside
        // when written outside the object's usage (14.8): for it, only how the object is reached
        // counts, as for a set above
        let priority = matches!(op, TargetOp::Priority { .. });
        let direct = !a.sel && (priority || !ctx.selected);
        self.apply_op(s, &op, a.obj, ctx.pos.clone(), direct, ctx)?;
        Ok((a.obj, a.sel))
    }

    pub(crate) fn is_generated(&self, id: ObjId) -> bool {
        match self.o(id) {
            ObjData::Usage(u) => u.generated,
            ObjData::Wire { generated, .. } | ObjData::Text { generated, .. } | ObjData::KeyText { generated, .. } => {
                *generated
            }
            _ => false,
        }
    }

    fn symbol_modifier(&mut self, kw: &str, obj: ObjId, seg: &Segment, ctx: &Ctx, op: &TargetOp) -> R<ObjId> {
        let _ = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
        match (kw, self.o(obj).clone(), op) {
            ("PLACE", ObjData::SymLabel { symbol, index }, TargetOp::Place { x, y, .. }) => {
                self.symbol_mut(symbol).labels[index].position = Some((x.unwrap_or(0.0), y.unwrap_or(0.0)));
                return Ok(obj);
            }
            ("SET-SYMBOL", ObjData::Usage(u), TargetOp::Symbol { symbol, map, off, .. }) if u.in_symbol => {
                self.check_symbol_pin_names(*symbol, Some(obj), ctx, *off)?;
                let sym = if let Some(map) = map {
                    let target = super::terminal_maps::MapTarget::roles(self.def(u.definition));
                    self.mapped_symbol(*symbol, map, &target, ctx, *off)?
                } else {
                    *symbol
                };
                self.usage_mut(obj).sym_symbol = Some(sym);
                return Ok(obj);
            }
            ("ORIENT", ObjData::Usage(u), TargetOp::Orient { code, .. }) if u.in_symbol => {
                self.usage_mut(obj).sym_orient = *code;
                return Ok(obj);
            }
            _ => {}
        }
        self.fail(
            ctx,
            codes::KEYWORD_SYMBOL_MODIFIER_TARGET,
            format!("in a symbol body @{kw} applies only to a text on `?` or a usage in the body (9.2.2)"),
            seg.start,
        )
    }

    fn kw_set_symbol(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let params = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
        let map = get(&params, "terminals")
            .map(|value| self.terminal_map(value, ctx, seg.call.as_ref().unwrap().start))
            .transpose()?;
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let sym = self.anon_def(mapped["symbol"].obj);
        if !matches!(self.o(sym), ObjData::Symbol(_)) {
            let l = self.label(sym);
            // a symbol of the standard series 0.2 whose name the series 0.3 gives to a definition
            let why = l.strip_prefix('$').and_then(|n| self.no_longer_a_symbol(n, ctx));
            let why = why.map(|w| format!(": {w}")).unwrap_or_default();
            return self.fail(
                ctx,
                codes::KEYWORD_SET_SYMBOL_NOT_A_SYMBOL,
                format!("the last object of SET-SYMBOL must be a symbol, not {l} (11.3){why}"),
                mapped["symbol"].off,
            );
        }
        let name = seg.args.iter().find_map(|arg| {
            if arg.start != mapped["symbol"].off {
                return None;
            }
            let crate::syntax::ObjExpr::Ref(reference) = &arg.obj else { return None };
            (reference.prefix == '$' && reference.path.is_empty() && reference.places.is_none())
                .then(|| reference.name.as_ref().map(|name| format!("${name}")))
                .flatten()
        });
        if mapped["target"].obj == LEVEL {
            return Ok((self.block_symbol(sym, name, map.as_deref(), seg, ctx)?, false));
        }
        self.target_each_mapped(
            kw,
            mapped,
            seg,
            ctx,
            4,
            TargetOp::Symbol { symbol: sym, name, map, off: seg.start },
            "target",
        )
    }

    /// `# @SET-SYMBOL $X` in a subcircuit gives the default look of its instances: the block symbol
    /// (9.2.6). Written in the same subcircuit several times, the later one wins (8.3).
    fn block_symbol(
        &mut self,
        sym: ObjId,
        symbol_name: Option<String>,
        map: Option<&super::terminal_maps::TerminalMap>,
        seg: &Segment,
        ctx: &Ctx,
    ) -> R<ObjId> {
        if self.follows_0_4(self.source_unit(ctx)) && ctx.frame_kind() == Some(FrameKind::Definition) {
            let call = seg.call.as_ref().unwrap();
            let diagnostic = match self.body_errors.get(&super::eval::ptr(call)) {
                Some(diagnostic) => *diagnostic,
                None => {
                    let diagnostic = self.err(
                        ctx,
                        codes::DEFINE_BODY_BLOCK_SYMBOL,
                        "a definition body cannot set the block symbol of its level (6.9, 9.2.6)",
                        call.start,
                    );
                    self.body_errors.insert(super::eval::ptr(call), diagnostic);
                    diagnostic
                }
            };
            return Err(Stop::Fail(Some(diagnostic)));
        }
        let u = ctx.unit;
        let level = self.units[u].level.filter(|_| self.units[u].kind == super::unit::UnitKind::Level);
        match level {
            Some(lv) if ctx.frame.is_none() && self.doc.levels[lv].kind == crate::host::LevelKind::Subckt => {
                self.check_symbol_pin_names(sym, None, ctx, seg.start)?;
                let sym = if let Some(map) = map {
                    let level = &self.doc.levels[lv];
                    let target = super::terminal_maps::MapTarget {
                        names: level
                            .ports
                            .iter()
                            .enumerate()
                            .map(|(i, name)| (i as u32 + 1, Some(name.clone())))
                            .collect(),
                        roles: false,
                        case_sensitive: level.case_sensitive,
                    };
                    self.mapped_symbol(sym, map, &target, ctx, seg.start)?
                } else {
                    sym
                };
                let name = self.doc.level_key(lv);
                let (rec, k) = match &ctx.member_key {
                    MemberKey::M { rec, k, .. } => (*rec as i64, *k as i64),
                    _ => (0, 0),
                };
                self.block_symbols.entry(name).or_default().insert(
                    ctx.member_key.clone(),
                    super::scene::BlockSymbol { pos: pos(&[rec, k]), symbol: sym, name: symbol_name },
                );
            }
            _ => {
                let at = seg.call.as_ref().unwrap().start;
                self.warn(ctx, codes::KEYWORD_SET_SYMBOL_BLOCK_NO_EFFECT, "only `# @SET-SYMBOL` in a subcircuit is a block symbol; a file level has no instances, so this has no effect (9.2.6)", at);
            }
        }
        Ok(LEVEL)
    }

    // ------------------------------------------------------------------ sets (5.2.2)

    fn kw_mod(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx, add: bool) -> R<(ObjId, bool)> {
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let items = mapped["items"].obj;
        let target = mapped["target"].clone();
        if self.follows_0_4(self.source_unit(ctx)) && self.is_set(items) && !self.is_lazy(items) {
            self.rule_input(items, seg.start, 0, ctx)?;
        }
        if self.follows_0_4(self.source_unit(ctx)) && !self.rule_reaches(ctx, [items]) {
            return Err(Stop::OwnLevel);
        }
        let plain_set = matches!(self.o(target.obj), ObjData::Set(s) if
            self.follows_0_4(self.source_unit(ctx)) || matches!(s.kind, SetKind::Plain | SetKind::Select { .. }));
        if !plain_set {
            return self.fail(
                ctx,
                codes::SET_EDIT_TARGET_NOT_SET,
                format!("the target of {kw} must be a set; nothing is wrapped (5.2.2)"),
                target.off,
            );
        }
        // a set from a package, edited level by level, is evaluated per level like a selection (10.3)
        let per_level = self.is_lazy(items) || (self.is_set(items) && !self.scene_bound(items));
        if per_level {
            self.note_rule(ctx);
        }
        if ctx.scene.is_none() && !per_level {
            return self.scene_only(ctx, seg.start);
        }
        // the sources kept last that have since come to stay not lazy, such as a level's set edited
        // with a selection after it was added here, are let go, so later levels do not go through them
        while let Some(&o) = self.set(target.obj).lazy_sources.last() {
            if !self.stays_not_lazy(o) {
                break;
            }
            self.set_mut(target.obj).lazy_sources.pop();
        }
        let m = SetMod { pos: ctx.pos.clone(), add, source: items, ctx: Rc::new(ctx.clone()), off: seg.start };
        self.check_edit(target.obj, &m, ctx)?;
        let (lazy, for_good) = (!self.stays_not_lazy(items), self.lazy_for_good(items));
        self.set_mut(target.obj).add_mod(m, lazy, for_good);
        Ok((target.obj, self.is_lazy(target.obj)))
    }

    // ------------------------------------------------------------------ selections (10.1)

    fn selection_params(&mut self, kw: &str, seg: &Segment, ctx: &Ctx, at_definition: bool) -> R<Vec<(String, Lit)>> {
        let call = seg.call.as_ref().unwrap();
        if call.params.as_ref().is_some_and(|p| !p.positional.is_empty()) {
            return self.fail(
                ctx,
                codes::SELECT_POSITIONAL_CONDITION,
                "the conditions of a selection are written by name only (10.1)",
                call.start,
            );
        }
        let spec = self.sig_params(kw, ctx);
        let p = if at_definition {
            self.params_at_definition(kw, seg, ctx, &spec, false)?
        } else {
            self.params(kw, seg, ctx, &spec, None, false, None)?
        };
        if let Some(n) = get(&p, "terminals").and_then(Lit::num) {
            if n < 0.0 || n.fract() != 0.0 {
                return self.fail(
                    ctx,
                    codes::ASSERT_TERMINALS_NOT_WHOLE,
                    "terminals= is a whole number of terminals (10.1)",
                    call.start,
                );
            }
        }
        Ok(p)
    }

    fn kw_select(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let call = seg.call.as_ref().unwrap();
        let p = self.selection_params(kw, seg, ctx, false)?;
        let scope = mapped["scope"].obj;
        let mut dfn = mapped.get("definition").map(|a| a.obj);
        if (kw == "SELECT-USAGE" && dfn.is_some()) || kw == "SELECT-ANCESTOR-USAGE" {
            let d = self.anon_def(dfn.unwrap_or(NOTHING));
            if !matches!(self.o(d), ObjData::Definition(_)) || d == NOTHING {
                let l = self.label(d);
                return self.fail(
                    ctx,
                    codes::SELECT_DEFINITION_NOT_DEFINITION,
                    format!("the last object of {kw} must be an ordinary definition, not {l} (10.1)"),
                    seg.start,
                );
            }
            if kw == "SELECT-ANCESTOR-USAGE" && scope == LEVEL {
                return self.fail(
                    ctx,
                    codes::SELECT_ANCESTOR_SCOPE_LEVEL,
                    "the scope of SELECT-ANCESTOR-USAGE cannot be `#`: this level is no one's member (10.1)",
                    seg.start,
                );
            }
            dfn = Some(d);
        }
        if get(&p, "role").is_some()
            && !(matches!(self.o(scope), ObjData::Usage(_)) || self.is_lazy(scope) || self.is_set(scope))
        {
            self.warn(
                ctx,
                codes::SELECT_ROLE_WITHOUT_USAGE_SCOPE,
                "role= means something only when the scope is a usage (K3.12)",
                call.start,
            );
        }
        // a rule even where it fails for closing a cycle of sets (5.2.2), which it may not do below
        self.note_rule(ctx);
        self.check_closing(ctx, seg.start)?;
        let key = ("sel", ptr_seg(seg), self.frame_key(ctx), if scope == LEVEL { 0 } else { scope.0 as usize + 1 });
        if let Some(&s) = self.node_memo.get(&key) {
            return Ok((s, true));
        }
        let mut params: BTreeMap<String, Lit> = p.into_iter().collect();
        if self.follows_0_4(self.source_unit(ctx)) {
            params.entry("recursive".into()).or_insert(Lit::Bool(false));
        }
        let sd = SetData {
            kind: SetKind::Select {
                keyword: kw.to_string(),
                scope,
                params,
                writer: self.source_unit(ctx),
                definition: dfn,
                site: Some(Rc::new(ctx.clone())),
                off: seg.start,
            },
            writer: Some(ctx.unit),
            ..SetData::plain(Vec::new(), false)
        };
        let s = self.new_obj(ObjData::Set(Box::new(sd)));
        self.note_made(s, ctx, seg.start);
        self.node_memo.insert(key, s);
        Ok((s, true))
    }

    // ------------------------------------------------------------------ wires, constraints

    fn kw_wire(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mut spec = self.sig_params(kw, ctx);
        spec.extend(style_specs("wire_style"));
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let nets_a = mapped["net"].clone();
        let pts_a = mapped["points"].clone();
        let Some(s) = ctx.scene else {
            if self.is_lazy(nets_a.obj) || self.is_lazy(pts_a.obj) {
                self.note_rule(ctx);
                return Ok((NOTHING, true));
            }
            return self.scene_only(ctx, seg.start);
        };
        // in the draft 0.3, given one wire in place of a net, WIRE makes no new wire: it rewrites
        // that one (its 13.1). In 0.2 a wire there is of the wrong kind, as a set that holds one is
        // in both
        let rewrite = matches!(self.o(nets_a.obj), ObjData::Wire { .. }) && self.statement_follows_0_3(ctx);
        let (mut nets, mut bus) = if rewrite || self.is_empty_node(nets_a.obj) {
            (Vec::new(), false)
        } else if self.is_set(nets_a.obj) {
            let nets = self.flatten_inputs(s, nets_a.obj, 3, Some(ctx));
            for member in &nets {
                ctx.depend_on(&member.dependencies);
            }
            let nets: Vec<_> = nets.into_iter().map(|member| member.object).collect();
            if nets.is_empty() {
                return self.fail(
                    ctx,
                    codes::WIRE_NET_EMPTY_SET,
                    "the net of a wire is an empty set, so the wire fails (5.2.1, 13.1)",
                    nets_a.off,
                );
            }
            (nets, true)
        } else {
            (vec![nets_a.obj], false)
        };
        if self.follows_0_4(self.source_unit(ctx)) {
            let mut converted = Vec::new();
            for net in nets {
                // This conversion does not depend on symbol assignment (5.2.4).
                let value = if matches!(self.o(net), ObjData::Term { .. }) && !self.role_inputs.contains_key(&net) {
                    self.term_net(s, net, ctx, nets_a.off)?
                } else {
                    net
                };
                bus |= self.is_set(value);
                converted.extend(self.flatten_inputs(s, value, 3, Some(ctx)).into_iter().map(|member| {
                    ctx.depend_on(&member.dependencies);
                    member.object
                }));
            }
            nets = converted;
        }
        for n in &nets {
            if let Some(d) = self.failed_obj(*n) {
                let id = self.err_cause(
                    ctx,
                    codes::WIRE_NET_FAILED,
                    "a net of the wire cannot be found: the whole wire fails (13.1)",
                    nets_a.off,
                    d,
                );
                return Err(Stop::Fail(Some(id)));
            }
            let deferred_net = self.follows_0_4(self.source_unit(ctx))
                && (matches!(self.o(*n), ObjData::NamedSubobject { .. } | ObjData::ScenePin { .. })
                    || self.role_inputs.contains_key(n));
            if !deferred_net && !matches!(self.o(*n), ObjData::Net { .. }) {
                let l = self.label(*n);
                return self.fail(
                    ctx,
                    codes::WIRE_NET_WRONG_KIND,
                    format!("the net position of a wire received {l}, the wrong kind (13.1)"),
                    nets_a.off,
                );
            }
        }
        let points = self.flatten_inputs(s, pts_a.obj, 3, Some(ctx));
        for member in &points {
            ctx.depend_on(&member.dependencies);
        }
        let pts: Vec<_> = points.into_iter().map(|member| member.object).collect();
        if pts.len() < 2 {
            return self.fail(ctx, codes::WIRE_TOO_FEW_POINTS, "a wire has at least two points (13.1)", pts_a.off);
        }
        for q in &pts {
            if let Some(d) = self.failed_obj(*q) {
                let id = self.err_cause(ctx, codes::WIRE_POINT_FAILED, "a point of the wire failed", pts_a.off, d);
                return Err(Stop::Fail(Some(id)));
            }
            let deferred_pin = self.follows_0_4(self.source_unit(ctx))
                && matches!(self.o(*q), ObjData::NamedSubobject { .. } | ObjData::ScenePin { .. });
            if !deferred_pin
                && !matches!(self.o(*q), ObjData::Term { .. } | ObjData::Point { .. } | ObjData::Pos { .. })
            {
                let l = self.label(*q);
                return self.fail(
                    ctx,
                    codes::WIRE_POINT_WRONG_KIND,
                    format!("the points of a wire are terminals or points, not {l} (13.1)"),
                    pts_a.off,
                );
            }
        }
        let (style, rest) = split_style(&p);
        let mode = get(&rest, "mode").and_then(Lit::str).unwrap_or("auto").to_string();
        if rewrite {
            // read as a new WIRE: its list and its mode replace the wire's, whose net stays; the
            // handle passed on is the wire's (5.3)
            self.scene_rewrite_wire(s, nets_a.obj, pts, mode, style, ctx, seg.start);
            return Ok((nets_a.obj, false));
        }
        let w = self.new_obj(ObjData::Wire {
            nets,
            bus,
            points: pts,
            mode,
            style,
            generated: ctx.frame_generated(),
            failed: false,
            site: Some(Rc::new(ctx.clone())),
            off: seg.start,
        });
        self.scene_add_wire(s, w, ctx.pos.clone(), ctx);
        self.note_birth(s, w, ctx, seg.call.as_ref().unwrap().start, None);
        Ok((w, false))
    }

    fn kw_table(&mut self, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let p = self.params("TABLE", seg, ctx, &self.sig_params("TABLE", ctx), None, false, None)?;
        let Some(s) = ctx.scene else { return Err(self.final_set_needs_scene()) };
        let mapped = self.map_args("TABLE", args, seg, ctx, sig_positions("TABLE"))?;
        let rows = mapped["rows"].obj;
        if let Some(cause) = self.failed_obj(rows) {
            let id = self.err_cause(
                ctx,
                codes::TABLE_ROWS_FAILED,
                "the table of @TABLE failed, so the whole TABLE fails (7.3, 14.11)",
                seg.start,
                cause,
            );
            return Err(Stop::Fail(Some(id)));
        }
        let selected = self.constraint_input(rows, seg.start, 0, ctx)? || self.is_lazy(rows);
        let chain = self.apply_chain(ctx);
        let site = Rc::new(ctx.clone());
        let c = ConstraintData {
            kind: ConstraintKind::Table,
            args: vec![rows],
            table: None,
            params: ConstraintParams {
                axis: Some(if get(&p, "transpose").and_then(Lit::boolean).unwrap_or(false) { 'x' } else { 'y' }),
                sense: None,
                min: None,
                max: None,
                relation: None,
                relations: Vec::new(),
            },
            priority_param: priority_of(&p, ctx),
            chain: chain.clone(),
            status: None,
            eff: None,
            failed: false,
            site: Some(site.clone()),
            off: seg.start,
            name_owner: None,
        };
        let id = self.new_obj(ObjData::Constraint(Box::new(c)));
        self.scene_add_constraint(s, id, ctx.pos.clone(), ctx);
        self.scenes[s].groups.push(GroupFact {
            pos: ctx.pos.clone(),
            set: rows,
            chain,
            site,
            off: seg.start,
            table: Some(id),
            spacing_x: None,
            spacing_y: None,
        });
        if selected {
            self.note_rule(ctx);
        }
        Ok((id, false))
    }

    fn kw_constraint(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let call_start = seg.call.as_ref().unwrap().start;
        let follows_0_4 = self.follows_0_4(self.source_unit(ctx));
        if kw == "ORDER" || kw == "DISTANCE" {
            self.old_direction(kw, seg, ctx)?;
        }
        if follows_0_4 && matches!(kw, "ORDER" | "DISTRIBUTE" | "DISTANCE") {
            self.axis_without_direction(kw, seg, ctx)?;
        }
        let p = self.params(kw, seg, ctx, &self.sig_params(kw, ctx), None, false, None)?;
        let mut cp =
            ConstraintParams { axis: None, sense: None, min: None, max: None, relation: None, relations: Vec::new() };
        match kw {
            // in a file of 0.4 DISTRIBUTE takes a direction as ORDER does (14.4)
            "ORDER" | "DISTANCE" | "DISTRIBUTE" if get(&p, "direction").is_some() => {
                let (axis, sense) =
                    direction(get(&p, "direction").and_then(Lit::str).unwrap_or("x")).unwrap_or(('x', "either"));
                cp.axis = Some(axis);
                cp.sense = Some(sense);
            }
            "ALIGN" | "DISTRIBUTE" => {
                cp.axis = get(&p, "axis").and_then(Lit::str).and_then(|a| a.chars().next());
            }
            // one relation, or in a file of 0.4 a list of them of which one is to hold (14.9)
            "ORIENT-RELATION" => {
                let codes: Vec<Orient> = match get(&p, "relation") {
                    Some(Lit::Arr(items)) => items.iter().filter_map(|v| v.str().and_then(Orient::parse)).collect(),
                    Some(v) => v.str().and_then(Orient::parse).into_iter().collect(),
                    None => Vec::new(),
                };
                cp.relation = codes.first().copied();
                if codes.len() > 1 {
                    cp.relations = codes;
                }
            }
            _ => {}
        }
        if kw == "DISTANCE" {
            cp.min = get(&p, "min").and_then(Lit::num);
            cp.max = get(&p, "max").and_then(Lit::num);
            if let Some(e) = get(&p, "exact").and_then(Lit::num) {
                // the exact distance, short for min = max (14.5, PR15)
                if cp.min.is_some() || cp.max.is_some() {
                    return self.fail(
                        ctx,
                        codes::PARAM_DISTANCE_EXACT_WITH_BOUNDS,
                        "exact of DISTANCE cannot be written with min or max (14.5)",
                        call_start,
                    );
                }
                cp.min = Some(e);
                cp.max = Some(e);
            }
            if cp.min.is_none() && cp.max.is_none() {
                return self.fail(
                    ctx,
                    codes::PARAM_DISTANCE_NO_BOUND,
                    "DISTANCE takes at least one of min, max and exact (14.5)",
                    call_start,
                );
            }
            if follows_0_4 {
                for name in ["min", "max", "exact"] {
                    if let Some(value) = get(&p, name).and_then(Lit::num).filter(|v| *v < 0.0) {
                        return self.fail(
                            ctx,
                            codes::DISTANCE_NEGATIVE_BOUND_04,
                            format!("{name}={value} of DISTANCE is negative: its bounds must be nonnegative; for the other way, write the other direction (14.5)"),
                            call_start,
                        );
                    }
                }
                cp.min.get_or_insert(0.0);
            }
            if let (Some(a), Some(b)) = (cp.min, cp.max) {
                if a > b {
                    return self.fail(
                        ctx,
                        codes::PARAM_DISTANCE_MIN_ABOVE_MAX,
                        "the min of DISTANCE cannot be greater than its max (14.5)",
                        call_start,
                    );
                }
            }
            if cp.sense == Some("either") && (cp.min.unwrap_or(0.0) < 0.0 || cp.max.unwrap_or(0.0) < 0.0) {
                return self.fail(
                    ctx,
                    codes::PARAM_DISTANCE_NEGATIVE_WITHOUT_SENSE,
                    "without a sense (\"x\", \"y\") the bounds of DISTANCE cannot be negative (14.5)",
                    call_start,
                );
            }
        }
        if follows_0_4 {
            if ctx.scene.is_none() {
                return Err(self.final_set_needs_scene());
            }
            let mut selected = false;
            for (k, arg) in args.iter().enumerate() {
                selected |= self.constraint_input(arg.obj, seg.start, k, ctx)? || self.is_lazy(arg.obj);
            }
            let template = ConstraintData {
                kind: ConstraintKind::parse(kw).unwrap(),
                args: Vec::new(),
                params: cp,
                table: None,
                priority_param: priority_of(&p, ctx),
                chain: self.apply_chain(ctx),
                status: None,
                eff: None,
                failed: false,
                site: Some(Rc::new(ctx.clone())),
                off: seg.start,
                name_owner: None,
            };
            return self.constraint_call(template, args, seg, ctx, selected);
        }
        let positions = sig_positions(kw);
        let mapped = self.map_args(kw, args, seg, ctx, positions)?;
        let arglist: Vec<ObjId> = positions.iter().map(|q| mapped[&q.name].obj).collect();
        let lazy = arglist.iter().any(|a| self.is_lazy(*a));
        let Some(s) = ctx.scene else {
            if lazy {
                self.note_rule(ctx);
                return Ok((NOTHING, true));
            }
            return self.scene_only(ctx, seg.start);
        };
        for a in &arglist {
            if let Some(d) = self.failed_obj(*a) {
                let id = self.err_cause(
                    ctx,
                    codes::CONSTRAINT_MEMBER_FAILED,
                    format!("a member of {kw} failed, so the whole constraint fails (7.3)"),
                    seg.start,
                    d,
                );
                return Err(Stop::Fail(Some(id)));
            }
        }
        let c = ConstraintData {
            kind: ConstraintKind::parse(kw).unwrap(),
            args: arglist,
            params: cp,
            table: None,
            priority_param: priority_of(&p, ctx),
            chain: self.apply_chain(ctx),
            status: None,
            eff: None,
            failed: false,
            site: Some(Rc::new(ctx.clone())),
            off: seg.start,
            name_owner: None,
        };
        let id = self.new_obj(ObjData::Constraint(Box::new(c)));
        self.scene_add_constraint(s, id, ctx.pos.clone(), ctx);
        if lazy {
            self.note_rule(ctx);
        }
        Ok((id, false))
    }

    /// ORDER and DISTANCE take one `direction` now: axis= and "increasing", "decreasing",
    /// "either" fail with a hint (14.3, 14.5).
    /// In a file of 0.4 ORDER, DISTRIBUTE and DISTANCE take a direction, which writes the sense: the
    /// axis alone, `"x"` or `"y"`, or DISTRIBUTE's `axis=` of 0.3, is an error (14.1, 14.3).
    fn axis_without_direction(&mut self, kw: &str, seg: &Segment, ctx: &Ctx) -> R<()> {
        let Some(p) = seg.call.as_ref().and_then(|c| c.params.as_ref()) else { return Ok(()) };
        let mut bad = None;
        if let Some(Value::Str(s)) = p.positional.first() {
            if s == "x" || s == "y" {
                bad = Some((format!("{s:?}"), p.start));
            }
        }
        for n in &p.named {
            let std = if n.quoted { n.name.clone() } else { self.word(&n.name, ctx.unit) };
            match (std.as_str(), &n.value) {
                ("direction", Value::Str(s)) if s == "x" || s == "y" => bad = Some((format!("{s:?}"), n.off)),
                ("axis", _) if kw == "DISTRIBUTE" => bad = Some(("axis=".to_string(), n.off)),
                _ => {}
            }
        }
        let Some((written, at)) = bad else { return Ok(()) };
        self.fail(
            ctx,
            codes::PARAM_AXIS_WITHOUT_DIRECTION,
            format!("@{kw} takes a direction, one of \"left-to-right\", \"right-to-left\", \"bottom-to-top\" and \"top-to-bottom\", not {written}: a file of 0.4 writes the sense (14.1, 14.3)"),
            at,
        )
    }

    fn old_direction(&mut self, kw: &str, seg: &Segment, ctx: &Ctx) -> R<()> {
        let Some(p) = seg.call.as_ref().and_then(|c| c.params.as_ref()) else { return Ok(()) };
        let mut bad = None;
        let mut written: Vec<&Value> = p.positional.iter().take(1).collect();
        for n in &p.named {
            let std = if n.quoted { n.name.clone() } else { self.word(&n.name, ctx.unit) };
            if std == "axis" {
                bad = Some(n.off);
            }
            if std == "direction" {
                written.push(&n.value);
            }
        }
        if written
            .iter()
            .any(|v| matches!(v, Value::Str(s) if ["either", "increasing", "decreasing"].contains(&s.as_str())))
        {
            bad = Some(p.start);
        }
        if let Some(at) = bad {
            let sec = if kw == "ORDER" { "14.3" } else { "14.5" };
            // a file of 0.4 writes the sense: "x" and "y" are no longer read (14.1)
            let no_sense =
                if self.follows_0_4(self.source_unit(ctx)) { "" } else { "; with no sense write \"x\" or \"y\"" };
            return self.fail(
                ctx,
                codes::PARAM_OLD_DIRECTION, format!("axis and direction of @{kw} are one parameter now, direction, written first by position, such as @ORDER(\"left-to-right\") or @DISTANCE(\"bottom-to-top\", min=2){no_sense} ({sec})"),
                at,
            );
        }
        Ok(())
    }

    /// The usages whose bodies a statement is in, outermost first.
    pub(crate) fn apply_chain(&self, ctx: &Ctx) -> Vec<ObjId> {
        let mut out = Vec::new();
        let mut a = ctx.frame_usage();
        while let Some(u) = a {
            out.push(u);
            a = self.usage(u).parent;
        }
        out.reverse();
        out
    }
}

/// One AS or ATTACH, applied once or once per member.
#[derive(Debug)]
pub struct Application {
    pub dfn: ObjId,
    pub filled: Vec<(String, ArgV)>,
    pub params: Vec<(String, Option<Lit>)>,
    /// The parameters the statement writes, without the definition's defaults.
    pub given: Vec<(String, Lit)>,
    pub priority: Option<Priority>,
    pub seg_start: usize,
    pub seg: Segment,
}

impl Interp<'_> {
    /// The parameters of keyword `kw` as the file of `ctx` has them, by its language version (16.1).
    fn sig_params(&self, kw: &str, ctx: &Ctx) -> Vec<ParamSpec> {
        let version = self.version_of(Some(self.source_unit(ctx)));
        words().keyword(kw).map(|s| s.params_in(&version)).unwrap_or_default()
    }
}

pub(super) fn sig_positions(kw: &str) -> &'static [PositionSpec] {
    words().keyword(kw).map(|s| s.position_list()).unwrap_or(&[])
}

fn priority_of(p: &[(String, Lit)], ctx: &Ctx) -> Option<Priority> {
    get(p, "priority").and_then(Lit::str).and_then(Priority::parse).or(ctx.header.priority)
}

pub(crate) fn ptr_seg(seg: &Segment) -> usize {
    seg as *const Segment as usize
}

/// A terminal is drawn by one pin: the later pin takes it (9.2.4, 9.2.7).
pub(crate) fn add_pin(sym: &mut SymbolDef, pin: Pin) {
    add_pin_with_case(sym, pin, true);
}

pub(crate) fn add_pin_with_case(sym: &mut SymbolDef, pin: Pin, case_sensitive: bool) {
    if !sym.independent_pins {
        for old in sym.pins.iter_mut() {
            old.terms.retain(|t| !pin.terms.iter().any(|n| t.same(n, case_sensitive)));
        }
        sym.pins.retain(|q| !q.terms.is_empty());
    }
    sym.pins.push(pin);
}

pub(crate) fn merge_symbol(sym: &mut SymbolDef, other: &SymbolDef) {
    for pin in &other.pins {
        add_pin(sym, pin.clone());
    }
    let offset = sym.prims.len() + sym.usages.len();
    sym.prim_order.extend(other.prim_order.iter().map(|n| offset + n));
    sym.usage_order.extend(other.usage_order.iter().map(|n| offset + n));
    sym.prims.extend(other.prims.iter().cloned());
    sym.labels.extend(other.labels.iter().cloned());
    sym.usages.extend(other.usages.iter().cloned());
    if sym.bbox.is_none() && other.bbox.is_some() {
        sym.bbox = other.bbox;
    }
}
