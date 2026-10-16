//! What the runtime gives a solver (standard 2.8, 8.5, 10.5, 13.7, 14.8, 17): the levels read
//! without the solver's records, the problem of each level, the record written back from a
//! solution, the hash, and the tolerance of the checks.

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, path_str, read_host, FsProvider};
use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::problem::*;
use rnl_runtime::solution::*;
use rnl_runtime::source::{Layered, MemoryProvider, Provider};
use rnl_runtime::writeback::{replace_generated, strip_generated};
use rnl_runtime::Options;

fn case(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases").join(name)
}

/// The cases written for the brainstorm on the solver's algorithms.
fn solver_case(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration/solver").join(name)
}

fn problems(f: &Path, files: &dyn Provider) -> Vec<LevelProblem> {
    interpret_file(f, None, files, &Options::solver()).unwrap().problems
}

fn level<'a>(ps: &'a [LevelProblem], title: &str) -> &'a LevelProblem {
    ps.iter().find(|p| p.title == title).unwrap()
}

fn object(p: &LevelProblem, label: &str) -> usize {
    p.objects.iter().position(|o| o.label == label).unwrap_or_else(|| panic!("no object {label}"))
}

/// The object a generated usage draws, by what fills the first role of the usage it is a local of.
fn generated_local(p: &LevelProblem, root_role: &str, path: &str) -> usize {
    p.objects
        .iter()
        .position(|o| match &o.reference {
            Reference::Generated { usage, path: ps } => {
                ps.concat() == path && p.usages[*usage].roles[0].written.as_deref() == Some(root_role)
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("no {path} of the usage on {root_role}"))
}

fn line_of(f: &ProblemFact) -> u32 {
    f.source.as_ref().map(|l| l.line).unwrap_or(0)
}

const OPTIONS: &str = "tolerance=0.000001";

#[test]
fn generated_records_are_left_out_in_every_file() {
    let fs = FsProvider::new(&[]);
    let f = case("04-hierarchy.sp");
    let with = interpret_file(&f, None, &fs, &Options::default()).unwrap();
    let without = interpret_file(&f, None, &fs, &Options { generated: false, ..Options::default() }).unwrap();
    // the main file, the subcircuit amp in it, and rdiv in the included file all lose their positions
    for title in ["04-hierarchy.sp", "amp", "rdiv"] {
        let a = with.model.levels.iter().find(|l| l.title == title).unwrap();
        let b = without.model.levels.iter().find(|l| l.title == title).unwrap();
        assert!(a.unplaced.is_empty(), "{title}: {:?}", a.unplaced);
        assert!(!b.unplaced.is_empty(), "{title}");
        assert!(b.items.len() < a.items.len(), "{title}");
    }
    let rdiv = without.model.levels.iter().find(|l| l.title == "rdiv").unwrap();
    assert_eq!(rdiv.unplaced, ["#R1", "#R2", "a usage of $IOPORT", "a usage of $IOPORT", "a usage of $IOPORT"]);
}

#[test]
fn generated_records_left_out_report_nothing() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\n* @RNL(type=\"generated\", priority=\"high\") { #R9 @PLACE(0, 0); #R1 @PLACE(1, 2)}\n.end\n";
    let files = MemoryProvider::new().with("t.sp", text);
    let options = Options { std: false, generated: false, ..Options::default() };
    let r = interpret_file(Path::new("t.sp"), None, &files, &options).unwrap();
    assert!(r.diagnostics().iter().all(|d| !d.message.contains("R9")), "{:?}", r.diagnostics());
    assert_eq!(r.model.levels[0].unplaced, ["#R1"]);
    // read as usual, the record places R1 and reports the unknown R9
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    assert!(r.diagnostics().iter().any(|d| d.message.contains("R9")));
    assert_eq!(r.model.levels[0].items[0].at, (1.0, 2.0));
}

#[test]
fn the_tolerance_is_an_option() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\nR2 a 0 1k\n* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(4, 0.001); {#R1 #R2} @ALIGN(\"y\")}\n.end\n";
    let files = MemoryProvider::new().with("t.sp", text);
    let status = |tolerance: f64| {
        let options = Options { std: false, tolerance, ..Options::default() };
        let r = interpret_file(Path::new("t.sp"), None, &files, &options).unwrap();
        r.model.levels[0].constraints[0].status.clone()
    };
    assert_ne!(status(rnl_runtime::geom::TOL), "ok");
    assert_eq!(status(0.01), "ok");
}

#[test]
fn problems_are_built_only_when_asked() {
    let fs = FsProvider::new(&[]);
    let r = interpret_file(&case("01-rc-lowpass.sp"), None, &fs, &Options::default()).unwrap();
    assert!(r.problems.is_empty());
    let r = interpret_file(&case("01-rc-lowpass.sp"), None, &fs, &Options::solver()).unwrap();
    assert_eq!(r.problems.len(), r.model.levels.len());
}

#[test]
fn the_problem_holds_what_a_solver_places() {
    let ps = problems(&case("01-rc-lowpass.sp"), &FsProvider::new(&[]));
    let p = &ps[0];
    assert_eq!((p.title.as_str(), p.kind, p.level), ("01-rc-lowpass.sp", "file", 0));
    // the standard package and its sub-packages, as it brings them in (16.3)
    let std = ["style", "symbols", "virtual", "core", "fill", "semantics", "layout"].map(|n| format!("rnl-std-{n}"));
    let want: Vec<(String, String)> =
        std.into_iter().chain(["rnl-std".to_string()]).map(|n| (n, "0.2".to_string())).collect();
    assert_eq!(p.packages, want);
    assert_eq!(p.names, ["j", "o"]);

    // instances: free, with a symbol whose pins carry the terminals
    let v1 = object(p, "#V1");
    let o = &p.objects[v1];
    assert_eq!((o.kind, &o.reference), (ObjectKind::Instance, &Reference::Name("#V1".into())));
    assert_eq!(o.position, Position::Own { x: Slot::Free, y: Slot::Free });
    assert_eq!(o.orient, Some(OrientSlot::Free));
    let sym = &p.symbols[o.symbol.unwrap()];
    assert_eq!(sym.name, "$VSRC");
    assert_eq!(sym.pins.iter().map(|q| q.terminals.concat()).collect::<Vec<_>>(), ["1", "2"]);

    // a named point with its coordinates, each the rank of the fact that writes it
    let j = object(p, "%j");
    let Position::Own { x: Slot::Fixed { value: 12.0, fact: fx }, y: Slot::Fixed { value: 6.0, fact: fy } } =
        p.objects[j].position
    else {
        panic!("{:?}", p.objects[j].position)
    };
    assert_eq!(p.facts[fx].kind, FactKind::PointCoordinate { point: j, axis: Axis::X, value: 12.0 });
    assert_eq!(p.facts[fy].kind, FactKind::PointCoordinate { point: j, axis: Axis::Y, value: 6.0 });

    // the junction is drawn on %j: no position of its own, and no name to write its orientation
    // with, only its number after the three wires, which a file of 0.2 cannot write (the draft 0.3,
    // 5.9)
    let dot = object(p, "a usage of $JUNCTION");
    assert_eq!(p.objects[dot].position, Position::At(PointRef::Origin(j)));
    assert_eq!(p.objects[dot].reference, Reference::Number { number: 4, path: Vec::new() });
    assert_eq!(p.version, "0.2");
    assert_eq!(p.usages[p.objects[dot].usage.unwrap()].definition, "$JUNCTION");

    // the grounds that the fill of the standard package makes: generated, reached through their stubs
    let g = generated_local(p, "#V1:n", "::g");
    assert!(p.objects[g].generated);
    let Reference::Generated { usage, .. } = &p.objects[g].reference else { unreachable!() };
    let stub = &p.usages[*usage];
    assert_eq!(stub.definition, "$GND_STUB");
    assert!(stub.generated && stub.rule_made && stub.object.is_none());
    assert_eq!(stub.params.len(), 1);
    // its constraints come from the definition body, with the stub in their chain
    let from_stub: Vec<&ProblemFact> = p.facts.iter().filter(|f| f.chain == [*usage]).collect();
    assert_eq!(from_stub.iter().map(|f| f.kind.name()).collect::<Vec<_>>(), ["DISTANCE", "ALIGN"]);
    // written in the body of $GND_STUB, which is in the sub-package of the virtual elements (16.3)
    let source = from_stub[0].source.as_ref().unwrap();
    assert!(source.file.ends_with("rnl-std-virtual.rnl"), "{source:?}");

    // texts: by the symbol, or free with offsets from the point they are attached to
    let name = object(p, "#R1::name");
    assert!(matches!(p.objects[name].position, Position::InSymbol { owner, .. } if owner == object(p, "#R1")));
    assert_eq!(p.objects[name].reference, Reference::Name("#R1::name".into()));
    let o_pt = object(p, "%o");
    let vout =
        p.objects.iter().position(|o| o.text.as_ref().is_some_and(|t| t.attach == Filler::Object(o_pt))).unwrap();
    assert_eq!(p.objects[vout].position, Position::Own { x: Slot::Free, y: Slot::Free });
    let offsets: Vec<&FactKind> = p
        .facts
        .iter()
        .map(|f| &f.kind)
        .filter(|k| matches!(k, FactKind::Offset { text, .. } if *text == vout))
        .collect();
    assert_eq!(
        offsets,
        [
            &FactKind::Offset { text: vout, from: PointRef::Origin(o_pt), axis: Axis::Y, value: 0.8 },
            &FactKind::Offset { text: vout, from: PointRef::Origin(o_pt), axis: Axis::X, value: 0.4 },
        ]
    );

    // wires: the user's, and the generated ones of the stubs
    assert_eq!(p.wires.len(), 5);
    assert_eq!(p.wires.iter().filter(|w| w.generated).count(), 2);
    assert_eq!(p.wires[1].anchors[1], PointRef::Origin(j));
}

#[test]
fn facts_are_in_the_order_of_8_5() {
    let ps = problems(&solver_case("T4-miller-conflicts.sp"), &FsProvider::new(&[]));
    let p = &ps[0];
    // priority first
    assert!(p.facts.windows(2).all(|w| w[0].priority >= w[1].priority));
    let m1 = object(p, "#M1");
    assert_eq!(p.facts[0].kind, FactKind::Place { at: PointRef::Origin(m1), axis: Axis::Y, value: 0.0 });
    assert_eq!(p.facts[1].kind, FactKind::Place { at: PointRef::Origin(m1), axis: Axis::X, value: 0.0 });
    assert!(p.facts[..2].iter().all(|f| f.priority == Priority::High));
    // then the later first: the second record before the first, each from its end
    let direct: Vec<u32> =
        p.facts.iter().filter(|f| f.priority == Priority::Normal && f.chain.is_empty()).map(line_of).collect();
    assert_eq!(direct, [40, 39, 38, 30, 29, 28, 27]);
    // the expansions take the place of their AS, after the statements written later; the fill
    // of the standard package comes last of all
    let first_expanded = p.facts.iter().position(|f| !f.chain.is_empty()).unwrap();
    assert_eq!(p.usages[p.facts[first_expanded].chain[0]].definition, "$TAIL_CENTER");
    let n = p.facts.len();
    assert_eq!((p.facts[n - 2].kind.name(), p.facts[n - 2].priority), ("ORDER", Priority::Low));
    assert_eq!(p.usages[p.facts[n - 2].chain[0]].definition, "$FLOW");
    assert_eq!((p.facts[n - 1].kind.name(), line_of(&p.facts[n - 1])), ("ALIGN", 31));
    // a set given to DISTANCE counts by its extent, one given to ORDER by its centre
    let stages = p.facts.iter().find(|f| line_of(f) == 27).unwrap();
    let FactKind::Distance { axis: Axis::X, sense: Sense::Increasing, min: Some(3.0), max: None, from, to } =
        &stages.kind
    else {
        panic!("{:?}", stages.kind)
    };
    assert_eq!((from.len(), to.len()), (5, 3));
    let FactKind::Order { members, .. } = &p.facts[n - 2].kind else { unreachable!() };
    assert!(matches!(&members[0], PointRef::Centre(c) if c.len() == 2));
    // ORIENT-RELATION of the mirror pairs, and the groups
    assert_eq!(p.facts.iter().filter(|f| f.kind.name() == "ORIENT-RELATION").count(), 2);
    assert_eq!(
        p.groups.iter().map(|g| (g.label.as_str(), g.members.len())).collect::<Vec<_>>(),
        [("%stage1", 5), ("%stage2", 3)]
    );
    // the slots: the first coordinate fact of each axis in this order
    let m2 = object(p, "#M2");
    assert_eq!(
        p.objects[m1].position,
        Position::Own { x: Slot::Fixed { value: 0.0, fact: 1 }, y: Slot::Fixed { value: 0.0, fact: 0 } }
    );
    assert!(matches!(p.objects[m2].position, Position::Own { x: Slot::Free, y: Slot::Fixed { value: 3.0, .. } }));
    // the free axis point of the mirror pairs is named, and so is the middle point of the tail,
    // through the name of its usage
    assert_eq!(p.objects[object(p, "%ax")].reference, Reference::Name("%ax".into()));
    assert_eq!(p.objects[object(p, "%mid")].reference, Reference::Name("%tc::mid".into()));
    // without that name it is reached only through the number of its usage, the third without a
    // name (the draft 0.3, 5.9)
    let f = solver_case("T4-miller-conflicts.sp");
    let fs = FsProvider::new(&[]);
    let text = fs.read(&path_str(&f)).unwrap().replace("%tc = #M1(a)", "#M1(a)");
    let unnamed = MemoryProvider::new().with(&path_str(&f), &text);
    let ps = problems(&f, &Layered { layers: vec![&unnamed, &fs] });
    let p = &ps[0];
    assert_eq!(p.objects[object(p, "%mid")].reference, Reference::Number { number: 3, path: vec!["::mid".into()] });
}

#[test]
fn the_later_segment_of_a_pipe_comes_first() {
    // the facts of the segments of one pipe follow the order of the text, the later segment first,
    // before what a segment adds: its members, its nested statements, the axes of a PLACE (8.5, as
    // the draft 0.3 makes it clear, in every version)
    let text = "* pipes\nR1 a b 1k\nR2 c d 1k\nR3 e f 1k\nR4 g h 1k\n\
        * @RNL(type=\"header\") { @VERSION(\"0.2\")}\n\
        * @RNL {\n\
        *   $TALL = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (0, 10) @PIN(terminal=2) };\n\
        *   {#R1 #R2 #R3 #R4} @SET-SYMBOL $TALL;\n\
        *   #R1 @PLACE(0, 0) => @PLACE(5, 5);\n\
        *   {#R4 @PLACE(40, 0)} => @PLACE(45, 5);\n\
        *   #R2 @PLACE(20, 0) => @ORIENT(\"R90\");\n\
        *   %row = {#R2:2 #R3} @ALIGN(\"y\");\n\
        *   #R3 @PLACE(30, 10)\n\
        * }\n.end\n";
    let files = MemoryProvider::new().with("p.sp", text);
    let r = interpret_file(Path::new("p.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error));
    let p = &r.problems[0];
    let labels: Vec<&str> = p.facts.iter().map(|f| f.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "the y of #R3 = 10",
            "the x of #R3 = 30",
            "%row",
            "the orientation R90 of #R2",
            "the y of #R2 = 0",
            "the x of #R2 = 20",
            "the y of #R4 = 5",
            "the x of #R4 = 45",
            "the y of #R4 = 0",
            "the x of #R4 = 40",
            "the y of #R1 = 5",
            "the x of #R1 = 5",
            "the y of #R1 = 0",
            "the x of #R1 = 0",
        ]
    );
    // pin 2 of R2 is 10 above its origin, and the row written later puts it at y = 10. Taken in
    // after the row, the ORIENT of the later segment turns the pin 10 to the left, so the row holds
    // R2 at y = 10 and the PLACE of the earlier segment loses its y = 0, keeping x = 20. Had the
    // PLACE come first, "R0" would have kept it, and the ORIENT would have been dropped.
    let FactKind::Align { members, .. } = &p.facts[2].kind else { panic!("{:?}", p.facts[2].kind) };
    let r2 = object(p, "#R2");
    let pin = |xy: Pt, orient: Orient| pin_at(p, &[(r2, xy, orient)], &members[0]);
    assert_eq!(pin((20.0, 10.0), Orient::R90), (10.0, 10.0));
    assert_eq!(pin((20.0, 0.0), Orient::R90).1, 0.0);
    assert_eq!(pin((20.0, 0.0), Orient::R0).1, 10.0);
    // a viewer keeps the same one of two coordinates of an object: R4 is where the later segment
    // puts it, not the statement nested in the earlier one
    let drawn = |id: &str| r.model.levels[0].items.iter().find(|i| i.id == id).map(|i| i.at).unwrap();
    assert_eq!(drawn("#R1"), (5.0, 5.0));
    assert_eq!(drawn("#R4"), (45.0, 5.0));
}

#[test]
fn disabled_facts_are_left_out() {
    let ps = problems(&case("10-constraints-and-priority.sp"), &FsProvider::new(&[]));
    let p = &ps[0];
    // %c4 is disabled, and so is the row of $ROW by a selection; R3's PLACE has priority="disabled"
    assert!(p.facts.iter().all(|f| f.label != "%c4" && f.label != "%row"));
    assert_eq!(p.objects[object(p, "#R3")].position, Position::Own { x: Slot::Free, y: Slot::Free });
    // a PRIORITY on a local from outside raises it over its usage's low
    let ord = p.facts.iter().find(|f| f.label == "%ord").unwrap();
    assert_eq!((ord.priority, &ord.reference), (Priority::High, &Some(Reference::Name("%r::ord".into()))));
    // the editor's record is read: R4's y is high, ahead of every normal fact
    let r4 = object(p, "#R4");
    let Position::Own { y: Slot::Fixed { value, fact }, .. } = p.objects[r4].position else { panic!() };
    assert_eq!((value, p.facts[fact].priority), (-2.0, Priority::High));
    // the record head's priority="low" reaches %c7
    assert_eq!(p.facts.iter().find(|f| f.label == "%c7").unwrap().priority, Priority::Low);
}

#[test]
fn every_level_has_its_problem() {
    let ps = problems(&case("04-hierarchy.sp"), &FsProvider::new(&[]));
    assert_eq!(ps.len(), 3);
    let rdiv = level(&ps, "rdiv");
    assert!(rdiv.file.ends_with("04-hierarchy-lib.inc") && rdiv.end_line > rdiv.line);
    let ports: Vec<&UsageEntry> = rdiv.usages.iter().filter(|u| u.definition == "$IOPORT").collect();
    assert_eq!(ports.len(), 3);
    assert!(ports.iter().all(|u| u.rule_made && u.object.is_some()));
    assert_eq!(ports.iter().map(|u| u.roles[0].written.clone().unwrap()).collect::<Vec<_>>(), ["&top", "&mid", "&bot"]);
    let amp = level(&ps, "amp");
    assert!(amp.packages.iter().any(|(n, _)| n == "iec-resistor"));
    assert_eq!(amp.objects[object(amp, "%pin")].reference, Reference::Name("%pin".into()));
    // written into amp from outside with scope=, the text of R1 replaces the symbol's
    let r1 = &amp.objects[object(amp, "#R1::name")];
    assert_eq!(rnl_runtime::markup::plain(&r1.text.as_ref().unwrap().lines[0]), "RL");
}

#[test]
fn routes_give_their_pins_and_wires() {
    let ps = problems(&case("14-route.sp"), &FsProvider::new(&[]));
    let p = &ps[0];
    let nets: Vec<&str> = p.routes.iter().map(|r| r.net.as_str()).collect();
    assert_eq!(nets, ["c", "b", "e", "in", "out"]);
    let out = p.routes.iter().find(|r| r.net == "out").unwrap();
    assert_eq!(out.wires, [0]);
    assert_eq!(out.pins.len(), 2);
    assert!(out.pins.iter().all(|q| matches!(q, PointRef::Pin { .. })));
    // an annotation placed by offsets from its point
    let dx: Vec<f64> = p
        .facts
        .iter()
        .filter_map(|f| match f.kind {
            FactKind::Offset { axis: Axis::X, value, .. } => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(dx, [-0.5]);
}

/// Where a pin of a placed object is: its position plus the pin turned with it (9.2.4, 11.2).
fn pin_at(p: &LevelProblem, at: &[(usize, Pt, Orient)], q: &PointRef) -> Pt {
    let PointRef::Pin { object, pin, .. } = q else { panic!("{q:?}") };
    let (_, xy, orient) = at.iter().find(|(o, _, _)| o == object).unwrap();
    let d = orient.apply(p.symbols[p.objects[*object].symbol.unwrap()].pins[*pin].pos);
    (xy.0 + d.0, xy.1 + d.1)
}

/// Write a solution back into a file of the corpus and read it again: the file text, and the
/// interpretation of it.
fn write_back(f: &Path, p: &LevelProblem, written: &Written) -> (String, rnl_runtime::Interpretation) {
    let fs = FsProvider::new(&[]);
    let path = path_str(f);
    let text = fs.read(&path).unwrap();
    let doc = read_host(f, None, &fs, &mut Sink::new()).unwrap();
    let out = replace_generated(&doc, p.level, &text, &written.record);
    let mem = MemoryProvider::new().with(&path, &out);
    let both = Layered { layers: vec![&mem, &fs] };
    let r = interpret_file(f, None, &both, &Options::default()).unwrap();
    (out, r)
}

#[test]
fn a_solution_written_back_reads_back() {
    let f = case("01-rc-lowpass.sp");
    let p = &problems(&f, &FsProvider::new(&[]))[0];
    let place = |object: usize, x: f64, y: f64, orient: Option<Orient>| Placement { object, at: Some((x, y)), orient };
    let solution = Solution {
        placements: vec![
            place(object(p, "#C1"), 12.0, 3.0, None),
            place(object(p, "#V1"), 0.0, 0.0, Some(Orient::R0)),
            place(object(p, "#R1"), 6.0, 6.0, Some(Orient::R90)),
            place(generated_local(p, "#V1:n", "::g"), 0.0, -5.0, None),
            place(generated_local(p, "#C1:n", "::g"), 12.0, -1.0, None),
            // the junction has no position of its own, and in a file of 0.2 no name to write its
            // orientation with
            place(object(p, "a usage of $JUNCTION"), 1.0, 1.0, Some(Orient::R90)),
        ],
        ..Solution::default()
    };
    let hash = sig1(p, OPTIONS);
    let written = write_record(p, &solution, "rnl-solver test", &hash);
    // neither is left out silently (the draft 0.3, 14.8)
    let unsolved: Vec<String> = written.unsolved.iter().map(|u| u.to_string()).collect();
    assert_eq!(
        unsolved,
        [
            "the position of a usage of $JUNCTION: it is drawn at the point of its main object and has no position of its own (12.1)",
            "the orientation of a usage of $JUNCTION: it has no name, and a file of 0.2 cannot refer to it; the draft 0.3 refers to it by its number (5.9)",
        ]
    );
    assert!(written.notes.is_empty());
    assert_eq!(
        written.record,
        format!(
            "@RNL(type=\"generated\", priority=\"high\", author=\"rnl-solver test\", hash=\"{hash}\") {{
  %gnd_stub_V1_n = $GND_STUB @ATTACH(gap=2, priority=\"normal\") #V1:n;
  %gnd_stub_C1_n = $GND_STUB @ATTACH(gap=2, priority=\"normal\") #C1:n;
  #V1 @PLACE(0, 0) => @ORIENT(\"R0\");
  #R1 @PLACE(6, 6) => @ORIENT(\"R90\");
  #C1 @PLACE(12, 3);
  %gnd_stub_V1_n::g @PLACE(0, -5);
  %gnd_stub_C1_n::g @PLACE(12, -1)
}}"
        )
    );
    let (text, r) = write_back(&f, p, &written);
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let lv = &r.model.levels[0];
    assert!(lv.unplaced.is_empty(), "{:?}", lv.unplaced);
    let r1 = lv.items.iter().find(|i| i.id == "#R1").unwrap();
    assert_eq!((r1.at, r1.orient), ((6.0, 6.0), Orient::R90));
    let grounds: Vec<Pt> = lv.items.iter().filter(|i| i.kind == "usage" && i.id == "%g").map(|i| i.at).collect();
    assert_eq!(grounds, [(0.0, -5.0), (12.0, -1.0)]);
    // the stubs written out take the place of the generated ones: no third ground
    assert_eq!(lv.items.iter().filter(|i| i.id == "%g").count(), 2);

    // read as a solver reads it, the file has the same input: the hash still holds
    let mem = MemoryProvider::new().with(&path_str(&f), &text);
    let fs = FsProvider::new(&[]);
    let again = problems(&f, &Layered { layers: vec![&mem, &fs] });
    assert_eq!(same_input(&hash, &again[0]), Some(true));
    assert_eq!(again[0].end_line, p.end_line + 2);
    assert_eq!(again[0].objects, p.objects);
    assert_eq!(again[0].usages, p.usages);
}

#[test]
fn a_routed_net_written_back_reads_back() {
    let f = solver_case("T1-rc-lowpass.sp");
    let p = &problems(&f, &FsProvider::new(&[]))[0];
    let (vin, r1, c1) = (object(p, "#VIN"), object(p, "#R1"), object(p, "#C1"));
    let (g1, g2) = (generated_local(p, "#VIN:n", "::g"), generated_local(p, "#C1:n", "::g"));
    let at = [
        (vin, (0.0, 0.0), Orient::R0),
        (r1, (6.0, 6.0), Orient::R90),
        (c1, (12.0, 3.0), Orient::R0),
        (g1, (0.0, -5.0), Orient::R0),
        (g2, (12.0, -1.0), Orient::R0),
    ];
    let placements = at.iter().map(|(o, xy, r)| Placement { object: *o, at: Some(*xy), orient: Some(*r) }).collect();
    let route = |net: &str| p.routes.iter().position(|r| r.net == net).unwrap();
    // the pins each ROUTE joins
    let pin = |object: usize, terminal: &str| -> PointRef {
        p.routes
            .iter()
            .flat_map(|r| &r.pins)
            .find(|q| matches!(q, PointRef::Pin { object: o, terminal: t, .. } if *o == object && t == terminal))
            .unwrap()
            .clone()
    };
    let (vp, rp) = (pin(vin, "p"), pin(r1, "p"));
    let (rn, cp) = (pin(r1, "n"), pin(c1, "p"));
    let (vp_at, rp_at, rn_at, cp_at) =
        (pin_at(p, &at, &vp), pin_at(p, &at, &rp), pin_at(p, &at, &rn), pin_at(p, &at, &cp));
    let solution = Solution {
        placements,
        branches: vec![Branch { at: (cp_at.0, rn_at.1), junction: true }],
        wires: vec![
            RoutedWire {
                route: route("in"),
                points: vec![WirePoint::Anchor(vp), WirePoint::Bend((vp_at.0, rp_at.1)), WirePoint::Anchor(rp)],
            },
            RoutedWire { route: route("out"), points: vec![WirePoint::Anchor(rn), WirePoint::Branch(0)] },
            RoutedWire { route: route("out"), points: vec![WirePoint::Branch(0), WirePoint::Anchor(cp)] },
        ],
        ..Solution::default()
    };
    let written = write_record(p, &solution, "rnl-solver test", &sig1(p, OPTIONS));
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.record.contains("&in @WIRE(mode=\"straight\") {#VIN:p (0, 6) #R1:p}"), "{}", written.record);
    assert!(written.record.contains("%rt1 = (12, 6)") && written.record.contains("$JUNCTION @ATTACH %rt1"));
    let (_, r) = write_back(&f, p, &written);
    let lv = &r.model.levels[0];
    assert!(lv.unplaced.is_empty(), "{:?}", lv.unplaced);
    // the ROUTEs are wired now: no flight lines are left
    assert!(lv.flylines.is_empty(), "{:?}", lv.flylines);
    let straight: Vec<&Vec<Pt>> = lv.wires.iter().filter(|w| w.mode == "straight").map(|w| &w.points).collect();
    assert_eq!(
        straight,
        [&vec![vp_at, (vp_at.0, rp_at.1), rp_at], &vec![rn_at, (12.0, 6.0)], &vec![(12.0, 6.0), cp_at]]
    );
    assert!(lv.items.iter().any(|i| i.kind == "usage" && i.at == (12.0, 6.0)));
}

/// The input digests of every level of a text, read as a solver reads it.
fn digests(f: &Path, text: &str) -> Vec<(String, String)> {
    let mem = MemoryProvider::new().with(&path_str(f), text);
    let fs = FsProvider::new(&[]);
    problems(f, &Layered { layers: vec![&mem, &fs] }).iter().map(|p| (p.title.clone(), input_digest(p))).collect()
}

#[test]
fn the_hash_follows_the_input_and_nothing_else() {
    let f = case("10-constraints-and-priority.sp");
    let text = FsProvider::new(&[]).read(&path_str(&f)).unwrap();
    let base = digests(&f, &text);
    assert_eq!(digests(&f, &text), base, "the same input gives the same hash");

    // white space, comments and lines moved down do not change it
    let spaced = text
        .replace("%c1 = {#R1 #R2 #R3} @ALIGN(\"y\");", "%c1  =  { #R1  #R2 #R3 }   @ALIGN(\"y\") /* a row */ ;")
        .replacen("R1 a b 1k", "* a comment\nR1 a b 1k", 1);
    assert_ne!(spaced, text);
    assert_eq!(digests(&f, &spaced), base);

    // the generated records and the metadata are not part of it
    let doc = read_host(&f, None, &FsProvider::new(&[]), &mut Sink::new()).unwrap();
    let stripped = strip_generated(&doc, &doc.main().file, &text);
    assert_eq!(digests(&f, &stripped), base);
    let moved = text.replace("#R1 @PLACE(0, 0) => @ORIENT(\"R90\");", "#R1 @PLACE(5, 5) => @ORIENT(\"R180\");");
    assert_ne!(moved, text);
    assert_eq!(digests(&f, &moved), base);
    let author = text.replace("author=\"rnl-editor 0.1\"", "author=\"rnl-editor 0.2\", modified=\"2026-10-15\"");
    assert_ne!(author, text);
    assert_eq!(digests(&f, &author), base);

    // a constraint, a priority or a position written by the user does
    for changed in [
        text.replace("%c1 = {#R1 #R2 #R3} @ALIGN(\"y\");", "%c1 = {#R1 #R2 #R3} @ALIGN(\"x\");"),
        text.replace("%c4 @PRIORITY(\"disabled\");", "%c4 @PRIORITY(\"low\");"),
        text.replace("#R2 @PLACE(7, 0);", "#R2 @PLACE(7, 1);"),
    ] {
        assert_ne!(changed, text);
        assert_ne!(digests(&f, &changed), base);
    }

    // the options are a segment of their own, which a viewer does not compare
    let p = &problems(&f, &FsProvider::new(&[]))[0];
    let (a, b) = (sig1(p, OPTIONS), sig1(p, "tolerance=0.01"));
    assert_eq!(a.len(), "sig1:".len() + 33);
    assert_eq!(a[..21], b[..21]);
    assert_ne!(a[22..], b[22..]);
    assert_eq!(same_input(&b, p), Some(true));
}

#[test]
fn a_change_above_changes_the_hash_below() {
    // the rules that flow down are part of a subcircuit's input (17); the level that writes the
    // rule draws no resistor, so its own input stays
    let f = case("04-hierarchy.sp");
    let text = FsProvider::new(&[]).read(&path_str(&f)).unwrap();
    let base = digests(&f, &text);
    let changed = text.replace(
        "# @SELECT-INSTANCE(name=\"R*\") => @SET-STYLE $grey;",
        "# @SELECT-INSTANCE(name=\"R*\") => @ORIENT(\"R90\");",
    );
    assert_ne!(changed, text);
    let after = digests(&f, &changed);
    for (before, now) in base.iter().zip(&after) {
        assert_eq!(before.0, now.0);
        assert_eq!(before.1 == now.1, before.0 == "04-hierarchy.sp", "{}", before.0);
    }
}

#[test]
fn the_brainstorm_cases_give_whole_problems() {
    let fs = FsProvider::new(&[]);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/integration/solver");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sp"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 7);
    for f in &files {
        let ps = problems(f, &fs);
        assert!(!ps.is_empty(), "{}", f.display());
        // the same input gives the same problem
        assert_eq!(problems(f, &fs), ps, "{}", f.display());
        for p in &ps {
            let n = p.objects.len();
            let ok = |r: &PointRef| -> bool {
                fn check(r: &PointRef, n: usize) -> bool {
                    match r {
                        PointRef::Origin(k) | PointRef::Pin { object: k, .. } => *k < n,
                        PointRef::Centre(rs) => rs.iter().all(|r| check(r, n)),
                    }
                }
                check(r, n)
            };
            for fact in &p.facts {
                assert!(fact.chain.iter().all(|u| *u < p.usages.len()));
                match &fact.kind {
                    FactKind::Align { members, .. }
                    | FactKind::Order { members, .. }
                    | FactKind::Distribute { members, .. } => {
                        assert!(members.iter().all(ok))
                    }
                    FactKind::Distance { from, to, .. } => assert!(from.iter().chain(to).all(ok)),
                    FactKind::Place { at, .. } => assert!(ok(at)),
                    _ => {}
                }
            }
            for w in &p.wires {
                assert!(w.anchors.iter().all(ok));
            }
            for r in &p.routes {
                assert!(r.pins.iter().all(ok));
                assert!(r.pins.len() >= 2, "{}: {}", f.display(), r.net);
            }
            for o in &p.objects {
                assert!(o.symbol.is_none_or(|s| s < p.symbols.len()));
            }
        }
    }
    // T2b: the solver's record is not read, the editor's drag after it is
    let ps = problems(&solver_case("T2b-common-source-dragged.sp"), &fs);
    let p = &ps[0];
    assert_eq!(p.objects[object(p, "#M1")].position, Position::Own { x: Slot::Free, y: Slot::Free });
    let vin = &p.objects[object(p, "#VIN")];
    let Position::Own { x: Slot::Fixed { value: -4.0, fact }, y: Slot::Free } = vin.position else {
        panic!("{:?}", vin.position)
    };
    assert_eq!((fact, p.facts[fact].priority), (0, Priority::High));
}

#[test]
fn rnl_problem_gives_a_file_of_0_2_no_numbers() {
    // the numbers of the objects without a name are computed in every version, but only a file of
    // the draft 0.3 writes them (5.9): `rnl problem` gives a file of 0.2 no number, and the draft
    // switch, kept for compatibility, changes nothing
    let fs = FsProvider::new(&[]);
    let mut files = Vec::new();
    for dir in [case(""), solver_case("")] {
        for e in std::fs::read_dir(dir).unwrap() {
            let f = e.unwrap().path();
            if f.extension().is_some_and(|x| x == "sp" || x == "scs") {
                files.push(f);
            }
        }
    }
    files.sort();
    assert!(files.len() > 20);
    for f in files {
        let json = |options: &Options| -> Vec<serde_json::Value> {
            let r = interpret_file(&f, None, &fs, options).unwrap();
            r.problems.iter().map(|p| rnl_cli::problem::level(p, OPTIONS)).collect()
        };
        let levels = json(&Options::solver());
        for level in &levels {
            assert!(level.get("authored_numbers").is_none(), "{}", f.display());
            assert!(!has_key(level, "number"), "{}", f.display());
        }
        assert_eq!(json(&Options { draft: true, ..Options::solver() }), levels, "{}", f.display());
    }
}

/// Whether an object in `v` has the key `key`.
fn has_key(v: &serde_json::Value, key: &str) -> bool {
    match v {
        serde_json::Value::Object(m) => m.contains_key(key) || m.values().any(|x| has_key(x, key)),
        serde_json::Value::Array(a) => a.iter().any(|x| has_key(x, key)),
        _ => false,
    }
}

#[test]
fn a_usage_has_the_place_it_is_written() {
    // where each usage is written (UsageEntry::source), so that the hints of 8.5 can name it by its
    // line: the AS of the level for the mirror pairs, the rule of the standard package for a stub
    // that it makes, and the body of $GND_STUB for the ground in it
    let ps = problems(&solver_case("T4-miller-conflicts.sp"), &FsProvider::new(&[]));
    let p = &ps[0];
    let at = |u: &UsageEntry| u.source.as_ref().map(|l| (l.file.clone(), l.line)).unwrap();
    let pairs: Vec<(String, u32)> = p.usages.iter().filter(|u| u.definition == "$MIRROR_PAIR").map(at).collect();
    assert_eq!(pairs, [("T4-miller-conflicts.sp".to_string(), 22), ("T4-miller-conflicts.sp".to_string(), 23)]);
    let stub = p.usages.iter().position(|u| u.definition == "$GND_STUB" && u.rule_made).unwrap();
    assert_eq!(at(&p.usages[stub]).0, "rnl-std-fill.rnl");
    let ground = p.usages.iter().find(|u| u.parent == Some(stub)).unwrap();
    assert_eq!((ground.definition.as_str(), at(ground).0.as_str()), ("$GND", "rnl-std-virtual.rnl"));
    // like every place, it is left out of the hash: the test of the hash moves lines down
}
