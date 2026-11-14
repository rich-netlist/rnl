//! A dot that junction completion cannot place costs only itself: the rest of the level is written,
//! and a fact that the dots do not make, left unresolved by the first pass, does not block them.
use rnl_cli::{
    interpret_file,
    junction_completion_driver::only,
    solve::{solve_using, BackendResult, SolveOptions, SolveRun},
};
use rnl_runtime::{
    diag::codes,
    problem::{LevelProblem, Reference},
    solution::{write_record, Branch, Placement, Solution},
    source::MemoryProvider,
    Options,
};
use std::path::Path;

fn solve_with(text: &str, solution: &dyn Fn(&LevelProblem) -> Solution) -> SolveRun {
    let files = MemoryProvider::new().with("main.sp", text);
    let options = SolveOptions { std: false, ..Default::default() };
    solve_using(Path::new("main.sp"), &files, &options, &|_| true, &|p, _| {
        Ok(BackendResult {
            solved: rnl_solver::Solved { solution: solution(p), ..Default::default() },
            author: "test".into(),
            report_available: true,
        })
    })
    .expect("the level is solved and written")
}

fn file(dot: &str, more: &str) -> String {
    format!(
        "* partial\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @SOLVER-CONFIG(junction=\"$DOT\") }}\nR1 a 0 1k\n* @RNL {{\n*   $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (0,2) @PIN(terminal=2) }};\n*   #R1 @SET-SYMBOL $S; #R1 @PLACE(0,0);\n*   $DOT = @DEFINE(roles=[[\"p\",\"point\"]]) {{ {dot} }}{more}\n* }}\n.end\n"
    )
}

const DOT: &str = "{? ?p} @ALIGN(\"x\"); {? ?p} @ALIGN(\"y\")";

#[test]
fn a_fact_of_a_rule_usage_left_unresolved_does_not_block_a_dot() {
    // the first pass places the point of each usage that the rule makes on x alone: its ALIGN y
    // stays unresolved, as it was before any dot was placed, and the dot is placed all the same
    let rule = ";\n*   $TAG = @DEFINE(roles=[[\"t\",\"pin\"]]) { %l = (_, _); {?t %l} @ALIGN(\"y\") };\n*   $TAG_SET = #R1 @SELECT-PIN;\n*   $TAG @ATTACH $TAG_SET";
    let run = solve_with(&file(DOT, rule), &|p| {
        let placements = p
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| matches!(&o.reference, Reference::Generated { path, .. } if path == &["::l"]))
            .map(|(object, _)| Placement { object, x: Some(9.), y: None, orient: None })
            .collect::<Vec<_>>();
        assert_eq!(placements.len(), 2, "a point of each usage the rule makes");
        Solution { placements, branches: vec![Branch { at: (5., 5.), junction: true }], ..Default::default() }
    });
    let level = &run.levels[0];
    assert!(level.record.contains("%tag_R1_p::l @PLACE(x=9)"), "{}", level.record);
    assert!(level.record.contains("$DOT @ATTACH %rt1 => @PLACE(5, 5)"), "{}", level.record);
    assert!(level.written.unsolved.is_empty(), "{:?}", level.written.unsolved);
}

#[test]
fn a_dot_that_cannot_be_placed_is_left_out_and_the_rest_is_written() {
    // no place keeps both distances of the dot: it is unsolved, and the branch point and the
    // placement are written without it
    let dot = "?p ? @DISTANCE(\"left-to-right\", exact=2); ?p ? @DISTANCE(\"left-to-right\", exact=3)";
    let run = solve_with(&file(dot, ""), &|_| Solution {
        branches: vec![Branch { at: (5., 5.), junction: true }],
        ..Default::default()
    });
    let level = &run.levels[0];
    assert!(level.record.contains("%rt1 = (5, 5)"), "{}", level.record);
    assert!(!level.record.contains("$DOT @ATTACH"), "{}", level.record);
    let unsolved: Vec<_> = level.written.unsolved.iter().map(|u| (u.code, u.label.as_str())).collect();
    assert_eq!(unsolved, [(codes::UNSOLVED_JUNCTION_UNPLACED, "%rt1")]);
    let written = &run.files.iter().find(|f| f.path == "main.sp").expect("the file is written").text;
    assert!(written.contains("%rt1 = (5, 5)"), "{written}");
}

#[test]
fn only_keeps_the_junctions_asked_for_with_their_spans() {
    let text = file(DOT, "");
    let files = MemoryProvider::new().with("main.sp", &text);
    let opts = Options { std: false, problems: true, generated: false, ..Default::default() };
    let read = interpret_file(Path::new("main.sp"), None, &files, &opts).unwrap();
    let branches = [(0., 0.), (3., 4.), (6., 8.)].map(|at| Branch { at, junction: true }).to_vec();
    let written = write_record(&read.problems[0], &Solution { branches, ..Default::default() }, "test", "test");
    assert_eq!(written.junctions.len(), 3);
    for keep in [vec![], vec![0], vec![1], vec![2], vec![0, 2], vec![1, 2], vec![0, 1, 2]] {
        let kept = only(&written, &keep);
        assert_eq!(kept.junctions.len(), keep.len());
        for (k, junction) in keep.iter().zip(&kept.junctions) {
            assert_eq!(&kept.record[junction.span.clone()], format!("$DOT @ATTACH %rt{}", k + 1), "{keep:?}");
        }
        for k in 0..3 {
            let attach = format!("$DOT @ATTACH %rt{}", k + 1);
            assert_eq!(kept.record.contains(&attach), keep.contains(&k), "{keep:?}: {}", kept.record);
        }
        assert!(kept.record.contains("%rt3 = (6, 8);") || kept.record.contains("%rt3 = (6, 8)\n"), "{}", kept.record);
        assert!(!kept.record.contains(";;") && !kept.record.contains(";\n}"), "{keep:?}: {}", kept.record);
    }
}
