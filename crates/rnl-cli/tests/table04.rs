//! TABLE preserves nesting and ranks the entire table as one fact (14.11).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::problem::{input_digest, Axis, FactKind, LevelProblem, TableCell};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

const SYMBOLS: &str = r#"$S = @DEFINE-SYMBOL { %p = (-1,0) @PIN(terminal=1) => %q; (1,0) @PIN(terminal=2); (-0.5,-0.5) (0.5,0.5) @RECTANGLE }; # @SELECT-INSTANCE => @SET-SYMBOL $S"#;
fn source(version: &str, body: &str) -> String {
    format!("* tables\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a 0 1k\nR2 b 0 2k\nR3 c 0 3k\nR4 d 0 4k\n* @RNL {{ {SYMBOLS}; {} }}\n", body.replace('\n', " "))
}
fn read_version(version: &str, body: &str) -> Interpretation {
    let text = source(version, body);
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}
fn read(body: &str) -> Interpretation {
    read_version("0.4", body)
}
fn clean(r: &Interpretation) {
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}
fn tables(p: &LevelProblem) -> Vec<&FactKind> {
    p.facts.iter().filter_map(|f| matches!(f.kind, FactKind::Table { .. }).then_some(&f.kind)).collect()
}
fn orders(r: &Interpretation) -> Vec<(Axis, String, String)> {
    let p = &r.problems[0];
    let FactKind::Table { axis, rows } = tables(p)[0] else { unreachable!() };
    rows.orders(*axis)
        .into_iter()
        .map(|(a, x, y)| (a, rnl_runtime::problem::point_name(p, x), rnl_runtime::problem::point_name(p, y)))
        .collect()
}
#[track_caller]
fn has_error(r: &Interpretation, code: rnl_runtime::diag::Code) {
    assert!(
        r.diagnostics().iter().any(|d| d.code == code && d.severity == Severity::Error),
        "expected {code:?}: {:?}; tables {:?}",
        r.diagnostics(),
        tables(&r.problems[0])
    );
    assert!(tables(&r.problems[0]).is_empty());
    assert!(r.problems[0].groups.is_empty());
}

#[test]
fn nested_lists_preserve_axes_and_only_make_one_fact_and_group() {
    let r = read("@TABLE layout: {{#R1 #R2} {#R3 #R4}}");
    clean(&r);
    let p = &r.problems[0];
    assert_eq!(tables(p).len(), 1);
    assert_eq!(p.groups.len(), 1);
    let actual = orders(&r);
    assert_eq!(actual.iter().filter(|(a, _, _)| *a == Axis::Y).count(), 4);
    assert_eq!(actual.iter().filter(|(a, _, _)| *a == Axis::X).count(), 2);
    assert!(!p.facts.iter().any(|f| matches!(f.kind, FactKind::Order { .. } | FactKind::Distance { .. })));
    let FactKind::Table { rows: TableCell::List(rows), .. } = tables(p)[0] else { panic!("missing list") };
    assert_eq!(rows.len(), 2);
    let data = serde_json::to_value(rnl_view::constraint(
        p,
        p.facts.iter().position(|f| matches!(f.kind, FactKind::Table { .. })).unwrap(),
    ))
    .unwrap();
    assert!(data.to_string().contains("TABLE"));
    assert!(data.to_string().contains("list"));
}

#[test]
fn singleton_nested_row_is_not_split_and_transpose_reverses_axes() {
    for (body, axis) in [
        ("@TABLE {{#R1 #R2 #R3}}", Axis::X),
        ("@TABLE(transpose=true) {{#R1 #R2 #R3}}", Axis::Y),
        ("%s={#R1 #R2 #R3}; @TABLE %s", Axis::Y),
    ] {
        let r = read(body);
        clean(&r);
        assert_eq!(orders(&r).len(), 2);
        assert!(orders(&r).iter().all(|(a, _, _)| *a == axis));
    }
}

#[test]
fn reference_items_keep_nesting_and_final_set_edits() {
    let r = read("%row={#R1 #R2}; %outer={%row #R4}; @TABLE %outer; #R2 @REMOVE-FROM %row; #R3 @ADD-TO %row");
    clean(&r);
    let pairs = orders(&r);
    assert_eq!(pairs.len(), 3);
    assert!(pairs.contains(&(Axis::X, "#R1".into(), "#R3".into())));
    assert!(!pairs.iter().any(|(_, a, b)| a == "#R2" || b == "#R2"));
}

#[test]
fn empty_lists_bridge_neighbors_and_one_cell_has_no_group() {
    let r = read("@TABLE {#R1 {} {{}} #R2}");
    clean(&r);
    assert_eq!(orders(&r), vec![(Axis::Y, "#R1".into(), "#R2".into())]);
    for body in ["@TABLE {}", "@TABLE {{#R1}}"] {
        let r = read(body);
        clean(&r);
        assert!(orders(&r).is_empty());
        assert!(r.problems[0].groups.is_empty());
    }
}

#[test]
fn duplicate_failed_and_unlocatable_members_fail_the_whole_table() {
    has_error(&read("@TABLE {{#R1 #R2} {#R3 #R1}}"), codes::TABLE_OBJECT_IN_TWO_CELLS);
    has_error(&read("@TABLE {#R1 &a}"), codes::TABLE_MEMBER_NOT_LOCATABLE);
    has_error(&read("%bad=#missing; @TABLE {#R1 %bad}"), codes::TABLE_MEMBER_FAILED);
    has_error(&read("@TABLE {#R1::p #R1::q}"), codes::TABLE_OBJECT_IN_TWO_CELLS);
}

#[test]
fn usage_point_is_a_cell_but_the_group_expands_its_roles() {
    let r = read(r#"$D = @DEFINE(roles=["a","b"]) {}; %u=#R1 #R2 @AS $D; @TABLE {%u #R3}"#);
    clean(&r);
    assert_eq!(orders(&r), vec![(Axis::Y, "%u".into(), "#R3".into())]);
    let p = &r.problems[0];
    assert_eq!(
        p.groups[0].members.iter().map(|i| p.objects[*i].label.as_str()).collect::<Vec<_>>(),
        ["#R1", "#R2", "#R3"]
    );
}

#[test]
fn priority_can_disable_and_reenable_the_whole_table() {
    let r = read(r#"%t={#R1 #R2} @TABLE; %t @PRIORITY("disabled")"#);
    clean(&r);
    assert!(tables(&r.problems[0]).is_empty());
    assert!(r.problems[0].groups.is_empty());
    let r = read(r#"%t={#R1 #R2} @TABLE(priority="disabled"); %t @PRIORITY("high")"#);
    clean(&r);
    assert_eq!(tables(&r.problems[0]).len(), 1);
    assert_eq!(r.problems[0].groups.len(), 1);
}

#[test]
fn nesting_limits_fail_without_partial_facts() {
    let body = format!("@TABLE {}#R1{}", "{".repeat(65), "}".repeat(65));
    has_error(&read(&body), codes::TABLE_TOO_DEEP);
}

#[test]
fn digest_distinguishes_nesting_and_transposition() {
    let a = read("@TABLE {#R1 #R2}");
    let b = read("@TABLE {{#R1 #R2}}");
    let c = read("@TABLE(transpose=true) {#R1 #R2}");
    for r in [&a, &b, &c] {
        clean(r);
    }
    assert_ne!(input_digest(&a.problems[0]), input_digest(&b.problems[0]));
    assert_ne!(input_digest(&a.problems[0]), input_digest(&c.problems[0]));
}

#[test]
fn older_files_do_not_gain_table() {
    for version in ["0.2", "0.3"] {
        let r = read_version(version, "@TABLE {#R1 #R2}");
        assert!(tables(&r.problems[0]).is_empty());
        assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error && d.message.contains("TABLE")));
    }
}

#[test]
fn solver_keeps_or_discards_the_entire_table() {
    for priority in ["low", "high"] {
        let r = read(&format!(
            "%t = {{#R1 #R2 #R3}} @TABLE(priority=\"{priority}\"); #R1 @PLACE(0,0); #R2 @PLACE(0,3); #R3 @PLACE(0,-3)"
        ));
        clean(&r);
        let p = &r.problems[0];
        let k = p.facts.iter().position(|f| matches!(f.kind, FactKind::Table { .. })).unwrap();
        let result = rnl_solver::resolve(p, &rnl_solver::SolverOptions::without_time_limit());
        assert!(result.failed.is_empty(), "{:?}", result.failed);
        if priority == "low" {
            assert_eq!(result.discarded.iter().map(|d| d.fact).collect::<Vec<_>>(), [k]);
            assert!(!result.kept.contains(&k));
        } else {
            assert!(result.kept.contains(&k));
            assert!(!result.discarded.is_empty());
            let FactKind::Table { axis, rows } = &p.facts[k].kind else { unreachable!() };
            for (axis, a, b) in rows.orders(*axis) {
                let a = result.point(p, a).unwrap();
                let b = result.point(p, b).unwrap();
                let gap = if axis == Axis::X { b.0 - a.0 } else { a.1 - b.1 };
                assert!(gap >= 1.0 - 2e-6);
            }
        }
        assert_eq!(p.groups.len(), 1);
    }
}

#[test]
fn solver_satisfies_both_axes_in_nested_tables() {
    let r = read("@TABLE {{#R1 #R2} {#R3 #R4}}");
    clean(&r);
    let p = &r.problems[0];
    let result = rnl_solver::resolve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(result.failed.is_empty() && result.discarded.is_empty());
    let FactKind::Table { axis, rows } = tables(p)[0] else { unreachable!() };
    for (axis, a, b) in rows.orders(*axis) {
        let a = result.point(p, a).unwrap();
        let b = result.point(p, b).unwrap();
        let gap = if axis == Axis::X { b.0 - a.0 } else { a.1 - b.1 };
        assert!(gap >= 1.0 - 2e-6);
    }
}

#[test]
fn selections_and_generated_usages_supply_cells_in_creation_order() {
    let r = read("%s = # @SELECT-INSTANCE; @TABLE %s");
    clean(&r);
    assert_eq!(orders(&r).len(), 3);
    let r = read(r#"$D = @DEFINE(roles=["a"]) {}; %u = # @SELECT-INSTANCE => @AS $D; @TABLE %u"#);
    clean(&r);
    let p = &r.problems[0];
    let FactKind::Table { rows, .. } = tables(p)[0] else { unreachable!() };
    assert_eq!(rows.points().len(), 4);
    assert_eq!(orders(&r).len(), 3);
}

#[test]
fn late_failed_role_fails_the_table_and_removes_its_group() {
    let r = read(r#"$D = @DEFINE(roles=[["i","instance"]]) {}; %u = #R1::p @AS $D; @TABLE {%u #R2}"#);
    has_error(&r, codes::TABLE_MEMBER_FAILED);
}

#[test]
fn indirect_rows_keep_failure_even_after_a_nested_removal() {
    let r = read("%bad = #missing; %r = {#R1 %bad #R2}; %s={%r #R3}; #R2 @REMOVE-FROM %s; @TABLE %s");
    has_error(&r, codes::TABLE_MEMBER_FAILED);
}

#[test]
fn inherited_priority_and_external_override_control_one_table() {
    let body = r#"$D = @DEFINE(roles=["a","b"]) { %t = {?a ?b} @TABLE }; %u = #R1 #R2 @AS(priority="disabled") $D"#;
    let r = read(body);
    clean(&r);
    assert!(tables(&r.problems[0]).is_empty());
    assert!(r.problems[0].groups.is_empty());
    let r = read(&format!("{body}; %u::t @PRIORITY(\"high\")"));
    clean(&r);
    assert_eq!(tables(&r.problems[0]).len(), 1);
    assert_eq!(r.problems[0].groups.len(), 1);
}

#[test]
fn limits_count_cross_item_point_pairs_before_expansion() {
    let left = (0..65).map(|i| format!("({i},100)")).collect::<Vec<_>>().join(" ");
    let right = (0..65).map(|i| format!("({i},0)")).collect::<Vec<_>>().join(" ");
    has_error(&read(&format!("@TABLE {{{{{left}}} {{{right}}}}}")), codes::TABLE_TOO_MANY_ORDERS);
}

#[test]
fn terminal_and_named_pin_cells_convert_after_symbols() {
    let r = read("@TABLE {#R1:1 #R2::p}");
    clean(&r);
    assert_eq!(orders(&r).len(), 1);
    has_error(&read("@TABLE {#R1:1 #R1::p}"), codes::TABLE_OBJECT_IN_TWO_CELLS);
}

#[test]
fn failed_body_cannot_contribute_a_table_on_an_external_set() {
    let r = read(
        r#"%s={#R2 #R3}; $D = @DEFINE(roles=[["i","instance"],"{items}"]) { @TABLE ?items }; %u=#R1::p %s @AS $D"#,
    );
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error));
    assert!(tables(&r.problems[0]).is_empty());
    assert!(r.problems[0].groups.is_empty());
}

#[test]
fn grouped_set_priority_does_not_change_the_table_fact() {
    let r = read(r#"%s={#R1 #R2}; @TABLE %s; %s @PRIORITY("disabled")"#);
    clean(&r);
    assert_eq!(tables(&r.problems[0]).len(), 1);
    assert!(r.problems[0].groups.is_empty());
}

#[test]
fn table_check_uses_both_axes_and_keeps_group_after_conflict() {
    let r = read(
        r#"@TABLE(priority="low") {{#R1 #R2} {#R3 #R4}}; #R1 @PLACE(3,3); #R2 @PLACE(0,3); #R3 @PLACE(0,0); #R4 @PLACE(3,0)"#,
    );
    clean(&r);
    assert_eq!(r.model.levels[0].constraints.iter().find(|c| c.kind == "TABLE").unwrap().status, "dropped");
    assert_eq!(r.problems[0].groups.len(), 1);
    let kind = tables(&r.problems[0])[0];
    let at = |point: &rnl_runtime::problem::PointRef, axis: Axis| {
        let rnl_runtime::problem::PointRef::Origin(i) = point else { return None };
        let xy = [(3.0, 3.0), (0.0, 3.0), (0.0, 0.0), (3.0, 0.0)][*i];
        Some(if axis == Axis::X { xy.0 } else { xy.1 })
    };
    assert_eq!(rnl_solver::fact_holds(kind, &at, &|_| None, 1e-6), Some(false));
}

#[test]
fn table_availability_follows_the_record_writer_in_mixed_includes() {
    for (main, child, count) in [("0.4", "0.3", 1), ("0.3", "0.4", 0)] {
        let source=format!("* mixed\n* @RNL(type=\"header\") {{ @VERSION(\"{main}\") }}\n.inc child.sp\n* @RNL(scope=\"child\") {{ @TABLE {{#R1 #R2}} }}\n");
        let included=format!("* included\n* @RNL(type=\"header\") {{ @VERSION(\"{child}\") }}\n.subckt child a b\nR1 a 0 1k\nR2 b 0 1k\n* @RNL {{ {SYMBOLS} }}\n.ends\n");
        let files = MemoryProvider::new().with("main.sp", &source).with("child.sp", &included);
        let r =
            interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
        assert_eq!(
            tables(r.problems.iter().find(|p| p.title == "child").unwrap()).len(),
            count,
            "{:?}",
            r.diagnostics()
        );
        if count == 1 {
            clean(&r);
        } else {
            assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error && d.message.contains("TABLE")));
        }
    }
}

#[test]
fn solved_table_writes_back_and_reads_as_one_satisfied_fact() {
    let body = "%t = {{#R1 #R2} {#R3 #R4}} @TABLE";
    let r = read(body);
    clean(&r);
    let p = &r.problems[0];
    let solved = rnl_solver::solve(p, &rnl_solver::SolverOptions::without_time_limit());
    assert!(solved.report.failed.is_empty() && solved.report.discarded.is_empty());
    let written = rnl_runtime::solution::write_record(
        p,
        &solved.solution,
        rnl_solver::AUTHOR,
        &rnl_runtime::problem::sig2(p, ""),
    );
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    let text = source("0.4", body) + &written.record.lines().map(|line| format!("* {line}\n")).collect::<String>();
    let files = MemoryProvider::new().with("main.sp", &text);
    let viewed =
        interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    clean(&viewed);
    assert_eq!(viewed.model.levels[0].constraints.iter().find(|c| c.kind == "TABLE").unwrap().status, "ok");
    let again =
        interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    clean(&again);
    assert_eq!(tables(&again.problems[0]).len(), 1);
    assert_eq!(input_digest(p), input_digest(&again.problems[0]));
}

#[test]
fn failed_member_diagnostic_keeps_its_original_cause() {
    let r = read("%bad = #missing; @TABLE {#R1 %bad}");
    has_error(&r, codes::TABLE_MEMBER_FAILED);
    let diagnostics = r.diagnostics();
    let error = diagnostics.iter().find(|d| d.code == codes::TABLE_MEMBER_FAILED).unwrap();
    assert!(error.cause.is_some());
    assert!(error.loc.is_some());
}

#[test]
fn new_table_words_do_not_expand_old_alias_originals() {
    for version in ["0.2", "0.3", "0.4"] {
        let head = r#"; @ALIAS("@T","@TABLE"); @ALIAS("flip","transpose")"#;
        let text = source(version, "{#R1 #R2} @T(flip=true)")
            .replace(&format!("@VERSION(\"{version}\")"), &format!("@VERSION(\"{version}\"){head}"));
        let r = interpret_file(
            Path::new("t.sp"),
            None,
            &MemoryProvider::new().with("t.sp", &text),
            &Options { std: false, ..Options::solver() },
        )
        .unwrap();
        if version == "0.4" {
            clean(&r);
            assert!(r.problems.iter().any(|p| p.facts.iter().any(|f| matches!(f.kind, FactKind::Table { .. }))));
        } else {
            assert_eq!(
                r.diagnostics()
                    .iter()
                    .filter(|d| d.code == rnl_runtime::diag::codes::ALIAS_ORIGINAL_NOT_STANDARD)
                    .count(),
                2
            );
            assert!(r.diagnostics().iter().any(
                |d| d.code == rnl_runtime::diag::codes::KEYWORD_UNKNOWN && d.message == "unknown keyword @T (16.4)"
            ));
        }
    }
}

#[test]
fn old_table_keyword_spelling_can_still_be_an_alias_and_unknown_text_is_unchanged() {
    for version in ["0.2", "0.3"] {
        let text = source(version, r#"(0,0) @TABLE("old alias")"#).replace(
            &format!("@VERSION(\"{version}\")"),
            &format!("@VERSION(\"{version}\"); @ALIAS(\"@TABLE\",\"@TEXT\")"),
        );
        let r = interpret_file(
            Path::new("t.sp"),
            None,
            &MemoryProvider::new().with("t.sp", &text),
            &Options { std: false, ..Options::solver() },
        )
        .unwrap();
        clean(&r);
        let r = read_version(version, "{#R1 #R2} @TABLE");
        assert!(
            r.diagnostics()
                .iter()
                .any(|d| d.code == rnl_runtime::diag::codes::KEYWORD_UNKNOWN
                    && d.message == "unknown keyword @TABLE (16.4)"),
            "{:?}",
            r.diagnostics()
        );
    }
}

#[test]
fn table_aliases_imported_and_declared_in_packages_keep_version_boundaries() {
    for version in ["0.2", "0.3", "0.4"] {
        let text = source(version, "{#R1 #R2} @T(flip=true)").replace(
            &format!("@VERSION(\"{version}\")"),
            &format!("@VERSION(\"{version}\"); @USE-ALIASES(name=\"alias\"); @USE-PACKAGE(name=\"ordinary\")"),
        );
        let a=format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"alias\",version=\"1.0\"); @ALIAS(\"@T\",\"@TABLE\"); @ALIAS(\"flip\",\"transpose\") }}\n");
        let p=format!("@RNL(type=\"header\") {{ @VERSION(\"{version}\"); @PACKAGE(name=\"ordinary\",version=\"1.0\"); @ALIAS(\"flip\",\"transpose\") }}\n@RNL {{ $D = @DEFINE(roles=[\"p\"]) {{}} }}\n");
        let mut fs = MemoryProvider::new().with("t.sp", &text).with("pkg/a.rnl", &a).with("pkg/p.rnl", &p);
        fs.package_dirs.push("pkg".into());
        let r = interpret_file(Path::new("t.sp"), None, &fs, &Options { std: false, ..Options::solver() }).unwrap();
        if version == "0.4" {
            clean(&r);
            assert!(r.problems.iter().any(|p| p.facts.iter().any(|f| matches!(f.kind, FactKind::Table { .. }))));
        } else {
            assert_eq!(
                r.diagnostics()
                    .iter()
                    .filter(|d| d.code == rnl_runtime::diag::codes::USE_ALIASES_ORIGINAL_NOT_IN_VERSION)
                    .count(),
                2
            );
            assert!(r.diagnostics().iter().any(|d| d.code == rnl_runtime::diag::codes::ALIAS_ORIGINAL_NOT_STANDARD));
        }
    }
}
