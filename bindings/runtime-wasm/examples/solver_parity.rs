//! A test-only native/WASM executable. No solver API is added to the runtime binding.
use rnl_runtime::{
    problem::{sig2, PointRef, RouteIntent},
    source::MemoryProvider,
    Options,
};
use rnl_solver::{RunLimit, SolverOptions};
use std::{cell::RefCell, collections::BTreeMap};

fn record(case: u32, work: u32) -> (String, u64) {
    let n = if case % 2 == 0 { 2 } else { 4 };
    let mut text = "* solver parity\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n".to_string();
    for k in 1..=n {
        text.push_str(&format!("R{k} shared n{k} 1k\n"));
    }
    text.push_str("* @RNL {\n* $S = @DEFINE-SYMBOL { %p = (-2,0) @PIN(terminal=1,direction=\"left\"); (-1,-1) (1,-1) (0,1) (-1,-1) @LINE };\n* {");
    for k in 1..=n {
        text.push_str(&format!("#R{k} "));
    }
    text.push_str("} @SET-SYMBOL $S;\n");
    for k in 1..=if matches!(case, 0 | 1 | 6 | 7) { n } else { 0 } {
        text.push_str(&format!("* #R{k} @PLACE({}, {}) => @ORIENT(\"R0\");\n", k * 7, (k % 2) * 4));
    }
    if case >= 2 {
        text.push_str("* {#R1 #R2} @GROUP;\n");
    }
    text.push_str("* }\n.end\n");
    let files = MemoryProvider::new().with("parity.sp", &text);
    let read = rnl_view::interpret_with(
        "parity.sp",
        None,
        &BTreeMap::new(),
        &files,
        &Options { std: false, ..Options::solver() },
    )
    .unwrap();
    assert!(!read.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error));
    let mut p = read.problems[0].clone();
    let pins = p
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.symbol.is_some())
        .map(|(object, _)| PointRef::Pin { object, pin: 0, terminal: "1".into() })
        .collect();
    p.routes.push(RouteIntent {
        tree: true,
        net: "shared".into(),
        mode: "auto".into(),
        style: Vec::new(),
        pins,
        wires: Vec::new(),
    });
    if case == 3 {
        p.facts.push(rnl_runtime::problem::ProblemFact {
            kind: rnl_runtime::problem::FactKind::PinOrient {
                pin: rnl_runtime::problem::DirectedPin { object: 0, pin: 0, direction: 180 },
                codes: vec![rnl_runtime::geom::Orient::R0],
            },
            priority: rnl_runtime::interp::obj::Priority::Normal,
            label: "pin direction".into(),
            reference: None,
            source: None,
            chain: vec![],
        });
    }
    let mut options = SolverOptions { run_limit: Some(RunLimit::Work(u64::from(work))), ..SolverOptions::default() };
    if case >= 2 {
        options.settings.layout = rnl_solver::Layout::Tree;
        options = options
            .with_overrides("objectives=overlaps,crowding,misleading,wire_over_symbol,crossings,turns,wire_length,area")
            .unwrap();
    }
    if case >= 8 {
        options.settings.layout = rnl_solver::Layout::Lines;
    }
    if case >= 10 {
        options.run_limit = Some(RunLimit::Seed);
    }
    if (6..8).contains(&case) {
        let preview =
            rnl_solver::experimental::compact_drawn_with_clock(&p, &options, &rnl_solver::StoppedClock).unwrap();
        if work == 1_000_000 {
            assert!(preview.drawing.available);
            assert!(preview.drawing.notes.iter().any(|note| note.starts_with("x pass")));
        }
        let drawing = &preview.drawing;
        let text = serde_json::json!({
            "available":drawing.available,
            "positions":drawing.positions.iter().map(|p| (&p.label,p.object,p.x,p.y)).collect::<Vec<_>>(),
            "relaxed":drawing.relaxed_fact_indices,"preserved":drawing.preserved_fact_indices,
            "errors":drawing.errors,"warnings":drawing.warnings,"notes":drawing.notes,
            "options":preview.canonical_options,
        })
        .to_string();
        return (text, preview.budget.work);
    }
    let solved = if (4..6).contains(&case) {
        rnl_solver::experimental::solve_skeleton_with_clock(&p, &options, 0, &rnl_solver::StoppedClock).unwrap()
    } else {
        rnl_solver::solve(&p, &options)
    };
    let hash = sig2(&p, solved.report.canonical_options.as_ref().unwrap());
    let written = rnl_runtime::solution::write_record(&p, &solved.solution, rnl_solver::AUTHOR, &hash).record;
    if case >= 8 {
        let skeleton = solved.report.skeleton.as_ref().unwrap();
        if work == 100_000_000 {
            if case == 8 {
                assert!(!skeleton.fallback, "{:?}", solved.report.notes);
                assert!(solved.report.notes.iter().any(|note| note.starts_with("lines, round 0:")));
            } else {
                assert!(skeleton.fallback);
                assert_eq!(solved.report.budget.as_ref().unwrap().reason, rnl_solver::StopReason::Work);
            }
        }
        if case >= 10 {
            assert_eq!(solved.report.budget.as_ref().unwrap().reason, rnl_solver::StopReason::Complete);
            assert!(solved.report.notes.iter().any(|note| note.starts_with("lines, round 0:")));
        }
        let text = serde_json::json!({
            "record":written,
            "work":solved.report.budget.as_ref().unwrap().work,
            "fallback":skeleton.fallback,
            "tables":skeleton.tables.iter().map(|t| serde_json::json!({
                "group":t.group,"rows":t.rows,"columns":t.columns,"xs":t.xs,"ys":t.ys,
                "items":t.items.iter().map(|i| (&i.label,i.group,i.rows,i.columns)).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "lines":skeleton.lines.iter().map(|l| (l.axis,l.kind.name(),l.rank,l.at,&l.owner,l.tree)).collect::<Vec<_>>(),
            "trees":skeleton.trees.iter().map(|t| (t.route,&t.net,&t.xs,&t.ys)).collect::<Vec<_>>(),
        }).to_string();
        return (text, solved.report.budget.unwrap().work);
    }
    (written, solved.report.budget.unwrap().work)
}

thread_local! { static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) }; }

#[no_mangle]
pub extern "C" fn parity_run(case: u32, work: u32) -> usize {
    OUTPUT.with(|out| {
        *out.borrow_mut() = record(case, work).0;
        out.borrow().len()
    })
}

#[no_mangle]
pub extern "C" fn parity_ptr() -> *const u8 {
    OUTPUT.with(|out| out.borrow().as_ptr())
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    for case in 0..12 {
        let limits: &[u32] = if case >= 10 {
            &[0]
        } else if case >= 8 {
            &[0, 64, 4096, 100_000_000]
        } else {
            &[0, 64, 4096, 1_000_000]
        };
        for &work in limits {
            let start = std::time::Instant::now();
            let (record, used) = record(case, work);
            println!("{}", serde_json::to_string(&(case, work, record, used, start.elapsed().as_secs_f64())).unwrap());
        }
    }
}
