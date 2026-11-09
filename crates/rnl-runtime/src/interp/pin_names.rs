//! Keep pin names reachable through the sub-object lookup order (6.5).

use std::collections::HashMap;
use std::rc::Rc;

use crate::diag::{codes, Severity};
use crate::words::words;

use super::ctx::Ctx;
use super::eval::R;
use super::obj::{ObjData, ObjId, Pos, SceneId};
use super::Interp;

/// Source of one scene TEXT layer, retained until the effective symbol is known.
#[derive(Clone, Debug)]
pub(crate) struct PinTextSource {
    pub text: ObjId,
    pub order: Pos,
    pub ctx: Rc<Ctx>,
    pub off: usize,
    pub generated: bool,
}

impl Interp<'_> {
    /// Reject this assignment before it enters the symbol queue. The reusable
    /// symbol is still valid for other owners, and an earlier assignment survives.
    pub(crate) fn check_symbol_pin_names(
        &mut self,
        symbol: ObjId,
        owner: Option<ObjId>,
        ctx: &Ctx,
        off: usize,
    ) -> R<()> {
        let unit = self.source_unit(ctx);
        if !self.follows_0_4(unit) {
            return Ok(());
        }
        let names: Vec<String> = self.symbol(symbol).pins.iter().flat_map(|p| p.names.clone()).collect();
        let (own, aliases) = match owner {
            Some(o) if matches!(self.o(o), ObjData::Usage(_)) => self.usage_subnames(o, unit),
            _ => (Vec::new(), Vec::new()),
        };
        for name in names {
            let usage_clash = own.iter().find(|(n, _)| *n == name).map(|(_, kind)| format!("a {kind}")).or_else(|| {
                aliases.iter().find(|(alias, _)| *alias == name).map(|(_, original)| format!("an alias of {original}"))
            });
            if let Some(clash) = usage_clash {
                let label = self.label(owner.unwrap());
                return self.fail(ctx, codes::PIN_NAME_IS_USAGE_NAME,
                    format!("the pin name {name} is {clash} of the usage {label}: `::` finds it before the pin; this SET-SYMBOL fails (6.5, 11.3, 16.5)"), off);
            }
            let standard = self.word(&name, unit);
            if words().is_subobject(&standard) {
                let what = if name == standard {
                    format!("the standard sub-object name {standard}")
                } else {
                    format!("an alias of the standard sub-object name {standard}")
                };
                return self.fail(ctx, codes::PIN_NAME_IS_SUBOBJECT,
                    format!("the pin name {name} is {what}: `::` finds it before the pin; this SET-SYMBOL fails (6.5, 11.3, 16.5)"), off);
            }
        }
        Ok(())
    }

    pub(crate) fn symbol_has_pin_name(&self, symbol: ObjId, name: &str) -> bool {
        self.symbol(symbol).pins.iter().any(|pin| pin.names.iter().any(|n| n == name))
    }

    /// Texts may precede the assignment or come from generation. Check only after
    /// both symbol queues, against the effective symbol rather than a losing one.
    pub(crate) fn check_scene_pin_text_names(&mut self, s: SceneId) {
        let sources = std::mem::take(&mut self.scenes[s].pin_text_sources);
        let mut kept = HashMap::new();
        let mut kept_direct = HashMap::new();
        let mut rejected: HashMap<ObjId, Vec<Pos>> = HashMap::new();
        for source in &sources {
            let ObjData::KeyText { attach, key, .. } = self.o(source.text) else { continue };
            let collision = self.follows_0_4(self.source_unit(&source.ctx))
                && self.scenes[s].symbol.get(attach).is_some_and(|symbol| self.symbol_has_pin_name(*symbol, key));
            if !collision {
                kept.insert(source.text, source);
                if !source.generated {
                    kept_direct.insert(source.text, source);
                }
                continue;
            }
            let label = self.label(*attach);
            self.diag_site(Severity::Error, codes::PIN_NAME_IS_TEXT_KEY_SCENE, &Some(source.ctx.clone()), source.off,
                format!("the text key {key} of {label} is a pin name of its symbol: `::` finds the pin first; this TEXT fails (6.5, 13.3)"));
            rejected.entry(source.text).or_default().push(source.order.clone());
        }
        for (text, mut orders) in rejected {
            orders.sort();
            let last = kept.get(&text);
            let last_direct = kept_direct.get(&text);
            if let ObjData::KeyText { layers, site, off, direct, generated, failed, .. } = self.om(text) {
                layers.retain(|(order, _, _)| orders.binary_search(order).is_err());
                *site = last.map(|source| source.ctx.clone());
                *off = last.map_or(0, |source| source.off);
                *generated = last.is_some_and(|source| source.generated);
                *direct = last_direct.map(|source| (source.ctx.clone(), source.off));
                *failed = layers.is_empty();
            }
        }
    }
}
