//! Own usage points and general body coordinates in 0.4 (9.4, 11.1, 12.1, 14.1).

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::problem::{FactKind, ObjectKind, Position, Slot};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn netlist(version: &str, body: &str) -> String {
    format!("* usage points\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {body} }}\n.end\n")
}
fn read(version: &str, body: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", &netlist(version, body));
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}
fn at(r: &Interpretation, label: &str) -> Option<(f64, f64)> {
    let level = &r.model.levels[0];
    level
        .objects
        .iter()
        .find(|o| o.id == label)
        .map(|o| o.at)
        .or_else(|| level.points.iter().find(|p| p.id == label).map(|p| p.at))
        .unwrap()
}
const SYMBOL: &str = "$S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE }; {#R1 #R2} @SET-SYMBOL $S;";

#[test]
fn a_usage_owns_its_point_even_without_a_symbol() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"a\"]) {}; %u = #R1 @AS $D; %u @PLACE(10,20); #R1 @PLACE(1,2)");
    assert_eq!(errors(&r), []);
    let u = r.problems[0].objects.iter().find(|o| o.label == "%u").unwrap();
    assert_eq!(u.kind, ObjectKind::Usage);
    assert_eq!(u.symbol, None);
    assert_eq!(u.orient, None);
    assert!(matches!(
        u.position,
        Position::Own { x: Slot::Fixed { value: 10.0, .. }, y: Slot::Fixed { value: 20.0, .. } }
    ));
    assert_eq!(at(&r, "%u"), Some((10.0, 20.0)));
    assert_eq!(at(&r, "#R1"), Some((1.0, 2.0)));
}

#[test]
fn body_place_offsets_translate_in_scene_axes() {
    let r = read("0.4", &format!("{SYMBOL} $D = @DEFINE(roles=[\"a\"]) {{ ?a @PLACE(-2,3); ? @SET-SYMBOL $S }}; %u = #R1 @AS $D; %u @PLACE(10,20) => @ORIENT(\"R90\")"));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(at(&r, "#R1"), Some((8.0, 23.0)));
    assert_eq!(at(&r, "%u"), Some((10.0, 20.0)));
    assert_eq!(r.problems[0].facts.iter().filter(|f| matches!(f.kind, FactKind::Distance { .. })).count(), 2);
}

#[test]
fn placing_the_role_can_fix_the_usage_in_reverse() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"a\"]) { ?a @PLACE(-2,3) }; %u = #R1 @AS $D; #R1 @PLACE(8,23)");
    assert_eq!(errors(&r), []);
    assert_eq!(at(&r, "%u"), Some((10.0, 20.0)));
}

#[test]
fn self_relative_place_is_an_error_including_the_position_path() {
    for target in ["?", "?::position"] {
        let r = read("0.4", &format!("$D = @DEFINE(roles=[\"a\"]) {{ {target} @PLACE(0,0) }}; %u = #R1 @AS $D"));
        assert_eq!(errors(&r), [codes::PLACE_SELF_RELATIVE_04]);
        assert!(r.problems[0].facts.is_empty());
    }
}

#[test]
fn bare_usage_and_position_paths_are_locatable_but_not_implicit_wire_pins() {
    let r = read("0.4", &format!("{SYMBOL} $D = @DEFINE(roles=[\"a\"]) {{ {{? ?a}} @ALIGN(\"y\") }}; %u = #R1 @AS $D; %u @PLACE(2,3); &a @WIRE {{%u::position (6,3)}}"));
    assert_eq!(errors(&r), []);
    assert_eq!(r.problems[0].wires.len(), 1);
    assert!(r.problems[0].facts.iter().any(|f| matches!(f.kind, FactKind::Align { .. }) && !f.chain.is_empty()));
    let r = read("0.4", "$D = @DEFINE(roles=[\"a\"]) {}; %u = #R1 @AS $D; %u @ORIENT(\"R0\"); &a @WIRE {%u (6,3)}");
    assert!(errors(&r).contains(&codes::PLACE_USAGE_NO_SYMBOL));
    assert!(r.problems[0].wires.is_empty());
}

#[test]
fn nested_usage_coordinates_and_local_points_keep_their_own_bases() {
    let r = read("0.4", &format!("{SYMBOL} $INNER = @DEFINE(roles=[\"a\"]) {{ ?a @PLACE(2,3); %p = (-1,_) }}; $OUTER = @DEFINE(roles=[\"a\"]) {{ %child = ?a @AS $INNER; %child @PLACE(5,-6) }}; %u = #R1 @AS $OUTER; %u @PLACE(10,20)"));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(
        r.model.levels[0].objects.iter().find(|o| o.definition.as_deref() == Some("$INNER")).unwrap().at,
        Some((15.0, 14.0))
    );
    assert_eq!(at(&r, "#R1"), Some((17.0, 17.0)));
    let p = &r.problems[0];
    assert_eq!(p.facts.iter().filter(|f| matches!(f.kind, FactKind::Distance { .. })).count(), 5);
    assert!(p
        .facts
        .iter()
        .filter(|f| matches!(f.kind, FactKind::Distance { .. }))
        .all(|f| f.source.is_some() && !f.chain.is_empty()));
}

#[test]
fn relative_and_absolute_coordinates_resolve_in_fact_priority_order() {
    for (priority, discarded) in [("normal", true), ("high", true), ("disabled", false)] {
        let r = read("0.4", &format!("$D = @DEFINE(roles=[\"a\"]) {{ ?a @PLACE(-2,3) }}; %u = #R1 @AS $D; %u @PLACE(10,20,priority=\"high\"); #R1 @PLACE(99,23,priority=\"{priority}\")"));
        assert_eq!(errors(&r), []);
        // At equal priority the later segment/statement wins. The normal external PLACE is
        // later than the body, so it wins too; the usage remains fixed and the relative x loses.
        if priority == "disabled" {
            assert_eq!(at(&r, "#R1"), Some((8.0, 23.0)));
        } else {
            assert_eq!(at(&r, "#R1"), Some((99.0, 23.0)));
        }
        assert_eq!(r.diagnostics().iter().any(|d| d.code == codes::CONFLICT_FACT_DISCARDED_WITH_SETS), discarded);
    }
}

#[test]
fn idle_hints_follow_facts_not_roles_and_skip_disabled_or_discarded_links() {
    let base = format!("{SYMBOL} $D = @DEFINE(roles=[\"a\"]) {{}}; %u = #R1 @AS $D; %v = #R2 @AS $D; %p = (_, _); {{%u %p}} @ALIGN(\"x\");");
    for (tail, count) in [
        ("", 1),
        ("{%p #R1} @ALIGN(\"x\")", 0),
        ("{%p #R1} @ALIGN(\"y\")", 1),
        ("{%p #R1} @ALIGN(\"x\",priority=\"disabled\")", 1),
        ("{%p %v} @ALIGN(\"x\"); {%v #R2} @ALIGN(\"x\")", 0),
        ("%p @PLACE(0,0,priority=\"high\"); #R1 @PLACE(2,2,priority=\"high\"); {%p #R1} @ALIGN(\"x\",priority=\"low\")", 1),
        ("&a @WIRE {%u::position (3,4)}", 0),
        ("%u::position @CIRCLE(radius=1)", 0),
        ("%u::position @TEXT(\"here\")", 0),
    ] {
        let r = read("0.4", &format!("{base} {tail}"));
        assert_eq!(errors(&r), [], "{tail}: {:?}", r.diagnostics());
        assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::USAGE_POINT_IDLE_04).count(), count, "{tail}: {:?}", r.diagnostics());
    }
}

#[test]
fn older_usages_keep_their_main_role_position_and_bare_hole_error() {
    let r = read(
        "0.3",
        &format!("{SYMBOL} $D = @DEFINE(roles=[\"a\"]) {{ ?a @PLACE(1,2) }}; %u = #R1 @AS $D; %u @SET-SYMBOL $S"),
    );
    assert_eq!(errors(&r), []);
    assert_eq!(at(&r, "%u"), Some((1.0, 2.0)));
    assert!(r.problems[0].objects.iter().any(|o| o.label == "%u" && matches!(o.position, Position::At(_))));
    let r = read("0.3", "$D = @DEFINE(roles=[\"a\"]) { ? @PLACE(0,0) }; %u = #R1 @AS $D; %u @PLACE(4,5)");
    assert_eq!(errors(&r), [codes::REF_BARE_HOLE_OUTSIDE_SYMBOL, codes::PLACE_USAGE_NO_SYMBOL]);
    assert!(!r.problems[0].objects.iter().any(|o| o.kind == ObjectKind::Usage));
}

#[test]
fn parameters_and_point_literals_are_relative_but_free_axes_stay_free() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"a\"],parameters=[[\"dx\",2]]) { %p = (?dx,3); %free = (_,4); ?a @PLACE(x=?dx) }; %u = #R1 @AS(dx=-5) $D; %u @PLACE(10,20); %u::p @CIRCLE(radius=1); %u::free @CIRCLE(radius=1)");
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    let shape = r.problems[0].facts.iter().filter(|f| matches!(f.kind, FactKind::Distance { .. })).count();
    assert_eq!(shape, 4);
    assert_eq!(r.model.levels[0].shapes.len(), 1);
    assert_eq!(r.model.levels[0].shapes[0].points, [(5.0, 23.0)]);
    assert_eq!(at(&r, "#R1"), None);
}

#[test]
fn positional_conflicts_keep_both_axes_independent_and_name_the_whole_cause() {
    let r = read("0.4", "$D = @DEFINE(roles=[\"a\"]) { ?a @PLACE(-2,3) }; %u = #R1 @AS(priority=\"high\") $D; %u @PLACE(10,20,priority=\"high\"); #R1 @PLACE(99,23)");
    assert_eq!(errors(&r), []);
    assert_eq!(at(&r, "#R1"), Some((8.0, 23.0)));
    let d: Vec<_> = r
        .diagnostics()
        .iter()
        .filter(|d| d.code == codes::CONFLICT_FACT_DISCARDED_WITH_SETS)
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("the x of %u = 10") && d[0].contains("the x of #R1 = the x of %u + -2"), "{d:?}");
    let solved = rnl_solver::solve(&r.problems[0], &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.report.failed.is_empty());
    assert_eq!(solved.report.discarded.len(), 1);
}

#[test]
fn disabled_usages_disable_body_point_coordinates_as_well_as_place() {
    let r = read(
        "0.4",
        "$D = @DEFINE(roles=[\"a\"]) { %p = (1,2); ?a @PLACE(3,4) }; %u = #R1 @AS(priority=\"disabled\") $D",
    );
    assert_eq!(errors(&r), []);
    assert!(r.problems[0].facts.is_empty());
    assert_eq!(at(&r, "#R1"), None);
}

#[test]
fn source_file_version_controls_usages_in_scoped_records() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ $D = @DEFINE(roles=[\"a\"]) {{}}; %u = #R1 @AS $D; %u @PLACE(4,5) }}\n.end\n"))
            .with("inc.sp", &format!("* inc\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a\nR1 a 0 1k\n.ends\n"));
        let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        if outer == "0.4" {
            assert_eq!(errors(&r), []);
            assert!(r.problems.iter().flat_map(|p| &p.objects).any(|o| o.kind == ObjectKind::Usage));
        } else {
            assert_eq!(errors(&r), [codes::PLACE_USAGE_NO_SYMBOL]);
            assert!(!r.problems.iter().flat_map(|p| &p.objects).any(|o| o.kind == ObjectKind::Usage));
        }
    }
}

#[test]
fn solver_places_usage_points_and_writes_absolute_coordinates_that_read_back() {
    let body = format!("{SYMBOL} $D = @DEFINE(roles=[\"a\"]) {{ ?a @PLACE(-2,3) }}; %u = #R1 @AS $D; #R1 @PLACE(8,23); #R2 @PLACE(40,20)");
    let r = read("0.4", &body);
    let p = &r.problems[0];
    let options = rnl_solver::SolverOptions::without_time_limit();
    let solved = rnl_solver::solve(p, &options);
    assert!(solved.report.failed.is_empty() && solved.report.discarded.is_empty());
    let u = p.objects.iter().position(|o| o.label == "%u").unwrap();
    let placement = solved.solution.placements.iter().find(|p| p.object == u).unwrap();
    assert_eq!(placement.at(), Some((10.0, 20.0)));
    assert_eq!(placement.orient, None);
    let written = rnl_runtime::solution::write_record(
        p,
        &solved.solution,
        rnl_solver::AUTHOR,
        &rnl_runtime::problem::sig2(p, ""),
    );
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.record.contains("%u @PLACE(10, 20)"), "{}", written.record);
    let record = written.record.lines().map(|line| format!("* {line}\n")).collect::<String>();
    let text = netlist("0.4", &body).replace(".end\n", &format!("{record}.end\n"));
    let files = MemoryProvider::new().with("t.sp", &text);
    let viewed =
        interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    assert_eq!(errors(&viewed), [], "{:?}", viewed.diagnostics());
    assert_eq!(at(&viewed, "%u"), Some((10.0, 20.0)));
    assert_eq!(at(&viewed, "#R1"), Some((8.0, 23.0)));
    let again = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    assert_eq!(rnl_runtime::problem::same_input(&rnl_runtime::problem::sig2(p, ""), &again.problems[0]), Some(true));
    let solved_again = rnl_solver::solve(&again.problems[0], &options);
    assert_eq!(solved.solution, solved_again.solution);
}

#[test]
fn position_aliases_share_one_point_and_discarded_body_offsets_do_not_hide_idle_hints() {
    let r = read(
        "0.4",
        "$D = @DEFINE(roles=[\"a\"]) {}; %u = #R1 @AS $D; %u @PLACE(4,5,priority=\"high\"); %u::position @PLACE(8,9)",
    );
    assert_eq!(errors(&r), []);
    assert_eq!(at(&r, "%u"), Some((4.0, 5.0)));
    for (place, idle) in [("0,0", 1), ("0,11", 0)] {
        let r = read("0.4", &format!("{SYMBOL} $D = @DEFINE(roles=[\"a\"]) {{ ?a @PLACE(1,1,priority=\"low\") }}; %u = #R1 @AS $D; %u @PLACE(10,10,priority=\"high\"); #R1 @PLACE({place},priority=\"high\"); {{%u (_,_)}} @ALIGN(\"x\")"));
        assert_eq!(errors(&r), []);
        assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::USAGE_POINT_IDLE_04).count(), idle);
    }
}

#[test]
fn body_place_replaces_symbol_text_coordinates_and_later_text_offsets_replace_it() {
    let base = "$S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE; ? @TEXT(\"label\",key=\"label\") => @PLACE(0,1) }; #R1 @SET-SYMBOL $S; $D = @DEFINE(roles=[\"a\"]) { ?a::label @PLACE(2,3) }; %u = #R1 @AS $D; %u @PLACE(10,20); #R1 @PLACE(50,60); {%u (_, _)} @ALIGN(\"x\");";
    for (tail, expected, idle) in
        [("", (12.0, 23.0), 0), ("#R1 @TEXT(\"label\",key=\"label\",dx=7,dy=8)", (57.0, 68.0), 1)]
    {
        let r = read("0.4", &format!("{base} {tail}"));
        assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].texts[0].anchor, expected);
        let p = &r.problems[0];
        let text = p.objects.iter().position(|o| o.kind == ObjectKind::Text).unwrap();
        assert!(matches!(p.objects[text].position, Position::Own { .. }));
        let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
        assert!(solved.report.discarded.is_empty(), "{:?}", solved.report.discarded);
        assert_eq!(solved.solution.placements.iter().find(|p| p.object == text).unwrap().at(), Some(expected));
        assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::USAGE_POINT_IDLE_04).count(), idle);
    }
}
