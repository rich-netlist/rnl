//! The previous result of the solver as hints for a warm start, mapped onto the problem a solver
//! reads, and the solve status of each level that a viewer asks for (the draft 0.3, 2.8, 5.9,
//! 14.8, 17; decision 0008). A hand-made solution is written back as the record of the solver, and
//! the file read again: the hints are what the record holds, by name, by number, and through the
//! usages that rules make, and the status says whether the record is up to date.

use std::path::Path;
use std::process::Command;

use rnl_runtime::geom::{Orient, Pt};
use rnl_runtime::previous::{Previous, PreviousRoute, PreviousWire};
use rnl_runtime::problem::*;
use rnl_runtime::solution::*;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::replace_generated;
use rnl_runtime::Options;
use rnl_view::{build, read_previous, BuildOptions};
use serde_json::Value;

const OPTIONS: &str = "tolerance=0.000001";

fn draft() -> Options {
    Options { draft: true, ..Options::default() }
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

/// Two grounded instances: the standard package grounds V1:n and R1:n with a `$GND_STUB` each,
/// whose wire is "auto" and has no name in its body.
fn stubs(version: &str) -> String {
    format!(
        "* stubs\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){}}}\nV1 a 0 1\nR1 a 0 1k\n\
         * @RNL {{ (9, 9) @TEXT(\"note\") }}\n.end\n",
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

/// Where the solutions put the instances: M1 and R1 on one x, as the ALIGN of `amp` asks.
fn instance_at(label: &str) -> Option<Pt> {
    match label {
        "#M1" => Some((10.0, 0.0)),
        "#R1" => Some((10.0, 8.0)),
        "#C1" => Some((20.0, 4.0)),
        "#V1" => Some((0.0, 0.0)),
        "#R2" => Some((10.0, 0.0)),
        "#R3" => Some((10.0, -10.0)),
        _ => None,
    }
}

/// Where a point of the problem is in a solution: a placed object, or a pin of one.
fn point_at(p: &LevelProblem, s: &Solution, r: &PointRef) -> Pt {
    let placed = |k: usize| {
        let pl = s.placements.iter().find(|pl| pl.object == k).unwrap();
        (pl.at.unwrap(), pl.orient.unwrap_or_default())
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

/// A solution by hand: every object that the level leaves free placed, every one with a free
/// orientation turned to R0, and every "auto" wire routed through its points with a bend between
/// each two.
fn solve(p: &LevelProblem) -> Solution {
    let mut placements = Vec::new();
    for (k, o) in p.objects.iter().enumerate() {
        let free = matches!(o.position, Position::Own { x: Slot::Free, .. } | Position::Own { y: Slot::Free, .. });
        let at = instance_at(&o.label).or_else(|| free.then_some((30.0 + 2.0 * k as f64, 10.0 + k as f64)));
        let orient = matches!(o.orient, Some(OrientSlot::Free)).then_some(Orient::R0);
        if at.is_some() || orient.is_some() {
            placements.push(Placement { object: k, at, orient });
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

/// The points of a wire of a solution, in the scene.
fn path(p: &LevelProblem, s: &Solution, points: &[WirePoint]) -> Vec<Pt> {
    let at = |q: &WirePoint| match q {
        WirePoint::Anchor(r) => point_at(p, s, r),
        WirePoint::Bend(b) => *b,
        WirePoint::Branch(i) => s.branches[*i].at,
    };
    points.iter().map(at).collect()
}

/// The problem a solver reads, its solution, the record written from it, and the file with the
/// record in place.
struct Solved {
    p: LevelProblem,
    solution: Solution,
    written: Written,
    text: String,
}

fn solved_with(text: &str, title: &str, solver: impl Fn(&LevelProblem) -> Solution) -> Solved {
    let files = MemoryProvider::new().with("t.sp", text);
    let r = rnl_view::interpret("t.sp", None, &files, &Options { draft: true, ..Options::solver() }).unwrap();
    let p = r.problems.iter().find(|p| p.title == title).unwrap().clone();
    let solution = solver(&p);
    let written = write_record(&p, &solution, "rnl-solver test", &sig1(&p, OPTIONS));
    let record = rnl_view::finish_record(&r.doc, &files, &draft(), &p, &written.record);
    let text = replace_generated(&r.doc, p.level, text, &record);
    Solved { p, solution, written, text }
}

fn solved(text: &str, title: &str) -> Solved {
    solved_with(text, title, solve)
}

/// What a solver reads of a text: the problem of a level, read without the records of the solver,
/// and its previous result.
fn hints(text: &str, title: &str) -> (LevelProblem, Option<Previous>) {
    let files = MemoryProvider::new().with("t.sp", text);
    let (r, previous) = read_previous("t.sp", None, &files, &draft()).unwrap();
    assert_eq!(r.problems.len(), previous.len());
    let k = r.problems.iter().position(|p| p.title == title).unwrap();
    (r.problems[k].clone(), previous[k].clone())
}

impl Solved {
    fn unsolved(&self, decision: Decision) -> bool {
        self.written.unsolved.iter().any(|u| u.decision == decision)
    }

    /// The positions and orientations that the record holds, as hints.
    fn placements(&self) -> Vec<Placement> {
        let mut out: Vec<Placement> = Vec::new();
        for pl in &self.solution.placements {
            let at = pl.at.filter(|_| !self.unsolved(Decision::Position { object: pl.object }));
            let orient = pl.orient.filter(|_| !self.unsolved(Decision::Orientation { object: pl.object }));
            if at.is_some() || orient.is_some() {
                out.push(Placement { object: pl.object, at, orient });
            }
        }
        out.sort_by_key(|pl| pl.object);
        out
    }

    /// The routes of the wires that the record holds, as hints.
    fn routes(&self) -> Vec<PreviousRoute> {
        let routes = self.solution.wire_routes.iter().filter(|r| !self.unsolved(Decision::WireRoute { wire: r.wire }));
        routes.map(|r| PreviousRoute { wire: r.wire, points: path(&self.p, &self.solution, &r.points) }).collect()
    }
}

/// The solve status of each level of a text, by title, as a viewer asks for it.
fn statuses(text: &str) -> Vec<(String, Option<&'static str>)> {
    let options = BuildOptions { runtime: draft(), svg: false, status: true, ..BuildOptions::default() };
    let d = build("t.sp", &MemoryProvider::new().with("t.sp", text), &options).unwrap();
    d.levels.iter().map(|l| (l.title.clone(), l.solve)).collect()
}

fn status(text: &str, title: &str) -> &'static str {
    let all = statuses(text);
    all.iter().find(|(t, _)| t == title).and_then(|(_, s)| *s).unwrap_or_else(|| panic!("{all:?}"))
}

#[test]
fn the_hints_of_a_level_of_the_draft_map_by_name_and_by_number() {
    let s = solved(&amp("0.3"), "amp");
    let (p, previous) = hints(&s.text, "amp");
    assert_eq!(p, LevelProblem { end_line: p.end_line, ..s.p.clone() });
    let previous = previous.unwrap();
    assert_eq!(previous.hash, Some(sig1(&p, OPTIONS)));
    assert_eq!((previous.same_input, previous.numbering), (Some(true), Some(true)));
    // every position and orientation of the record, on the objects of the problem
    let placements = s.placements();
    assert_eq!(previous.placements, placements);
    let by = |placed: &Placement| match &p.objects[placed.object].reference {
        Reference::Name(_) => "name",
        Reference::Number { .. } => "number",
        Reference::Generated { .. } => "usage",
        other => panic!("{other:?}"),
    };
    let mut kinds: Vec<&str> = placements.iter().map(by).collect();
    kinds.dedup();
    assert!(kinds.contains(&"name") && kinds.contains(&"number") && kinds.contains(&"usage"), "{kinds:?}");
    // the routes of the "auto" wires, the unnamed ones by number and %w by name, as drawn
    assert_eq!(previous.wire_routes, s.routes());
    assert_eq!(previous.wire_routes.len(), 3);
    assert!(previous.wires.is_empty());
}

#[test]
fn the_hints_reach_through_a_usage_that_a_rule_makes() {
    let s = solved(&stubs("0.3"), "t.sp");
    assert!(s.written.unsolved.is_empty(), "{:?}", s.written.unsolved);
    let (p, previous) = hints(&s.text, "t.sp");
    let previous = previous.unwrap();
    assert_eq!((previous.same_input, previous.numbering), (Some(true), Some(true)));
    assert_eq!(previous.placements, s.placements());
    // the grounds of the stubs, through the names the record writes the stubs out under
    let grounds: Vec<&Placement> = previous
        .placements
        .iter()
        .filter(|pl| matches!(&p.objects[pl.object].reference, Reference::Generated { path, .. } if path == &["::g"]))
        .collect();
    assert_eq!(grounds.len(), 2);
    // the "auto" wires of the stubs, which have no name, by their place in the stubs' bodies
    assert_eq!(previous.wire_routes, s.routes());
    let stub_wires: Vec<usize> = (0..p.wires.len()).filter(|&k| p.wires[k].generated).collect();
    assert_eq!(previous.wire_routes.iter().map(|r| r.wire).collect::<Vec<_>>(), stub_wires);
    assert!(stub_wires.iter().all(|&k| matches!(p.wires[k].reference, Reference::GeneratedNumber { index: 1, .. })));
    assert!(previous.wire_routes.iter().all(|r| r.points.len() == 3));
}

#[test]
fn in_a_file_of_0_2_the_hints_are_what_the_record_holds() {
    let s = solved(&amp("0.2"), "amp");
    let (p, previous) = hints(&s.text, "amp");
    let previous = previous.unwrap();
    // no numbers to check, and no rewrites
    assert_eq!((previous.same_input, previous.numbering), (Some(true), None));
    assert_eq!(previous.placements, s.placements());
    assert!(previous.placements.iter().all(|pl| !matches!(p.objects[pl.object].reference, Reference::Number { .. })));
    assert!(previous.wire_routes.is_empty() && previous.wires.is_empty());

    // the grounds of the stubs, without the routes of their wires
    let s = solved(&stubs("0.2"), "t.sp");
    let (_, previous) = hints(&s.text, "t.sp");
    let previous = previous.unwrap();
    assert_eq!(previous.placements, s.placements());
    assert_eq!(previous.placements.len(), 4);
    assert!(previous.wire_routes.is_empty());
}

#[test]
fn the_wires_written_for_route_are_hints_with_their_nets() {
    let text = "* route\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a b 1k\nR2 b c 1k\nR3 b d 1k\n\
                * @RNL { &b @ROUTE }\n.end\n";
    let s = solved_with(text, "t.sp", |p| {
        let mut solution = solve(p);
        assert_eq!(p.routes.len(), 1);
        let pins = p.routes[0].pins.clone();
        assert_eq!(pins.len(), 3);
        solution.branches.push(Branch { at: (5.0, 0.0), junction: true });
        for (k, pin) in pins.into_iter().enumerate() {
            let mut points = vec![WirePoint::Anchor(pin), WirePoint::Branch(0)];
            if k == 0 {
                points.insert(1, WirePoint::Bend((2.0, 0.0)));
            }
            solution.wires.push(RoutedWire { route: 0, points });
        }
        solution
    });
    assert!(s.written.unsolved.is_empty(), "{:?}", s.written.unsolved);
    let (_, previous) = hints(&s.text, "t.sp");
    let previous = previous.unwrap();
    let wires: Vec<PreviousWire> = s
        .solution
        .wires
        .iter()
        .map(|w| PreviousWire { nets: vec!["b".into()], points: path(&s.p, &s.solution, &w.points) })
        .collect();
    assert_eq!(previous.wires, wires);
    assert_eq!(previous.placements, s.placements());
}

#[test]
fn a_change_to_the_authored_text_is_another_input_and_keeps_the_hints() {
    let s = solved(&amp("0.3"), "amp");
    // the ALIGN changes the input, and no statement that makes an object without a name
    let changed = s.text.replace("#M1 #R1 @ALIGN(\"x\")", "#M1 #R1 @ALIGN(\"y\")");
    assert_ne!(changed, s.text);
    let (p, previous) = hints(&changed, "amp");
    let previous = previous.unwrap();
    assert_eq!((previous.same_input, previous.numbering), (Some(false), Some(true)));
    assert_ne!(sig1(&p, OPTIONS), sig1(&s.p, OPTIONS));
    assert_eq!(previous.placements, s.placements());
    assert_eq!(previous.wire_routes, s.routes());
    assert_eq!(status(&s.text, "amp"), "solved");
    assert_eq!(status(&changed, "amp"), "stale");
}

#[test]
fn a_numbering_out_of_date_drops_the_hints_by_number() {
    let s = solved(&amp("0.3"), "amp");
    let at = s.text.find("numbering=\"").unwrap() + "numbering=\"".len();
    let tampered = format!("{}0000000000000000{}", &s.text[..at], &s.text[at + 16..]);
    let (p, previous) = hints(&tampered, "amp");
    let previous = previous.unwrap();
    // the input is the same: only the numbers are out of date
    assert_eq!((previous.same_input, previous.numbering), (Some(true), Some(false)));
    // the statements that use a number take no effect, so only the hints by name are left
    let by_name = |object: usize| !matches!(p.objects[object].reference, Reference::Number { .. });
    let named: Vec<Placement> = s.placements().into_iter().filter(|pl| by_name(pl.object)).collect();
    assert!(named.len() < s.placements().len());
    assert_eq!(previous.placements, named);
    let routes: Vec<usize> = previous.wire_routes.iter().map(|r| r.wire).collect();
    assert_eq!(routes, [p.wires.iter().position(|w| w.label == "%w").unwrap()]);
    assert_eq!(status(&tampered, "amp"), "stale");
}

#[test]
fn each_level_has_its_solve_status_when_asked() {
    let one = |records: &str| {
        format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\nR1 a b 1k\nR2 b c 1k\n{records}.end\n")
    };
    // nothing to decide: every slot fixed, no "auto" wire, no ROUTE, no constraint needing a solver
    let authored = "* @RNL { #R1 @PLACE(0, 0) => @ORIENT(\"R0\"); #R2 @PLACE(10, 0) => @ORIENT(\"R0\") }\n";
    let fixed = one(authored);
    assert_eq!(statuses(&fixed), [("t.sp".to_string(), Some("fixed"))]);
    // a free orientation, an "auto" wire, a ROUTE, a constraint that comes before the coordinates
    for open in [
        "* @RNL { #R1 @PLACE(0, 0) => @ORIENT(\"R0\"); #R2 @PLACE(10, 0) }\n",
        "* @RNL { #R1 @PLACE(0, 0) => @ORIENT(\"R0\"); #R2 @PLACE(10, 0) => @ORIENT(\"R0\"); &b @WIRE {#R1:2 #R2:1} }\n",
        "* @RNL { #R1 @PLACE(0, 0) => @ORIENT(\"R0\"); #R2 @PLACE(10, 0) => @ORIENT(\"R0\"); &b @ROUTE }\n",
        "* @RNL { #R1 @PLACE(0, 0) => @ORIENT(\"R0\"); #R2 @PLACE(10, 3) => @ORIENT(\"R0\");\n\
         *   #R1 #R2 @ALIGN(\"y\", priority=\"high\") }\n",
    ] {
        assert_eq!(statuses(&one(open)), [("t.sp".to_string(), Some("unsolved"))], "{open}");
    }
    // with a record of the solver, as one writes it with nothing to decide: solved, and stale once
    // R2 moves
    let r = rnl_view::interpret("t.sp", None, &MemoryProvider::new().with("t.sp", &fixed), &Options::solver());
    let hash = sig1(&r.unwrap().problems[0], OPTIONS);
    let recorded = one(&format!("{authored}* @RNL(type=\"generated\", priority=\"high\", hash=\"{hash}\") {{}}\n"));
    assert_eq!(statuses(&recorded), [("t.sp".to_string(), Some("solved"))]);
    let moved = recorded.replace("#R2 @PLACE(10, 0)", "#R2 @PLACE(12, 0)");
    assert_eq!(statuses(&moved), [("t.sp".to_string(), Some("stale"))]);
    // a hash of another algorithm is not compared (17)
    assert_eq!(statuses(&moved.replace(&hash, "sig2:0")), [("t.sp".to_string(), Some("solved"))]);

    // a level of the draft, before and after it is solved, and the level of the file around it
    let text = amp("0.3");
    assert_eq!(statuses(&text), [("t.sp".to_string(), Some("fixed")), ("amp".to_string(), Some("unsolved"))]);
    let s = solved(&text, "amp");
    assert_eq!(statuses(&s.text), [("t.sp".to_string(), Some("fixed")), ("amp".to_string(), Some("solved"))]);
    // a record that uses no number needs no numbering: a wrong one leaves it solved (5.9)
    let named = solved_with(&stubs("0.3"), "t.sp", |p| {
        let at = |label: &str| p.objects.iter().position(|o| o.label == label).unwrap();
        let place = |k: usize| Placement { object: k, at: Some((k as f64 * 10.0, 0.0)), orient: Some(Orient::R0) };
        Solution { placements: vec![place(at("#V1")), place(at("#R1"))], ..Solution::default() }
    });
    assert!(!named.text.contains("%#") && !named.text.contains("numbering"));
    let wrong = named.text.replace("hash=", "numbering=\"0000000000000000\", hash=");
    assert_eq!(status(&wrong, "t.sp"), "solved");

    // not asked for, the levels have no status
    let d = build("t.sp", &MemoryProvider::new().with("t.sp", &s.text), &BuildOptions::default()).unwrap();
    assert!(d.levels.iter().all(|l| l.solve.is_none()));
}

#[test]
fn a_text_that_its_symbol_places_is_open_on_an_axis_that_an_offset_gives() {
    // R1's symbol places its name; an offset on one axis replaces that axis (13.3), and the text's
    // coordinate there is the solver's to decide, as with offsets on both axes; a PLACE fixes it
    let one = |scene: &str| {
        format!(
            "* t\nR1 a b 1k\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n* @RNL {{\n\
             *   $TAG = @DEFINE-SYMBOL(box=[-2, -0.5, 2, 0.5]) {{\n\
             *     (-2, 0) @PIN(terminal=1, direction=\"left\"); (2, 0) @PIN(terminal=2, direction=\"right\");\n\
             *     ? @TEXT(\"${{name}}\", key=\"name\") => @PLACE(1, 1)\n\
             *   }};\n\
             *   # @SELECT-INSTANCE => @SET-SYMBOL $TAG;\n\
             *   #R1 @PLACE(0, 0) => @ORIENT(\"R0\"){scene}\n\
             * }}\n.end\n"
        )
    };
    let file = |status: &'static str| [("t.sp".to_string(), Some(status))];
    assert_eq!(statuses(&one("")), file("fixed"));
    assert_eq!(statuses(&one(";\n*   #R1::name @PLACE(x=-3)")), file("fixed"));
    for open in
        [";\n*   #R1 @TEXT(\"${name}\", key=\"name\", dx=3)", ";\n*   #R1 @TEXT(\"${name}\", key=\"name\", dx=3, dy=1)"]
    {
        assert_eq!(statuses(&one(open)), file("unsolved"), "{open}");
    }
}

#[test]
fn asking_for_the_status_changes_nothing_else_in_the_document() {
    let s = solved(&amp("0.3"), "amp");
    for text in [amp("0.2"), s.text.clone(), fixed_text()] {
        let files = MemoryProvider::new().with("t.sp", &text);
        let plain = build("t.sp", &files, &BuildOptions { runtime: draft(), ..BuildOptions::default() }).unwrap();
        let options = BuildOptions { runtime: draft(), status: true, ..BuildOptions::default() };
        let mut with = serde_json::to_value(build("t.sp", &files, &options).unwrap()).unwrap();
        for level in with["levels"].as_array_mut().unwrap() {
            assert!(level.as_object_mut().unwrap().remove("solve").is_some());
        }
        assert_eq!(with, serde_json::to_value(plain).unwrap());
    }
}

fn fixed_text() -> String {
    "* t\nR1 a b 1k\n* @RNL { #R1 @PLACE(0, 0) => @ORIENT(\"R0\") }\n.end\n".to_string()
}

/// `rnl` on a text written to a file.
fn rnl(name: &str, text: &str, args: &[&str]) -> Value {
    let f = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&f, text).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).arg(args[0]).arg(&f).args(&args[1..]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn the_command_gives_the_hints_when_asked() {
    let s = solved(&amp("0.3"), "amp");
    let (_, previous) = hints(&s.text, "amp");
    let previous = previous.unwrap();
    let v = rnl("previous-amp.sp", &s.text, &["problem", "--draft", "--previous", "--level", "amp"]);
    let hints = &v["levels"][0]["previous"];
    assert_eq!(hints["same_input"], true);
    assert_eq!(hints["numbering"], true);
    assert_eq!(hints["placements"].as_array().unwrap().len(), previous.placements.len());
    assert_eq!(hints["placements"][0]["object"], previous.placements[0].object);
    assert_eq!(hints["wire_routes"].as_array().unwrap().len(), 3);
    // without the flag, as before
    let v = rnl("previous-amp.sp", &s.text, &["problem", "--draft"]);
    assert!(v["levels"].as_array().unwrap().iter().all(|l| l.get("previous").is_none()));
    // a level without a record of the solver has none
    let v = rnl("previous-amp.sp", &s.text, &["problem", "--draft", "--previous", "--level", "previous-amp.sp"]);
    assert_eq!(v["levels"][0]["previous"], Value::Null);
}

#[test]
fn the_command_gives_the_status_when_asked() {
    let s = solved(&amp("0.3"), "amp");
    let v = rnl("status-amp.sp", &s.text, &["view", "--draft", "--no-svg", "--status"]);
    let solve: Vec<&Value> = v["levels"].as_array().unwrap().iter().map(|l| &l["solve"]).collect();
    assert_eq!(solve, ["fixed", "solved"]);
    let v = rnl("status-amp.sp", &s.text, &["view", "--draft", "--no-svg"]);
    assert!(v["levels"].as_array().unwrap().iter().all(|l| l.get("solve").is_none()));
}
