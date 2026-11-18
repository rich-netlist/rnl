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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases-0.4/from-0.2").join(name)
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
    assert_eq!(
        rdiv.unplaced,
        ["#R1", "#R2", "%l", "%l", "a usage of $IO_PORT", "a usage of $IO_PORT", "a usage of $IO_PORT"]
    );
}

#[test]
fn generated_records_left_out_report_nothing() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\nR1 a 0 1k\n* @RNL(type=\"generated\", priority=\"high\") { #R9 @PLACE(0, 0); #R1 @PLACE(1, 2)}\n.end\n";
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
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\nR1 a 0 1k\nR2 a 0 1k\n* @RNL { #R1 @PLACE(0, 0); #R2 @PLACE(4, 0.001); {#R1 #R2} @ALIGN(\"y\")}\n.end\n";
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
    // the packages of the standard series that its header uses, and those they use, as it brings
    // them in (16.3)
    let std = [
        "style",
        "basic",
        "semiconductors",
        "fill",
        "semantics",
        "functional",
        "rf",
        "logic",
        "signal-flow",
        "spice",
        "spectre",
        "all",
    ];
    let want: Vec<(String, String)> = std.iter().map(|n| (format!("rnl-std-{n}"), "0.4".to_string())).collect();
    assert_eq!(p.packages, want);
    assert_eq!(p.names, ["input_corner", "j", "o"]);

    // instances: placed by the record of priority high written by hand, each fact the rank of the
    // fact that writes it, with a symbol whose pins carry the terminals
    let v1 = object(p, "#V1");
    let o = &p.objects[v1];
    assert_eq!((o.kind, &o.reference), (ObjectKind::Instance, &Reference::Name("#V1".into())));
    let Position::Own { x: Slot::Fixed { value: 0.0, fact: fx }, y: Slot::Fixed { value: 0.0, fact: fy } } = o.position
    else {
        panic!("{:?}", o.position)
    };
    assert_eq!(p.facts[fx].kind, FactKind::Place { at: PointRef::Origin(v1), axis: Axis::X, value: 0.0 });
    assert_eq!(p.facts[fy].kind, FactKind::Place { at: PointRef::Origin(v1), axis: Axis::Y, value: 0.0 });
    let sym = &p.symbols[o.symbol.unwrap()];
    assert!(matches!(o.orient, Some(OrientSlot::Fixed { code: Orient::R0, .. })), "{:?}", o.orient);
    assert_eq!(sym.name, "$VOLTAGE_SOURCE");
    assert_eq!(sym.pins.iter().map(|q| q.terminals.concat()).collect::<Vec<_>>(), ["p", "n"]);

    // a named point with its coordinates, each the rank of the fact that writes it
    let j = object(p, "%j");
    let Position::Own { x: Slot::Fixed { value: 12.0, fact: fx }, y: Slot::Fixed { value: 6.0, fact: fy } } =
        p.objects[j].position
    else {
        panic!("{:?}", p.objects[j].position)
    };
    assert_eq!(p.facts[fx].kind, FactKind::PointCoordinate { point: j, axis: Axis::X, value: 12.0 });
    assert_eq!(p.facts[fy].kind, FactKind::PointCoordinate { point: j, axis: Axis::Y, value: 6.0 });

    // the junction attached to %j: no name, only its number after the three wires (5.9), and a
    // position of its own, which the ALIGNs of the body of $JUNCTION put on %j (12.1)
    let dot = object(p, "a usage of $JUNCTION");
    assert_eq!(p.objects[dot].position, Position::Own { x: Slot::Free, y: Slot::Free });
    assert_eq!(p.objects[dot].reference, Reference::Number { number: 4, path: Vec::new() });
    assert_eq!(p.version, "0.4");
    let junction = p.objects[dot].usage.unwrap();
    assert_eq!(p.usages[junction].definition, "$JUNCTION");
    let on_j: Vec<&FactKind> = p.facts.iter().filter(|f| f.chain == [junction]).map(|f| &f.kind).collect();
    let members = vec![PointRef::Origin(dot), PointRef::Origin(j)];
    assert_eq!(
        on_j,
        [&FactKind::Align { axis: Axis::Y, members: members.clone() }, &FactKind::Align { axis: Axis::X, members }]
    );

    // the grounds that the fill of the standard package makes: generated, reached through their stubs
    let g = generated_local(p, "#V1::n", "::g");
    assert!(p.objects[g].generated);
    let Reference::Generated { usage, .. } = &p.objects[g].reference else { unreachable!() };
    let stub = &p.usages[*usage];
    assert_eq!(stub.definition, "$GROUND_STUB");
    assert!(stub.generated && stub.rule_made && stub.object.is_some());
    assert_eq!(stub.params.len(), 1);
    // its constraints come from the definition body, with the stub in their chain
    let from_stub: Vec<&ProblemFact> = p.facts.iter().filter(|f| f.chain == [*usage]).collect();
    assert_eq!(from_stub.iter().map(|f| f.kind.name()).collect::<Vec<_>>(), ["ORIENT", "DISTANCE"]);
    // written in the body of $GROUND_STUB, which is in the package of the basic elements (16.3)
    let source = from_stub[1].source.as_ref().unwrap();
    assert!(source.file.ends_with("rnl-std-basic.rnl"), "{source:?}");

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
    // then the later first: the second record before the first, each from its end; the DISTANCE
    // of two groups of line 41 is one fact for each pair of their members, the ORIENT of line 37 one
    // for each object, and the ORDER of line 36 one for each choice of a member of each of its
    // usages
    let direct: Vec<u32> =
        p.facts.iter().filter(|f| f.priority == Priority::Normal && f.chain.is_empty()).map(line_of).collect();
    let mut expected = vec![51, 50, 49, 44, 43, 42];
    expected.extend([41; 15]);
    expected.push(40);
    expected.extend([37; 5]);
    expected.extend([36; 30]);
    assert_eq!(direct, expected);
    let stages: Vec<_> = p.facts.iter().filter(|f| line_of(f) == 41).collect();
    assert!(
        stages.iter().all(|f| matches!(&f.kind,
        FactKind::Distance { axis: Axis::X, sense: Sense::Increasing, min: Some(3.0), max: None, from, to }
            if from.len() == 1 && to.len() == 1)),
        "{stages:?}"
    );
    // the one low statement
    let low: Vec<u32> =
        p.facts.iter().filter(|f| f.priority == Priority::Low && f.chain.is_empty()).map(line_of).collect();
    assert_eq!(low, [45]);
    // the expansions take the place of their AS, after the statements written later; the fill
    // of the standard package comes last of all
    let first_expanded = p.facts.iter().position(|f| !f.chain.is_empty()).unwrap();
    assert_eq!(first_expanded, 2 + 57 + 1);
    assert_eq!(p.usages[p.facts[first_expanded].chain[0]].definition, "$CURRENT_MIRROR_P");
    let last = p.facts.last().unwrap();
    assert!(last.chain.iter().any(|&u| p.usages[u].definition == "$GROUND_STUB"), "{last:?}");
    // ORIENT-RELATION of the mirrored layouts (a current mirror relates no orientations), and the groups
    assert_eq!(p.facts.iter().filter(|f| f.kind.name() == "ORIENT-RELATION").count(), 2);
    assert_eq!(
        p.groups.iter().map(|g| (g.label.as_str(), g.members.len())).collect::<Vec<_>>(),
        [("%stage1", 5), ("%stage2", 3), ("%src", 2)]
    );
    // the slots: the first coordinate fact of each axis in this order
    let m2 = object(p, "#M2");
    assert_eq!(
        p.objects[m1].position,
        Position::Own { x: Slot::Fixed { value: 0.0, fact: 1 }, y: Slot::Fixed { value: 0.0, fact: 0 } }
    );
    assert!(matches!(p.objects[m2].position, Position::Own { x: Slot::Free, y: Slot::Fixed { value: 3.0, .. } }));
}

#[test]
fn the_later_segment_of_a_pipe_comes_first() {
    // the facts of the segments of one pipe follow the order of the text, the later segment first,
    // before what a segment adds: its members, its nested statements, the axes of a PLACE (8.5, as
    // the draft 0.3 makes it clear, in every version)
    let text = "* pipes\nR1 a b 1k\nR2 c d 1k\nR3 e f 1k\nR4 g h 1k\n\
        * @RNL(type=\"header\") { @VERSION(\"0.4\")}\n\
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
    let ports: Vec<&UsageEntry> = rdiv.usages.iter().filter(|u| u.definition == "$IO_PORT").collect();
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
    assert!(out.pins.iter().all(|q| matches!(q, PointRef::SymbolPin { .. })), "{:?}", out.pins);
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
    let (PointRef::Pin { object, pin, .. } | PointRef::SymbolPin { object, pin, .. }) = q else { panic!("{q:?}") };
    let (_, xy, orient) = at.iter().find(|(o, _, _)| o == object).unwrap();
    let d = orient.apply(p.symbols[p.objects[*object].symbol.unwrap()].pins[*pin].pos);
    (xy.0 + d.0, xy.1 + d.1)
}

/// Write a solution back into a file of the corpus, its record finished with the check of the
/// numbers as a solver writes it (5.9), and read it again: the file text, and the interpretation
/// of it.
fn write_back(f: &Path, p: &LevelProblem, written: &Written) -> (String, rnl_runtime::Interpretation) {
    let fs = FsProvider::new(&[]);
    let path = path_str(f);
    let text = fs.read(&path).unwrap();
    let doc = read_host(f, None, &fs, &mut Sink::new()).unwrap();
    let record = rnl_view::finish_record(&doc, &fs, &Options::default(), p, &written.record);
    let out = replace_generated(&doc, p.level, &text, &record);
    let mem = MemoryProvider::new().with(&path, &out);
    let both = Layered { layers: vec![&mem, &fs] };
    let r = interpret_file(f, None, &both, &Options::default()).unwrap();
    (out, r)
}

#[test]
fn a_solution_written_back_reads_back() {
    let f = case("01-rc-lowpass.sp");
    let p = &problems(&f, &FsProvider::new(&[]))[0];
    let place =
        |object: usize, x: f64, y: f64, orient: Option<Orient>| Placement { object, x: Some(x), y: Some(y), orient };
    let solution = Solution {
        placements: vec![
            place(object(p, "#C1"), 12.0, 3.0, None),
            place(object(p, "#V1"), 0.0, 0.0, Some(Orient::R0)),
            place(object(p, "#R1"), 6.0, 6.0, Some(Orient::R90)),
            place(generated_local(p, "#V1::n", "::g"), 0.0, -5.0, None),
            place(generated_local(p, "#C1::n", "::g"), 12.0, -1.0, None),
            // the junction, on %j where its definition puts it, has no name: the record refers to
            // it by its number (5.9)
            place(object(p, "a usage of $JUNCTION"), 12.0, 6.0, Some(Orient::R90)),
        ],
        ..Solution::default()
    };
    let hash = sig2(p, OPTIONS);
    let written = write_record(p, &solution, "rnl-solver test", &hash);
    // nothing is left out (14.8)
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.notes.is_empty());
    assert_eq!(
        written.record,
        format!(
            "@RNL(type=\"generated\", priority=\"high\", author=\"rnl-solver test\", hash=\"{hash}\") {{
  %ground_stub_V1_n = $GROUND_STUB @ATTACH(gap=3) #V1::n;
  %ground_stub_C1_n = $GROUND_STUB @ATTACH(gap=3) #C1::n;
  #V1 @PLACE(0, 0) => @ORIENT(\"R0\");
  #R1 @PLACE(6, 6) => @ORIENT(\"R90\");
  #C1 @PLACE(12, 3);
  %#4 @PLACE(12, 6) => @ORIENT(\"R90\");
  %ground_stub_V1_n::g @PLACE(0, -5);
  %ground_stub_C1_n::g @PLACE(12, -1)
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

    // read as a solver reads it, the file has the same input: the hash still holds, and the level
    // ends as many lines lower as the text grew
    let mem = MemoryProvider::new().with(&path_str(&f), &text);
    let fs = FsProvider::new(&[]);
    let before = fs.read(&path_str(&f)).unwrap();
    let again = problems(&f, &Layered { layers: vec![&mem, &fs] });
    assert_eq!(same_input(&hash, &again[0]), Some(true));
    let grown = text.lines().count() as i64 - before.lines().count() as i64;
    assert_eq!(again[0].end_line as i64, p.end_line as i64 + grown);
    assert_eq!(again[0].objects, p.objects);
    assert_eq!(again[0].usages, p.usages);
}

#[test]
fn a_routed_net_written_back_reads_back() {
    // T2b routes every net but its ground and its supply (13.7): the solution routes in with a
    // bend, and out through a branch point with a dot
    let f = solver_case("T2b-common-source-dragged.sp");
    let p = &problems(&f, &FsProvider::new(&[]))[0];
    let (m1, rd, vdd, vin) = (object(p, "#M1"), object(p, "#RD"), object(p, "#VDD"), object(p, "#VIN"));
    let at = [
        (vin, (0.0, 0.0), Orient::R0),
        (m1, (10.0, 0.0), Orient::R0),
        (rd, (10.0, 8.0), Orient::R0),
        (vdd, (20.0, 4.0), Orient::R0),
        (generated_local(p, "#M1::s", "::g"), (10.0, -5.0), Orient::R0),
        (generated_local(p, "#VDD::n", "::g"), (20.0, -1.0), Orient::R0),
        (generated_local(p, "#VIN::n", "::g"), (0.0, -5.0), Orient::R0),
        (generated_local(p, "#RD::p", "::s"), (10.0, 13.0), Orient::R0),
        (generated_local(p, "#VDD::p", "::s"), (20.0, 9.0), Orient::R0),
    ];
    let placements =
        at.iter().map(|(o, xy, r)| Placement { object: *o, x: Some(xy.0), y: Some(xy.1), orient: Some(*r) }).collect();
    let route = |net: &str| p.routes.iter().position(|r| r.net == net).unwrap();
    // the pins each ROUTE-NET joins, by their index in the symbol
    let pin = |object: usize, index: usize| -> PointRef {
        p.routes
            .iter()
            .flat_map(|r| &r.pins)
            .find(|q| matches!(q, PointRef::SymbolPin { object: o, pin: k, .. } if *o == object && *k == index))
            .unwrap()
            .clone()
    };
    let (vp, g) = (pin(vin, 0), pin(m1, 1));
    let (d, rn) = (pin(m1, 0), pin(rd, 1));
    let (vp_at, g_at, d_at, rn_at) = (pin_at(p, &at, &vp), pin_at(p, &at, &g), pin_at(p, &at, &d), pin_at(p, &at, &rn));
    let branch = (d_at.0, (d_at.1 + rn_at.1) / 2.0);
    let solution = Solution {
        placements,
        branches: vec![Branch { at: branch, junction: true }],
        wires: vec![
            RoutedWire {
                route: route("in"),
                points: vec![WirePoint::Anchor(vp), WirePoint::Bend((g_at.0, vp_at.1)), WirePoint::Anchor(g)],
            },
            RoutedWire { route: route("out"), points: vec![WirePoint::Anchor(d), WirePoint::Branch(0)] },
            RoutedWire { route: route("out"), points: vec![WirePoint::Branch(0), WirePoint::Anchor(rn)] },
        ],
        ..Solution::default()
    };
    let mut written = write_record(p, &solution, "rnl-solver test", &sig2(p, OPTIONS));
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.record.contains("&in @WIRE(mode=\"straight\") {#VIN::p (7, 2) #M1::g}"), "{}", written.record);
    assert!(written.record.contains("%rt1 = (10, 4)") && written.record.contains("$JUNCTION @ATTACH %rt1"));
    // the dot is left to the junction postprocessing of `rnl solve`, which places it on its point
    // (13.2), as it is placed here
    let [dot] = written.junctions.as_slice() else { panic!("{:?}", written.junctions) };
    assert_eq!(dot.target, "%rt1");
    let statement = written.record[dot.span.clone()].to_string();
    written.record = written.record.replace(&statement, &format!("{statement} => @PLACE(10, 4)"));
    let (_, r) = write_back(&f, p, &written);
    let lv = &r.model.levels[0];
    assert!(lv.unplaced.is_empty(), "{:?}", lv.unplaced);
    // the ROUTE-NETs are wired now: no flight lines are left on their nets
    assert!(!lv.flylines.iter().any(|f| p.routes.iter().any(|r| f.nets.contains(&r.net))), "{:?}", lv.flylines);
    let straight: Vec<&Vec<Pt>> = lv.wires.iter().filter(|w| w.mode == "straight").map(|w| &w.points).collect();
    assert_eq!(straight, [&vec![vp_at, (g_at.0, vp_at.1), g_at], &vec![d_at, branch], &vec![branch, rn_at]]);
    assert!(lv.items.iter().any(|i| i.kind == "usage" && i.at == branch));
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
    let (a, b) = (sig2(p, OPTIONS), sig2(p, "tolerance=0.01"));
    assert_eq!(a.len(), "sig2:".len() + 33);
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
        "# @SELECT-INSTANCE(name=\"R*\", recursive=true) => @SET-STYLE $grey;",
        "# @SELECT-INSTANCE(name=\"R*\", recursive=true) => @ORIENT(\"R90\");",
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
    assert_eq!(files.len(), 2);
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
                        PointRef::Origin(k)
                        | PointRef::Pin { object: k, .. }
                        | PointRef::SymbolPin { object: k, .. }
                        | PointRef::PinPosition { object: k, .. } => *k < n,
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
fn a_usage_has_the_place_it_is_written() {
    // where each usage is written (UsageEntry::source), so that the hints of 8.5 can name it by its
    // line: the AS of the level for the mirrored layouts, the rule of the standard package for a
    // stub that it makes, and the body of $GROUND_STUB for the ground in it
    let ps = problems(&solver_case("T4-miller-conflicts.sp"), &FsProvider::new(&[]));
    let p = &ps[0];
    let at = |u: &UsageEntry| u.source.as_ref().map(|l| (l.file.clone(), l.line)).unwrap();
    let pairs: Vec<(String, u32)> = p.usages.iter().filter(|u| u.definition == "$MIRRORED_LAYOUT").map(at).collect();
    assert_eq!(pairs, [("T4-miller-conflicts.sp".to_string(), 29), ("T4-miller-conflicts.sp".to_string(), 30)]);
    let stub = p.usages.iter().position(|u| u.definition == "$GROUND_STUB" && u.rule_made).unwrap();
    assert_eq!(at(&p.usages[stub]).0, "rnl-std-fill.rnl");
    let ground = p.usages.iter().find(|u| u.parent == Some(stub)).unwrap();
    assert_eq!((ground.definition.as_str(), at(ground).0.as_str()), ("$GROUND", "rnl-std-basic.rnl"));
    // like every place, it is left out of the hash: the test of the hash moves lines down
}
