//! Annotation layout, transport, failure reporting and writeback contracts.
use rnl_cli::{
    interpret_file,
    solve::{solve_using, BackendResult, SolveOptions},
};
use rnl_runtime::{
    problem::{ObjectKind, PointRef, Position},
    source::MemoryProvider,
    Options,
};
use rnl_solver::Solved;
use std::path::Path;

fn interpreted(body: &str) -> rnl_runtime::Interpretation {
    let text = format!("* annotation solver\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{ $S = @DEFINE-SYMBOL {{ @RECTANGLE {{(0,0) (2,1)}} }}; {body} }}\n.end\n");
    let files = MemoryProvider::new().with("test.sp", &text);
    interpret_file(Path::new("test.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

#[test]
fn literal_and_follow_have_one_real_non_electrical_anchor() {
    let r = interpreted("%a = (_,_) @ANNOTATION $S; %a::position @PLACE(7,8); %b = #R1 @ANNOTATION $S");
    let p = &r.problems[0];
    assert_eq!(p.annotations.len(), 2);
    let a = p.annotations[0].object.expect("mapped independent anchor");
    let b = p.annotations[1].object.expect("mapped follower anchor");
    assert_eq!(p.objects[a].kind, ObjectKind::Annotation);
    assert_eq!(p.objects[b].kind, ObjectKind::Annotation);
    let Position::At(PointRef::Origin(anchor)) = &p.objects[a].position else {
        panic!("annotation is not attached to its own point")
    };
    assert_eq!(p.objects[*anchor].kind, ObjectKind::Point);
    let Position::At(PointRef::Origin(owner)) = &p.objects[b].position else { panic!("follower is not attached") };
    assert_eq!(p.objects[*owner].kind, ObjectKind::Instance);
    assert!(p.objects[a].symbol.is_none() && p.objects[b].symbol.is_none());
    assert!(p.objects[a].orient.is_none() && p.objects[b].orient.is_none());
    assert!(p.objects[a].nets.is_empty() && p.objects[b].nets.is_empty());
    let encoded = rnl_solver_io::encode_problem(p).unwrap();
    assert_eq!(rnl_solver_io::decode_problem(&encoded).unwrap(), *p);
    let mut invalid = encoded.clone();
    invalid["annotations"][0]["object"] = serde_json::json!(p.objects.len());
    assert!(rnl_solver_io::decode_problem(&invalid).is_err());
    let mut invalid = encoded;
    invalid["annotations"][0]["anchor"] = serde_json::json!([1.0, 0.0]);
    assert!(rnl_solver_io::decode_problem(&invalid).is_err());
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.solution.placements.iter().all(|pl| pl.object != a && pl.object != b));
}

#[test]
fn annotation_group_keeps_member_identity() {
    let r = interpreted("%a = (0,0) @ANNOTATION $S; {%a #R1} @GROUP");
    let p = &r.problems[0];
    let a = p.annotations[0].object.unwrap();
    assert!(p.groups.iter().any(|g| g.members.contains(&a)), "annotation must remain identifiable as a group member");
}

#[test]
fn fatal_annotation_preserves_existing_record() {
    let text =
        "* title\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL(type=\"generated\") { }\n.end\n";
    let files = MemoryProvider::new().with("fatal.sp", text);
    let backend = |_: &rnl_runtime::problem::LevelProblem, _: &rnl_solver::SolverOptions| {
        let mut solved = Solved::default();
        solved.report.fatal_annotation = Some(0);
        solved.report.errors.push("invalid annotation bounds".into());
        Ok(BackendResult { solved, author: "test".into(), report_available: true })
    };
    let run = solve_using(Path::new("fatal.sp"), &files, &SolveOptions::default(), &|_| true, &backend).unwrap();
    assert!(!run.levels.is_empty());
    assert!(run.files.is_empty());
}

#[test]
fn zero_work_keeps_seed_exception_without_fatal_annotation() {
    let r = interpreted("%a = (0,0) @ANNOTATION $S");
    let p = &r.problems[0];
    let options = rnl_solver::SolverOptions::default().with_config("work=0").unwrap();
    let solved = rnl_solver::solve(p, &options);
    assert_eq!(solved.report.fatal_annotation, None, "{:?}", solved.report.errors);
    assert!(solved.report.budget.is_some());
}

#[test]
fn group_distance_expands_annotation_anchor_member() {
    use rnl_runtime::problem::FactKind;
    let r = interpreted("%a = (0,0) @ANNOTATION $S; {%a #R1} @DISTANCE(\"left-to-right\",min=1)");
    let p = &r.problems[0];
    let a = p.annotations[0].object.unwrap();
    let Position::At(ref anchor) = p.objects[a].position else { panic!("annotation must have an anchor") };
    assert!(
        p.facts.iter().any(|fact| match &fact.kind {
            FactKind::Distance { from, to, .. } => from.iter().chain(to).any(|point| point == anchor),
            _ => false,
        }),
        "DISTANCE must include the annotation anchor"
    );
}

#[test]
fn routed_wire_detours_around_annotation_ink() {
    use rnl_runtime::solution::WirePoint;
    let text = r#"* route avoids annotation
* @RNL(type="header") { @VERSION("0.4") }
R1 n a 1k
R2 n b 1k
* @RNL {
* $P = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
* $S = @DEFINE-SYMBOL { @RECTANGLE {(-1,-1) (1,1)} };
* {#R1 #R2} @SET-SYMBOL $P;
* #R1 @PLACE(-6,0); #R2 @PLACE(6,0);
* %a = (0,0) @ANNOTATION $S; &n @ROUTE-NET
* }
.end
"#;
    let files = MemoryProvider::new().with("test.sp", text);
    let r = interpret_file(Path::new("test.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let p = &r.problems[0];
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert_eq!(solved.report.fatal_annotation, None, "{:?}", solved.report.errors);
    assert!(!solved.solution.wires.is_empty());
    assert!(
        solved.solution.wires.iter().flat_map(|w| &w.points).any(|point| matches!(point, WirePoint::Bend(_))),
        "route should leave the obstacle's interior"
    );
}

#[test]
fn literal_anchor_is_the_only_free_point_and_has_one_writable_identity() {
    use rnl_runtime::{problem::Reference, solution::write_record};
    let r = interpreted("%a = (_,_) @ANNOTATION $S");
    let p = &r.problems[0];
    let a = p.annotations[0].object.unwrap();
    let Position::At(PointRef::Origin(point)) = &p.objects[a].position else { panic!("missing literal anchor") };
    assert_eq!(
        p.objects.iter().filter(|o| o.kind == ObjectKind::Point).count(),
        1,
        "a literal must not allocate an orphan point"
    );
    assert_eq!(p.objects[*point].reference, Reference::Name("%a::position".into()));
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert_eq!(solved.solution.placements.iter().filter(|pl| pl.object == *point).count(), 1);
    let written = write_record(p, &solved.solution, "test", "hash");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert_eq!(written.record.matches("%a::position @PLACE").count(), 1, "{}", written.record);
}

#[test]
fn named_point_follower_does_not_make_a_second_point() {
    let r = interpreted("%p = (_, _); %a = %p @ANNOTATION $S; %p @PLACE(3,4)");
    let p = &r.problems[0];
    let a = p.annotations[0].object.unwrap();
    let Position::At(PointRef::Origin(point)) = &p.objects[a].position else { panic!("missing followed point") };
    assert_eq!(p.objects[*point].reference, rnl_runtime::problem::Reference::Name("%p".into()));
    assert_eq!(p.objects.iter().filter(|o| o.kind == ObjectKind::Point).count(), 1);
}

#[test]
fn literal_anchor_write_reread_resolve_replaces_record_without_orphans() {
    use rnl_cli::solve::{solve_with_clock, SolveOptions};
    use rnl_runtime::problem::input_digest;
    use rnl_solver::{SolverOptions, StoppedClock};
    let text = "* roundtrip\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { $S = @DEFINE-SYMBOL { @RECTANGLE {(0,0) (2,1)} }; %a = (_,_) @ANNOTATION $S }\n.end\n";
    let options = SolveOptions { std: false, solver: SolverOptions::without_time_limit(), ..Default::default() };
    let first_files = MemoryProvider::new().with("test.sp", text);
    let first = solve_with_clock(Path::new("test.sp"), &first_files, &options, &StoppedClock).unwrap();
    assert_eq!(first.errors, 0);
    assert!(first.levels.iter().all(|l| l.written.unsolved.is_empty()), "{:?}", first.levels);
    let saved = &first.files[0].text;
    assert_eq!(saved.matches("@RNL(type=\"generated\"").count(), 1);
    assert_eq!(saved.matches("%a::position @PLACE").count(), 1, "{saved}");
    let second_files = MemoryProvider::new().with("test.sp", saved);
    let reread =
        interpret_file(Path::new("test.sp"), None, &second_files, &Options { std: false, ..Options::solver() })
            .unwrap();
    let p = &reread.problems[0];
    assert_eq!(p.objects.iter().filter(|o| o.kind == ObjectKind::Point).count(), 1);
    assert_eq!(p.authored_numbers, first.levels[0].problem.authored_numbers);
    assert_eq!(input_digest(p), input_digest(&first.levels[0].problem));
    let second = solve_with_clock(Path::new("test.sp"), &second_files, &options, &StoppedClock).unwrap();
    assert_eq!(second.errors, 0);
    assert!(second.levels.iter().all(|l| l.written.unsolved.is_empty()), "{:?}", second.levels);
    let twice = &second.files[0].text;
    assert_eq!(twice.matches("@RNL(type=\"generated\"").count(), 1);
    assert_eq!(twice.matches("%a::position @PLACE").count(), 1, "{twice}");
    assert_eq!(input_digest(&second.levels[0].problem), input_digest(p));
}

#[test]
fn anonymous_literal_anchor_roundtrip_preserves_numbering() {
    use rnl_cli::solve::{solve_with_clock, SolveOptions};
    use rnl_solver::{SolverOptions, StoppedClock};
    let text="* anonymous annotation\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n* @RNL { $S = @DEFINE-SYMBOL { @RECTANGLE {(0,0) (2,1)} }; (_,_) @ANNOTATION $S }\n.end\n";
    let files = MemoryProvider::new().with("test.sp", text);
    let options = SolveOptions { std: false, solver: SolverOptions::without_time_limit(), ..Default::default() };
    let first = solve_with_clock(Path::new("test.sp"), &files, &options, &StoppedClock).unwrap();
    assert_eq!(first.errors, 0);
    assert!(first.levels.iter().all(|l| l.written.unsolved.is_empty()), "{:?}", first.levels);
    assert_eq!(first.levels[0].problem.objects.iter().filter(|o| o.kind == ObjectKind::Point).count(), 1);
    let saved = &first.files[0].text;
    assert_eq!(saved.matches("@RNL(type=\"generated\"").count(), 1);
    let again = MemoryProvider::new().with("test.sp", saved);
    let read = interpret_file(
        Path::new("test.sp"),
        None,
        &again,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(
        read.diagnostics().iter().all(|d| d.severity != rnl_runtime::diag::Severity::Error),
        "{:?}",
        read.diagnostics()
    );
    assert_eq!(read.problems[0].authored_numbers, first.levels[0].problem.authored_numbers);
    let second = solve_with_clock(Path::new("test.sp"), &again, &options, &StoppedClock).unwrap();
    assert!(second.levels.iter().all(|l| l.written.unsolved.is_empty()), "{:?}", second.levels);
    assert_eq!(second.files[0].text.matches("@RNL(type=\"generated\"").count(), 1);
}

#[test]
fn dynamic_text_payload_roundtrips_and_changes_digest() {
    use rnl_runtime::{model::AnnotationTextRef, problem::input_digest};
    let r = interpreted(r#"$T = @DEFINE-SYMBOL { ? @TEXT("${x}:${y}") }; %a = (1,2) @ANNOTATION $T"#);
    let p = &r.problems[0];
    let graphics = &p.annotations[0].view.graphics;
    assert!(graphics
        .iter()
        .any(|g| g.text_refs.contains(&AnnotationTextRef::X) && g.text_refs.contains(&AnnotationTextRef::Y)));
    let encoded = rnl_solver_io::encode_problem(p).unwrap();
    assert_eq!(rnl_solver_io::decode_problem(&encoded).unwrap(), *p);
    let mut changed = p.clone();
    let text =
        changed.annotations[0].view.graphics.iter_mut().find(|g| g.text_refs.contains(&AnnotationTextRef::X)).unwrap();
    *text.text_refs.iter_mut().find(|v| **v == AnnotationTextRef::X).unwrap() = AnnotationTextRef::Y;
    assert_ne!(input_digest(&changed), input_digest(p));
}

#[test]
fn unmapped_annotation_is_fatal_not_a_zero_or_invisible_box() {
    let r = interpreted("%a = (0,0) @ANNOTATION $S");
    let mut p = r.problems[0].clone();
    p.annotations[0].object = None;
    let solved = rnl_solver::solve(&p, &rnl_solver::SolverOptions::without_time_limit());
    assert_eq!(solved.report.fatal_annotation, Some(0));
    assert!(
        solved
            .report
            .errors
            .iter()
            .any(|e| e.contains(&p.annotations[0].view.object_id) && e.contains("unmapped annotation object")),
        "{:?}",
        solved.report.errors
    );
    assert!(solved.solution.placements.is_empty() && solved.solution.wires.is_empty());
}

#[test]
fn fatal_annotation_transport_uses_annotation_index_domain() {
    use rnl_runtime::problem::input_digest;
    use rnl_solver_io::{LevelResult, ResultDocument, SolverIdentity};
    let r = interpreted("%a = (0,0) @ANNOTATION $S");
    let p = &r.problems[0];
    let report = rnl_solver::SolveReport {
        fatal_annotation: Some(0),
        errors: vec!["unrepresentable annotation".into()],
        ..Default::default()
    };
    let document = ResultDocument {
        solver: SolverIdentity { name: "test".into(), version: "1".into() },
        levels: vec![LevelResult {
            request: 0,
            level: p.level,
            input: input_digest(p),
            options: String::new(),
            solution: Default::default(),
            report: Some(report),
        }],
    };
    let encoded = rnl_solver_io::encode_result(&document).unwrap();
    assert_eq!(rnl_solver_io::decode_result(&encoded, std::slice::from_ref(p)).unwrap(), document);
    let mut bad = encoded.clone();
    bad["levels"][0]["report"]["fatal_annotation"] = serde_json::json!(p.annotations.len());
    assert!(rnl_solver_io::decode_result(&bad, std::slice::from_ref(p)).is_err());
    let mut no_diagnostic = encoded;
    no_diagnostic["levels"][0]["report"]["errors"] = serde_json::json!([]);
    assert!(rnl_solver_io::decode_result(&no_diagnostic, std::slice::from_ref(p)).is_err());
}

#[test]
fn free_annotation_is_moved_out_of_a_fixed_component_when_feasible() {
    let text = r#"* movable annotation
* @RNL(type="header") { @VERSION("0.4") }
R1 a b 1k
* @RNL {
* $C = @DEFINE-SYMBOL(box=[0,0,2,2]) { @RECTANGLE {(0,0) (2,2)} };
* $S = @DEFINE-SYMBOL { @RECTANGLE {(0,0) (2,1)} };
* #R1 @SET-SYMBOL $C; #R1 @PLACE(0,0); %a = (_,_) @ANNOTATION $S
* }
.end
"#;
    let files = MemoryProvider::new().with("test.sp", text);
    let r = interpret_file(Path::new("test.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let p = &r.problems[0];
    let a = p.annotations[0].object.unwrap();
    let Position::At(PointRef::Origin(anchor)) = &p.objects[a].position else { panic!("missing anchor") };
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert_eq!(solved.report.fatal_annotation, None, "{:?}", solved.report.errors);
    let placed = solved.solution.placements.iter().find(|v| v.object == *anchor).expect("free point must be placed");
    let (x, y) = (placed.x.unwrap(), placed.y.unwrap());
    let mut view = p.annotations[0].view.clone();
    view.anchor = (x, y);
    let [lx, ly, rx, ry] =
        rnl_render::annotation_bounds(&view, rnl_render::RenderOptions::default().scale).unwrap().unwrap();
    assert!(
        x + rx <= 0.0 || x + lx >= 2.0 || y + ry <= 0.0 || y + ly >= 2.0,
        "annotation still overlaps fixed component: ({x},{y}) bbox [{lx},{ly},{rx},{ry}]"
    );
}

#[test]
fn wire_can_leave_the_pin_through_its_own_following_annotation() {
    let text = r#"* route leaves annotation owner
* @RNL(type="header") { @VERSION("0.4") }
R1 n a 1k
R2 n b 1k
* @RNL {
* $P = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
* $S = @DEFINE-SYMBOL { @RECTANGLE {(-1,-1) (1,1)} };
* {#R1 #R2} @SET-SYMBOL $P; #R1 @PLACE(0,0); #R2 @PLACE(6,0);
* %a = #R1 @ANNOTATION $S; &n @ROUTE-NET
* }
.end
"#;
    for anchor in ["#R1", "#R1:1"] {
        let text = text.replace("%a = #R1 @ANNOTATION", &format!("%a = {anchor} @ANNOTATION"));
        let files = MemoryProvider::new().with("test.sp", &text);
        let r =
            interpret_file(Path::new("test.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        let solved = rnl_solver::solve(&r.problems[0], &rnl_solver::SolverOptions::without_time_limit());
        assert_eq!(solved.report.fatal_annotation, None, "{:?}", solved.report.errors);
        assert!(solved.report.unrouted.is_empty(), "{:?}", solved.report.unrouted);
        assert!(!solved.solution.wires.is_empty());
        assert!(
            !solved.report.notes.iter().any(|n| n.contains("joined through obstacles")),
            "{:?}",
            solved.report.notes
        );
    }
}

#[test]
fn coincident_free_point_does_not_inherit_pin_owner_exemption() {
    let text = r#"* independent point at a pin
* @RNL(type="header") { @VERSION("0.4") }
R1 n a 1k
R2 n b 1k
* @RNL {
* $P = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
* $S = @DEFINE-SYMBOL { @RECTANGLE {(-1,-1) (1,1)} };
* {#R1 #R2} @SET-SYMBOL $P; #R1 @PLACE(0,0); #R2 @PLACE(6,0);
* %p = (0,0); %a = %p @ANNOTATION $S; &n @ROUTE-NET
* }
.end
"#;
    for annotation in ["%p = (0,0); %a = %p @ANNOTATION $S", "%a = (0,0) @ANNOTATION $S"] {
        let text = text.replace("%p = (0,0); %a = %p @ANNOTATION $S", annotation);
        let files = MemoryProvider::new().with("test.sp", &text);
        let r =
            interpret_file(Path::new("test.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        let solved = rnl_solver::solve(&r.problems[0], &rnl_solver::SolverOptions::without_time_limit());
        assert_eq!(solved.report.fatal_annotation, None, "{:?}", solved.report.errors);
        assert!(
            solved.report.notes.iter().any(|n| n.contains("joined through obstacles")),
            "coincident unrelated point must remain an obstacle: {:?}",
            solved.report.notes
        );
    }
}

#[test]
fn annotation_work_stops_and_replays_without_fatal() {
    let r = interpreted("%a = (_,_) @ANNOTATION $S; %b = (_,_) @ANNOTATION $S");
    let p = &r.problems[0];
    let options = rnl_solver::SolverOptions::default().with_config("work=0").unwrap();
    let a = rnl_solver::solve(p, &options);
    let b = rnl_solver::solve(p, &options);
    assert_eq!(a.report.fatal_annotation, None, "{:?}", a.report.errors);
    assert_eq!(a.report.budget.as_ref().unwrap().reason, rnl_solver::StopReason::Work);
    assert_eq!(a.report.budget.as_ref().unwrap().work, b.report.budget.as_ref().unwrap().work);
    assert_eq!(format!("{:?}", a.solution), format!("{:?}", b.solution));
}
