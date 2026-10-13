//! The keywords (spec-draft chapters 9–15): what each statement creates or adds.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::geom::Orient;
use crate::syntax::{Member, ObjExpr, Segment, SetExpr, Value};
use crate::words::{words, ParamSpec, ParamType, PositionSpec, Takes};

use super::ctx::{Ctx, Frame, FrameKind, FrameRef, Nested};
use super::eval::{get, ArgV, Stop, R};
use super::obj::*;
use super::scene::{TargetOp, Task};
use super::Interp;

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

fn style_specs(group: &str) -> Vec<ParamSpec> {
    words()
        .group(group)
        .iter()
        .filter_map(|n| words().style_prop(n))
        .map(|s| ParamSpec { name: s.name.clone(), ty: s.ty.clone(), required: false })
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
    pub(crate) fn keyword(
        &mut self,
        kw: &str,
        args: Vec<ArgV>,
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
    ) -> R<(ObjId, bool)> {
        match kw {
            "DEFINE-SYMBOL" => self.kw_define_symbol(kw, &args, seg, ctx, member),
            "DEFINE-STYLE" => self.kw_define_style(kw, &args, seg, ctx, member),
            "DEFINE" => self.kw_define(kw, &args, seg, ctx, member),
            "AS" => self.kw_usage(kw, args, seg, ctx, false),
            "ATTACH" => self.kw_usage(kw, args, seg, ctx, true),
            "PIN" => self.kw_pin(kw, &args, seg, ctx),
            "LINE" | "RECTANGLE" | "CIRCLE" | "ELLIPSE" | "ARC" | "BEZIER" => self.kw_prim(kw, &args, seg, ctx),
            "TEXT" => self.kw_text(kw, &args, seg, ctx),
            "PLACE" => {
                let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
                if get(&p, "x").is_none() && get(&p, "y").is_none() {
                    let at = seg.call.as_ref().unwrap().start;
                    return self.fail(ctx, "PLACE takes x, y or both (11.1)", at);
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
                let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
                let code = Orient::parse(get(&p, "code").and_then(Lit::str).unwrap_or("R0")).unwrap_or_default();
                let op = TargetOp::Orient { code, priority: priority_of(&p, ctx), off: seg.start };
                self.target_each(kw, &args, seg, ctx, 7, op, "target")
            }
            "LAYER" => {
                let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
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
                let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
                let d = get(&p, "direction").and_then(Lit::str).unwrap_or("unspecified").to_string();
                self.target_each(kw, &args, seg, ctx, 3, TargetOp::Current { direction: d, off: seg.start }, "terminal")
            }
            "PRIORITY" => {
                let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
                let pr = Priority::parse(get(&p, "priority").and_then(Lit::str).unwrap_or("normal"))
                    .unwrap_or(Priority::Normal);
                let off = seg.call.as_ref().unwrap().start;
                self.target_each(kw, &args, seg, ctx, 7, TargetOp::Priority { priority: pr, off }, "target")
            }
            "GROUP" => {
                let mapped = self.map_args(kw, &args, seg, ctx, sig_positions(kw))?;
                let obj = mapped["members"].obj;
                if !self.is_set(obj) {
                    return self.fail(ctx, "GROUP takes a set (11.5)", seg.start);
                }
                let Some(s) = ctx.scene else {
                    if self.is_lazy(obj) {
                        self.rules.insert(ctx.member_key.clone());
                        return Ok((obj, true));
                    }
                    return self.scene_only(ctx, seg.start);
                };
                self.scenes[s].groups.push((ctx.pos.clone(), obj));
                Ok((obj, self.is_lazy(obj)))
            }
            "ADD-TO" => self.kw_mod(kw, &args, seg, ctx, true),
            "REMOVE-FROM" => self.kw_mod(kw, &args, seg, ctx, false),
            "UNION" => {
                let mapped = self.map_args(kw, &args, seg, ctx, sig_positions(kw))?;
                let src = mapped["sets"].obj;
                let lazy = self.is_lazy(src);
                if lazy {
                    let key = ("union", ptr_seg(seg), self.frame_key(ctx), 0usize);
                    if let Some(&u) = self.node_memo.get(&key) {
                        return Ok((u, true));
                    }
                    let u = self.new_obj(ObjData::Set(Box::new(SetData {
                        kind: SetKind::Union { source: src },
                        ..SetData::plain(Vec::new(), false)
                    })));
                    self.node_memo.insert(key, u);
                    return Ok((u, true));
                }
                let u = self.new_obj(ObjData::Set(Box::new(SetData {
                    kind: SetKind::Union { source: src },
                    ..SetData::plain(Vec::new(), false)
                })));
                Ok((u, false))
            }
            k if k.starts_with("SELECT-") => self.kw_select(kw, &args, seg, ctx),
            "WIRE" => self.kw_wire(kw, &args, seg, ctx),
            "ROUTE" => {
                let mut spec = sig_params(kw);
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
                    return self.fail(ctx, "BUNDLE received an empty set and fails (5.2.1, 11.8)", seg.start);
                }
                for m in &members {
                    if let Some(d) = self.failed_obj(*m) {
                        let id = self.err_cause(
                            ctx,
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
                            format!("the members of a bundle are host instances, not {l} (11.8)"),
                            seg.start,
                        );
                    }
                }
                let b = self.new_obj(ObjData::Bundle { members, empty: false, site: Some(Rc::new(ctx.clone())) });
                self.scene_add_bundle(s, b, ctx.pos.clone());
                Ok((b, false))
            }
            "ALIGN" | "ORDER" | "DISTRIBUTE" | "DISTANCE" | "ORIENT-RELATION" => {
                self.kw_constraint(kw, &args, seg, ctx)
            }
            _ => {
                let at = seg.call.as_ref().unwrap().start;
                self.fail(ctx, format!("@{kw} cannot be used here"), at)
            }
        }
    }

    pub(crate) fn failed_obj(&self, id: ObjId) -> Option<Option<crate::diag::DiagId>> {
        match self.o(id) {
            ObjData::Failed(d) => Some(*d),
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
            self.warn(ctx, "a definition at the top of a record without a name is never used (9.1)", at);
        }
    }

    fn body_arg(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<Option<Rc<SetExpr>>> {
        if seg.args.is_empty() {
            if !args.is_empty() {
                return self.fail(ctx, format!("@{kw} takes one set as its body (9.1)"), seg.start);
            }
            return Ok(None);
        }
        match (&seg.args[0].obj, seg.args.len(), args.len()) {
            (ObjExpr::Set(s), 1, 1) => Ok(Some(s.clone())),
            _ => self.fail(ctx, format!("@{kw} takes one set as its body (9.1)"), seg.start),
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
        let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
        let body = self.body_arg(kw, args, seg, ctx)?;
        let sym = self.new_obj(ObjData::Symbol(Box::default()));
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
                return self.fail(ctx, "box= is written [x1, y1, x2, y2] (9.2.3)", at);
            }
        }
        if let Some(body) = body {
            let defining = self.defining_for(ctx, seg, member);
            self.eval_symbol_body(sym, &body, ctx, defining)?;
            let usages = self.symbol(sym).usages.clone();
            for a in usages {
                let u = self.usage(a).clone();
                if u.own_of == Some(sym) && u.sym_symbol.is_none() {
                    let site = u.site.clone().unwrap();
                    let l = self.label(u.definition);
                    self.warn(&site, format!("the usage of {l} in a symbol body is given no symbol with SET-SYMBOL and is not drawn (9.2.7)"), u.off);
                }
            }
        }
        self.unnamed_definition_warning(ctx, seg, member);
        Ok((sym, false))
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
                    let e = err.clone();
                    self.err(ctx, e, m.error_pos);
                    continue;
                }
                for sg in &m.segments {
                    if sg.call.is_some() || m.bind().is_some() || m.segments.len() != 1 {
                        self.err(ctx, "the set of DEFINE-STYLE only references other styles (9.3)", m.start);
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
                                self.err(ctx, "the set of DEFINE-STYLE only references other styles (9.3)", a.start);
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
        let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
        let body = self.body_arg(kw, args, seg, ctx)?;
        let at = seg.call.as_ref().unwrap().start;
        let mut roles = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        if let Some(Lit::Arr(rs)) = get(&p, "roles") {
            for r in rs.clone() {
                let (name, kinds) = match &r {
                    Lit::Str(s) => (s.as_str().to_string(), None),
                    Lit::Arr(v) if v.len() == 2 && matches!(v[0], Lit::Str(_)) => {
                        let kinds: Vec<String> = match &v[1] {
                            Lit::Arr(ks) => ks.iter().filter_map(|k| k.str().map(String::from)).collect(),
                            Lit::Str(k) => vec![k.as_str().to_string()],
                            _ => return self.fail(ctx, "a role is a name or [name, kind] (9.4)", at),
                        };
                        for k in &kinds {
                            if !["instance", "terminal", "net", "point", "wire", "usage"].contains(&k.as_str()) {
                                return self.fail(ctx, format!("unknown kind name {k} (9.4)"), at);
                            }
                        }
                        (v[0].str().unwrap().to_string(), Some(kinds))
                    }
                    _ => return self.fail(ctx, "a role is a name or [name, kind] (9.4)", at),
                };
                let is_set = name.starts_with('{') && name.ends_with('}') && name.len() >= 2;
                let name = if is_set { name[1..name.len() - 1].to_string() } else { name };
                if seen.contains(&name) || name == "position" || name == "net" {
                    return self.fail(
                        ctx,
                        format!("the role name {name} is used twice or is a standard sub-object name (9.4)"),
                        at,
                    );
                }
                seen.push(name.clone());
                roles.push(Role { name, kinds, is_set });
            }
        }
        let mut params: Vec<(String, Option<Lit>)> = Vec::new();
        if let Some(Lit::Arr(qs)) = get(&p, "parameters") {
            for q in qs.clone() {
                match &q {
                    Lit::Arr(v) if v.len() == 2 && matches!(v[0], Lit::Str(_)) => {
                        params.push((v[0].str().unwrap().to_string(), Some(v[1].clone())))
                    }
                    Lit::Str(s) => params.push((s.as_str().to_string(), None)),
                    _ => return self.fail(ctx, "a parameter is a name or [name, default] (9.4)", at),
                }
                let n = params.last().unwrap().0.clone();
                if seen.contains(&n) || n == "priority" || n == "position" || n == "net" {
                    return self.fail(ctx, format!("the parameter name {n} cannot be used (9.4)"), at);
                }
                seen.push(n);
            }
        }
        let defining = self.defining_for(ctx, seg, member);
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
            self.check_definition(id, ctx);
        }
        self.unnamed_definition_warning(ctx, seg, member);
        Ok((id, false))
    }

    /// Checks that do not depend on the arguments, once per definition (6.9, M5).
    fn check_definition(&mut self, id: ObjId, ctx: &Ctx) {
        let d = self.def(id).clone();
        let roles: Vec<String> = d.roles.iter().map(|r| r.name.clone()).collect();
        let params: Vec<String> = d.params.iter().map(|(n, _)| n.clone()).collect();
        let locals = d.local_names();
        let mut names: Vec<String> = Vec::new();
        for m in d.body.as_ref().unwrap().members.iter() {
            if let Some(err) = &m.error {
                let e = err.clone();
                self.err(ctx, e, m.error_pos);
                self.note_failed(ctx.rec, m.start, m.end, false);
                continue;
            }
            for b in m.binds.iter().filter(|b| b.prefix == '%') {
                if roles.contains(&b.name) || b.name == "position" || b.name == "net" {
                    self.err(
                        ctx,
                        format!(
                            "the local name {} is spelled like a role name or a standard sub-object name (9.4)",
                            b.name
                        ),
                        b.off,
                    );
                }
                if names.contains(&b.name) {
                    self.err(ctx, format!("the local name {} is bound twice (6.6)", b.name), b.off);
                }
                names.push(b.name.clone());
            }
            let mut refs = Vec::new();
            crate::syntax::walk_refs(m, &mut refs);
            for r in refs {
                match r {
                    crate::syntax::RefOrHole::Ref(r) => {
                        if r.prefix == '#' || r.prefix == '&' {
                            self.err(ctx, "the body of an ordinary definition cannot reference host objects; they come in through roles (6.9)", r.start);
                            self.note_failed_at(ctx.rec, r.start, false);
                        }
                        if r.prefix == '$' || r.prefix == '%' {
                            let n = r.name.clone().unwrap_or_default();
                            if let Some((member, unit)) = d.defining.get(&(r.prefix, n.clone())).cloned() {
                                if !(r.prefix == '%' && locals.contains(&n)) {
                                    if let super::eval::Found::NoOuter = self.outer_lookup(r.prefix, &n, &member, unit)
                                    {
                                        let msg = super::eval::no_outer_message(r);
                                        let diag = self.err(ctx, msg, r.start);
                                        self.body_errors.insert(super::eval::ptr(r), diag);
                                        self.note_failed_at(ctx.rec, r.start, false);
                                    }
                                }
                            }
                        }
                        if r.prefix == '?' {
                            if let Some(n) = &r.name {
                                if !roles.contains(n) {
                                    if params.contains(n) {
                                        self.err(
                                            ctx,
                                            format!(
                                                "?{n} is a parameter and is written only where a literal goes (6.10)"
                                            ),
                                            r.start,
                                        );
                                    } else {
                                        self.err(
                                            ctx,
                                            format!("?{n} is not in the role list or the parameter list (6.10)"),
                                            r.start,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    crate::syntax::RefOrHole::Hole(n) => {
                        if !params.iter().any(|p| p == n) && !roles.iter().any(|p| p == n) {
                            self.err(ctx, format!("?{n} is not in the parameter list (6.10)"), 0);
                        }
                    }
                }
            }
            for sg in &m.segments {
                if let Some(call) = &sg.call {
                    let k = self.kw(&call.kw, ctx.unit);
                    if !words().is_keyword(&k) {
                        self.err(ctx, format!("unknown keyword @{} (16.4)", call.kw), call.start);
                    } else if k == "PIN" {
                        self.err(ctx, "PIN is written only in a DEFINE-SYMBOL body (9.2.4)", call.start);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ usages: AS, ATTACH (9.4, 10.5, 12)

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
                format!("the definition of @{kw} is written {wher} and cannot come down a pipe (5.6)"),
                seg.start,
            );
        }
        let dfn_arg = if first { written[0].clone() } else { written[written.len() - 1].clone() };
        if dfn_arg.role.is_some() {
            return self.fail(
                ctx,
                format!("the definition of @{kw} is written {wher} and carries no role (4.2 rule 7)"),
                dfn_arg.off,
            );
        }
        let dfn = self.anon_def(dfn_arg.obj);
        if !matches!(self.o(dfn), ObjData::Definition(_)) {
            let l = self.label(dfn);
            return self.fail(
                ctx,
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
        let own: Vec<String> = d.params.iter().map(|(n, _)| n.clone()).collect();
        let prio: Vec<String> = match self.file_unit(ctx.unit) {
            Some(f) => self.units[f].aliases.iter().filter(|(_, o)| *o == "priority").map(|(a, _)| a.clone()).collect(),
            None => Vec::new(),
        };
        let dlabel = self.label(dfn);
        let resolve = |s: &mut Self, name: &str, quoted: bool, off: usize| -> R<String> {
            if quoted {
                return Ok(name.to_string());
            }
            if let Some((_, o)) = scoped.iter().find(|(a, _)| a == name) {
                return Ok(o.clone());
            }
            if prio.iter().any(|p| p == name) {
                if own.iter().any(|p| p == name) {
                    return s.fail(ctx, format!("{name} is both a parameter of the definition {dlabel} and an alias of priority; its meaning is not unique (16.5)"), off);
                }
                return Ok("priority".to_string());
            }
            Ok(name.to_string())
        };
        // parameters: the definition's parameter list plus `priority` (9.4, 14.8)
        let mut pspec: Vec<ParamSpec> =
            d.params.iter().map(|(n, _)| ParamSpec { name: n.clone(), ty: ParamType::Any, required: false }).collect();
        pspec.push(ParamSpec {
            name: "priority".into(),
            ty: ParamType::Enum(
                ["disabled", "low", "normal", "high"]
                    .iter()
                    .map(|s| crate::words::Literal::Str(s.to_string()))
                    .collect(),
            ),
            required: false,
        });
        let order: Vec<String> = d.params.iter().map(|(n, _)| n.clone()).collect();
        let p = self.params(kw, seg, ctx, &pspec, Some(&order), false, Some(&resolve))?;
        let priority = get(&p, "priority").and_then(Lit::str).and_then(Priority::parse).or(ctx.header.priority);
        // a string passed in remembers the file it is written in (16.5)
        let params: Vec<(String, Option<Lit>)> = d
            .params
            .iter()
            .map(|(n, def)| {
                (
                    n.clone(),
                    match get(&p, n) {
                        Some(v) => Some(written_in(v, ctx.unit)),
                        None => def.clone(),
                    },
                )
            })
            .collect();
        // roles (5.2.1, 9.4)
        let mut filled: Vec<(String, ArgV)> = Vec::new();
        let labeled = objs.iter().filter(|a| a.role.is_some()).count();
        if labeled > 0 && labeled != objs.len() {
            return self.fail(ctx, "label the roles of all objects or of none (4.2 rule 6)", seg.start);
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
                    return self.fail(
                        ctx,
                        format!("the ordinary definition {dlabel} has no role {r}{more} (9.4){then}"),
                        a.off,
                    );
                }
                if filled.iter().any(|(n, _)| *n == role) {
                    return self.fail(
                        ctx,
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
                "at most one single role of an application receives a set of several members (5.2.1)",
                seg.start,
            );
        }
        let app = Rc::new(Application { dfn, filled, params, priority, seg_start: seg.start, seg: seg.clone() });
        if let Some(rname) = lazy_roles.first().cloned() {
            let key = ("gen", ptr_seg(seg), self.frame_key(ctx), 0usize);
            let gen = match self.node_memo.get(&key) {
                Some(&g) => g,
                None => {
                    let g = self.new_obj(ObjData::Set(Box::new(SetData {
                        kind: SetKind::Gen { by_scene: HashMap::new() },
                        ..SetData::plain(Vec::new(), false)
                    })));
                    self.node_memo.insert(key, g);
                    g
                }
            };
            if ctx.scene.is_none() {
                self.rules.insert(ctx.member_key.clone());
                return Ok((gen, true));
            }
            let lazy = app.filled.iter().find(|(n, _)| *n == rname).unwrap().1.obj;
            let c = Rc::new(ctx.clone());
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, phase: usize| {
                let members = it.flatten(s, lazy, phase, Some(&c));
                let mut out = Vec::new();
                for (k, mobj) in members.into_iter().enumerate() {
                    let mut f2 = app.filled.clone();
                    for e in f2.iter_mut() {
                        if e.0 == rname {
                            e.1 = ArgV {
                                obj: mobj,
                                role: None,
                                role_quoted: false,
                                sel: true,
                                off: e.1.off,
                                explicit: false,
                            };
                        }
                    }
                    let mut sub = (*c).clone();
                    sub.pos = pos_plus(&c.pos, "g", k as i64);
                    sub.origin = Rc::from(if c.origin.is_empty() {
                        "generated by a rule".to_string()
                    } else {
                        format!("{}; generated by a rule", c.origin)
                    });
                    sub.selected = true;
                    if let Ok(a) = it.usage_once(&app, &f2, &sub, true) {
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
            let members = self.flatten(s, src.obj, 3, Some(ctx));
            let mut out = Vec::new();
            for (k, mobj) in members.into_iter().enumerate() {
                let mut f2 = app.filled.clone();
                for e in f2.iter_mut() {
                    if e.0 == rname {
                        e.1 = ArgV {
                            obj: mobj,
                            role: None,
                            role_quoted: false,
                            sel: src.sel,
                            off: src.off,
                            explicit: false,
                        };
                    }
                }
                let sub = ctx.with_pos(pos_plus(&ctx.pos, "e", k as i64));
                out.push(self.usage_once(&app, &f2, &sub, false)?);
            }
            let set = self
                .new_obj(ObjData::Set(Box::new(SetData::plain(out.into_iter().map(|a| (a, true)).collect(), false))));
            return Ok((set, false));
        }
        let filled = app.filled.clone();
        Ok((self.usage_once(&app, &filled, ctx, false)?, false))
    }

    pub(crate) fn usage_once(
        &mut self,
        app: &Application,
        filled: &[(String, ArgV)],
        ctx: &Ctx,
        generated: bool,
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
                if !self.is_set(obj) {
                    obj = self.new_obj(ObjData::Set(Box::new(SetData::plain(vec![(obj, true)], true))));
                }
                let members = if self.is_lazy(obj) { Vec::new() } else { self.flatten(s, obj, 3, Some(ctx)) };
                for m in members {
                    self.check_kind(dfn, &r.name, &r.kinds, m, ctx, app.seg_start)?;
                }
            } else {
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
        self.scene_add_usage(s, a, ctx.pos.clone());
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
                    format!("cyclic application: {}; this application fails (9.4)", path.join(" -> ")),
                    off,
                );
            }
            u = ud.parent;
        }
        Ok(())
    }

    fn check_kind(
        &mut self,
        dfn: ObjId,
        role: &str,
        kinds: &Option<Vec<String>>,
        obj: ObjId,
        ctx: &Ctx,
        off: usize,
    ) -> R<()> {
        let Some(kinds) = kinds else { return Ok(()) };
        let mut k = self.kind_of(obj);
        if matches!(self.o(obj), ObjData::Bundle { .. }) {
            k = "instance";
        }
        if let ObjData::Term { owner, .. } = self.o(obj) {
            if matches!(self.o(*owner), ObjData::Bundle { .. }) {
                k = "terminal";
            }
        }
        if !kinds.iter().any(|x| x == k) {
            let dl = self.label(dfn);
            let ol = self.label(obj);
            return self.fail(
                ctx,
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
        for (k, m) in body.members.iter().enumerate() {
            if m.error.is_some() {
                continue;
            }
            let mut sub = ctx.clone();
            sub.frame = Some(fr.clone());
            sub.origin = origin.clone();
            sub.nested = None;
            self.definition_member(m, k, &sub)?;
        }
        Ok(())
    }

    pub(crate) fn definition_member(&mut self, m: &Rc<Member>, k: usize, ctx: &Ctx) -> R<ObjId> {
        let fr = ctx.frame.clone().unwrap();
        let usage = fr.borrow().usage.unwrap();
        let key = (super::eval::ptr(&**m), usage);
        if let Some(v) = self.tm_memo.get(&key) {
            return Ok(*v);
        }
        if self.tm_in_progress.contains(&key) {
            return self.fail(ctx, "the name refers to itself in a cycle (6.6)", m.text_start());
        }
        self.tm_in_progress.insert(key);
        let dfn = fr.borrow().definition.unwrap();
        let d = self.def(dfn).clone();
        let origin = self.usage(usage).ctx_origin.clone();
        let mut sub = ctx.clone();
        sub.unit = d.def_unit;
        sub.rec = d.rec;
        sub.header = d.header.clone();
        sub.pos = pos_plus(&ctx.pos, "b", k as i64);
        sub.origin = if origin.is_empty() { ctx.origin.clone() } else { origin };
        sub.nested = None;
        sub.defining = Some(d.defining.clone());
        sub.def_unit = Some(d.def_unit);
        let (r, bound) = self.eval_member(m, &sub);
        self.tm_in_progress.remove(&key);
        let val = match r {
            Ok(v) => v,
            Err(Stop::Fail(dg)) => self.new_obj(ObjData::Failed(dg)),
            Err(Stop::LocalRef) => self.new_obj(ObjData::Failed(None)),
            Err(Stop::NoCircuit) => return Err(Stop::NoCircuit),
        };
        self.tm_memo.insert(key, val);
        let uname = self.objs[usage.0 as usize].name.clone();
        for b in m.binds.iter().filter(|b| b.prefix == '%') {
            let v = bound.iter().find(|(kk, _)| kk.0 == '%' && kk.1 == b.name).map(|(_, v)| *v).unwrap_or(val);
            fr.borrow_mut().set_local(&b.name, v);
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
        let Some(pt) = pt.filter(|p| matches!(self.o(*p), ObjData::Point { .. })) else {
            return self.fail(
                ctx,
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
        self.symbol_mut(symbol).usages.push(a);
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
            defining: Some(defining),
            def_unit: Some(ctx.def_unit.unwrap_or(ctx.unit)),
            scope_in: false,
        };
        for m in body.members.iter() {
            if let Some(err) = &m.error {
                let e = err.clone();
                self.err(ctx, e, m.error_pos);
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
                            merge_symbol(self.symbol_mut(sym), &other);
                        }
                        ObjData::Point { .. } => {}
                        _ => {
                            self.err(
                                &sctx,
                                "a symbol body holds only graphics, pins, points, merged symbols, usages that only draw and texts on `?` (9.2.2)",
                                a.start,
                            );
                            break;
                        }
                    }
                }
                continue;
            }
            let (r, bound) = self.eval_member(m, &sctx);
            for (key, v) in bound {
                nested.borrow_mut().insert(key, Nested::Val(v));
            }
            match r {
                Ok(_) | Err(Stop::Fail(_)) => {}
                Err(Stop::NoCircuit) => {
                    self.err(&sctx, "this statement cannot be written in a symbol body (9.2.2)", m.start);
                }
                Err(Stop::LocalRef) => return Err(Stop::LocalRef),
            }
        }
        Ok(())
    }

    fn kw_pin(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        if !ctx.in_symbol() {
            let at = seg.call.as_ref().unwrap().start;
            return self.fail(
                ctx,
                "pins cannot be written in the scene; PIN is written only in a DEFINE-SYMBOL body (13.4)",
                at,
            );
        }
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
        let pt = mapped["position"].obj;
        let ObjData::Point { x: Some(x), y: Some(y), .. } = self.o(pt).clone() else {
            return self.fail(ctx, "PIN takes one point (9.2.4)", seg.start);
        };
        let term = get(&p, "terminal").cloned().unwrap();
        let raw = match term {
            Lit::Arr(v) => v,
            other => vec![other],
        };
        if raw.is_empty() {
            let at = seg.call.as_ref().unwrap().start;
            return self.fail(ctx, "the terminal= array of a bus pin has at least one element (9.2.4)", at);
        }
        let terms: Vec<TermKey> = raw
            .iter()
            .filter_map(|t| match t {
                Lit::Num(n) => Some(TermKey::Num(*n as u32)),
                Lit::Str(s) => Some(TermKey::Name(s.as_str().to_string())),
                _ => None,
            })
            .collect();
        let label = get(&p, "label").and_then(Lit::str).map(String::from);
        if let Some(l) = &label {
            self.check_markup(l, ctx, seg);
        }
        let dir = get(&p, "direction").and_then(Lit::str).map(|d| match d {
            "right" => 0,
            "up" => 90,
            "left" => 180,
            _ => 270,
        });
        let pin = Pin { terms: terms.clone(), pos: (x, y), label, dir };
        let symbol = ctx.frame.as_ref().unwrap().borrow().symbol.unwrap();
        add_pin(self.symbol_mut(symbol), pin);
        if x != x.round() || y != y.round() {
            self.hint_at(ctx, "pins are best placed on integer points (9.2.4)", seg.start);
        }
        Ok((pt, false))
    }

    fn kw_prim(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let sig = words().keyword(kw).unwrap();
        let mut spec = sig.param_list().to_vec();
        spec.extend(style_specs(sig.style.as_deref().unwrap_or("line_style")));
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let positions = [PositionSpec { name: "points".into(), takes: Takes::Set, kinds: Vec::new(), optional: false }];
        let mapped = self.map_args(kw, args, seg, ctx, &positions)?;
        let obj = mapped["points"].obj;
        if self.is_lazy(obj) {
            return self.fail(ctx, format!("the points of @{kw} take no selection (9.2.5)"), seg.start);
        }
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
        let pts = self.shape_points(kw, obj, ctx, seg)?;
        if centred {
            if matches!(shape, Shape::Rectangle | Shape::Ellipse)
                && !(params.width.is_some() && params.height.is_some())
            {
                let at = seg.call.as_ref().unwrap().start;
                return self.fail(ctx, format!("write both width and height of @{kw} (9.2.5)"), at);
            }
            if self.is_set(obj) && pts.len() > 1 {
                let mut outs = Vec::new();
                for q in pts {
                    outs.push(self.make_prim(shape, vec![q], &params, &style, ctx, seg)?);
                }
                let s = self.new_obj(ObjData::Set(Box::new(SetData::plain(
                    outs.into_iter().map(|o| (o, true)).collect(),
                    false,
                ))));
                return Ok((s, false));
            }
            return Ok((self.make_prim(shape, pts, &params, &style, ctx, seg)?, false));
        }
        let n = pts.len();
        let ok = match shape {
            Shape::Line => n >= 2,
            Shape::Rectangle | Shape::Circle | Shape::Ellipse => n == 2,
            Shape::Arc => n == 3,
            Shape::Bezier => n >= 4 && (n - 1) % 3 == 0,
            Shape::Text => false,
        };
        if !ok {
            return self.fail(ctx, format!("@{kw} has the wrong number of points (9.2.5)"), seg.start);
        }
        Ok((self.make_prim(shape, pts, &params, &style, ctx, seg)?, false))
    }

    fn shape_points(&mut self, kw: &str, obj: ObjId, ctx: &Ctx, seg: &Segment) -> R<Vec<ObjId>> {
        let pts: Vec<ObjId> =
            if self.is_set(obj) { self.set(obj).items.iter().map(|(o, _)| *o).collect() } else { vec![obj] };
        for p in &pts {
            if !matches!(self.o(*p), ObjData::Point { .. } | ObjData::Term { .. } | ObjData::Pos { .. }) {
                return self.fail(ctx, format!("the points of @{kw} are points (9.2.5)"), seg.start);
            }
        }
        Ok(pts)
    }

    fn make_prim(
        &mut self,
        shape: Shape,
        pts: Vec<ObjId>,
        params: &PrimParams,
        style: &StyleMap,
        ctx: &Ctx,
        seg: &Segment,
    ) -> R<ObjId> {
        if ctx.in_symbol() {
            let mut local = Vec::new();
            for q in &pts {
                match self.o(*q) {
                    ObjData::Point { x: Some(x), y: Some(y), local: true } => local.push((*x, *y)),
                    _ => return self.fail(ctx, "graphics in a symbol body use points of the body (9.2.1)", seg.start),
                }
            }
            let symbol = ctx.frame.as_ref().unwrap().borrow().symbol.unwrap();
            self.symbol_mut(symbol).prims.push(SymPrim {
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
            }));
        }
        let Some(s) = ctx.scene else { return self.scene_only(ctx, seg.start) };
        let prim = self.new_obj(ObjData::Prim { shape, points: pts, params: params.clone(), style: style.clone() });
        self.scene_add_prim(s, prim, ctx.pos.clone(), ctx);
        Ok(prim)
    }

    // ------------------------------------------------------------------ TEXT (13.3)

    fn kw_text(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mut spec = sig_params(kw);
        spec.extend(style_specs("text_style"));
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let att = mapped["attach"].clone();
        let (style, rest) = split_style(&p);
        let key = get(&rest, "key").cloned();
        let mut items = TextItems {
            text: get(&rest, "text").and_then(|v| match v {
                Lit::Str(s) => Some(match written_in(&Lit::Str(s.clone()), ctx.unit) {
                    Lit::Str(w) => w,
                    _ => unreachable!(),
                }),
                _ => None,
            }),
            halign: get(&rest, "halign").and_then(Lit::str).map(String::from),
            valign: get(&rest, "valign").and_then(Lit::str).map(String::from),
            rotate: get(&rest, "rotate").and_then(Lit::num),
            dx: get(&rest, "dx").and_then(Lit::num),
            dy: get(&rest, "dy").and_then(Lit::num),
        };
        let call_start = seg.call.as_ref().unwrap().start;
        if let Some(t) = &items.text {
            let t = t.as_str().to_string();
            self.check_markup(&t, ctx, seg);
        }
        // the file the key is written in: its aliases decide what `::` finds first (13.3, 16.5)
        let key_unit = match &key {
            Some(Lit::Str(s)) => s.unit.unwrap_or(ctx.unit),
            _ => ctx.unit,
        };
        let key: Option<String> = key.and_then(|k| k.str().map(String::from));
        if let Some(k) = &key {
            if let Some(clash) = self.key_clash(k, key_unit, None) {
                return self.fail(ctx, clash, call_start);
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
            if let ObjData::Point { x: Some(x), y: Some(y), .. } = self.o(att.obj).clone() {
                if key.is_some() {
                    return self.fail(ctx, "a text attached to a point takes no key= (13.3)", call_start);
                }
                let d = off.unwrap_or((0.0, 0.0));
                self.symbol_mut(symbol).prims.push(SymPrim {
                    shape: Shape::Text,
                    points: vec![(x + d.0, y + d.1)],
                    params: PrimParams::default(),
                    style,
                    text: Some(items),
                });
                return Ok((att.obj, false));
            }
            return self.fail(ctx, "a TEXT in a symbol body is attached to a point or to `?` (9.2.5)", seg.start);
        }
        if self.is_lazy(att.obj) {
            if ctx.scene.is_none() {
                self.rules.insert(ctx.member_key.clone());
                return Ok((NOTHING, true));
            }
            let c = Rc::new(ctx.clone());
            let lazy = att.obj;
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, phase: usize| {
                let members = it.flatten(s, lazy, phase, Some(&c));
                for (k, obj) in members.into_iter().enumerate() {
                    let _ = it.scene_add_text(
                        s,
                        obj,
                        key.clone(),
                        items.clone(),
                        style.clone(),
                        pos_plus(&c.pos, "g", k as i64),
                        &c,
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
            let members = self.flatten(s, att.obj, 3, Some(ctx));
            let mut outs = Vec::new();
            for (k, obj) in members.into_iter().enumerate() {
                let t = self.scene_add_text(
                    s,
                    obj,
                    key.clone(),
                    items.clone(),
                    style.clone(),
                    pos_plus(&ctx.pos, "e", k as i64),
                    ctx,
                    false,
                    key_unit,
                    call_start,
                )?;
                self.set_text_off(t, seg.start);
                outs.push(t);
            }
            let set = self
                .new_obj(ObjData::Set(Box::new(SetData::plain(outs.into_iter().map(|t| (t, true)).collect(), false))));
            return Ok((set, att.sel));
        }
        let t =
            self.scene_add_text(s, att.obj, key, items, style, ctx.pos.clone(), ctx, false, key_unit, call_start)?;
        self.set_text_off(t, seg.start);
        Ok((t, att.sel))
    }

    fn set_text_off(&mut self, t: ObjId, off: usize) {
        match self.om(t) {
            ObjData::Text { off: o, .. } | ObjData::KeyText { off: o, .. } => *o = off,
            _ => {}
        }
    }

    pub(crate) fn check_markup(&mut self, text: &str, ctx: &Ctx, seg: &Segment) {
        let at = seg.call.as_ref().map(|c| c.start).unwrap_or(seg.start);
        for msg in crate::markup::problems(text) {
            self.warn(ctx, format!("{msg} (13.6)"), at);
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
        if let Some(d) = self.failed_obj(a.obj) {
            let id = self.err_cause(ctx, format!("the object of @{kw} failed"), a.off, d);
            return Err(Stop::Fail(Some(id)));
        }
        if self.is_lazy(a.obj) {
            if ctx.scene.is_none() {
                self.rules.insert(ctx.member_key.clone());
                return Ok((a.obj, true));
            }
            let c = Rc::new(ctx.clone());
            let lazy = a.obj;
            let op2 = op.clone();
            let task: Task = Rc::new(move |it: &mut Interp, s: SceneId, ph: usize| {
                let members = it.flatten(s, lazy, ph, Some(&c));
                for (k, m) in members.into_iter().enumerate() {
                    let _ = it.apply_op(s, &op2, m, pos_plus(&c.pos, "g", k as i64), false, &c);
                }
            });
            self.defer(ctx, phase, task)?;
            if let TargetOp::Symbol { .. } = op {
                // the same symbol queue runs again over generated objects (8.2 step 6)
                let c = Rc::new(ctx.clone());
                let task6: Task = Rc::new(move |it: &mut Interp, s: SceneId, ph: usize| {
                    let members = it.flatten(s, lazy, ph, Some(&c));
                    for (k, m) in members.into_iter().enumerate() {
                        if it.is_generated(m) {
                            let _ = it.apply_op(s, &op, m, pos_plus(&c.pos, "g", k as i64), false, &c);
                        }
                    }
                });
                if let Some(s) = ctx.scene {
                    self.scenes[s].tasks[6].push((ctx.pos.clone(), task6));
                }
            }
            return Ok((a.obj, true));
        }
        let Some(s) = ctx.scene else {
            if ctx.in_symbol() {
                return Ok((self.symbol_modifier(kw, a.obj, seg, ctx, &mapped, &op)?, false));
            }
            return self.scene_only(ctx, seg.start);
        };
        if self.is_set(a.obj) {
            let members = self.flatten(s, a.obj, 3, Some(ctx));
            let sd = self.set(a.obj);
            if members.is_empty() && sd.explicit && sd.items.is_empty() {
                self.warn(ctx, "an empty set written directly: nothing happens (5.2.1)", a.off);
            }
            for (k, m) in members.into_iter().enumerate() {
                self.apply_op(s, &op, m, pos_plus(&ctx.pos, "e", k as i64), !a.sel, ctx)?;
            }
            return Ok((a.obj, a.sel));
        }
        self.apply_op(s, &op, a.obj, ctx.pos.clone(), !a.sel && !ctx.selected, ctx)?;
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

    fn symbol_modifier(
        &mut self,
        kw: &str,
        obj: ObjId,
        seg: &Segment,
        ctx: &Ctx,
        mapped: &HashMap<String, ArgV>,
        op: &TargetOp,
    ) -> R<ObjId> {
        let _ = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
        match (kw, self.o(obj).clone(), op) {
            ("PLACE", ObjData::SymLabel { symbol, index }, TargetOp::Place { x, y, .. }) => {
                self.symbol_mut(symbol).labels[index].position = Some((x.unwrap_or(0.0), y.unwrap_or(0.0)));
                return Ok(obj);
            }
            ("SET-SYMBOL", ObjData::Usage(u), _) if u.in_symbol => {
                let sym = self.anon_def(mapped["symbol"].obj);
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
            format!("in a symbol body @{kw} applies only to a text on `?` or a usage in the body (9.2.2)"),
            seg.start,
        )
    }

    fn kw_set_symbol(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let sym = self.anon_def(mapped["symbol"].obj);
        if !matches!(self.o(sym), ObjData::Symbol(_)) {
            let l = self.label(sym);
            return self.fail(
                ctx,
                format!("the last object of SET-SYMBOL must be a symbol, not {l} (11.3)"),
                mapped["symbol"].off,
            );
        }
        if mapped["target"].obj == LEVEL {
            return Ok((self.block_symbol(sym, seg, ctx), false));
        }
        self.target_each_mapped(kw, mapped, seg, ctx, 4, TargetOp::Symbol { symbol: sym, off: seg.start }, "target")
    }

    /// `# @SET-SYMBOL $X` in a subcircuit gives the default look of its instances: the block symbol
    /// (9.2.6). Written in the same subcircuit several times, the later one wins (8.3).
    fn block_symbol(&mut self, sym: ObjId, seg: &Segment, ctx: &Ctx) -> ObjId {
        let u = ctx.unit;
        let level = self.units[u].level.filter(|_| self.units[u].kind == super::unit::UnitKind::Level);
        match level {
            Some(lv) if ctx.frame.is_none() && self.doc.levels[lv].kind == crate::host::LevelKind::Subckt => {
                let name = self.doc.key(&self.doc.levels[lv].name);
                let (rec, k) = match &ctx.member_key {
                    MemberKey::M { rec, k, .. } => (*rec as i64, *k as i64),
                    _ => (0, 0),
                };
                self.block_symbols.entry(name).or_default().insert(ctx.member_key.clone(), (pos(&[rec, k]), sym));
            }
            _ => {
                let at = seg.call.as_ref().unwrap().start;
                self.warn(ctx, "only `# @SET-SYMBOL` in a subcircuit is a block symbol; a file level has no instances, so this has no effect (9.2.6)", at);
            }
        }
        LEVEL
    }

    // ------------------------------------------------------------------ sets (5.2.2)

    fn kw_mod(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx, add: bool) -> R<(ObjId, bool)> {
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let items = mapped["items"].obj;
        let target = mapped["target"].clone();
        let plain_set =
            matches!(self.o(target.obj), ObjData::Set(s) if matches!(s.kind, SetKind::Plain | SetKind::Select { .. }));
        if !plain_set {
            return self.fail(ctx, format!("the target of {kw} must be a set; nothing is wrapped (5.2.2)"), target.off);
        }
        // a set from a package, edited level by level, is evaluated per level like a selection (10.3)
        let per_level = self.is_lazy(items) || (self.is_set(items) && !self.scene_bound(items));
        if per_level {
            self.rules.insert(ctx.member_key.clone());
        }
        if ctx.scene.is_none() && !per_level {
            return self.scene_only(ctx, seg.start);
        }
        let m = SetMod { pos: ctx.pos.clone(), add, source: items, ctx: Rc::new(ctx.clone()) };
        let lazy = !self.never_lazy(items);
        self.set_mut(target.obj).add_mod(m, lazy);
        Ok((target.obj, self.is_lazy(target.obj)))
    }

    // ------------------------------------------------------------------ selections (10.1)

    fn kw_select(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let call = seg.call.as_ref().unwrap();
        if call.params.as_ref().is_some_and(|p| !p.positional.is_empty()) {
            return self.fail(ctx, "the conditions of a selection are written by name only (10.1)", call.start);
        }
        let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
        let scope = mapped["scope"].obj;
        let mut dfn = mapped.get("definition").map(|a| a.obj);
        if (kw == "SELECT-USAGE" && dfn.is_some()) || kw == "SELECT-ANCESTOR-USAGE" {
            let d = self.anon_def(dfn.unwrap_or(NOTHING));
            if !matches!(self.o(d), ObjData::Definition(_)) || d == NOTHING {
                let l = self.label(d);
                return self.fail(
                    ctx,
                    format!("the last object of {kw} must be an ordinary definition, not {l} (10.1)"),
                    seg.start,
                );
            }
            if kw == "SELECT-ANCESTOR-USAGE" && scope == LEVEL {
                return self.fail(
                    ctx,
                    "the scope of SELECT-ANCESTOR-USAGE cannot be `#`: this level is no one's member (10.1)",
                    seg.start,
                );
            }
            dfn = Some(d);
        }
        if get(&p, "role").is_some()
            && !(matches!(self.o(scope), ObjData::Usage(_)) || self.is_lazy(scope) || self.is_set(scope))
        {
            self.warn(ctx, "role= means something only when the scope is a usage (K3.12)", call.start);
        }
        let key = ("sel", ptr_seg(seg), self.frame_key(ctx), if scope == LEVEL { 0 } else { scope.0 as usize + 1 });
        if let Some(&s) = self.node_memo.get(&key) {
            self.rules.insert(ctx.member_key.clone());
            return Ok((s, true));
        }
        let params: BTreeMap<String, Lit> = p.into_iter().collect();
        let sd = SetData {
            kind: SetKind::Select {
                keyword: kw.to_string(),
                scope,
                params,
                writer: ctx.unit,
                definition: dfn,
                site: Some(Rc::new(ctx.clone())),
                off: seg.start,
            },
            writer: Some(ctx.unit),
            ..SetData::plain(Vec::new(), false)
        };
        let s = self.new_obj(ObjData::Set(Box::new(sd)));
        self.node_memo.insert(key, s);
        self.rules.insert(ctx.member_key.clone());
        Ok((s, true))
    }

    // ------------------------------------------------------------------ wires, constraints

    fn kw_wire(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let mut spec = sig_params(kw);
        spec.extend(style_specs("wire_style"));
        let p = self.params(kw, seg, ctx, &spec, None, false, None)?;
        let mapped = self.map_args(kw, args, seg, ctx, sig_positions(kw))?;
        let nets_a = mapped["net"].clone();
        let pts_a = mapped["points"].clone();
        let Some(s) = ctx.scene else {
            if self.is_lazy(nets_a.obj) || self.is_lazy(pts_a.obj) {
                self.rules.insert(ctx.member_key.clone());
                return Ok((NOTHING, true));
            }
            return self.scene_only(ctx, seg.start);
        };
        let (nets, bus) = if self.is_set(nets_a.obj) {
            let nets = self.flatten(s, nets_a.obj, 3, Some(ctx));
            if nets.is_empty() {
                return self.fail(
                    ctx,
                    "the net of a wire is an empty set, so the wire fails (5.2.1, 13.1)",
                    nets_a.off,
                );
            }
            (nets, true)
        } else {
            (vec![nets_a.obj], false)
        };
        for n in &nets {
            if let Some(d) = self.failed_obj(*n) {
                let id = self.err_cause(
                    ctx,
                    "a net of the wire cannot be found: the whole wire fails (13.1)",
                    nets_a.off,
                    d,
                );
                return Err(Stop::Fail(Some(id)));
            }
            if !matches!(self.o(*n), ObjData::Net { .. }) {
                let l = self.label(*n);
                return self.fail(
                    ctx,
                    format!("the net position of a wire received {l}, the wrong kind (13.1)"),
                    nets_a.off,
                );
            }
        }
        let pts = if self.is_set(pts_a.obj) { self.flatten(s, pts_a.obj, 3, Some(ctx)) } else { vec![pts_a.obj] };
        if pts.len() < 2 {
            return self.fail(ctx, "a wire has at least two points (13.1)", pts_a.off);
        }
        for q in &pts {
            if let Some(d) = self.failed_obj(*q) {
                let id = self.err_cause(ctx, "a point of the wire failed", pts_a.off, d);
                return Err(Stop::Fail(Some(id)));
            }
            if !matches!(self.o(*q), ObjData::Term { .. } | ObjData::Point { .. } | ObjData::Pos { .. }) {
                let l = self.label(*q);
                return self.fail(
                    ctx,
                    format!("the points of a wire are terminals or points, not {l} (13.1)"),
                    pts_a.off,
                );
            }
        }
        let (style, rest) = split_style(&p);
        let mode = get(&rest, "mode").and_then(Lit::str).unwrap_or("auto").to_string();
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
        Ok((w, false))
    }

    fn kw_constraint(&mut self, kw: &str, args: &[ArgV], seg: &Segment, ctx: &Ctx) -> R<(ObjId, bool)> {
        let call_start = seg.call.as_ref().unwrap().start;
        if kw == "ORDER" || kw == "DISTANCE" {
            self.old_direction(kw, seg, ctx)?;
        }
        let p = self.params(kw, seg, ctx, &sig_params(kw), None, false, None)?;
        let mut cp = ConstraintParams { axis: None, sense: None, min: None, max: None, relation: None };
        match kw {
            "ORDER" | "DISTANCE" => {
                let (axis, sense) =
                    direction(get(&p, "direction").and_then(Lit::str).unwrap_or("x")).unwrap_or(('x', "either"));
                cp.axis = Some(axis);
                cp.sense = Some(sense);
            }
            "ALIGN" | "DISTRIBUTE" => {
                cp.axis = get(&p, "axis").and_then(Lit::str).and_then(|a| a.chars().next());
            }
            "ORIENT-RELATION" => {
                cp.relation = get(&p, "relation").and_then(Lit::str).and_then(Orient::parse);
            }
            _ => {}
        }
        if kw == "DISTANCE" {
            cp.min = get(&p, "min").and_then(Lit::num);
            cp.max = get(&p, "max").and_then(Lit::num);
            if let Some(e) = get(&p, "exact").and_then(Lit::num) {
                // the exact distance, short for min = max (14.5, PR15)
                if cp.min.is_some() || cp.max.is_some() {
                    return self.fail(ctx, "exact of DISTANCE cannot be written with min or max (14.5)", call_start);
                }
                cp.min = Some(e);
                cp.max = Some(e);
            }
            if cp.min.is_none() && cp.max.is_none() {
                return self.fail(ctx, "DISTANCE takes at least one of min, max and exact (14.5)", call_start);
            }
            if let (Some(a), Some(b)) = (cp.min, cp.max) {
                if a > b {
                    return self.fail(ctx, "the min of DISTANCE cannot be greater than its max (14.5)", call_start);
                }
            }
            if cp.sense == Some("either") && (cp.min.unwrap_or(0.0) < 0.0 || cp.max.unwrap_or(0.0) < 0.0) {
                return self.fail(
                    ctx,
                    "without a sense (\"x\", \"y\") the bounds of DISTANCE cannot be negative (14.5)",
                    call_start,
                );
            }
        }
        let positions = sig_positions(kw);
        let mapped = self.map_args(kw, args, seg, ctx, positions)?;
        let arglist: Vec<ObjId> = positions.iter().map(|q| mapped[&q.name].obj).collect();
        let lazy = arglist.iter().any(|a| self.is_lazy(*a));
        let Some(s) = ctx.scene else {
            if lazy {
                self.rules.insert(ctx.member_key.clone());
                return Ok((NOTHING, true));
            }
            return self.scene_only(ctx, seg.start);
        };
        for a in &arglist {
            if let Some(d) = self.failed_obj(*a) {
                let id = self.err_cause(
                    ctx,
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
            priority_param: priority_of(&p, ctx),
            chain: self.apply_chain(ctx),
            status: None,
            eff: None,
            failed: false,
            site: Some(Rc::new(ctx.clone())),
            off: seg.start,
        };
        let id = self.new_obj(ObjData::Constraint(Box::new(c)));
        self.scene_add_constraint(s, id, ctx.pos.clone());
        if lazy {
            self.rules.insert(ctx.member_key.clone());
        }
        Ok((id, false))
    }

    /// ORDER and DISTANCE take one `direction` now: axis= and "increasing", "decreasing",
    /// "either" fail with a hint (14.3, 14.5).
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
            return self.fail(
                ctx,
                format!("axis and direction of @{kw} are one parameter now, direction, written first by position, such as @ORDER(\"left-to-right\") or @DISTANCE(\"bottom-to-top\", min=2); with no sense write \"x\" or \"y\" ({sec})"),
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
    pub priority: Option<Priority>,
    pub seg_start: usize,
    pub seg: Segment,
}

fn sig_params(kw: &str) -> Vec<ParamSpec> {
    words().keyword(kw).map(|s| s.param_list().to_vec()).unwrap_or_default()
}

fn sig_positions(kw: &str) -> &'static [PositionSpec] {
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
    for old in sym.pins.iter_mut() {
        old.terms.retain(|t| !pin.terms.iter().any(|n| t.same(n)));
    }
    sym.pins.retain(|q| !q.terms.is_empty());
    sym.pins.push(pin);
}

pub(crate) fn merge_symbol(sym: &mut SymbolDef, other: &SymbolDef) {
    for pin in &other.pins {
        add_pin(sym, pin.clone());
    }
    sym.prims.extend(other.prims.iter().cloned());
    sym.labels.extend(other.labels.iter().cloned());
    sym.usages.extend(other.usages.iter().cloned());
    if sym.bbox.is_none() && other.bbox.is_some() {
        sym.bbox = other.bbox;
    }
}
