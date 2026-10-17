//! A solution written back as the record of the solver and read again (the draft 0.3, 5.9, 13.1,
//! 14.8). In a level of a file of the draft, every decision is in the record: the objects without
//! a name by their numbers, the routes of wires as WIRE given the wire, with the check of the
//! numbers in the head. In a level of 0.2 no number and no rewrite is written. Either way, the
//! writer lists every decision it does not write, with the reason.

use std::path::Path;
use std::process::Command;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::numbering::level_check;
use rnl_runtime::problem::*;
use rnl_runtime::solution::*;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::replace_generated;
use rnl_runtime::{Interpretation, Options};

const OPTIONS: &str = "tolerance=0.000001";

fn read(text: &str, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), options).unwrap()
}

fn problem<'a>(r: &'a Interpretation, title: &str) -> &'a LevelProblem {
    r.problems.iter().find(|p| p.title == title).unwrap_or_else(|| panic!("no level {title}"))
}

fn level_index(r: &Interpretation, title: &str) -> usize {
    r.doc.levels.iter().position(|l| l.title() == title).unwrap()
}

fn diagnostics(r: &Interpretation, severity: Severity) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| r.sink.render(d)).collect()
}

fn out_of_date(r: &Interpretation) -> usize {
    diagnostics(r, Severity::Hint).iter().filter(|m| m.contains("out of date for the objects without a name")).count()
}

/// The example of the draft's 5.9 with a capacitor: an unnamed usage, a free point in a wire, a
/// label on each of two instances, an "auto" wire between two pins, a named wire, and a text
/// without a key that a rule makes on C1.
fn amp(version: &str) -> String {
    format!(
        "* a round trip\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){}}}\n\
         .subckt amp in out vdd vss\nM1 out in vss vss nch\nR1 vdd out 10k\nC1 out vss 1p\n\
         * @RNL {{\n\
         *   #M1 #R1 @ALIGN(\"x\");\n\
         *   $GND @ATTACH &vss;\n\
         *   &out @WIRE {{#M1:1 (_, _) #R1:2}};\n\
         *   {{#M1 #R1}} @TEXT(\"${{name}}\");\n\
         *   &vss @WIRE {{#M1:3 #C1:2}};\n\
         *   %w = &out @WIRE {{#R1:2 #C1:1}};\n\
         *   # @SELECT-INSTANCE(name=\"C*\") => @TEXT(\"${{value}}\")\n\
         * }}\n.ends amp\n.end\n",
        pin_02(version)
    )
}

/// A file of 0.3 uses the standard series 0.3 (16.3), whose names differ: these tests write and
/// look for those of the series 0.2, such as $GND and $GND_STUB, so a file of 0.3 pins that series
/// in its header record.
fn pin_02(version: &str) -> &'static str {
    if version == "0.3" {
        "; @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")"
    } else {
        ""
    }
}

/// Where the solution puts the instances: M1 and R1 on one x, as the ALIGN asks.
fn instance_at(label: &str) -> Option<Pt> {
    match label {
        "#M1" => Some((10.0, 0.0)),
        "#R1" => Some((10.0, 8.0)),
        "#C1" => Some((20.0, 4.0)),
        "#V1" => Some((0.0, 0.0)),
        _ => None,
    }
}

/// Where a point of the problem is in a solution: a placed object, or a pin of one.
fn point_at(p: &LevelProblem, s: &Solution, r: &PointRef) -> Pt {
    let placed = |k: usize| {
        let pl = s.placements.iter().find(|pl| pl.object == k).unwrap();
        (pl.at().unwrap(), pl.orient.unwrap_or_default())
    };
    match r {
        PointRef::Origin(k) => placed(*k).0,
        PointRef::Pin { object, pin, .. } => {
            let (at, orient) = placed(*object);
            let d = orient.apply(p.symbols[p.objects[*object].symbol.unwrap()].pins[*pin].pos);
            (at.0 + d.0, at.1 + d.1)
        }
        PointRef::Centre(_) => panic!("{r:?}"),
    }
}

/// A solution by hand: every object that the level leaves free placed, every one that has an
/// orientation turned to R0, and every "auto" wire routed through its points with a bend between
/// each two, keeping its points as a solver should (13.1).
fn solve(p: &LevelProblem) -> Solution {
    let mut placements = Vec::new();
    for (k, o) in p.objects.iter().enumerate() {
        let free = matches!(o.position, Position::Own { x: Slot::Free, .. } | Position::Own { y: Slot::Free, .. });
        let at = instance_at(&o.label).or_else(|| free.then_some((30.0 + 2.0 * k as f64, 10.0 + k as f64)));
        let orient = matches!(o.orient, Some(OrientSlot::Free)).then_some(Orient::R0);
        if at.is_some() || orient.is_some() {
            placements.push(Placement { object: k, x: at.map(|a| a.0), y: at.map(|a| a.1), orient });
        }
    }
    let mut solution = Solution { placements, ..Solution::default() };
    for (k, w) in p.wires.iter().enumerate().filter(|(_, w)| w.mode == "auto") {
        let mut points = vec![WirePoint::Anchor(w.anchors[0].clone())];
        for pair in w.anchors.windows(2) {
            let (a, b) = (point_at(p, &solution, &pair[0]), point_at(p, &solution, &pair[1]));
            points.push(WirePoint::Bend((a.0, b.1)));
            points.push(WirePoint::Anchor(pair[1].clone()));
        }
        solution.wire_routes.push(WireRoute { wire: k, points });
    }
    solution
}

/// The points of a route in a solution.
fn route_points(p: &LevelProblem, s: &Solution, route: &WireRoute) -> Vec<Pt> {
    let at = |q: &WirePoint| match q {
        WirePoint::Anchor(r) => point_at(p, s, r),
        WirePoint::Bend(b) => *b,
        WirePoint::Branch(_) => panic!("{q:?}"),
    };
    route.points.iter().map(at).collect()
}

/// The problem a solver reads, its solution, the record written and finished, and the file with
/// the record in place.
struct Solved {
    p: LevelProblem,
    solution: Solution,
    written: Written,
    record: String,
    text: String,
}

fn solved(text: &str, title: &str) -> Solved {
    let files = MemoryProvider::new().with("t.sp", text);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options::solver()).unwrap();
    let p = problem(&r, title).clone();
    let solution = solve(&p);
    let written = write_record(&p, &solution, "rnl-solver test", &sig1(&p, OPTIONS));
    let record = rnl_view::finish_record(&r.doc, &files, &Options::default(), &p, &written.record);
    let text = replace_generated(&r.doc, p.level, text, &record);
    Solved { p, solution, written, record, text }
}

/// The `numbering` in the head of a record.
fn numbering(record: &str) -> Option<String> {
    let at = record.find("numbering=\"")? + "numbering=\"".len();
    Some(record[at..at + 16].to_string())
}

/// How a level read back with its record of the solver refers to an object of the problem: the
/// usages a rule made are the ones the record writes out, the rest as they were.
fn read_back(p: &LevelProblem, r: &Reference) -> Reference {
    match r {
        Reference::Generated { usage, path } => {
            let name = generated_names(p)[*usage].clone().unwrap();
            Reference::Name(format!("%{name}{}", path.concat()))
        }
        other => other.clone(),
    }
}

fn fixed_at(position: &Position) -> Option<Pt> {
    match position {
        Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } => Some((*x, *y)),
        _ => None,
    }
}

/// `rnl check` on a text: its exit code and the line of counts.
fn rnl_check(name: &str, text: &str) -> (bool, String) {
    let f = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&f, text).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(["check", &f.to_string_lossy()]).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (out.status.success(), stdout.lines().last().unwrap_or_default().to_string())
}

#[test]
fn a_solution_of_the_draft_reads_back_as_it_was_solved() {
    let s = solved(&amp("0.3"), "amp");
    // every decision is written, but the position of the text that the rule makes on C1, which
    // nothing can refer to (13.3)
    let unsolved: Vec<String> = s.written.unsolved.iter().map(|u| u.to_string()).collect();
    assert_eq!(
        unsolved,
        ["the position of a text: a text without a key that a rule makes cannot be referred to: the solver does not decide its position, the viewer places it (13.3, 14.8)"]
    );
    assert!(s.written.notes.is_empty(), "{:?}", s.written.notes);
    // by name, by number, and the routes as WIRE given the wire
    for written in [
        "#M1 @PLACE(10, 0) => @ORIENT(\"R0\")",
        "%ioport_in = $IOPORT @ATTACH(priority=\"normal\") &in",
        "%ioport_in @PLACE(",
        "%#1 @PLACE(",
        "%#3 @PLACE(",
        "%#5 @PLACE(",
        "%#2 @WIRE(mode=\"straight\") {#M1:d (",
        "%#6 @WIRE(mode=\"straight\") {#M1:s (",
        "%w @WIRE(mode=\"straight\") {#R1:n (",
    ] {
        assert!(s.record.contains(written), "{written}\n{}", s.record);
    }
    assert!(s
        .record
        .starts_with("@RNL(type=\"generated\", priority=\"high\", author=\"rnl-solver test\", hash=\"sig1:"));
    let check = numbering(&s.record).unwrap();

    // read back with the switch: no error, and the check of the level is the record's
    let r = read(&s.text, &Options { problems: true, ..Options::default() });
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert_eq!(out_of_date(&r), 0);
    assert_eq!(level_check(&r, level_index(&r, "amp")), Some(check.as_str()));
    // every object where the solution put it, turned as it turned it
    let back = problem(&r, "amp");
    for pl in &s.solution.placements {
        let o = &s.p.objects[pl.object];
        if s.written.unsolved.iter().any(|u| u.decision == Decision::Position { object: pl.object, axis: None }) {
            continue;
        }
        let reference = read_back(&s.p, &o.reference);
        let b = back.objects.iter().find(|b| b.reference == reference).unwrap_or_else(|| panic!("{reference:?}"));
        assert_eq!(fixed_at(&b.position), pl.at(), "{}", o.label);
        if let Some(code) = pl.orient {
            assert!(matches!(b.orient, Some(OrientSlot::Fixed { code: c, .. }) if c == code), "{}", o.label);
        }
    }
    // the "auto" wires drawn through the points routed
    let drawn = &r.model.levels.iter().find(|l| l.title == "amp").unwrap().wires;
    assert_eq!(s.solution.wire_routes.len(), 3);
    for route in &s.solution.wire_routes {
        let w = &drawn[route.wire];
        assert_eq!(w.nets, s.p.wires[route.wire].nets);
        assert_eq!((w.mode.as_str(), &w.points), ("straight", &route_points(&s.p, &s.solution, route)));
    }
    // a viewer that does not solve draws all of it: nothing is left without a position
    assert!(drawn.len() == 3 && r.model.levels.iter().all(|l| l.unplaced.is_empty()));
    let (ok, counts) = rnl_check("solution-amp.sp", &s.text);
    assert!(ok && counts.starts_with("0 errors"), "{counts}");

    // read with its record, the level numbers the record's own objects after the authored ones,
    // such as the bends: a new record that replaces it does not refer to them
    let bend = back.objects.iter().position(|o| o.label == "(10, 18)").unwrap();
    assert_eq!(back.objects[bend].reference, Reference::Number { number: 7, path: Vec::new() });
    let again = Solution {
        placements: vec![Placement { object: bend, x: Some(1.0), y: Some(1.0), orient: None }],
        ..Solution::default()
    };
    let written = write_record(back, &again, "rnl-solver test", "sig1:0000000000000000.0000000000000000");
    assert_eq!(
        written.unsolved.iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        ["the position of (10, 18): it is an object of a record of the solver, which this record replaces: a solver reads the level without them (2.8)"]
    );
}

#[test]
fn changing_the_authored_text_puts_the_numbers_out_of_date() {
    let s = solved(&amp("0.3"), "amp");
    let swapped = s.text.replace("{#M1 #R1} @TEXT", "{#R1 #M1} @TEXT");
    assert_ne!(swapped, s.text);
    let r = read(&swapped, &Options { problems: true, ..Options::default() });
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert_eq!(out_of_date(&r), 1);
    let back = problem(&r, "amp");
    // the statements with numbers are left out: the ground, the free point and the labels are free
    for n in [1, 3, 4, 5] {
        let o = back.objects.iter().find(|o| o.reference == Reference::Number { number: n, path: Vec::new() }).unwrap();
        assert_eq!(fixed_at(&o.position), None, "%#{n}");
    }
    // and so are the rewrites of the wires without a name: they are drawn as written
    assert_eq!(
        back.wires.iter().map(|w| (w.label.as_str(), w.mode.as_str(), w.anchors.len())).collect::<Vec<_>>(),
        [("a wire", "auto", 3), ("a wire", "auto", 2), ("%w", "straight", 3)]
    );
    // the statements with names still apply: the instances, the ports written out, and %w above
    for (name, at) in [("#M1", (10.0, 0.0)), ("#R1", (10.0, 8.0)), ("#C1", (20.0, 4.0))] {
        let o = back.objects.iter().find(|o| o.reference == Reference::Name(name.into())).unwrap();
        assert_eq!(fixed_at(&o.position), Some(at));
    }
    let ports: Vec<&ProblemObject> =
        back.objects.iter().filter(|o| o.kind == ObjectKind::Usage && o.label.starts_with("%ioport_")).collect();
    assert_eq!(ports.len(), 4);
    assert!(ports.iter().all(|o| fixed_at(&o.position).is_some()));
}

#[test]
fn in_a_file_of_0_2_no_number_and_no_rewrite_is_written() {
    let s = solved(&amp("0.2"), "amp");
    // the record is as written: no numbers, no rewrites, and no numbering to finish it with
    assert!(!s.record.contains("%#") && !s.record.contains("@WIRE") && !s.record.contains("numbering"));
    assert_eq!(s.record, s.written.record);
    // everything else is written
    for written in ["#M1 @PLACE(10, 0) => @ORIENT(\"R0\")", "#C1 @PLACE(20, 4)", "%ioport_vss @PLACE("] {
        assert!(s.record.contains(written), "{written}\n{}", s.record);
    }
    // and what is not, with the reason
    let no_name =
        "it has no name, and a file of 0.2 cannot refer to it; the draft 0.3 refers to it by its number (5.9)";
    let no_rewrite =
        "a file of 0.2 cannot rewrite a wire; the draft 0.3 writes its route with WIRE given the wire (13.1)";
    let unsolved: Vec<String> = s.written.unsolved.iter().map(|u| u.to_string()).collect();
    assert_eq!(
        unsolved,
        [
            format!("the position of a usage of $GND: {no_name}"),
            format!("the orientation of a usage of $GND: {no_name}"),
            format!("the position of (_, _): {no_name}"),
            format!("the position of a text: {no_name}"),
            format!("the position of a text: {no_name}"),
            "the position of a text: a text without a key that a rule makes cannot be referred to: the solver does not decide its position, the viewer places it (13.3, 14.8)".to_string(),
            format!("the route of a wire: {no_rewrite}"),
            format!("the route of a wire: {no_rewrite}"),
            format!("the route of %w: {no_rewrite}"),
        ]
    );
    // read back: what is written is placed, the rest is free, as if the level were not solved
    let r = read(&s.text, &Options { problems: true, ..Options::default() });
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    let back = problem(&r, "amp");
    let m1 = back.objects.iter().find(|o| o.reference == Reference::Name("#M1".into())).unwrap();
    assert_eq!(fixed_at(&m1.position), Some((10.0, 0.0)));
    let point = back.objects.iter().find(|o| o.label == "(_, _)").unwrap();
    assert_eq!(fixed_at(&point.position), None);
    assert!(back.wires.iter().all(|w| w.label == "%w" || w.mode == "auto"));
    assert_eq!(back.wires.iter().find(|w| w.label == "%w").unwrap().mode, "auto");
}

#[test]
fn the_wire_of_a_ground_that_a_rule_makes_is_routed_by_its_number() {
    // the standard package grounds V1:n and R1:n with a $GND_STUB each, whose wire, "auto" and
    // without a name in its body, the record numbers once it writes the stub out (5.9, 10.5)
    let text = format!(
        "* stubs\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"){}}}\nV1 a 0 1\nR1 a 0 1k\n\
         * @RNL {{ (9, 9) @TEXT(\"note\") }}\n.end\n",
        pin_02("0.3")
    );
    let s = solved(&text, "t.sp");
    assert_eq!(s.p.authored_numbers, 2);
    let stubs: Vec<&ProblemWire> = s.p.wires.iter().filter(|w| w.generated).collect();
    assert_eq!(stubs.len(), 2);
    for w in &stubs {
        assert!(matches!(&w.reference, Reference::GeneratedNumber { index: 1, path, .. } if path.is_empty()));
    }
    // the stubs are written out first, and their wires are the record's own 3 and 4
    assert!(s.written.unsolved.is_empty(), "{:?}", s.written.unsolved);
    for written in [
        "%gnd_stub_V1_n = $GND_STUB @ATTACH(gap=2, priority=\"normal\") #V1:n",
        "%gnd_stub_R1_n = $GND_STUB @ATTACH(gap=2, priority=\"normal\") #R1:n",
        "%gnd_stub_V1_n::g @PLACE(",
        "%#2 @PLACE(",
        "%#3 @WIRE(mode=\"straight\") {#V1:n (",
        "%#4 @WIRE(mode=\"straight\") {#R1:n (",
    ] {
        assert!(s.record.contains(written), "{written}\n{}", s.record);
    }
    let r = read(&s.text, &Options { problems: true, ..Options::default() });
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert_eq!(out_of_date(&r), 0);
    assert_eq!(level_check(&r, 0), numbering(&s.record).as_deref());
    // the stubs written out take the place of the generated ones, and their wires are drawn as routed
    let back = problem(&r, "t.sp");
    assert!(back.wires.iter().all(|w| !w.generated && w.mode == "straight"));
    let numbers: Vec<&Reference> = back.wires.iter().map(|w| &w.reference).collect();
    assert_eq!(
        numbers,
        [&Reference::Number { number: 3, path: Vec::new() }, &Reference::Number { number: 4, path: Vec::new() }]
    );
    let drawn = &r.model.levels[0].wires;
    for (route, w) in s.solution.wire_routes.iter().zip(drawn) {
        assert_eq!(w.points, route_points(&s.p, &s.solution, route));
    }
    assert!(r.model.levels[0].unplaced.is_empty(), "{:?}", r.model.levels[0].unplaced);
    let (ok, counts) = rnl_check("solution-stubs.sp", &s.text);
    assert!(ok && counts.starts_with("0 errors"), "{counts}");

    // in a file of 0.2 the stubs are written out and their grounds placed, the wires not routed
    let s = solved(&text.replace("\"0.3\"", "\"0.2\""), "t.sp");
    assert!(s.record.contains("%gnd_stub_R1_n::g @PLACE(") && !s.record.contains("@WIRE"));
    let routes: Vec<&Unsolved> =
        s.written.unsolved.iter().filter(|u| matches!(u.decision, Decision::WireRoute { .. })).collect();
    assert_eq!(routes.len(), 2);
}

#[test]
fn the_writer_says_what_it_does_not_write_and_what_a_solver_should_not_do() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a b 1k\nC1 a 0 1p\n\
                * @RNL { #R1 @PLACE(0, 0); #C1 @PLACE(10, 0); %w = &a @WIRE(mode=\"manhattan\") {#R1:1 #C1:1};\n\
                * %v = &b @WIRE {#R1:2 (5, -5)} }\n.end\n";
    let r = read(text, &Options::solver());
    let p = problem(&r, "t.sp");
    let wire = |label: &str| p.wires.iter().position(|w| w.label == label).unwrap();
    let (w, v) = (wire("%w"), wire("%v"));
    let anchor = |k: usize, i: usize| WirePoint::Anchor(p.wires[k].anchors[i].clone());
    let solution = Solution {
        wire_routes: vec![
            // not "auto": its route is its mode's, not the solver's
            WireRoute { wire: w, points: vec![anchor(w, 0), WirePoint::Bend((5.0, 3.0)), anchor(w, 1)] },
            // a bend after the last point
            WireRoute { wire: v, points: vec![anchor(v, 0), anchor(v, 1), WirePoint::Bend((7.0, -5.0))] },
            // what the record cannot hold
            WireRoute { wire: v, points: vec![anchor(v, 0)] },
            WireRoute { wire: v, points: vec![anchor(v, 0), WirePoint::Bend((f64::NAN, 0.0)), anchor(v, 1)] },
            WireRoute { wire: 99, points: Vec::new() },
        ],
        placements: vec![Placement { object: 0, x: Some(f64::INFINITY), y: Some(0.0), orient: None }],
        ..Solution::default()
    };
    let written = write_record(p, &solution, "rnl-solver test", "sig1:0000000000000000.0000000000000000");
    assert_eq!(written.notes.len(), 2, "{:?}", written.notes);
    assert!(written.notes[0].starts_with("the route of %w: the wire is not \"auto\" but \"manhattan\""));
    assert!(written.notes[1].starts_with("the route of %v does not keep the points of the wire in order"));
    assert!(written.record.contains("%w @WIRE(mode=\"straight\") {#R1:p (5, 3) #C1:p}"), "{}", written.record);
    // the coordinate that is not a number is dropped, its other one is written
    assert!(written.record.contains("#R1 @PLACE(y=0)"), "{}", written.record);
    assert!(written.record.contains("%v @WIRE(mode=\"straight\") {#R1:n %#1 (7, -5)}"), "{}", written.record);
    let unsolved: Vec<String> = written.unsolved.iter().map(|u| u.to_string()).collect();
    assert_eq!(
        unsolved,
        [
            "the x of #R1: a coordinate is not a number of RNL, which has no NaN and no Infinity (3.7)",
            "the route of %v: a wire has at least two points (13.1)",
            "the route of %v: a bend of it: a coordinate is not a number of RNL, which has no NaN and no Infinity (3.7)",
            "the route of wire 99: it is not in the problem",
        ]
    );
}

#[test]
fn a_record_that_uses_no_number_has_no_numbering() {
    // in a file of the draft, a solution that reaches only named objects writes no number, and its
    // record needs no numbering (5.9)
    let text = "* named\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a b 1k\nC1 a b 1p\n.end\n";
    let files = MemoryProvider::new().with("t.sp", text);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options::solver()).unwrap();
    let p = problem(&r, "t.sp");
    let place = |label: &str, at: Pt| Placement {
        object: p.objects.iter().position(|o| o.label == label).unwrap(),
        x: Some(at.0),
        y: Some(at.1),
        orient: Some(Orient::R0),
    };
    let solution =
        Solution { placements: vec![place("#R1", (0.0, 0.0)), place("#C1", (10.0, 0.0))], ..Solution::default() };
    let written = write_record(p, &solution, "rnl-solver test", &sig1(p, OPTIONS));
    assert!(written.unsolved.is_empty() && !written.record.contains("%#"), "{}", written.record);
    let record = rnl_view::finish_record(&r.doc, &files, &Options::default(), p, &written.record);
    assert_eq!(record, written.record);
    assert!(!record.contains("numbering"));
    // read back with the switch: placed, with no error and no hint that it is out of date
    let back =
        read(&replace_generated(&r.doc, p.level, text, &record), &Options { problems: true, ..Options::default() });
    assert!(diagnostics(&back, Severity::Error).is_empty(), "{:?}", diagnostics(&back, Severity::Error));
    assert_eq!(out_of_date(&back), 0);
    let r1 = problem(&back, "t.sp").objects.iter().find(|o| o.label == "#R1").unwrap();
    assert_eq!(fixed_at(&r1.position), Some((0.0, 0.0)));
}

#[test]
fn a_position_on_one_axis_is_written_on_that_axis_alone() {
    // the solver may decide one coordinate alone: the record writes `@PLACE(x=…)` or
    // `@PLACE(y=…)` (11.1), and the other axis is no decision of the solver's, so not unsolved
    let text =
        "* one axis\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")}\n\
                R1 a b 1k\nR2 b c 1k\nR3 c 0 1k\n* @RNL { %p = (_, _) }\n.end\n";
    let r = read(text, &Options::solver());
    let p = problem(&r, "t.sp");
    let at = |label: &str| p.objects.iter().position(|o| o.label == label).unwrap();
    let place = |label: &str, x: Option<f64>, y: Option<f64>| Placement { object: at(label), x, y, orient: None };
    let solution = Solution {
        placements: vec![
            place("#R1", Some(4.0), None),
            place("#R2", None, Some(-2.0)),
            place("#R3", Some(1.0), Some(2.0)),
            place("%p", Some(7.0), None),
        ],
        ..Solution::default()
    };
    let written = write_record(p, &solution, "rnl-solver test", "sig1:0000000000000000.0000000000000000");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    for statement in ["#R1 @PLACE(x=4)", "#R2 @PLACE(y=-2)", "#R3 @PLACE(1, 2)", "%p @PLACE(x=7)"] {
        assert!(written.record.contains(statement), "{statement}\n{}", written.record);
    }
    // read back, the hints give each axis that the record writes, and no other
    let back = replace_generated(&r.doc, p.level, text, &written.record);
    let with = read(&back, &Options { problems: true, ..Options::default() });
    let hints = rnl_runtime::previous::previous(p, &with).unwrap();
    let given: Vec<(usize, Option<f64>, Option<f64>)> =
        hints.placements.iter().map(|pl| (pl.object, pl.x, pl.y)).collect();
    assert_eq!(
        given,
        [
            (at("#R1"), Some(4.0), None),
            (at("#R2"), None, Some(-2.0)),
            (at("#R3"), Some(1.0), Some(2.0)),
            (at("%p"), Some(7.0), None)
        ]
    );

    // in a file of 0.2, a point without a name: the axis given is unsolved, by itself
    let text = "* one axis\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a b 1k\nR2 b c 1k\n\
                * @RNL { &b @WIRE {#R1:2 (_, _) #R2:1} }\n.end\n";
    let r = read(text, &Options::solver());
    let p = problem(&r, "t.sp");
    let point = p.objects.iter().position(|o| o.label == "(_, _)").unwrap();
    let solution = Solution {
        placements: vec![Placement { object: point, x: Some(3.0), y: None, orient: None }],
        ..Solution::default()
    };
    let written = write_record(p, &solution, "rnl-solver test", "sig1:0000000000000000.0000000000000000");
    let unsolved: Vec<(Decision, String)> = written.unsolved.iter().map(|u| (u.decision, u.to_string())).collect();
    assert_eq!(
        unsolved,
        [(
            Decision::Position { object: point, axis: Some(Axis::X) },
            "the x of (_, _): it has no name, and a file of 0.2 cannot refer to it; the draft 0.3 refers to it by its \
             number (5.9)"
                .to_string()
        )]
    );
}
