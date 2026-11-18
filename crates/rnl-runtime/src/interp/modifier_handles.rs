//! A failed modifier keeps the handle it received (5.5, 6.6).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::syntax::{Member, Segment};
use crate::words::{words, PositionSpec, Takes};

use super::ctx::Ctx;
use super::eval::{ArgV, Stop, R};
use super::obj::{ObjData, ObjId, SetKind};
use super::Interp;

impl Interp<'_> {
    pub(super) fn modifier_keeps_input(&self, kw: &str) -> bool {
        words().keyword(kw).is_some_and(|sig| matches!(sig.output.as_str(), "pass" | "pass-target") || kw == "WIRE")
    }

    pub(super) fn remember_modifier_input(&self, kw: &str, mapped: &HashMap<String, ArgV>, ctx: &Ctx) {
        let Some(sig) = words().keyword(kw) else { return };
        let slot = match sig.output.as_str() {
            "pass" => sig.position_list().iter().find(|p| !matches!(p.takes, Takes::Def(_))).map(|p| p.name.as_str()),
            "pass-target" => Some("target"),
            _ if kw == "WIRE" => Some("net"),
            _ => None,
        };
        let Some(input) = slot.and_then(|slot| mapped.get(slot)) else { return };
        if kw == "WIRE" && !matches!(self.o(input.obj), ObjData::Wire { .. }) {
            return;
        }
        ctx.modifier_input.set(Some((input.obj, input.sel)));
    }

    pub(super) fn keyword_with_recovery(
        &mut self,
        kw: &str,
        args: Vec<ArgV>,
        seg: &Segment,
        ctx: &Ctx,
        member: &Rc<Member>,
        input_error: Option<Stop>,
    ) -> R<(ObjId, bool)> {
        if let Some(error) = input_error {
            let sig = words().keyword(kw).unwrap();
            // Loose inputs to a single target slot form a set. The valid members still get
            // their own descriptions; failed members are skipped by the ordinary set walk.
            let independent = sig.output == "pass"
                && sig.position_list().len() == 1
                && !matches!(sig.position_list()[0].takes, Takes::Def(_))
                && args.len() > 1;
            if !independent {
                return self.recover_modifier_failure(kw, &args, seg, ctx, error);
            }
            self.note_failed(ctx.rec, seg.start, seg.end, false);
        }
        let result = self.keyword(kw, args.clone(), seg, ctx, member);
        match result {
            Err(error @ Stop::Fail(_)) => self.recover_modifier_failure(kw, &args, seg, ctx, error),
            other => other,
        }
    }

    pub(super) fn recover_modifier_failure(
        &mut self,
        kw: &str,
        args: &[ArgV],
        seg: &Segment,
        ctx: &Ctx,
        error: Stop,
    ) -> R<(ObjId, bool)> {
        if ctx.modifier_input.get().is_none() {
            // Parameter checks can fail before slots are matched. Read only the already
            // evaluated arguments, and only when their shape uniquely matches the signature.
            let positions = words().keyword(kw).unwrap().position_list();
            if self.modifier_slots_match(args, positions, seg, ctx) {
                let _ = self.map_args(kw, args, seg, ctx, positions);
            }
        }
        if let Some((object, selected)) = ctx.modifier_input.get().filter(|(o, _)| !self.is_failed(*o)) {
            self.note_failed(ctx.rec, seg.start, seg.end, false);
            return Ok((object, selected));
        }
        Err(error)
    }

    /// A structural check without diagnostics or evaluation. The actual mapping stays in
    /// map_args, which also records the identity of a set made from loose objects.
    fn modifier_slots_match(&mut self, args: &[ArgV], positions: &[PositionSpec], seg: &Segment, ctx: &Ctx) -> bool {
        let has_def = positions.iter().any(|p| matches!(p.takes, Takes::Def(_)));
        let args = if has_def {
            let Some((last, rest)) = args.split_last() else { return false };
            if last.role.is_some() || (seg.piped && rest.is_empty()) {
                return false;
            }
            rest
        } else {
            args
        };
        let slots: Vec<_> = positions.iter().filter(|p| !matches!(p.takes, Takes::Def(_))).collect();
        if args.iter().any(|a| a.role.is_some()) {
            let mut names = Vec::new();
            for arg in args {
                let Some(name) = &arg.role else { return false };
                let name = if arg.role_quoted { name.clone() } else { self.word(name, ctx.unit) };
                if !slots.iter().any(|slot| slot.name == name) || names.contains(&name) {
                    return false;
                }
                names.push(name);
            }
            return slots.iter().all(|slot| slot.optional || names.contains(&slot.name));
        }
        if slots.len() == 1 {
            return !args.is_empty() && (!has_def || args.len() == 1);
        }
        if !has_def && args.len() == 1 && self.is_set(args[0].obj) && !self.is_lazy(args[0].obj) {
            if matches!(self.set(args[0].obj).kind, SetKind::Constraints { .. }) {
                return self.written_members(args[0].obj, ctx.scene, &mut HashSet::new()).len() == slots.len();
            }
            return self.set(args[0].obj).items.len() == slots.len();
        }
        args.len() == slots.len()
    }
}
