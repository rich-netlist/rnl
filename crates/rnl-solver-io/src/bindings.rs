//! Writable pin references come from the original problem, never from result text.
use crate::json::{Error, Result};
use rnl_runtime::problem::{written_name, FactKind, Filler, LevelProblem, PointRef, Position, Reference, TableCell};
use std::collections::{BTreeMap, BTreeSet};

type Pin = (usize, usize);
#[derive(Default)]
pub(crate) struct Bindings {
    terminals: BTreeMap<Pin, BTreeSet<String>>,
    electrical: BTreeMap<Pin, Vec<Reference>>,
    coordinates: BTreeMap<Pin, Vec<Reference>>,
}

impl Bindings {
    fn electrical(&mut self, pin: Pin, reference: Reference) {
        self.coordinates.entry(pin).or_default().push(reference.then("::position"));
        self.electrical.entry(pin).or_default().push(reference);
    }
    fn point(&mut self, p: &LevelProblem, r: &PointRef) {
        match r {
            PointRef::Pin { object, pin, terminal } => {
                self.terminals.entry((*object, *pin)).or_default().insert(terminal.clone());
                if let Some(o) = p.objects.get(*object) {
                    self.electrical((*object, *pin), o.reference.then(&format!(":{}", written_name(terminal))));
                }
            }
            PointRef::SymbolPin { object, pin, reference } => self.electrical((*object, *pin), reference.clone()),
            PointRef::PinPosition { object, pin, reference } => {
                self.coordinates.entry((*object, *pin)).or_default().push(reference.clone())
            }
            PointRef::Centre(members) => {
                for m in members {
                    self.point(p, m);
                }
            }
            PointRef::Origin(_) => {}
        }
    }
    fn filler(&mut self, p: &LevelProblem, f: &Filler) {
        if let Filler::Terminal { object, terminal, pin: Some(pin) } = f {
            self.point(p, &PointRef::Pin { object: *object, pin: *pin, terminal: terminal.clone() });
        }
    }
    fn table(&mut self, p: &LevelProblem, t: &TableCell<PointRef>) {
        match t {
            TableCell::Point(r) => self.point(p, r),
            TableCell::List(items) => {
                for i in items {
                    self.table(p, i);
                }
            }
        }
    }
    pub fn new(p: &LevelProblem) -> Self {
        let mut out = Self::default();
        for (k, o) in p.objects.iter().enumerate() {
            if let Position::At(r) = &o.position {
                out.point(p, r);
            }
            if let Some(t) = &o.text {
                out.filler(p, &t.attach);
            }
            if let Some(symbol) = o.symbol.and_then(|s| p.symbols.get(s)) {
                for (pin, view) in symbol.pins.iter().enumerate() {
                    if let Some(identity) = &view.identity {
                        for name in &identity.names {
                            out.electrical((k, pin), o.reference.then(&format!("::{}", written_name(name))));
                        }
                    }
                }
            }
        }
        for refs in p
            .nets
            .iter()
            .map(|n| &n.pins)
            .chain(p.routes.iter().map(|r| &r.pins))
            .chain(p.wires.iter().map(|w| &w.anchors))
        {
            for r in refs {
                out.point(p, r);
            }
        }
        for current in &p.currents {
            out.point(p, &current.pin);
        }
        for usage in &p.usages {
            for role in &usage.roles {
                for f in &role.members {
                    out.filler(p, f);
                }
            }
        }
        for f in &p.facts {
            match &f.kind {
                FactKind::Place { at, .. } => out.point(p, at),
                FactKind::Offset { from, .. } => out.point(p, from),
                FactKind::Align { members, .. }
                | FactKind::Order { members, .. }
                | FactKind::Distribute { members, .. } => {
                    for r in members {
                        out.point(p, r);
                    }
                }
                FactKind::Distance { from, to, .. } => {
                    for r in from.iter().chain(to) {
                        out.point(p, r);
                    }
                }
                FactKind::Table { rows, .. } => out.table(p, rows),
                _ => {}
            }
        }
        out
    }

    pub fn validate(&self, r: &PointRef, path: &str) -> Result<()> {
        let accepts = |map: &BTreeMap<Pin, Vec<Reference>>, pin: Pin, reference: &Reference| {
            *reference == Reference::Anonymous || map.get(&pin).is_some_and(|rs| rs.contains(reference))
        };
        let valid = match r {
            PointRef::Pin { object, pin, terminal } => {
                self.terminals.get(&(*object, *pin)).is_some_and(|ts| ts.contains(terminal))
            }
            PointRef::SymbolPin { object, pin, reference } => accepts(&self.electrical, (*object, *pin), reference),
            PointRef::PinPosition { object, pin, reference } => accepts(&self.coordinates, (*object, *pin), reference),
            PointRef::Centre(members) => {
                for (i, m) in members.iter().enumerate() {
                    self.validate(m, &format!("{path}.members[{i}]"))?;
                }
                true
            }
            PointRef::Origin(_) => true,
        };
        if valid {
            Ok(())
        } else {
            Err(Error {
                path: path.into(),
                message: "writable reference does not belong to this typed pin in the original problem".into(),
            })
        }
    }
}
