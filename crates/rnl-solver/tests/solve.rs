//! Solves of levels of 0.4 through the public interface only, as a tool that solves does it. A
//! solve written back and read again (standard 2.8, 13.2, 13.7, chapter 17) is checked by the tests
//! of `rnl solve`, which places the dots of the branch points that a level of 0.4 writes back.

use rnl_runtime::problem::LevelProblem;
use rnl_runtime::solution::WirePoint;
use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_runtime::Options;
use rnl_solver::{solve, SolverOptions};

fn problems(entry: &str, files: &dyn Provider) -> Vec<LevelProblem> {
    rnl_view::interpret(entry, None, files, &Options::solver()).unwrap_or_else(|e| panic!("{entry}: {e}")).problems
}

/// Two trees of one net, as two structures make them: routed apart by default, each joined by the
/// net's name, and as one with `join_trees`, every pin of the net in one connected drawing, each
/// wire given back by the first tree of its net. A tree of another net after them keeps its own
/// index, as the problem has it.
#[test]
fn join_trees_routes_the_trees_of_a_net_as_one() {
    let text = r#"* two trees
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }
.subckt two a b
R1 a n 1k
R2 n b 1k
R3 a n 1k
R4 n b 1k
R5 a m 1k
R6 m b 1k
* @RNL {
*   {#R1:2 #R2:1} @ROUTE-OBJECT;
*   {#R3:2 #R4:1} @ROUTE-OBJECT;
*   {#R5:2 #R6:1} @ROUTE-OBJECT;
* }
.ends
"#;
    let files = MemoryProvider::new().with("/w/two.sp", text);
    let p = problems("/w/two.sp", &files).into_iter().find(|p| p.title == "two").unwrap();
    assert_eq!(p.routes.iter().map(|r| r.net.as_str()).collect::<Vec<_>>(), ["n", "n", "m"]);
    // the parts the wires join the pins into, by their counts of pins
    let parts = |solved: &rnl_solver::Solved| {
        let mut part: Vec<std::collections::BTreeSet<String>> = Vec::new();
        for w in &solved.solution.wires {
            let mut ends: std::collections::BTreeSet<String> = w
                .points
                .iter()
                .filter_map(|q| match q {
                    WirePoint::Anchor(a) => Some(format!("{a:?}")),
                    WirePoint::Branch(b) => Some(format!("branch {b}")),
                    WirePoint::Bend(_) => None,
                })
                .collect();
            part.retain(|s| {
                let apart = s.is_disjoint(&ends);
                if !apart {
                    ends.extend(s.iter().cloned());
                }
                apart
            });
            part.push(ends);
        }
        let mut sizes: Vec<usize> =
            part.iter().map(|s| s.iter().filter(|k| !k.starts_with("branch")).count()).collect();
        sizes.sort_unstable();
        sizes
    };
    // in rows, which route the trees as the line layout does in far less time; the skeleton of the
    // line layout is given back as the wires are (`join_tests` in the crate)
    let options = SolverOptions::default().with_config(r#"layout="rows", limit="seed""#).unwrap();
    let apart = solve(&p, &options);
    assert!(apart.report.unrouted.is_empty(), "{:?}", apart.report.unrouted);
    assert_eq!(parts(&apart), [2, 2, 2], "{:?}", apart.solution.wires);
    let joined = solve(&p, &options.with_config("join_trees=true").unwrap());
    assert!(joined.report.unrouted.is_empty(), "{:?}", joined.report.unrouted);
    assert_eq!(parts(&joined), [2, 4], "{:?}", joined.solution.wires);
    // each wire by the first tree of its net in the problem: 0 for n, 2 for m
    let mut routes: Vec<usize> = joined.solution.wires.iter().map(|w| w.route).collect();
    routes.sort_unstable();
    routes.dedup();
    assert_eq!(routes, [0, 2], "{:?}", joined.solution.wires);
    let note = "join_trees: the 3 trees of the ROUTEs routed as 2, one for each net";
    assert!(joined.report.notes.iter().any(|n| n == note), "{:?}", joined.report.notes);
    assert!(joined.report.canonical_options.as_deref().is_some_and(|c| c.contains(";join_trees=true")));
}

/// A level whose search is stopped while it tries a group both stretched in bands and dissolved,
/// between the two: the solutions the bands made are held in the trial, the best among them, and
/// were lost, so that the level panicked, its best not made at a node. They are kept, and the level
/// is drawn. Two groups of two rows, their rows aligned across in the crossing order, try both.
/// The standard fill is dropped, so that what it adds to the ports and nets does not move the stop.
#[test]
fn a_search_stopped_between_bands_and_dissolved_keeps_what_the_bands_made() {
    let text = r#"* bands
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4"); @USE-PACKAGE(name="rnl-std-fill") {} }
.subckt bands vdd vss x y
R1 vdd g0c0r1 1k
R2 g0c0r1 vss 1k
R3 vdd g0c1r1 1k
R4 g0c1r1 vss 1k
R5 g0c0r1 g0c1r1 1k
R6 vdd g1c0r1 1k
R7 g1c0r1 vss 1k
R8 vdd g1c1r1 1k
R9 g1c1r1 vss 1k
R10 g1c0r1 g1c1r1 1k
R11 g0c1r1 g1c0r1 1k
* @RNL(author="model") {
*   {#R1::p #R2::p} @ALIGN("x");
*   {#R1 #R2} @ORDER("top-to-bottom");
*   {#R3::p #R4::p} @ALIGN("x");
*   {#R3 #R4} @ORDER("top-to-bottom");
*   {#R1 #R3} @ALIGN("y");
*   {#R2 #R4} @ALIGN("y");
*   {#R1 #R3 #R2 #R4 #R5} @GROUP;
*   {#R6::p #R7::p} @ALIGN("x");
*   {#R6 #R7} @ORDER("top-to-bottom");
*   {#R8::p #R9::p} @ALIGN("x");
*   {#R8 #R9} @ORDER("top-to-bottom");
*   {#R6 #R8} @ALIGN("y");
*   {#R7 #R9} @ALIGN("y");
*   {#R6 #R8 #R7 #R9 #R10} @GROUP;
*   #R3 #R6 @DISTANCE("left-to-right", min=12);
*   {#R3 #R7} @ALIGN("y");
*   {#R4 #R6} @ALIGN("y");
*   {&g0c0r1 &g0c1r1 &g1c0r1 &g1c1r1} @ROUTE-NET;
* }
.ends
"#;
    let files = MemoryProvider::new().with("/w/bands.sp", text);
    let p = problems("/w/bands.sp", &files).into_iter().find(|p| p.title == "bands").unwrap();
    // inside the first trial of the group dissolved, before it placed anything: another solver moves
    // the window, and it is found again from the work at which the level first tries both (here
    // about 118000000) to the last at which it has a batch of each half only (about
    // 130000000); this is its middle, so that a small change of the work before it keeps the stop inside. The group-tree
    // start of the lines layout refines as it did when the window was first found, before its
    // allowance was a share of the limit. The window was found with the line layout's order of then,
    // which it is given
    let mut options = SolverOptions::default()
        .with_config(
            r#"limit="work", work=124000000, tree_work=false, lines_order="current,crossings_and_bends,mirror_pairs,dots_near_pins,length,symbol_lines+wire_lines,area""#,
        )
        .unwrap();
    // the objectives the window was found with: crowding, the last default layer, ranks the
    // arrangements otherwise and moves it
    assert_eq!(options.objectives.layers.pop().map(|l| l.items[0].0.clone()).as_deref(), Some("crowding"));
    let solved = solve(&p, &options);
    let notes = solved.report.notes.join("\n");
    assert!(notes.contains("tried both stretched in bands and dissolved"), "{notes}");
    // stopped in the window: the group dissolved placed nothing, and the level has a batch of each
    // half only, where a stop after it has more
    assert!(notes.contains("in 2 batches over 2 settings of what is in it"), "{notes}");
    assert!(solved.report.errors.is_empty(), "{:?}", solved.report.errors);
    assert!(!solved.solution.placements.is_empty(), "{notes}");
}

/// A level whose facts kept fix the position and the orientation of every component is laid out as
/// its first arrangement has it, with no arrangement searched, and its ROUTEs routed there; with one
/// component left free, the search runs as before.
#[test]
fn a_level_its_facts_fix_takes_its_first_arrangement() {
    let text = |c1: &str| {
        format!(
            r#"* fixed
* @RNL(type="header") {{ @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }}
R1 1 2 1k
R2 2 3 1k
C1 1 3 1p
* @RNL {{
*   #R1 @PLACE(0, 0) => @ORIENT("R90");
*   #R2 @PLACE(12, 0) => @ORIENT("R90");
*   {c1}
*   {{#R1:2 #R2:1}} @ROUTE-OBJECT;
*   {{#R1:1 #C1:1}} @ROUTE-OBJECT;
*   {{#R2:2 #C1:2}} @ROUTE-OBJECT;
* }}
"#
        )
    };
    let note = "no arrangement is searched";
    let options = SolverOptions::default().with_config(r#"limit="seed""#).unwrap();
    for (c1, searched) in [(r#"#C1 @PLACE(6, 8) => @ORIENT("R0");"#, false), (r#"#C1 @ORIENT("R0");"#, true)] {
        let files = MemoryProvider::new().with("/w/fixed.sp", &text(c1));
        let p = problems("/w/fixed.sp", &files).into_iter().next().unwrap();
        let solved = solve(&p, &options);
        let notes = solved.report.notes.join("\n");
        assert_eq!(!notes.contains(note), searched, "{c1}: {notes}");
        assert_eq!(solved.solution.wires.len(), 3, "{c1}: the ROUTEs are routed");
        if !searched {
            let at = |k: usize| solved.solution.placements.iter().find(|pl| pl.object == k).and_then(|pl| pl.at());
            let place = |label: &str| p.objects.iter().position(|o| o.label == label).unwrap();
            assert_eq!(at(place("#R2")), Some((12.0, 0.0)));
            assert_eq!(at(place("#C1")), Some((6.0, 8.0)));
        }
    }
}
