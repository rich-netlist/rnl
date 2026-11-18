//! Current facts and the selection snapshots that can read them (10.4, 15.2).

use std::rc::Rc;

use crate::diag::{codes, Loc};

use super::ctx::Ctx;
use super::eval::R;
use super::obj::{ObjData, ObjId, Pos, Priority, SceneId};
use super::Interp;

#[derive(Clone)]
pub(crate) struct CurrentSource {
    input: ObjId,
    site: Rc<Ctx>,
    chain: Vec<ObjId>,
    off: usize,
    visible_from: usize,
    after_symbols: bool,
}

#[derive(Clone)]
pub struct CurrentFact {
    pub pos: Pos,
    pub terminal: Option<ObjId>,
    pub direction: String,
    pub loc: Option<Loc>,
    source: CurrentSource,
}

impl Interp<'_> {
    fn current_host_terminal(&self, object: ObjId) -> bool {
        matches!(self.o(object), ObjData::Term { owner, .. }
            if matches!(self.o(*owner), ObjData::Inst { .. } | ObjData::LevelObj { .. }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_current(
        &mut self,
        scene: SceneId,
        input: ObjId,
        direction: &str,
        off: usize,
        pos: Pos,
        ctx: &Ctx,
    ) -> R<()> {
        let after_symbols = self.late_input(input) || matches!(self.o(input), ObjData::ScenePin { .. });
        if !after_symbols && !self.current_host_terminal(input) {
            return self.fail(
                ctx,
                codes::CURRENT_NOT_HOST_TERMINAL,
                format!("CURRENT takes host terminals only, not {} (15.2)", self.label(input)),
                off,
            );
        }
        let source = CurrentSource {
            input,
            site: Rc::new(ctx.clone()),
            chain: self.apply_chain(ctx),
            off,
            // Runtime phase 5 is generation, and phase 6 is the generated-symbol queue.
            // Neither generation nor CURRENT rules can read the changes made in generation.
            visible_from: if after_symbols || self.scenes[scene].phase >= 5 { 6 } else { 3 },
            after_symbols,
        };
        let fact = CurrentFact {
            pos: super::obj::order_pos(&pos, &ctx.segments),
            terminal: (!after_symbols).then_some(input),
            direction: direction.to_owned(),
            loc: self.rec_loc(ctx.rec, off),
            source,
        };
        self.scenes[scene].currents.push(fact);
        Ok(())
    }

    /// Inspect only the role conversions needed by CURRENT. This does not resolve a role or
    /// report its failures early: the final role checks still run after the symbol queues.
    fn current_input_preview(&mut self, scene: SceneId, object: ObjId) -> Option<ObjId> {
        let Some(view) = self.role_inputs.get(&object).cloned() else {
            return Some(self.scene_value(object));
        };
        if let Some(resolved) = view.resolved {
            return Some(self.scene_value(resolved));
        }
        if view.collection {
            return None;
        }
        let source = self.current_input_preview(scene, view.source)?;
        if view.kinds.iter().any(|kind| self.role_kind_matches(source, kind)) {
            return Some(source);
        }
        match (self.o(source).clone(), self.conversion_kind(source, &view.kinds).as_deref()) {
            (ObjData::Term { owner, .. }, Some("pin")) => {
                let index = self.pin_for(scene, owner, source)?;
                Some(self.scene_pin_object(scene, owner, index))
            }
            (ObjData::ScenePin { owner, index }, Some("terminal")) => {
                if self.scene_pin_is_bus(scene, owner, index) {
                    return None;
                }
                let terminals = self.scene_pin_terminals(scene, owner, index);
                (terminals.len() == 1).then(|| terminals[0])
            }
            _ => None,
        }
    }

    fn current_terminal_value(&mut self, scene: SceneId, object: ObjId) -> Option<ObjId> {
        let terminal = match *self.o(object) {
            ObjData::ScenePin { owner, index } => {
                // A bus conversion gives a set even when it contains just one terminal.
                if self.scene_pin_is_bus(scene, owner, index) {
                    return None;
                }
                let terminals = self.scene_pin_terminals(scene, owner, index);
                if terminals.len() != 1 {
                    return None;
                }
                terminals[0]
            }
            _ => object,
        };
        self.current_host_terminal(terminal).then_some(terminal)
    }

    pub(crate) fn resolve_currents(&mut self, scene: SceneId, final_check: bool) {
        let inputs: Vec<_> = self.scenes[scene]
            .currents
            .iter()
            .enumerate()
            .filter(|(_, fact)| fact.source.after_symbols)
            .map(|(index, fact)| (index, fact.source.clone()))
            .collect();
        for (index, source) in inputs {
            let value = if final_check {
                Some(self.scene_value(source.input))
            } else {
                self.current_input_preview(scene, source.input)
            };
            let terminal = value.and_then(|value| self.current_terminal_value(scene, value));
            self.scenes[scene].currents[index].terminal = terminal;
            if final_check && terminal.is_none() {
                let failed = self.failed_obj(source.input).or_else(|| value.and_then(|v| self.failed_obj(v)));
                if let Some(cause) = failed {
                    self.err_cause(
                        &source.site,
                        codes::KEYWORD_OBJECT_FAILED,
                        "the object of @CURRENT failed",
                        source.off,
                        cause,
                    );
                } else {
                    self.err(
                        &source.site,
                        codes::CURRENT_NOT_HOST_TERMINAL,
                        format!(
                            "CURRENT takes one host terminal; {} does not convert to one (5.2.4, 15.2)",
                            self.label(source.input)
                        ),
                        source.off,
                    );
                }
            }
        }
    }

    pub(crate) fn current_fact_active(&self, scene: SceneId, fact: &CurrentFact, phase: usize) -> bool {
        if fact.terminal.is_none() {
            return false;
        }
        let source = &fact.source;
        phase >= source.visible_from
            && source.site.header.priority != Some(Priority::Disabled)
            && !self.failed_body(&source.site)
            && !self.is_failed(source.input)
            && !self.is_failed(self.scene_value(source.input))
            && self.eff_priority(scene, None, &source.chain, None) != Priority::Disabled
    }

    pub(crate) fn current_of_at(&self, scene: SceneId, terminal: ObjId, phase: usize) -> String {
        self.scenes[scene]
            .currents
            .iter()
            .filter(|fact| fact.terminal == Some(terminal) && self.current_fact_active(scene, fact, phase))
            .fold(None::<&CurrentFact>, |best, fact| {
                if best.is_none_or(|previous| fact.pos > previous.pos) {
                    Some(fact)
                } else {
                    best
                }
            })
            .map(|fact| fact.direction.clone())
            .unwrap_or_else(|| "unspecified".into())
    }

    pub(crate) fn current_of(&self, scene: SceneId, terminal: ObjId) -> String {
        self.current_of_at(scene, terminal, usize::MAX)
    }
}
