//! Local pin objects, their points and names in symbol bodies (9.2.4, 9.2.7).

use crate::diag::codes;
use crate::geom::Pt;

use super::ctx::{Ctx, Nested};
use super::obj::{ObjData, ObjId, SymbolDef};
use super::Interp;

impl Interp<'_> {
    /// A local pin can stand for its point without losing the pin's identity.
    pub(crate) fn symbol_point(&self, object: ObjId) -> Option<Pt> {
        match self.o(object) {
            ObjData::Point { x: Some(x), y: Some(y), local: true } => Some((*x, *y)),
            ObjData::SymbolPin { symbol, index } => Some(self.symbol(*symbol).pins[*index].pos),
            _ => None,
        }
    }

    /// Record every name of a direct body member, including pipe bindings. A point
    /// named before PIN does not name its result; a repeated reference creates no pin.
    pub(crate) fn bind_symbol_name(
        &mut self,
        symbol: ObjId,
        key: &(char, String),
        object: ObjId,
        ctx: &Ctx,
        off: usize,
    ) -> bool {
        let old = ctx.nested.as_ref().and_then(|n| n.borrow().get(key).cloned());
        if let Some(Nested::Val(old)) = old {
            if !self.is_failed(old) && self.is_failed(object) {
                return false;
            }
            if old != object && !self.is_failed(old) && !self.is_failed(object) {
                self.err(
                    ctx,
                    if matches!(self.o(old), ObjData::SymbolPin { .. }) || matches!(self.o(object), ObjData::SymbolPin { .. }) {
                        codes::PIN_NAME_TWICE
                    } else {
                        codes::NAME_BOUND_TWICE
                    },
                    format!("the local name {}{} is already bound to another object of this symbol; the first binding is kept (6.6, 9.2.4)", key.0, key.1),
                    off,
                );
                return false;
            }
        }
        if let ObjData::SymbolPin { symbol: owner, index } = *self.o(object) {
            if owner == symbol && key.0 == '%' {
                let names = &mut self.symbol_mut(symbol).pins[index].names;
                if !names.contains(&key.1) {
                    names.push(key.1.clone());
                }
            }
        }
        true
    }

    /// Check an inclusion before copying anything. Included pins receive new local
    /// objects, so repeated inclusion never aliases the original pin identities.
    pub(crate) fn include_pin_names(&mut self, symbol: ObjId, other: &SymbolDef, ctx: &Ctx, off: usize) -> bool {
        let Some(nested) = &ctx.nested else { return true };
        for pin in &other.pins {
            for name in &pin.names {
                let old = nested.borrow().get(&('%', name.clone())).cloned();
                if matches!(old, Some(Nested::Val(o)) if !self.is_failed(o)) {
                    self.err(
                        ctx,
                        codes::PIN_NAME_TWICE_INCLUDED,
                        format!("the included pin name {name} is already bound in this symbol; this inclusion fails (6.6, 9.2.4, 9.2.7)"),
                        off,
                    );
                    return false;
                }
            }
        }
        let start = self.symbol(symbol).pins.len();
        for (index, pin) in other.pins.iter().enumerate() {
            if pin.names.is_empty() {
                continue;
            }
            let object = self.new_obj(ObjData::SymbolPin { symbol, index: start + index });
            self.objs[object.0 as usize].name = pin.names[0].clone();
            for name in &pin.names {
                nested.borrow_mut().insert(('%', name.clone()), Nested::Val(object));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::Options;
    use super::*;
    use crate::diag::Severity;
    use crate::source::MemoryProvider;

    fn read<'a>(provider: &'a MemoryProvider) -> Interp<'a> {
        let mut it = Interp::new_package("pins.rnl", provider, &Options { std: false, ..Options::default() });
        it.run();
        it
    }

    fn source(version: &str, body: &str) -> MemoryProvider {
        MemoryProvider::new().with("pins.rnl", &format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"pins\", version=\"1.0\") }}\n@RNL {{ {body} }}"))
    }

    fn symbol<'a>(it: &'a Interp<'_>, name: &str) -> &'a SymbolDef {
        it.objs
            .iter()
            .find_map(|o| match &o.data {
                ObjData::Symbol(s) if o.name == name => Some(&**s),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no symbol {name}: {:?}", it.sink.all()))
    }

    fn no_errors(it: &Interp<'_>) {
        let errors: Vec<_> = it.sink.all().iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn pin_names_belong_to_distinct_results_and_include_every_alias() {
        let files = source("0.4", "$S = @DEFINE-SYMBOL { %point = (2,3); %p = %point @PIN(terminal=1) => %alias; %same = %p; %q = %point @PIN(terminal=1); @LINE {%p %q} }");
        let it = read(&files);
        no_errors(&it);
        let s = symbol(&it, "$S");
        assert_eq!(s.pins.iter().map(|p| p.names.clone()).collect::<Vec<_>>(), [vec!["p", "alias", "same"], vec!["q"]]);
        let pin_objects: Vec<_> = it.objs.iter().filter(|o| matches!(o.data, ObjData::SymbolPin { .. })).collect();
        assert_eq!(pin_objects.len(), 2);
        assert_eq!(s.prims[0].points, [(2.0, 3.0), (2.0, 3.0)]);
    }

    #[test]
    fn inclusion_preserves_names_and_local_point_use_without_rebinding_the_original() {
        let files = source("0.4", "$A = @DEFINE-SYMBOL { %p = (2,3) @PIN(terminal=1) => %alias }; $B = @DEFINE-SYMBOL { $A; %another = %alias; @LINE {%p (0,0)}; %alias @TEXT(text=\"label\", dx=1); %q = %p @PIN(terminal=2) }");
        let it = read(&files);
        no_errors(&it);
        assert_eq!(symbol(&it, "$A").pins[0].names, ["p", "alias"]);
        let b = symbol(&it, "$B");
        assert_eq!(b.pins[0].names, ["p", "alias", "another"]);
        assert_eq!(b.pins[1].names, ["q"]);
        assert_eq!(b.prims[0].points, [(2.0, 3.0), (0.0, 0.0)]);
        assert_eq!(b.prims[1].points, [(3.0, 3.0)]);
    }

    #[test]
    fn conflicting_inclusion_is_atomic_and_later_body_members_still_work() {
        let files = source("0.4", "$A = @DEFINE-SYMBOL { %p = (2,3) @PIN(terminal=1); @LINE {(0,0) (1,1)} }; $B = @DEFINE-SYMBOL { %p = (4,5) @PIN(terminal=2); $A; @LINE {%p (0,0)} }");
        let it = read(&files);
        assert_eq!(it.sink.all().iter().filter(|d| d.code == codes::PIN_NAME_TWICE_INCLUDED).count(), 1);
        let b = symbol(&it, "$B");
        assert_eq!(b.pins.len(), 1);
        assert_eq!(b.prims.len(), 1);
        assert_eq!(b.prims[0].points, [(4.0, 5.0), (0.0, 0.0)]);
    }

    #[test]
    fn a_second_pin_cannot_take_the_first_pins_name() {
        let files = source("0.4", "$S = @DEFINE-SYMBOL { %p = (2,3) @PIN(terminal=1); %p = (4,5) @PIN(terminal=2); %again = %p; @LINE {%p (0,0)} }");
        let it = read(&files);
        assert_eq!(it.sink.all().iter().filter(|d| d.code == codes::PIN_NAME_TWICE).count(), 1);
        let s = symbol(&it, "$S");
        assert_eq!(s.pins[0].names, ["p", "again"]);
        assert!(s.pins[1].names.is_empty());
        assert_eq!(s.prims[0].points, [(2.0, 3.0), (0.0, 0.0)]);
    }

    #[test]
    fn a_name_conflict_points_to_the_later_name_and_keeps_a_valid_binding() {
        let files = source("0.4", "$S = @DEFINE-SYMBOL { %p = (2,3) @PIN(terminal=1);\n%p = (4,5) @PIN(terminal=2);\n%p = %missing;\n%p = %p; @LINE {%p (0,0)} }");
        let it = read(&files);
        let conflicts: Vec<_> = it.sink.all().iter().filter(|d| d.code == codes::PIN_NAME_TWICE).collect();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].loc.as_ref().unwrap().line, 3);
        assert_eq!(conflicts[0].loc.as_ref().unwrap().col, 1);
        let s = symbol(&it, "$S");
        assert_eq!(s.pins[0].names, ["p"]);
        assert!(s.pins[1].names.is_empty());
        assert_eq!(s.prims[0].points, [(2.0, 3.0), (0.0, 0.0)]);
    }

    #[test]
    fn local_text_has_its_own_result_and_drawing_usages_accept_a_pin_point() {
        let files = source("0.4", "$D = @DEFINE(roles=[[\"at\", \"point\"]]) {}; $DOT = @DEFINE-SYMBOL { (0,0) @CIRCLE(radius=1) }; $S = @DEFINE-SYMBOL { %p = (2,3) @PIN(terminal=1); %t = %p @TEXT(text=\"label\"); %a = %p @AS $D => @SET-SYMBOL $DOT => @ORIENT(\"R90\") }");
        let it = read(&files);
        no_errors(&it);
        let s = symbol(&it, "$S");
        assert_eq!(s.pins[0].names, ["p"]);
        let sym = it.objs.iter().position(|o| o.name == "$S").unwrap();
        let children = it.symbol_children(ObjId(sym as u32));
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].2, (2.0, 3.0));
        assert_eq!(children[0].3, crate::geom::Orient::R90);
        let text = it.objs.iter().find(|o| o.name == "t").unwrap();
        assert!(matches!(text.data, ObjData::Prim { shape: super::super::obj::Shape::Text, .. }));
    }

    #[test]
    fn old_symbols_keep_point_results_and_do_not_acquire_pin_names() {
        for version in ["0.2", "0.3"] {
            let files = source(
                version,
                "$S = @DEFINE-SYMBOL { %p = (2,3) @PIN(terminal=1); %p = (4,5) @PIN(terminal=1); @LINE {%p (0,0)} }",
            );
            let it = read(&files);
            no_errors(&it);
            let s = symbol(&it, "$S");
            assert_eq!(s.pins.len(), 1);
            assert!(s.pins[0].names.is_empty());
            assert_eq!(s.prims[0].points, [(4.0, 5.0), (0.0, 0.0)]);
            assert!(!it.objs.iter().any(|o| matches!(o.data, ObjData::SymbolPin { .. })));
        }
    }
}
