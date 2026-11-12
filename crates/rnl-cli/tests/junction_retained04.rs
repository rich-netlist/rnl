//! Retained facts and unrelated pre-existing conflict outcomes during junction completion.
use rnl_cli::solve::{solve_using, BackendResult, SolveOptions};
use rnl_runtime::{
    problem::FactKind,
    solution::{Branch, Placement, Solution},
    source::MemoryProvider,
};
use std::path::Path;

fn run(body: &str, retain: fn(&FactKind) -> bool, place_inputs: bool) -> Result<rnl_cli::solve::SolveRun, String> {
    let text = format!(
        r#"* retained facts
* @RNL(type="header") {{ @VERSION("0.4"); @SOLVER-CONFIG(junction="$DOT") }}
R1 a b 1k
R2 a b 1k
* @RNL {{ $DOT = @DEFINE(roles=[["p","point"]]) {{ {{? ?p}} @ALIGN("x"); {{? ?p}} @ALIGN("y") }}; {body} }}
.end
"#
    );
    let files = MemoryProvider::new().with("main.sp", &text);
    solve_using(
        Path::new("main.sp"),
        &files,
        &SolveOptions { std: false, ..Default::default() },
        &|_| true,
        &|problem, _| {
            let kept: Vec<_> =
                problem.facts.iter().enumerate().filter_map(|(i, f)| retain(&f.kind).then_some(i)).collect();
            assert!(!kept.is_empty(), "the test must retain an actual fact");
            let placements = if place_inputs {
                [("#R1", 0.0), ("#R2", 3.0)]
                    .into_iter()
                    .map(|(label, y)| Placement {
                        object: problem.objects.iter().position(|o| o.label == label).expect("device exists"),
                        x: Some(0.0),
                        y: Some(y),
                        orient: None,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Ok(BackendResult {
                solved: rnl_solver::Solved {
                    solution: Solution {
                        placements,
                        branches: vec![Branch { at: (8., 8.), junction: true }],
                        ..Default::default()
                    },
                    report: rnl_solver::SolveReport { kept, ..Default::default() },
                },
                author: "test".into(),
                report_available: true,
            })
        },
    )
}

#[test]
fn baseline_dropped_distance_does_not_block_a_retained_alignment() {
    let body = r#"{#R1 #R2} @ALIGN("x",priority="high"); #R1 #R2 @DISTANCE("left-to-right",min=2)"#;
    let solved =
        run(body, |fact| matches!(fact, FactKind::Align { .. }), true).expect("existing dropped distance survives");
    assert!(solved.levels[0].record.contains("$DOT @ATTACH %rt1 => @PLACE(8, 8)"));
}

#[test]
fn falsely_reported_kept_distance_is_rejected() {
    let body = r#"{#R1 #R2} @ALIGN("x",priority="high"); #R1 #R2 @DISTANCE("left-to-right",min=2)"#;
    let failure =
        run(body, |fact| matches!(fact, FactKind::Distance { .. }), true).expect_err("false retained fact cannot pass");
    assert!(failure.contains("retained fact is unresolved or not satisfied"), "{failure}");
}

#[test]
fn unlocated_kept_alignment_is_not_accepted_as_proof() {
    let body = r#"%u=(_, _); %v=(_, _); {%u %v} @ALIGN("x")"#;
    let failure = run(body, |fact| matches!(fact, FactKind::Align { .. }), false)
        .expect_err("a kept claim without a proof in the drawing must refuse writeback");
    assert!(failure.contains("retained fact is unresolved or not satisfied"), "{failure}");
}
