//! Authored strict wires retain their complete path when a routing tree branches from them.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::Severity,
    problem::{LevelProblem, PointRef, RouteIntent},
    solution::{write_record, Branch, Placement, Solution, WirePoint, WireRoute},
    source::MemoryProvider,
    Options,
};
use std::path::Path;

fn problem(version: &str, mode: &str, body: &str, points: &str) -> LevelProblem {
    let source=format!("* split\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 1k\n* @RNL {{ $S= @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1) }}; {{#R1 #R2}} @SET-SYMBOL $S; {body}; %w=&a @WIRE(mode=\"{mode}\") {{{points}}} }}\n");
    let r = interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    let mut p = r.problems[0].clone();
    // The shared writer consumes an already-resolved tree; selection and priority are tested by routing.
    p.routes.push(RouteIntent {
        tree: version == "0.4",
        net: "a".into(),
        mode: "auto".into(),
        style: vec![],
        pins: p.nets.iter().find(|n| n.name == "a").unwrap().pins.clone(),
        wires: vec![0],
    });
    p
}
fn read(mode: &str) -> LevelProblem {
    problem("0.4", mode, "%a=(0,0); %b=(8,4)", "%a %b")
}
fn solution(p: &LevelProblem, branch: (f64, f64), middle: Vec<WirePoint>) -> Solution {
    let mut points = vec![WirePoint::Anchor(p.wires[0].anchors[0].clone())];
    points.extend(middle);
    points.push(WirePoint::Anchor(p.wires[0].anchors[1].clone()));
    Solution {
        branches: vec![Branch { at: branch, junction: false }],
        wire_routes: vec![WireRoute { wire: 0, points }],
        ..Default::default()
    }
}

#[test]
fn each_strict_mode_accepts_a_branch_without_changing_its_path() {
    use WirePoint::{Bend, Branch as Split};
    let cases = [
        ("straight", (4., 2.), vec![Split(0)]),
        ("horizontal-vertical", (4., 0.), vec![Split(0), Bend((8., 0.))]),
        ("vertical-horizontal", (0., 2.), vec![Split(0), Bend((0., 4.))]),
        ("manhattan", (4., 2.), vec![Bend((4., 0.)), Split(0), Bend((4., 4.))]),
        ("manhattan45", (4., 2.), vec![Bend((2., 0.)), Split(0), Bend((6., 4.))]),
    ];
    for (mode, branch, middle) in cases {
        let p = read(mode);
        let s = solution(&p, branch, middle);
        let w = write_record(&p, &s, "test", "hash");
        assert!(w.notes.is_empty() && w.unsolved.is_empty(), "{mode}: {:?} {:?}", w.notes, w.unsolved);
        assert!(w.record.contains("%w @WIRE(mode=\"straight\") {%a"), "{}", w.record);
        assert!(w.record.contains("%rt1") && w.record.contains("%b}"), "{}", w.record);
    }
}

#[test]
fn a_computed_corner_must_be_kept_even_when_the_branch_is_on_a_valid_leg() {
    let p = read("horizontal-vertical");
    let s = solution(&p, (4., 0.), vec![WirePoint::Branch(0)]);
    let w = write_record(&p, &s, "test", "hash");
    assert_eq!(w.notes.len(), 1, "{:?}", w.notes);
    assert!(w.notes[0].contains("not \"auto\""));
    assert!(w.record.contains("%w @WIRE")); // The established API notes invalid rewrites and writes them as given.
}

#[test]
fn off_path_branches_and_backtracking_are_not_shape_preserving() {
    let p = read("straight");
    for (branch, middle) in
        [((4., 3.), vec![WirePoint::Branch(0)]), ((4., 2.), vec![WirePoint::Bend((6., 3.)), WirePoint::Branch(0)])]
    {
        let w = write_record(&p, &solution(&p, branch, middle), "test", "hash");
        assert_eq!(w.notes.len(), 1, "{:?}", w.notes);
    }
}

#[test]
fn a_wire_outside_routing_and_a_rule_generated_wire_cannot_gain_a_branch() {
    for (routed, generated) in [(false, false), (true, true)] {
        let mut p = read("straight");
        if !routed {
            p.routes.clear();
        }
        p.wires[0].generated = generated;
        let w = write_record(&p, &solution(&p, (4., 2.), vec![WirePoint::Branch(0)]), "test", "hash");
        assert_eq!(w.notes.len(), 2, "{:?}", w.notes);
    }
}

#[test]
fn an_auto_wire_may_gain_a_branch_only_on_a_routed_net() {
    let mut p = read("auto");
    let s = solution(&p, (4., 3.), vec![WirePoint::Branch(0)]);
    assert!(write_record(&p, &s, "test", "hash").notes.is_empty());
    p.routes[0].net = "b".into();
    assert_eq!(write_record(&p, &s, "test", "hash").notes.len(), 1);
}

#[test]
fn authored_point_identity_and_order_still_matter_when_coordinates_are_equal() {
    let p = problem("0.4", "straight", "%a=(0,0); %b=(8,4); %other=(0,0)", "%a %b");
    let mut s = solution(&p, (4., 2.), vec![WirePoint::Branch(0)]);
    s.wire_routes[0].points.swap(0, 2);
    assert!(!write_record(&p, &s, "test", "hash").notes.is_empty());
    let other = p.objects.iter().position(|o| o.label == "%other").unwrap();
    s.wire_routes[0].points = vec![
        WirePoint::Anchor(PointRef::Origin(other)),
        WirePoint::Branch(0),
        WirePoint::Anchor(p.wires[0].anchors[1].clone()),
    ];
    assert!(!write_record(&p, &s, "test", "hash").notes.is_empty());
}

#[test]
fn a_free_authored_point_is_checked_at_its_solved_position() {
    let p = problem("0.4", "straight", "%a=(_,_); %b=(8,4)", "%a %b");
    let mut s = solution(&p, (4., 2.), vec![WirePoint::Branch(0)]);
    assert_eq!(write_record(&p, &s, "test", "hash").notes.len(), 1);
    let PointRef::Origin(object) = p.wires[0].anchors[0] else { panic!("point") };
    s.placements.push(Placement { object, x: Some(0.), y: Some(0.), orient: None });
    assert!(write_record(&p, &s, "test", "hash").notes.is_empty());
}

#[test]
fn repeated_and_zero_length_segments_keep_their_authored_handles() {
    let p = problem("0.4", "straight", "%a=(0,0); %b=(0,0)", "%a %b");
    let s = solution(&p, (0., 0.), vec![WirePoint::Branch(0)]);
    assert!(write_record(&p, &s, "test", "hash").notes.is_empty());
    let p = problem("0.4", "straight", "%a=(0,0); %b=(8,4)", "%a %b %a");
    let mut s = solution(&p, (4., 2.), vec![WirePoint::Branch(0)]);
    s.wire_routes[0].points.push(WirePoint::Anchor(p.wires[0].anchors[2].clone()));
    assert!(write_record(&p, &s, "test", "hash").notes.is_empty());
}

#[test]
fn released_strict_writeback_keeps_its_original_notes() {
    let p = problem("0.3", "straight", "%a=(0,0); %b=(8,4)", "%a %b");
    let w = write_record(&p, &solution(&p, (4., 2.), vec![WirePoint::Branch(0)]), "test", "hash");
    assert_eq!(w.notes.len(), 2, "{:?}", w.notes);
    assert!(w.notes[0].contains("not \"auto\""));
    assert!(w.notes[1].contains("bends only between them"));
}

#[test]
fn half_unit_bends_are_preserved_in_reverse_and_on_either_major_axis() {
    for (a, b, first, branch, last) in [
        ((0., 0.), (7., 4.), (1.5, 0.), (3.5, 2.), (5.5, 4.)),
        ((0., 0.), (4., 7.), (0., 1.5), (2., 3.5), (4., 5.5)),
        ((7., 4.), (0., 0.), (5.5, 4.), (3.5, 2.), (1.5, 0.)),
        ((0., 0.), (-7., 4.), (-1.5, 0.), (-3.5, 2.), (-5.5, 4.)),
    ] {
        let body = format!("%a=({},{}); %b=({},{})", a.0, a.1, b.0, b.1);
        let p = problem("0.4", "manhattan45", &body, "%a %b");
        let s = solution(&p, branch, vec![WirePoint::Bend(first), WirePoint::Branch(0), WirePoint::Bend(last)]);
        let w = write_record(&p, &s, "test", "hash");
        assert!(w.notes.is_empty() && w.unsolved.is_empty(), "{:?} {:?}", w.notes, w.unsolved);
        assert!(w.record.contains(".5"), "{}", w.record);
    }
}
