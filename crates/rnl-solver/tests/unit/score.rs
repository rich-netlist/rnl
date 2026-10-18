use super::*;
use crate::objectives::Inputs;
use crate::rational::int;
use crate::{Layer, Objectives};

fn entry(_overlaps: usize, _joins: usize, layers: &[i64], code: u32) -> Entry {
    Entry { layers: layers.iter().map(|v| int(*v)).collect(), code: vec![code] }
}
fn pt(x: i64, y: i64) -> [Q; 2] {
    [int(x), int(y)]
}
#[test]
fn only_configured_layers_and_canonical_code_rank_candidates() {
    let active =
        Active::new(&Objectives { layers: vec![Layer::single("area"), Layer::single("aspect")] }, &Inputs::default());
    let entries = [entry(999, 999, &[0, 0], 0), entry(0, 0, &[1, 0], 1), entry(0, 0, &[0, 0], 2)];
    assert_eq!(order(&active, &entries), vec![0, 2, 1]);
    let no_objectives = Active::new(&Objectives::none(), &Inputs::default());
    assert_eq!(order(&no_objectives, &entries), vec![0, 1, 2]);
}

#[test]
fn segments_touch_at_an_end_or_along_a_line_and_not_where_they_cross() {
    // along one line: a stretch shared, an end on the other, ends that meet, apart
    assert!(touch(&pt(0, 0), &pt(4, 0), &pt(2, 0), &pt(6, 0)));
    assert!(touch(&pt(0, 0), &pt(4, 0), &pt(4, 0), &pt(6, 0)));
    assert!(!touch(&pt(0, 0), &pt(4, 0), &pt(5, 0), &pt(6, 0)));
    // a T: the end of one inside the other; a bend on the other
    assert!(touch(&pt(0, 0), &pt(4, 0), &pt(2, 0), &pt(2, 3)));
    assert!(touch(&pt(0, 0), &pt(4, 0), &pt(2, -3), &pt(2, 0)));
    // a crossing, inside both: the objective crossings, not a touch; parallel lines apart
    assert!(!touch(&pt(0, 0), &pt(4, 0), &pt(2, -3), &pt(2, 3)));
    assert!(!touch(&pt(0, 0), &pt(4, 0), &pt(0, 1), &pt(4, 1)));
    // oblique: an end on it, and a crossing
    assert!(touch(&pt(0, 0), &pt(4, 4), &pt(2, 2), &pt(5, 0)));
    assert!(!touch(&pt(0, 0), &pt(4, 4), &pt(0, 4), &pt(4, 0)));
    assert!(on_segment(&pt(2, 2), &pt(0, 0), &pt(4, 4)) && !on_segment(&pt(5, 5), &pt(0, 0), &pt(4, 4)));
}

/// The one level of a netlist given as text, as a solver reads it, and its structure.
fn level(text: &str) -> (LevelProblem, Structure) {
    let files = rnl_runtime::source::MemoryProvider::new().with("t.sp", text);
    let mut ps = rnl_view::interpret("t.sp", None, &files, &rnl_runtime::Options::solver()).unwrap().problems;
    assert_eq!(ps.len(), 1);
    let p = ps.remove(0);
    let options = crate::SolverOptions::without_time_limit();
    let resolved = crate::resolve(&p, &options);
    let mut work = resolved.work;
    let s = crate::structure::analyse(&p, &resolved, &options, crate::options::CAPS, &mut work);
    (p, s)
}

/// Every object where the PLACEs and the coordinates of the file put it, turned R0; the others
/// at the origin.
fn as_written(p: &LevelProblem) -> Geometry {
    use rnl_runtime::problem::{Position, Slot};
    let mut g = Geometry::new(p.objects.len());
    for (k, o) in p.objects.iter().enumerate() {
        if let Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } = &o.position {
            g.origin[k] = [crate::rational::from_f64(*x).unwrap(), crate::rational::from_f64(*y).unwrap()];
        }
    }
    g
}

/// The places where wires mislead, counted on a layout drawn by hand, each case apart from the
/// others so that the count of each is known: the wire written of `a` runs over the pin of R1
/// on `c`, which the ROUTE draws; the bus `Q<1:0>` meets the wire of `Q<1>`, one of its nets,
/// which does not count, and the wire of `d`, which does; the wire written of `b`, a net that
/// a ROUTE asks for, is of the ROUTE's net (`drawn`), so that it meets a wire routed for `b`
/// without counting, and the wire of `e`, written first so that it has the index of the ROUTE
/// of `b` among the wires written, with.
#[test]
fn wires_written_mislead_over_pins_of_other_nets_and_where_they_meet_one() {
    let (p, s) = level(
        "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n\
             R1 c z1 1k\nR2 c z2 1k\nR3 b z3 1k\nR4 b z4 1k\nR5 Q<1> z5 1k\nR6 Q<0> z6 1k\nR7 d z7 1k\nR8 e z8 1k\n\
             R9 a z9 1k\n* @RNL {\n\
             *   #R1 @PLACE(10, -3); #R2 @PLACE(100, -40); #R3 @PLACE(110, -40); #R4 @PLACE(120, -40);\n\
             *   #R5 @PLACE(130, -40); #R6 @PLACE(140, -40); #R7 @PLACE(150, -40); #R8 @PLACE(160, -40);\n\
             *   #R9 @PLACE(170, -40);\n\
             *   {&b &c} @ROUTE;\n\
             *   &e @WIRE {(55, 20) (55, 15)};\n\
             *   &a @WIRE {(5, 0) (15, 0)};\n\
             *   &Q<1:0> @WIRE {(30, 0) (40, 0)};\n\
             *   &Q<1> @WIRE {(40, 0) (40, 5)};\n\
             *   &d @WIRE {(35, 0) (35, -5)};\n\
             *   &b @WIRE {(50, 20) (60, 20)}\n* }\n.end\n",
    );
    let g = as_written(&p);
    let origins = s.origins(&p, &g);
    let nets: Vec<String> = p.wires.iter().map(|w| w.nets.join(",")).collect();
    assert_eq!(nets, ["e", "a", "Q<1>,Q<0>", "Q<1>", "d", "b"]);
    let route_b = s.connections.drawn.iter().position(|d| d.routed && d.net == "b").unwrap();
    assert_eq!(route_b, 0);
    let written = drawn(&p, &s, &g, &Routing::default());
    assert_eq!(written.paths.len(), p.wires.len());
    // the wire written of b is of the ROUTE's net, the others their own
    assert_eq!(written.paths[5].0, route_b);
    assert_eq!(written.paths[0].0, s.connections.drawn.len() - p.wires.len());
    // the pin of R1 that the wire of a runs over, on c
    let r1 = p.objects.iter().position(|o| o.label == "#R1").unwrap();
    assert_eq!(s.pin_at(&origins, &g.orient, r1, 0), Some(pt(10, 0)));
    let routed_b = (route_b, vec![pt(60, 20), pt(60, 25)]);
    let count = |wires: &[usize], routed: bool| {
        let mut paths: Vec<(usize, Vec<[Q; 2]>)> = wires.iter().map(|&w| written.paths[w].clone()).collect();
        if routed {
            paths.push(routed_b.clone());
        }
        misleading(&p, &s, &origins, &g, &Routed { paths, ..Routed::default() })
    };
    assert_eq!(count(&[1], false), 1, "the pin of R1 on the wire of a");
    assert_eq!(count(&[2, 3], false), 0, "a bus and a wire of one of its nets");
    assert_eq!(count(&[2, 3, 4], false), 1, "and a wire of another net");
    assert_eq!(count(&[5], true), 0, "a wire written and one routed of the ROUTE's net");
    assert_eq!(count(&[5, 0], true), 1, "and a wire of another net");
    assert_eq!(count(&[0, 1, 2, 3, 4, 5], true), 3);
}

#[test]
fn a_segment_runs_through_a_box_only_inside_it() {
    let b = [int(0), int(0), int(4), int(2)];
    // across, and along the middle; in from the edge to the inside
    assert!(through(&b, &pt(-1, 1), &pt(5, 1)));
    assert!(through(&b, &pt(2, -3), &pt(2, 1)));
    // along an edge, up to an edge, past a corner, outside
    assert!(!through(&b, &pt(-1, 2), &pt(5, 2)));
    assert!(!through(&b, &pt(-3, 1), &pt(0, 1)));
    assert!(!through(&b, &pt(-1, 1), &pt(1, -1)));
    assert!(!through(&b, &pt(5, 0), &pt(5, 2)));
    // oblique, through the inside and only touching a corner
    assert!(through(&b, &pt(-1, -1), &pt(5, 3)));
    assert!(!through(&b, &pt(3, 3), &pt(5, 1)));
}

/// Within a layer's tolerance the next layer decides, and an objective that is off breaks no
/// tie: with no objective at all, the code alone orders candidates equal in the counts.
#[test]
fn tolerances_and_objectives_off() {
    let active = Active::new(
        &Objectives {
            layers: vec![Layer { items: vec![crate::Term::new("area")], tolerance: int(2) }, Layer::single("aspect")],
        },
        &Inputs::default(),
    );
    let entries = [entry(0, 0, &[1, 7], 0), entry(0, 0, &[2, 3], 1), entry(0, 0, &[4, 0], 2)];
    assert_eq!(order(&active, &entries), vec![1, 0, 2]);
    let none = Active::new(&Objectives::none(), &Inputs::default());
    let entries = [entry(0, 0, &[], 3), entry(0, 0, &[], 1), entry(0, 1, &[], 0), entry(0, 0, &[], 2)];
    assert_eq!(order(&none, &entries), vec![2, 1, 3, 0]);
}
