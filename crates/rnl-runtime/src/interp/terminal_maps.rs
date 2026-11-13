//! Terminal mappings of a symbol assignment in language 0.4 (6.4, 11.3).

use std::rc::Rc;

use crate::diag::codes;

use super::ctx::Ctx;
use super::eval::R;
use super::obj::{DefData, Lit, ObjData, ObjId, SceneId, TermKey};
use super::Interp;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TerminalMap {
    Order(Vec<TermKey>),
    Pairs(Vec<(TermKey, TermKey)>),
}

pub(crate) struct MapTarget {
    pub names: Vec<(u32, Option<String>)>,
    pub roles: bool,
    pub case_sensitive: bool,
}

impl MapTarget {
    pub(crate) fn roles(definition: &DefData) -> Self {
        Self {
            names: definition.roles.iter().enumerate().map(|(i, r)| (i as u32 + 1, Some(r.name.clone()))).collect(),
            roles: true,
            case_sensitive: true,
        }
    }

    fn index(&self, key: &TermKey) -> Option<u32> {
        match key {
            TermKey::Num(n) if !self.roles => Some(*n),
            TermKey::Name(name) => {
                self.names.iter().find_map(|(index, candidate)| {
                    candidate
                        .as_ref()
                        .filter(|candidate| {
                            if self.case_sensitive {
                                *candidate == name
                            } else {
                                candidate.eq_ignore_ascii_case(name)
                            }
                        })
                        .map(|_| *index)
                })
            }
            _ => None,
        }
    }
}

/// A terminal as a condition or reference of 0.4 writes it: a string made only of digits, quoted or
/// not, is the terminal number (6.4); anything else is as written.
pub(crate) fn terminal_value(value: &Lit) -> Lit {
    match terminal_key(value) {
        Some(TermKey::Num(n)) => Lit::Num(n as f64),
        _ => value.clone(),
    }
}

/// A quoted, digits-only terminal is still a position (6.4).
pub(crate) fn terminal_key(value: &Lit) -> Option<TermKey> {
    match value {
        Lit::Num(n) if *n >= 1.0 && *n <= u32::MAX as f64 && n.fract() == 0.0 => Some(TermKey::Num(*n as u32)),
        Lit::Str(s) if !s.as_str().is_empty() => {
            let s = s.as_str();
            if s.bytes().all(|b| b.is_ascii_digit()) {
                s.parse::<u32>().ok().filter(|n| *n > 0).map(TermKey::Num)
            } else {
                Some(TermKey::Name(s.to_owned()))
            }
        }
        _ => None,
    }
}

impl Interp<'_> {
    pub(crate) fn map_target(&mut self, scene: SceneId, object: ObjId) -> MapTarget {
        match self.o(object) {
            ObjData::Usage(u) => MapTarget::roles(self.def(u.definition)),
            _ => MapTarget {
                names: self.owner_terms(scene, object),
                roles: false,
                case_sensitive: self.host_case(object),
            },
        }
    }

    pub(crate) fn terminal_map(&mut self, value: &Lit, ctx: &Ctx, off: usize) -> R<Rc<TerminalMap>> {
        let parsed = match value {
            Lit::Arr(items) if items.iter().all(|item| !matches!(item, Lit::Arr(_))) => {
                items.iter().map(terminal_key).collect::<Option<Vec<_>>>().map(TerminalMap::Order)
            }
            Lit::Arr(items) => items
                .iter()
                .map(|item| match item {
                    Lit::Arr(pair) if pair.len() == 2 => terminal_key(&pair[0]).zip(terminal_key(&pair[1])),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(TerminalMap::Pairs),
            _ => None,
        };
        let Some(map) = parsed else {
            return self.fail(ctx, codes::PARAM_SYMBOL_TERMINALS_INVALID,
                "terminals= of SET-SYMBOL is a list of symbol terminal names or pairs [object terminal or role, symbol terminal]; terminal numbers start at 1, and this implementation supports them through 4294967295 (6.4, 11.3)", off);
        };
        if duplicate_names(&map, true) {
            return self.fail(ctx, codes::PARAM_SYMBOL_TERMINALS_INVALID,
                "terminals= of SET-SYMBOL names a terminal twice; each side of a mapping must have distinct names (11.3)", off);
        }
        Ok(Rc::new(map))
    }

    /// Make an assignment-specific view. The reusable definition, names and pin order stay intact.
    /// Matching is by the receiving object's case rules, including case-sensitive usage roles.
    pub(crate) fn mapped_symbol(
        &mut self,
        symbol: ObjId,
        map: &TerminalMap,
        target: &MapTarget,
        ctx: &Ctx,
        off: usize,
    ) -> R<ObjId> {
        let case_sensitive = target.case_sensitive;
        let repeated_target = match map {
            TerminalMap::Pairs(pairs) => pairs.iter().enumerate().any(|(i, (key, _))| {
                target
                    .index(key)
                    .is_some_and(|index| pairs[..i].iter().any(|(old, _)| target.index(old) == Some(index)))
            }),
            _ => false,
        };
        if duplicate_names(map, case_sensitive) || repeated_target {
            return self.fail(
                ctx,
                codes::PARAM_SYMBOL_TERMINALS_INVALID,
                "terminals= of SET-SYMBOL identifies a terminal twice under this object's name and number rules (6.4, 11.3)",
                off,
            );
        }
        let mut pins = self.symbol(symbol).pins.clone();
        for pin in &mut pins {
            pin.terms = pin
                .terms
                .iter()
                .filter_map(|key| match map {
                    TerminalMap::Order(list) => {
                        let index = list.iter().position(|name| name.same(key, case_sensitive))?;
                        if target.roles {
                            target.names.get(index).and_then(|(_, name)| name.clone()).map(TermKey::Name)
                        } else {
                            Some(TermKey::Num(index as u32 + 1))
                        }
                    }
                    TerminalMap::Pairs(pairs) => Some(
                        pairs
                            .iter()
                            .find(|(_, name)| name.same(key, case_sensitive))
                            .map_or_else(|| key.clone(), |(target, _)| target.clone()),
                    ),
                })
                .collect();
        }
        let bindings: Vec<Vec<TermKey>> = pins.iter().map(|pin| pin.terms.clone()).collect();
        let cache_key = (symbol, bindings);
        if let Some(&mapped) = self.mapped_symbols.get(&cache_key) {
            return Ok(mapped);
        }
        let mut view = self.symbol(symbol).clone();
        view.pins = pins;
        view.original = Some(view.original.unwrap_or(symbol));
        let mapped = self.new_obj(ObjData::Symbol(Box::new(view)));
        self.objs[mapped.0 as usize].name = self.objs[symbol.0 as usize].name.clone();
        self.mapped_symbols.insert(cache_key, mapped);
        Ok(mapped)
    }
}

fn duplicate_names(map: &TerminalMap, case_sensitive: bool) -> bool {
    let twice = |names: Vec<&TermKey>| {
        names.iter().enumerate().any(|(i, name)| names[..i].iter().any(|old| name.same(old, case_sensitive)))
    };
    match map {
        TerminalMap::Order(names) => twice(names.iter().collect()),
        TerminalMap::Pairs(pairs) => {
            twice(pairs.iter().map(|(name, _)| name).collect()) || twice(pairs.iter().map(|(_, name)| name).collect())
        }
    }
}
