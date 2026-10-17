//! The order of overriding through nested usages (standard 9.4, 14.8, as the draft 0.3 makes it
//! clear in its 7.3, in every version). A PRIORITY written through `::` in the body of an enclosing
//! definition reaches the local from outside the inner usage only: it beats the inner usage's
//! priority and is still subject to the priority of the enclosing usage. One written from outside
//! directly on a local usage makes that usage the outermost one with a priority for what it
//! expands to, over the usages around it; one that a selection reaches is an ordinary statement
//! on its target, under the usages around it. Of the statements that are not subject to another
//! priority, the one on the innermost object wins, and on one object the later. Constraints and
//! groups follow the same order. Before, a statement written in an enclosing body counted as
//! written outside every usage, and then the outermost usage with a priority decided, even over
//! a statement from outside on a usage inside it.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_cli::solve::{solve as solve_file, SolveOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use rnl_solver::SolverOptions;

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

/// `$PAIR` groups its two roles, aligns them as `%row` and orders them as `%col`; `$OUTER` applies
/// it as `%in` and orders its own roles as `%c`, followed by the statements of `body`.
fn nested(body: &str) -> String {
    format!(
        "$PAIR = @DEFINE(roles=[\"a\", \"b\"]) {{ %pair = {{?a ?b}} @GROUP; %row = {{?a ?b}} @ALIGN(\"y\"); %col = {{?a ?b}} @ORDER(\"x\") }};\n\
         $OUTER = @DEFINE(roles=[\"a\", \"b\"]) {{ %in = ?a(a) ?b(b) @AS $PAIR; %c = {{?a ?b}} @ORDER(\"x\"){body} }};\n"
    )
}

/// The constraints expanded from usages that are given to the solver, by the outermost usage and
/// their name, with their priorities; and the outermost usages whose `%pair` is given.
fn given(p: &LevelProblem) -> (Vec<(&str, &str, Priority)>, Vec<&str>) {
    let facts = p.facts.iter().filter(|f| f.label.starts_with('%') && !f.chain.is_empty());
    let mut facts: Vec<(&str, &str, Priority)> =
        facts.map(|f| (p.usages[f.chain[0]].label.as_str(), f.label.as_str(), f.priority)).collect();
    facts.sort();
    (facts, kept(p).1)
}

#[test]
fn a_statement_from_outside_on_a_local_usage_beats_the_usages_around_it() {
    // %o1 is disabled, and so is its own %c; but %in, turned on from outside, is the outermost
    // usage with a priority for what it expands to, its group too. %o2::c turns that one on
    let p = solve(&format!(
        "{}{}",
        nested(""),
        r#"
        %o1 = #R1(a) #R2(b) @AS(priority="disabled") $OUTER;
        %o1::in @PRIORITY("high");
        %o2 = #R3(a) #R4(b) @AS(priority="disabled") $OUTER;
        %o2::c @PRIORITY("high")
        "#,
    ));
    let high = Priority::High;
    assert_eq!(given(&p), (vec![("%o1", "%col", high), ("%o1", "%row", high), ("%o2", "%c", high)], vec!["%o1"]));

    // the same from the body of an enclosing definition: %mid::in beats the priority of %mid, as
    // long as the usage of $TOP has none of its own
    let p = solve(&format!(
        "{}{}",
        nested(""),
        r#"
        $TOP = @DEFINE(roles=["a", "b"]) { %mid = ?a(a) ?b(b) @AS(priority="disabled") $OUTER; %mid::in @PRIORITY("low") };
        %o1 = #R1(a) #R2(b) @AS $TOP;
        %o2 = #R3(a) #R4(b) @AS(priority="normal") $TOP
        "#,
    ));
    let (low, normal) = (Priority::Low, Priority::Normal);
    let (facts, groups) = given(&p);
    assert_eq!(facts[..2], [("%o1", "%col", low), ("%o1", "%row", low)]);
    assert_eq!(facts[2..], [("%o2", "%c", normal), ("%o2", "%col", normal), ("%o2", "%row", normal)]);
    assert_eq!(groups, ["%o1", "%o2"]);
}

#[test]
fn on_one_chain_the_innermost_object_wins() {
    // whichever is written first
    let p = solve(&format!(
        "{}{}",
        nested(""),
        r#"
        %o1 = #R1(a) #R2(b) @AS $OUTER;
        %o1::in @PRIORITY("low");
        %o1::in::row @PRIORITY("high");
        %o2 = #R3(a) #R4(b) @AS $OUTER;
        %o2::in::row @PRIORITY("high");
        %o2::in @PRIORITY("low")
        "#,
    ));
    let (low, normal, high) = (Priority::Low, Priority::Normal, Priority::High);
    assert_eq!(
        given(&p).0,
        [
            ("%o1", "%c", normal),
            ("%o1", "%col", low),
            ("%o1", "%row", high),
            ("%o2", "%c", normal),
            ("%o2", "%col", low),
            ("%o2", "%row", high)
        ]
    );

    // a usage deeper on the chain beats the one around it: %o1::mid::in over %o1::mid
    let p = solve(&format!(
        "{}{}",
        nested(""),
        r#"
        $TOP = @DEFINE(roles=["a", "b"]) { %mid = ?a(a) ?b(b) @AS $OUTER };
        %o1 = #R1(a) #R2(b) @AS $TOP;
        %o1::mid @PRIORITY("low");
        %o1::mid::in @PRIORITY("high")
        "#,
    ));
    assert_eq!(given(&p).0, [("%o1", "%c", low), ("%o1", "%col", high), ("%o1", "%row", high)]);
}

#[test]
fn on_one_object_the_later_statement_wins() {
    let p = solve(&format!(
        "{}{}",
        nested(""),
        r#"
        %o1 = #R1(a) #R2(b) @AS $OUTER;
        %o1::in @PRIORITY("low");
        %o1::in @PRIORITY("high");
        %o2 = #R3(a) #R4(b) @AS $OUTER;
        %o2::in @PRIORITY("high");
        %o2::in @PRIORITY("low");
        %o2::c @PRIORITY("disabled");
        %o2::c @PRIORITY("normal")
        "#,
    ));
    let (low, normal, high) = (Priority::Low, Priority::Normal, Priority::High);
    assert_eq!(
        given(&p).0,
        [
            ("%o1", "%c", normal),
            ("%o1", "%col", high),
            ("%o1", "%row", high),
            ("%o2", "%c", normal),
            ("%o2", "%col", low),
            ("%o2", "%row", low)
        ]
    );

    // so too from outside at two depths: the body's statement takes the place of the usage's AS
    // (9.4), which is after %o1::in::row and before %o2::in::row
    let p = solve(&format!(
        "{}{}",
        nested("; %in::row @PRIORITY(\"low\")"),
        r#"
        %o1::in::row @PRIORITY("high");
        %o1 = #R1(a) #R2(b) @AS $OUTER;
        %o2 = #R3(a) #R4(b) @AS $OUTER;
        %o2::in::row @PRIORITY("high")
        "#,
    ));
    let rows: Vec<(&str, &str, Priority)> = given(&p).0.into_iter().filter(|f| f.1 == "%row").collect();
    assert_eq!(rows, [("%o1", "%row", low), ("%o2", "%row", high)]);
}

#[test]
fn a_statement_in_an_enclosing_body_stays_under_the_enclosing_usage_against_one_from_outside() {
    // %o1 is disabled, which blocks the body's %in::row; the top-level %o1::in decides, high. %o2
    // has no priority of its own: the body's statement is not blocked and is on the inner object
    let p = solve(&format!(
        "{}{}",
        definitions("", "%in::row @PRIORITY(\"low\")"),
        r#"
        %o1 = #R1(a) #R2(b) @AS(priority="disabled") $OUTER;
        %o1::in @PRIORITY("high");
        %o2 = #R3(a) #R4(b) @AS $OUTER;
        %o2::in @PRIORITY("high")
        "#,
    ));
    assert_eq!(kept(&p), (vec![("%o1", Priority::High), ("%o2", Priority::Low)], vec!["%o1", "%o2"]));
}

#[test]
fn a_priority_that_a_selection_reaches_is_one_of_its_targets_own() {
    // the selection reaches %in, which it does not name through `::`: an ordinary PRIORITY on it
    // (14.8), so the priority of %o1 beats it; without one, %in is disabled
    let p = solve(&format!(
        "{}{}",
        nested(""),
        r#"
        %o1 = #R1(a) #R2(b) @AS(priority="normal") $OUTER;
        %o2 = #R3(a) #R4(b) @AS $OUTER;
        # @SELECT-USAGE $PAIR => @PRIORITY("disabled")
        "#,
    ));
    let normal = Priority::Normal;
    let (facts, groups) = given(&p);
    assert_eq!(facts, [("%o1", "%c", normal), ("%o1", "%col", normal), ("%o1", "%row", normal), ("%o2", "%c", normal)]);
    assert_eq!(groups, ["%o1"]);

    // a statement through `::` in the body of a usage that a rule makes still names its object:
    // it beats the priority of %in, as in a usage applied by hand
    let p = solve(&format!(
        "{}{}",
        definitions("(priority=\"disabled\")", "%in::row @PRIORITY(\"low\")"),
        r#"
        %o1 = #R1(a) #R2(b) @AS $OUTER;
        # @SELECT-INSTANCE(name="R3") => #R4 @AS $OUTER
        "#,
    ));
    assert_eq!(kept(&p), (vec![("%o1", Priority::Low), ("a usage of $OUTER", Priority::Low)], vec![]));
}

/// `$MIXED` relates its two roles by three constraints of its body, one low, one high and one
/// without a priority.
const MIXED: &str = "$MIXED = @DEFINE(roles=[\"a\", \"b\"]) { %low = {?a ?b} @ALIGN(\"y\", priority=\"low\"); \
                     %high = {?a ?b} @ORDER(\"left-to-right\", priority=\"high\"); \
                     %plain = ?a ?b @DISTANCE(\"left-to-right\", min=2) }";

/// The facts given to the solver of a file of `version` with R1 to R8, `$MIXED` and `statements` in
/// a record whose head writes `head`, by label, with their priorities.
fn in_record(version: &str, head: &str, statements: &str) -> Vec<(String, Priority)> {
    let mut text = String::from("* priority of the record\n");
    for k in 1..=8 {
        text += &format!("R{k} n{k} n{} 1k\n", k + 1);
    }
    text += &format!("* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n* @RNL {{ {MIXED} }}\n");
    text += &format!("* @RNL{head} {{\n*   {statements}\n* }}\n.end\n");
    let files = MemoryProvider::new().with("p.sp", &text);
    let r = interpret_file(Path::new("p.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let p = r.problems.into_iter().next().unwrap();
    let mut facts: Vec<(String, Priority)> = p.facts.iter().map(|f| (f.label.clone(), f.priority)).collect();
    facts.sort();
    facts
}

fn facts(of: &[(&str, Priority)]) -> Vec<(String, Priority)> {
    of.iter().map(|(l, p)| (l.to_string(), *p)).collect()
}

#[test]
fn in_0_3_the_head_of_a_record_gives_no_priority_to_a_usage() {
    // the priority of a record head is the default of the statements of the record that take one,
    // but not of AS and ATTACH, unless it is "disabled": a usage has a priority only when it writes
    // one, or a PRIORITY statement gives it one, which overrides those of its body (14.8 of 0.3)
    let high = "(priority=\"high\")";
    let own = [("%high", Priority::High), ("%low", Priority::Low), ("%plain", Priority::Normal)];
    assert_eq!(in_record("0.3", high, "#R1 #R2 @AS $MIXED"), facts(&own));
    assert_eq!(in_record("0.3", high, "$MIXED @ATTACH #R1 #R2"), facts(&own));
    // the record's own statements still take its priority
    let mut with_own = facts(&own);
    with_own.push(("%row".to_string(), Priority::High));
    with_own.sort();
    assert_eq!(in_record("0.3", high, "#R1 #R2 @AS $MIXED; %row = {#R3 #R4} @ALIGN(\"y\")"), with_own);
    // a priority written on the usage, or given by PRIORITY, overrides its body as before
    let all = |p: Priority| facts(&[("%high", p), ("%low", p), ("%plain", p)]);
    assert_eq!(in_record("0.3", high, "#R1 #R2 @AS(priority=\"normal\") $MIXED"), all(Priority::Normal));
    assert_eq!(in_record("0.3", "", "#R1 #R2 @AS(priority=\"high\") $MIXED"), all(Priority::High));
    assert_eq!(in_record("0.3", high, "%m = #R1 #R2 @AS $MIXED; %m @PRIORITY(\"low\")"), all(Priority::Low));
    // a record of priority "disabled" turns off everything in it, the bodies of its usages too
    assert_eq!(in_record("0.3", "(priority=\"disabled\")", "#R1 #R2 @AS $MIXED"), []);
    // a file of 0.2 keeps the rule of 0.2: the record's priority is the usage's too
    assert_eq!(in_record("0.2", high, "#R1 #R2 @AS $MIXED"), all(Priority::High));
}

#[test]
fn a_usage_that_a_rule_makes_keeps_the_priorities_of_its_body_through_a_solve() {
    // a usage that a rule makes, with constraints of different priorities in its body: the solver
    // writes it out, in its record of priority high, without a priority, so that each of them reads
    // back with its own; or with the priority that the rule writes on it, as written, or that of
    // the record of the rule when it is "disabled", which then overrides them all; the high of the
    // record of the rule is no priority of the usage's (10.5, 14.8). $MARK hangs a point under an
    // open pin and wires it, so that the pin is wired once it is written out and the rule does not
    // make it again
    let netlist = "* rule-made\nR1 n1 n2 1k\nR2 n2 n3 1k\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n* @RNL {\n\
                *   $MARK = @DEFINE(roles=[[\"t\", \"terminal\"]]) { %p = (_, _); ?t::net @WIRE {?t %p}; \
                %low = {?t %p} @ALIGN(\"x\", priority=\"low\"); %high = %p ?t @DISTANCE(\"bottom-to-top\", min=2, priority=\"high\"); \
                %plain = %p ?t @DISTANCE(\"bottom-to-top\", max=6) };\n\
                *   $END_NETS = # @SELECT-NET(name=\"n3\");\n\
                *   $OPEN = $END_NETS @SELECT-TERMINAL(wired=false)\n* }\n\
                * @RNLHEAD {\n*   MAKE\n* }\n.end\n";
    // the constraints of the level as a viewer reads it, with the record of the solver, by kind
    // and priority
    let constraints = |text: &str| -> Vec<(String, String)> {
        let files = MemoryProvider::new().with("/w/t.sp", text);
        let r = interpret_file(Path::new("/w/t.sp"), None, &files, &Options::default()).unwrap();
        let errors: Vec<String> =
            r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
        assert!(errors.is_empty(), "{errors:?}");
        let mut of: Vec<(String, String)> =
            r.model.levels[0].constraints.iter().map(|c| (c.kind.clone(), c.priority.clone())).collect();
        of.sort();
        of
    };
    let mixed = ["low", "high", "normal"];
    let (high, disabled) = ("(priority=\"high\")", "(priority=\"disabled\")");
    for (head, make, written_as, priorities) in [
        ("", "$MARK @ATTACH $OPEN", "= $MARK @ATTACH #R2:", &mixed[..]),
        ("", "$MARK @ATTACH(priority=\"high\") $OPEN", "= $MARK @ATTACH(priority=\"high\") #R2:", &["high"][..]),
        (high, "$MARK @ATTACH $OPEN", "= $MARK @ATTACH #R2:", &mixed[..]),
        (disabled, "$MARK @ATTACH $OPEN", "= $MARK @ATTACH(priority=\"disabled\") #R2:", &["disabled"][..]),
    ] {
        let text = netlist.replace("HEAD", head).replace("MAKE", make);
        let before = constraints(&text);
        let mut given: Vec<&str> = before.iter().map(|(_, p)| p.as_str()).collect();
        given.sort();
        given.dedup();
        let mut want = priorities.to_vec();
        want.sort();
        assert_eq!(given, want, "{make}");
        let files = MemoryProvider::new().with("/w/t.sp", &text);
        let p = &interpret_file(Path::new("/w/t.sp"), None, &files, &Options::solver()).unwrap().problems[0];
        assert!(p.usages.iter().any(|u| u.definition == "$MARK" && u.rule_made), "{:?}", p.usages);
        let options = SolveOptions { solver: SolverOptions::without_time_limit(), ..SolveOptions::default() };
        let run = solve_file(Path::new("/w/t.sp"), &files, &options).unwrap();
        let written = &run.files[0].text;
        assert!(written.contains(written_as), "{written}");
        assert_eq!(constraints(written), before, "{written}");
        // every object placed, by the usage written out: the rule did not make it again
        let back = interpret_file(
            Path::new("/w/t.sp"),
            None,
            &MemoryProvider::new().with("/w/t.sp", written),
            &Options::default(),
        )
        .unwrap();
        assert!(back.model.levels[0].unplaced.is_empty(), "{:?} {written}", back.model.levels[0].unplaced);
    }
}
