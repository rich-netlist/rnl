//! Headers sharing a physical line with an authored record must preserve source identities.
use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
use rnl_runtime::{
    solution::{Branch, Solution},
    source::MemoryProvider,
    Options,
};
use std::path::Path;

fn solve(files: &MemoryProvider, level: Option<&str>) -> Result<rnl_cli::solve::SolveRun, String> {
    solve_using(
        Path::new("main.sp"),
        files,
        &SolveOptions { std: false, level: level.map(str::to_owned), ..Default::default() },
        &|_| true,
        &|p, _| {
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        branches: vec![Branch { at: (0.0, 0.0), junction: true }],
                        ..Default::default()
                    },
                    report: rnl_solver::SolveReport { kept: (0..p.facts.len()).collect(), ..Default::default() },
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
}

#[test]
fn unchanged_header_sharing_a_line_keeps_the_authored_place_source() {
    let text = r#"* retained place next to header
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") } @RNL { #R1 @PLACE(1,2) }
R1 a b 1k
* @RNL { $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") } }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let result = solve(&files, None).expect("unchanged shared header preserves retained source");
    let saved = &result.files.iter().find(|file| file.path == "main.sp").unwrap().text;
    assert!(saved.contains("$DOT @ATTACH") && saved.contains("@PLACE(0, 0)"), "{saved}");
    let again = MemoryProvider::new().with("main.sp", saved);
    let read = rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &again,
        &Options { std: false, generated: true, problems: true, ..Default::default() },
    )
    .unwrap();
    let r1 = read.model.levels[0].objects.iter().find(|object| object.id == "#R1").unwrap();
    assert_eq!(r1.at, Some((1.0, 2.0)));
}

#[test]
fn changing_header_shared_with_authored_record_fails_closed() {
    let main = r#"* parent
* @RNL(type="header") { @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }
.include "child.inc"
X1 a b cell
.end
"#;
    let child = r#"* header inherits version
* @RNL(type="header") {} @RNL(scope="cell") { #R1 @PLACE(1,2) }
.subckt cell a b
R1 a b 1k
* @RNL { @SOLVER-CONFIG(junction="$DOT"); $DOT = @DEFINE(roles=[["p","point"]]) { {? ?p} @ALIGN("x"); {? ?p} @ALIGN("y") } }
.ends cell
"#;
    let files = MemoryProvider::new().with("main.sp", main).with("child.inc", child);
    let original = rnl_cli::interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, generated: false, problems: true, ..Default::default() },
    )
    .unwrap();
    let problem = original.problems.iter().find(|p| p.name == "cell").unwrap();
    assert_eq!(problem.title, "cell");
    let junction = problem.junction.effective();
    assert_eq!(junction.definition.as_deref(), Some("$DOT"));
    assert!(junction.invalid.is_none(), "{junction:?}");
    assert!(
        problem.facts.iter().any(|f| f.source.as_ref().is_some_and(|s| s.file == "child.inc" && s.line == 2)),
        "{:?}",
        problem.facts
    );
    let error = solve(&files, Some("cell")).expect_err("changing a mixed header line must not guess columns");
    assert!(error.contains("changed header shares a physical line with another record"), "{error}");
    assert_eq!(files.files.get("child.inc").unwrap(), child);
}
