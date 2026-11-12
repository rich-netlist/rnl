//! Pending facts during conservative junction validation.
use rnl_cli::{
    interpret_file,
    junction_completion_problem::{validate_interpreted_trial, TrialValidation},
};
use rnl_runtime::{problem::FactKind, source::MemoryProvider, Options};
use std::path::Path;

// Both readings use the real interpreter. The generated record exists so that
// previous::drawn can prove (or fail to prove) an original retained fact.
fn check(kept: bool, added: bool) -> Result<(), String> {
    let text = r#"* pending
* @RNL(type="header") { @VERSION("0.4") }
R1 a 0 1k
R2 b 0 1k
* @RNL { #R1 #R2 @DISTANCE("left-to-right",min=2,priority="high") }
* @RNL(type="generated",priority="low") { #R1 @PLACE(0,0); #R2 @PLACE(0,3); %rt1=(8,8) }
.end
"#;
    let files = MemoryProvider::new().with("main.sp", text);
    let opts = Options { std: false, problems: true, ..Default::default() };
    let original =
        interpret_file(Path::new("main.sp"), None, &files, &Options { generated: false, ..opts.clone() }).unwrap();
    let before = interpret_file(Path::new("main.sp"), None, &files, &Options { generated: true, ..opts }).unwrap();
    let problem = &original.problems[0];
    let index = problem.facts.iter().position(|f| matches!(f.kind, FactKind::Distance { .. })).unwrap();
    assert!(
        before.model.levels[0].constraints.iter().any(|c| c.status == "needs-solver"),
        "fixture must actually leave a constraint pending: {:?}",
        before.model.levels[0].constraints
    );
    let lines: Vec<_> = (0..=text.lines().count() as u32).map(Some).collect();
    let kept = if kept { vec![index] } else { vec![] };
    // This is a direct validator probe: mark the same unresolved real fact as
    // newly introduced, exactly as added_fact_indices would pass an added fact.
    let added = if added { vec![index] } else { vec![] };
    validate_interpreted_trial(TrialValidation {
        original: &original,
        original_problem: problem,
        host_file: "main.sp",
        line_map: &lines,
        kept: &kept,
        before: &before,
        after: &before,
        level: problem.level,
        targets: &[],
        added_facts: &added,
        tolerance: 1e-6,
    })
}

#[test]
fn unchanged_unretained_needs_solver_does_not_block() {
    check(false, false).expect("unchanged pending fact was not claimed as kept");
}

#[test]
fn pending_fact_reported_kept_still_requires_drawn_proof() {
    let error = check(true, false).expect_err("an unresolved kept fact has no proof");
    assert!(error.contains("retained fact is unresolved or not satisfied"), "{error}");
}

#[test]
fn newly_added_unresolved_fact_is_rejected() {
    let error = check(false, true).expect_err("an added pending fact has no proof");
    assert!(error.contains("new or changed fact is unresolved or not satisfied"), "{error}");
}
