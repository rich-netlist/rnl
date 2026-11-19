//! The levels reached by selections and the sets that depend on them (10.1, 10.4).
use std::collections::HashSet;
use std::rc::Rc;

use super::ctx::{Ctx, FrameKind};
use super::obj::{Lit, ObjData, ObjId, SceneId, SetKind, UnitId};
use super::unit::{Place, UnitKind};
use super::Interp;

/// Dependency and reach are separate: REMOVE-FROM can introduce a dependency without
/// extending the levels in which the edited set has members.
#[derive(Clone, Copy, Default)]
pub(super) struct SetReach {
    pub selected: bool,
    pub reaches: bool,
}

impl Interp<'_> {
    /// A set edited with a selection remains a rule input, including when the
    /// edit is reached through another set.
    pub(super) fn selected_by_selection_edit(&self, object: ObjId) -> bool {
        if !self.places.enabled {
            return false;
        }
        fn visit(it: &Interp<'_>, object: ObjId, edited: bool, seen: &mut HashSet<(ObjId, bool)>) -> bool {
            if !seen.insert((object, edited)) {
                return false;
            }
            let ObjData::Set(set) = it.o(object) else {
                return false;
            };
            if edited && matches!(set.kind, SetKind::Select { .. } | SetKind::Gen { .. }) {
                return true;
            }
            set.made_of().any(|child| visit(it, child, edited, seen))
                || set.mods.iter().any(|edit| visit(it, edit.source, true, seen))
        }
        visit(self, object, false, &mut HashSet::new())
    }

    pub(super) fn selection_at_home(&mut self, scene: SceneId, writer: UnitId, site: Option<&Ctx>) -> bool {
        if let Some(site) = site.filter(|site| site.frame_kind() == Some(FrameKind::Definition)) {
            return site.body_scene.or(site.scene) == Some(scene);
        }
        let level = self.scenes[scene].level;
        let writer = site.map_or(writer, |site| site.unit);
        if self.units[writer].kind == UnitKind::Level {
            return self.units[writer].level == Some(level);
        }
        let packages = self.effective_packages(level);
        let Some(package) = packages.entry(writer) else { return false };
        match package.place {
            Place::Level(at, _, _) => at == level,
            Place::Root(_, _) => {
                let at = package.decl.as_ref().and_then(|decl| self.units[self.use_writer(decl)].level);
                self.doc.file_level(at.unwrap_or(level)) == level
            }
        }
    }

    pub(super) fn selection_reaches(&mut self, scene: SceneId, object: ObjId) -> bool {
        let SetKind::Select { params, writer, site, .. } = self.set(object).kind.clone() else {
            return true;
        };
        self.selection_at_home(scene, writer, site.as_deref())
            || site.as_ref().is_some_and(|site| site.recursive_selection)
            || params.get("recursive").and_then(Lit::boolean) == Some(true)
    }

    pub(super) fn rule_reaches(&mut self, ctx: &Ctx, inputs: impl IntoIterator<Item = ObjId>) -> bool {
        let Some(scene) = ctx.scene else {
            return true;
        };
        if self.selection_at_home(scene, ctx.unit, Some(ctx)) {
            return true;
        }
        if ctx.written_selection {
            return ctx.recursive_selection;
        }
        let mut selected = false;
        for input in inputs {
            let reach = self.set_reach(scene, input, &mut HashSet::new());
            if reach.reaches {
                return true;
            }
            selected |= reach.selected;
        }
        !selected
    }

    pub(super) fn set_reach(&mut self, scene: SceneId, object: ObjId, seen: &mut HashSet<ObjId>) -> SetReach {
        if !seen.insert(object) {
            return SetReach::default();
        }
        let ObjData::Set(set) = self.o(object) else {
            seen.remove(&object);
            return SetReach::default();
        };
        // Only this scene's edits: a set that several levels edit holds in each level what that
        // level's edits give it (10.3), as `input_items` reads it. The edits of every level read
        // so far would make each level's reading go through those of all the levels before it.
        let edits: Vec<(bool, ObjId, Rc<Ctx>)> = set
            .mods_by_scene
            .get(&Some(scene))
            .into_iter()
            .flatten()
            .map(|&k| (set.mods[k].add, set.mods[k].source, set.mods[k].ctx.clone()))
            .collect();
        let (kind, items, reach_sources, reach_site) =
            (set.kind.clone(), set.items.clone(), set.reach_sources.clone(), set.reach_site.clone());
        #[cfg(test)]
        tests::SET_REACH_EDITS.with(|count| count.set(count.get() + edits.len()));
        let mut result = match kind {
            SetKind::Select { .. } => SetReach { selected: true, reaches: self.selection_reaches(scene, object) },
            SetKind::Union { source } => self.set_reach(scene, source, seen),
            SetKind::Plain | SetKind::Constraints { .. } => {
                let mut result = SetReach::default();
                for (member, _) in items {
                    let child = self.set_reach(scene, member, seen);
                    result.selected |= child.selected;
                    result.reaches |= child.reaches;
                }
                result
            }
            SetKind::Gen { .. } => {
                let reaches = reach_sources.iter().any(|source| self.set_reach(scene, *source, seen).reaches);
                SetReach { selected: true, reaches }
            }
        };
        if let Some(site) = &reach_site {
            let home = self.selection_at_home(scene, site.unit, Some(site));
            if site.written_selection {
                result.reaches = home || site.recursive_selection;
            } else {
                result.reaches |= home;
            }
        }
        for (add, source, ctx) in edits {
            let source = self.set_reach(scene, source, seen);
            result.selected |= source.selected;
            if add {
                result.reaches |= if ctx.written_selection {
                    self.selection_at_home(scene, ctx.unit, Some(&ctx)) || ctx.recursive_selection
                } else {
                    source.reaches
                };
            }
        }
        seen.remove(&object);
        result
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::cell::Cell;

    use crate::diag::{Severity, Sink};
    use crate::host::{Dialect, HostDocument, Instance, Level, LevelKind};
    use crate::records::{package_lines, scan_run};
    use crate::source::MemoryProvider;
    use crate::{interpret, Options};

    thread_local! {
        /// How many edits of sets the reach of a set went through, on this thread.
        pub(in crate::interp) static SET_REACH_EDITS: Cell<usize> = const { Cell::new(0) };
    }

    fn resistor(name: &str, a: &str, b: &str, order: usize) -> Instance {
        Instance {
            name: name.into(),
            letter: Some('R'),
            nodes: vec![a.into(), b.into()],
            value: Some("1k".into()),
            file: "rows.sp".into(),
            line: 1,
            order,
            default_terminals: Some(vec!["p".into(), "n".into()]),
            ..Instance::default()
        }
    }

    /// The edits that the reach of sets goes through while a main file with `n` subcircuits is
    /// read, whose rules, as the standard fill's do, edit a set in every level they reach.
    fn edits(n: usize) -> usize {
        let mut sink = Sink::new();
        let mut main = Level::new(LevelKind::File, "rows.sp", "rows.sp", "rows.sp");
        main.circuit = true;
        main.instances.push(resistor("R1", "n1", "n2", 0));
        for k in 0..n {
            main.instances.push(Instance {
                name: format!("X{k}"),
                letter: Some('X'),
                nodes: vec![format!("a{k}"), format!("b{k}")],
                model: Some(format!("cell{k}")),
                file: "rows.sp".into(),
                line: 2,
                order: k + 1,
                is_subckt: true,
                ..Instance::default()
            });
        }
        let records = r#"@RNL(type="header") { @VERSION("0.4")}
@RNL {
  $TP = @DEFINE(roles=[["t", ["net", "terminal"]]]) {};
  %pins = # @SELECT-NET(recursive=true) => @SELECT-PIN(wired=false, recursive=true);
  # @SELECT-PIN(terminal="n", recursive=true) => @REMOVE-FROM %pins;
  # @SELECT-USAGE(recursive=true) $TP => @SELECT-PIN(role="t", recursive=true) => @REMOVE-FROM %pins;
  $TP @ATTACH %pins
}"#;
        main.records = scan_run(&package_lines(records), "rows.sp", &mut sink);
        let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec![] };
        let mut doc = HostDocument::new(main, dialect);
        for k in 0..n {
            let name = format!("cell{k}");
            let mut level = Level::new(LevelKind::Subckt, &name, "rows.sp", "rows.sp");
            level.circuit = true;
            level.ports = vec!["a".into(), "b".into()];
            level.instances.push(resistor("R1", "a", "m", 0));
            level.instances.push(resistor("R2", "m", "b", 1));
            let id = doc.add_level(level, Some(0));
            doc.subckts.insert(doc.key(&name), id);
        }
        SET_REACH_EDITS.with(|count| count.set(0));
        let read = interpret(doc, &MemoryProvider::new(), &Options { std: false, ..Options::default() }, sink);
        let errors: Vec<_> = read.sink.all().iter().filter(|d| d.severity == Severity::Error).collect();
        assert!(errors.is_empty(), "{errors:?}");
        SET_REACH_EDITS.with(Cell::get)
    }

    #[test]
    fn the_reach_of_a_set_edited_in_every_level_grows_with_the_levels_not_their_square() {
        let (small, large) = (edits(20), edits(40));
        assert!(small > 0, "the rules go through the reach of the set they edit");
        // Twice the levels: the edits of each level's own reading double, those of every level
        // read so far, as before, would grow four times.
        assert!(large <= small * 5 / 2, "{small} edits for 20 levels, {large} for 40");
    }
}
