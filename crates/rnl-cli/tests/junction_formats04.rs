//! Junction numbering must use the same package format overrides as the original solve.
use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
use rnl_runtime::{
    problem::{ObjectKind, Position, Reference, Slot},
    solution::{write_record, Branch, Placement, Solution},
    source::MemoryProvider,
    Options,
};
use std::{collections::BTreeMap, path::Path};

const MAIN: &str = r#"* custom-format package and an anonymous point
* @RNL(type="header") { @VERSION("0.4"); @USE-PACKAGE(name="fixture",version="1.0"); @SOLVER-CONFIG(junction="$DOT") }
R1 a b 1k
* @RNL { #R1 @AS $WRAP; (_,_) }
.end
"#;
const GOOD: &str = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="fixture",version="1.0") }
@RNL {
  $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") };
  $WRAP = @DEFINE(roles=["x"]) { (2,3) }
}
"#;
// Same package name but intentionally incompatible definitions. It must not be discovered
// through the .rnl suffix after the caller explicitly selects another format for this file.
const DECOY: &str = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="fixture",version="1.0") }
@RNL { $DOT = @DEFINE(roles=[["p","point"]]) { ? @PLACE(99,99) }; $WRAP = @DEFINE(roles=["x"]) { (8,9); (7,6) } }
"#;

fn fixture() -> (MemoryProvider, BTreeMap<String, String>) {
    let mut files =
        MemoryProvider::new().with("main.sp", MAIN).with("packages/good.dat", GOOD).with("packages/decoy.rnl", DECOY);
    files.package_dirs.push("packages".into());
    let formats =
        BTreeMap::from([("packages/good.dat".into(), "rnl".into()), ("packages/decoy.rnl".into(), "text".into())]);
    (files, formats)
}

fn solution(p: &rnl_runtime::problem::LevelProblem) -> Solution {
    let anonymous = p
        .objects
        .iter()
        .position(|o| {
            o.kind == ObjectKind::Point
                && matches!(o.reference, Reference::Number { .. })
                && matches!(o.position, Position::Own { x: Slot::Free, y: Slot::Free })
        })
        .expect("authored unnamed point has a numbered solver identity");
    Solution {
        placements: vec![Placement { object: anonymous, x: Some(4.0), y: Some(5.0), orient: None }],
        branches: vec![Branch { at: (0.0, 0.0), junction: true }],
        ..Default::default()
    }
}

// Standalone anonymous points are not required to appear in LevelView.points. The solver
// problem exposes their stable numbered identity and final fixed coordinates instead.
fn assert_numbered_placement(read: &rnl_runtime::Interpretation, identity: &Reference) {
    let problem = &read.problems[0];
    let matches: Vec<_> = problem.objects.iter().filter(|o| &o.reference == identity).collect();
    assert_eq!(matches.len(), 1, "{identity:?}: {:?}", problem.objects);
    assert!(
        matches!(
            matches[0].position,
            Position::Own { x: Slot::Fixed { value: 4.0, .. }, y: Slot::Fixed { value: 5.0, .. } }
        ),
        "{identity:?}: {:?}",
        matches[0].position
    );
}

#[test]
fn formatted_package_supplies_the_numbering_context_without_changing_the_old_api() {
    let (files, formats) = fixture();
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let original = rnl_view::interpret_with("main.sp", None, &formats, &files, &opts).unwrap();
    assert!(
        !original.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        original.diagnostics()
    );
    let problem = &original.problems[0];
    let planned = solution(problem);
    let identity = problem.objects[planned.placements[0].object].reference.clone();
    let written = write_record(problem, &planned, "test", "test");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.record.contains("%#"), "{}", written.record);
    let finished =
        rnl_view::finish_record_with_formats(&original.doc, &files, &formats, &opts, problem, &written.record);
    let legacy = rnl_view::finish_record(&original.doc, &files, &opts, problem, &written.record);
    assert!(finished.contains("numbering="), "{finished}");
    assert_ne!(finished, legacy, "the format override must affect the check");
    let text =
        rnl_runtime::writeback::replace_generated_with_header(&original.doc, problem.level, MAIN, &finished, "0.4");
    let (mut saved, _) = fixture();
    saved.insert("main.sp", &text);
    let reread =
        rnl_view::interpret_with("main.sp", None, &formats, &saved, &Options { generated: true, ..opts }).unwrap();
    let numbered = &reread.numbering[problem.level].records;
    assert!(numbered.iter().any(|r| r.uses_numbers && r.holds()), "{numbered:?}");
    assert_numbered_placement(&reread, &identity);
}

#[test]
fn solve_using_keeps_numbered_placement_and_completes_junction_with_formats() {
    let (files, formats) = fixture();
    let options = SolveOptions { std: false, formats: formats.clone(), ..Default::default() };
    let solved = solve_using(Path::new("main.sp"), &files, &options, &|_| true, &|problem, _| {
        Ok(BackendResult {
            solved: rnl_solver::Solved { solution: solution(problem), ..Default::default() },
            author: "test".into(),
            report_available: true,
        })
    })
    .expect("format-aware numbered junction completion");
    let original = &solved.levels[0].problem;
    let identity = original.objects[solution(original).placements[0].object].reference.clone();
    let record = &solved.levels[0].record;
    assert!(record.contains("numbering=") && record.contains("%#"), "{record}");
    assert!(record.contains("$DOT @ATTACH") && record.contains("@PLACE(0, 0)"), "{record}");
    let output = solved.files.iter().find(|f| f.path == "main.sp").unwrap();
    let (mut saved, _) = fixture();
    saved.insert("main.sp", &output.text);
    let read = rnl_view::interpret_with(
        "main.sp",
        None,
        &formats,
        &saved,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(read.numbering[0].records.iter().any(|r| r.uses_numbers && r.holds()), "{:?}", read.numbering);
    assert_numbered_placement(&read, &identity);
}
