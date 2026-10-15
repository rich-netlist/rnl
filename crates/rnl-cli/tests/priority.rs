//! The order of overriding through nested usages (standard 9.4, 14.8). A PRIORITY written through
//! `::` in the body of an enclosing definition reaches the local from outside the inner usage
//! only: it beats the inner usage's priority and is still subject to the priority of the enclosing
//! usage. One written at the top level beats both. Constraints and groups follow the same order.
//! Before, a statement written in an enclosing body counted as written outside every usage.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;

/// `$PAIR` groups its two roles and aligns them; `$OUTER` applies `$PAIR` as `%in`, with what
/// `inner` adds to that statement, followed by the statements of `body`.
fn definitions(inner: &str, body: &str) -> String {
    format!(
        "$PAIR = @DEFINE(roles=[\"a\", \"b\"]) {{ %pair = {{?a ?b}} @GROUP; %row = {{?a ?b}} @ALIGN(\"y\") }};\n\
         $OUTER = @DEFINE(roles=[\"a\", \"b\"]) {{ %in = ?a(a) ?b(b) @AS{inner} $PAIR; {body} }};\n"
    )
}

/// What a solver is given of the main level, with R1 to R8 and `statements` in a record.
fn solve(statements: &str) -> LevelProblem {
    let mut text = String::from("* priority\n");
    for k in 1..=8 {
        text += &format!("R{k} n{k} n{} 1k\n", k + 1);
    }
    text += "* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n* @RNL {\n";
    for line in statements.lines().filter(|l| !l.trim().is_empty()) {
        text += &format!("*   {}\n", line.trim());
    }
    text += "* }\n.end\n";
    let files = MemoryProvider::new().with("p.sp", &text);
    let r = interpret_file(Path::new("p.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    r.problems.into_iter().next().unwrap()
}

/// The outermost usages whose `%row` is given to the solver, with its priority, and those whose
/// `%pair` is: a disabled constraint or group is left out.
fn kept(p: &LevelProblem) -> (Vec<(&str, Priority)>, Vec<&str>) {
    let outermost = |chain: &[usize]| p.usages[chain[0]].label.as_str();
    let rows = p.facts.iter().filter(|f| f.label == "%row").map(|f| (outermost(&f.chain), f.priority));
    let mut rows: Vec<(&str, Priority)> = rows.collect();
    rows.sort();
    // a group by its first member, R1 for %o1, R3 for %o2 and so on
    let groups = p
        .groups
        .iter()
        .map(|g| match p.objects[g.members[0]].label.as_str() {
            "#R1" => "%o1",
            "#R3" => "%o2",
            "#R5" => "%o3",
            "#R7" => "%o4",
            other => panic!("a group from {other}"),
        })
        .collect();
    (rows, groups)
}

#[test]
fn a_statement_in_an_enclosing_body_gives_way_to_the_enclosing_usage() {
    // the body of $OUTER turns the local of %in on again: %o1 is disabled, which still holds;
    // %o2 is "normal" by its own parameter, which decides over the body's "disabled"
    let p = solve(&format!(
        "{}{}",
        definitions("", "%in::row @PRIORITY(\"normal\"); %in::pair @PRIORITY(\"normal\")"),
        r#"
        %o1 = #R1(a) #R2(b) @AS(priority="disabled") $OUTER;
        $OUTER2 = @DEFINE(roles=["a", "b"]) { %in = ?a(a) ?b(b) @AS $PAIR; %in::row @PRIORITY("disabled"); %in::pair @PRIORITY("disabled") };
        %o2 = #R3(a) #R4(b) @AS(priority="normal") $OUTER2
        "#,
    ));
    assert_eq!(kept(&p), (vec![("%o2", Priority::Normal)], vec!["%o2"]));
}

#[test]
fn a_statement_in_an_enclosing_body_beats_the_inner_usage() {
    // %in is disabled by its own parameter; $OUTER's body turns its row and its group on again,
    // as long as the usage of $OUTER has no priority of its own
    let p = solve(&format!(
        "{}{}",
        definitions("(priority=\"disabled\")", "%in::row @PRIORITY(\"low\"); %in::pair @PRIORITY(\"normal\")"),
        r#"
        %o1 = #R1(a) #R2(b) @AS $OUTER;
        %o2 = #R3(a) #R4(b) @AS(priority="disabled") $OUTER
        "#,
    ));
    assert_eq!(kept(&p), (vec![("%o1", Priority::Low)], vec!["%o1"]));
}

#[test]
fn a_statement_at_the_top_level_beats_every_usage() {
    let p = solve(&format!(
        "{}{}",
        definitions("(priority=\"disabled\")", "%in::row @PRIORITY(\"disabled\"); %in::pair @PRIORITY(\"disabled\")"),
        r#"
        %o1 = #R1(a) #R2(b) @AS(priority="disabled") $OUTER;
        %o2 = #R3(a) #R4(b) @AS $OUTER;
        %o1::in::row @PRIORITY("high");
        %o1::in::pair @PRIORITY("normal")
        "#,
    ));
    assert_eq!(kept(&p), (vec![("%o1", Priority::High)], vec!["%o1"]));
}

#[test]
fn the_priority_of_a_usage_follows_the_same_order() {
    // $OUTER's body disables %in; written at the top level before the usage, and so earlier,
    // `%o2::in` still comes from outside and wins
    let p = solve(&format!(
        "{}{}",
        definitions("", "%in @PRIORITY(\"disabled\")"),
        r#"
        %o2::in @PRIORITY("low");
        %o1 = #R1(a) #R2(b) @AS $OUTER;
        %o2 = #R3(a) #R4(b) @AS $OUTER
        "#,
    ));
    assert_eq!(kept(&p), (vec![("%o2", Priority::Low)], vec!["%o2"]));
}
