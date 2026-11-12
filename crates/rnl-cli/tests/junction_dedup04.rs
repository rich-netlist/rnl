//! Existing junction point identities in generated writeback (13.2).
use rnl_cli::interpret_file;
use rnl_runtime::{
    problem::{PointRef, Reference},
    solution::{write_record, Decision, Solution},
    source::MemoryProvider,
    Options,
};
use std::path::Path;

fn read(body: &str) -> rnl_runtime::Interpretation {
    let text = format!(
        r#"* dot identity
* @RNL(type="header") {{ @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }}
R1 a b 1k
* @RNL {{ $DOT = @DEFINE(roles=[["p",["point","terminal"]]]) {{}}; {body} }}
.end
"#
    );
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap()
}
fn object(r: &rnl_runtime::Interpretation, name: &str) -> usize {
    r.problems[0].objects.iter().position(|o| o.reference == Reference::Name(name.into())).unwrap()
}

#[test]
fn authored_point_and_repeated_requests_are_deduped_but_coincident_points_are_not() {
    let r = read("%p=(0,0); %q=(0,0); $DOT @ATTACH %p");
    let p = &r.problems[0];
    let a = PointRef::Origin(object(&r, "%p"));
    let b = PointRef::Origin(object(&r, "%q"));
    assert!(p.existing_junctions.contains(&a), "authored identity must be exported");
    let written = write_record(p, &Solution { junctions: vec![a, b.clone(), b], ..Default::default() }, "test", "hash");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(!written.record.contains("$DOT @ATTACH %p"), "{}", written.record);
    assert_eq!(written.record.matches("$DOT @ATTACH %q").count(), 1, "{}", written.record);
}

#[test]
fn electrical_pin_identity_is_distinct_from_pin_position() {
    let r = read(r#"$S = @DEFINE-SYMBOL { %tip=(0,0) @PIN(terminal=1) }; #R1 @SET-SYMBOL $S; $DOT @ATTACH #R1:1"#);
    let owner = object(&r, "#R1");
    let terminal = PointRef::Pin { object: owner, pin: 0, terminal: "1".into() };
    let symbol_pin = PointRef::SymbolPin { object: owner, pin: 0, reference: Reference::Name("#R1::tip".into()) };
    let position =
        PointRef::PinPosition { object: owner, pin: 0, reference: Reference::Name("#R1::tip::position".into()) };
    let written = write_record(
        &r.problems[0],
        &Solution { junctions: vec![terminal, symbol_pin, position], ..Default::default() },
        "test",
        "hash",
    );
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert_eq!(written.record.matches("$DOT @ATTACH").count(), 1, "{}", written.record);
    assert!(written.record.contains("#R1::tip::position"), "{}", written.record);
}

#[test]
fn repeated_invalid_requests_each_remain_unsolved() {
    let r = read("%p=(0,0)");
    let bad = PointRef::Origin(usize::MAX);
    let written = write_record(
        &r.problems[0],
        &Solution { junctions: vec![bad.clone(), bad], ..Default::default() },
        "test",
        "hash",
    );
    assert_eq!(written.unsolved.len(), 2);
    assert_eq!(written.unsolved[0].decision, Decision::Junction { index: 0 });
    assert_eq!(written.unsolved[1].decision, Decision::Junction { index: 1 });
}

#[test]
fn solver_routes_to_the_existing_dot_point_without_a_second_branch_dot() {
    use rnl_cli::solve::{solve_with_clock, SolveOptions};
    use rnl_solver::{SolverOptions, StoppedClock};
    let text = r#"* reuse authored junction
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 n a 1k
R2 n b 1k
R3 n c 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
* $DS = @DEFINE-SYMBOL(box=[0,0,0,0]) { (0,0) @CIRCLE(radius=0.4, fill="linecolor") };
* $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") };
* # @SELECT-USAGE $DOT => @SET-SYMBOL $DS;
* {#R1 #R2 #R3} @SET-SYMBOL $S;
* #R1 @PLACE(-6,0); #R2 @PLACE(6,0); #R3 @PLACE(0,1);
* %p=(0,0); %dot=$DOT @ATTACH %p => @PLACE(0,0);
* &n @WIRE(mode="manhattan") {#R1:1 %p};
* &n @WIRE(mode="manhattan") {%p #R2:1}; &n @ROUTE-NET
* }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    for layout in ["rows", "lines"] {
        let options = SolveOptions {
            std: false,
            solver: SolverOptions::default().with_config(&format!("layout=\"{layout}\",limit=\"seed\"")).unwrap(),
            ..Default::default()
        };
        let solved = solve_with_clock(Path::new("main.sp"), &files, &options, &StoppedClock).unwrap();
        assert_eq!(solved.errors, 0, "{solved:?}");
        let saved = &solved.files[0].text;
        assert_eq!(saved.matches("$DOT @ATTACH").count(), 1, "{saved}");
        let generated = saved.split("@RNL(type=\"generated\"").nth(1).expect("solver record");
        assert!(generated.contains("%p") && generated.contains("#R3:p"), "{generated}");
        assert!(!generated.contains("$DOT @ATTACH") && !generated.contains("%rt1 = (0, 0)"), "{generated}");
    }
}

#[test]
fn already_dotted_anonymous_point_needs_no_writable_reference() {
    let r = read("%p=(0,0)");
    let point = object(&r, "%p");
    let mut p = r.problems[0].clone();
    p.objects[point].reference = Reference::Anonymous;
    p.existing_junctions = vec![PointRef::Origin(point)];
    let w =
        write_record(&p, &Solution { junctions: vec![PointRef::Origin(point)], ..Default::default() }, "test", "hash");
    assert!(w.unsolved.is_empty(), "{:?}", w.unsolved);
    assert!(!w.record.contains("$DOT @ATTACH"), "{}", w.record);
}

#[test]
fn malformed_metadata_cannot_hide_an_invalid_request() {
    let r = read("%p=(0,0)");
    let mut p = r.problems[0].clone();
    p.existing_junctions = vec![PointRef::Origin(usize::MAX)];
    let w = write_record(
        &p,
        &Solution { junctions: vec![PointRef::Origin(usize::MAX)], ..Default::default() },
        "test",
        "hash",
    );
    assert_eq!(w.unsolved.len(), 1);
    assert_eq!(w.unsolved[0].decision, Decision::Junction { index: 0 });
}

#[test]
fn aliases_use_definition_identity_and_other_definitions_do_not_count() {
    let r = read(
        r#"$ALSO=$DOT; $OTHER=@DEFINE(roles=[["p","point"]]) {}; %p=(0,0); %q=(1,0); $ALSO @ATTACH %p; $OTHER @ATTACH %q"#,
    );
    let p = &r.problems[0];
    assert!(p.existing_junctions.contains(&PointRef::Origin(object(&r, "%p"))));
    assert!(!p.existing_junctions.contains(&PointRef::Origin(object(&r, "%q"))));
}

#[test]
fn usage_from_replaced_solver_record_is_not_reused() {
    let text = r#"* dot generated
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL { $DOT=@DEFINE(roles=[["p","point"]]) {}; %p=(0,0) }
* @RNL(type="generated") { $DOT @ATTACH %p }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let r = interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, problems: true, generated: true, ..Default::default() },
    )
    .unwrap();
    assert!(r.problems[0].existing_junctions.is_empty(), "{:?}", r.problems[0].existing_junctions);
}

#[test]
fn explicit_tool_override_selects_definition_identity() {
    let text = r#"* override junction
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL { $DOT = @DEFINE(roles=[["p","point"]]) {}; $OTHER = @DEFINE(roles=[["p","point"]]) {}; %p=(0,0); %q=(1,0); $DOT @ATTACH %p; $OTHER @ATTACH %q }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let options = Options {
        std: false,
        problems: true,
        solver_config: rnl_runtime::solver_settings::parse_overrides("junction=\"$OTHER\"").unwrap(),
        ..Default::default()
    };
    let r = interpret_file(Path::new("main.sp"), None, &files, &options).unwrap();
    let p = &r.problems[0];
    assert!(!p.existing_junctions.contains(&PointRef::Origin(object(&r, "%p"))));
    assert!(p.existing_junctions.contains(&PointRef::Origin(object(&r, "%q"))));
}

#[test]
fn disabled_junction_usage_is_not_existing() {
    let r = read("%p=(0,0); $DOT @ATTACH %p => @PRIORITY(\"disabled\")");
    assert!(r.problems[0].existing_junctions.is_empty(), "{:?}", r.problems[0].existing_junctions);
}

#[test]
fn failed_junction_application_is_not_treated_as_existing() {
    let r = read("$DOT @ATTACH &a");
    assert!(
        r.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "fixture must really fail"
    );
    assert!(r.problems[0].existing_junctions.is_empty(), "{:?}", r.problems[0].existing_junctions);
}
