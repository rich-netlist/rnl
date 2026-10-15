//! PRIORITY on a set with GROUP (standard 5.2.1, 11.5, 14.8): it acts on the group itself, which
//! has no priority, only on and off. A disabled group is withheld from the solver; the constraints
//! written on the grouped set are not affected. Any other set is taken member by member. Before,
//! every set was split into its members, so a group could not be turned off at all. Several
//! objects given to one PRIORITY make one set whose referenced sets are spread (5.2.1); in a file
//! of the draft 0.3 each is taken as it is instead, so one statement turns off several groups
//! (draft 0.3, 7.2).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;

/// What a solver is given of the main level, and the errors shown.
struct Outcome {
    problem: LevelProblem,
    errors: Vec<String>,
}

impl Outcome {
    /// The groups given to the solver, each by its members.
    fn groups(&self) -> Vec<Vec<&str>> {
        let p = &self.problem;
        p.groups.iter().map(|g| g.members.iter().map(|m| p.objects[*m].label.as_str()).collect()).collect()
    }

    /// The constraints given to the solver, each with its effective priority.
    fn constraints(&self) -> Vec<(&str, Priority)> {
        self.problem.facts.iter().filter(|f| f.label.starts_with('%')).map(|f| (f.label.as_str(), f.priority)).collect()
    }
}

/// R1 to R6 in a row, with `statements`, one per line, in a record of the main file.
fn solve(statements: &str) -> Outcome {
    solve_in("0.2", false, statements)
}

/// The same in a file that declares `version`, read with the draft 0.3 or without it.
fn solve_in(version: &str, draft: bool, statements: &str) -> Outcome {
    let mut text = String::from("* groups\n");
    for k in 1..=6 {
        text += &format!("R{k} n{k} n{} 1k\n", k + 1);
    }
    text += &format!("* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n* @RNL {{\n");
    for line in statements.lines().filter(|l| !l.trim().is_empty()) {
        text += &format!("*   {}\n", line.trim());
    }
    text += "* }\n.end\n";
    let files = MemoryProvider::new().with("g.sp", &text);
    let options = Options { std: false, draft, ..Options::solver() };
    let r = interpret_file(Path::new("g.sp"), None, &files, &options).unwrap();
    let errors =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    Outcome { problem: r.problems.into_iter().next().unwrap(), errors }
}

#[test]
fn a_disabled_group_is_withheld_and_its_constraints_stay() {
    let r = solve(
        r#"
        %stage1_set = {#R1 #R2 #R5} @GROUP;
        %stage2_set = {#R3 #R4} @GROUP;
        %c1 = %stage1_set %stage2_set @DISTANCE("x", min=1);
        %stage1_set @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.groups(), [["#R3", "#R4"]]);
    // GROUP is not a constraint: the DISTANCE between the two stages still holds
    assert_eq!(r.constraints(), [("%c1", Priority::Normal)]);
}

#[test]
fn a_priority_on_a_group_only_turns_it_on_or_off() {
    // a later priority turns it on again (14.8); "low" and "high" change no constraint's priority
    let r = solve(
        r#"
        %stage1_set = {#R1 #R2} @GROUP;
        %stage2_set = {#R3 #R4} @GROUP;
        %c1 = %stage1_set %stage2_set @DISTANCE("x", min=1);
        %stage1_set @PRIORITY("disabled");
        %stage1_set @PRIORITY("low");
        %stage2_set @PRIORITY("high")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.groups(), [["#R1", "#R2"], ["#R3", "#R4"]]);
    assert_eq!(r.constraints(), [("%c1", Priority::Normal)]);
}

#[test]
fn a_priority_written_before_the_group_acts_on_the_group() {
    for statements in [
        // the name is bound later
        "%s @PRIORITY(\"disabled\"); %s = {#R1 #R2} @GROUP",
        // the set is grouped by a later statement
        "%s = {#R1 #R2}; %s @PRIORITY(\"disabled\"); %s @GROUP",
        // or later in the same pipe
        "{#R1 #R2} @PRIORITY(\"disabled\") => @GROUP",
    ] {
        let r = solve(&format!("{statements};\n{{#R3 #R4}} @GROUP"));
        assert!(r.errors.is_empty(), "{statements}: {:?}", r.errors);
        assert_eq!(r.groups(), [["#R3", "#R4"]], "{statements}");
    }
}

#[test]
fn a_set_without_group_is_taken_member_by_member() {
    let r = solve(
        r#"
        %c1 = {#R1 #R2} @ALIGN("y");
        %c2 = {#R3 #R4} @ALIGN("y");
        %c3 = {#R5 #R6} @ALIGN("y");
        %cs = {%c1 %c2};
        %cs @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.constraints(), [("%c3", Priority::Normal)]);
    // a set of instances without GROUP: PRIORITY reaches the first instance and fails there
    let r = solve("%s = {#R1 #R2}; %s @PRIORITY(\"disabled\")");
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains("PRIORITY takes constraints, usages and grouped sets only, not #R1 (14.8)"));
    // the same set of constraints with GROUP: the group is off, and the constraints stay
    let r = solve(
        r#"
        %c1 = {#R1 #R2} @ALIGN("y");
        %c2 = {#R3 #R4} @ALIGN("y");
        %cs = {%c1 %c2} @GROUP;
        %cs @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(r.groups().is_empty());
    assert_eq!(r.constraints(), [("%c2", Priority::Normal), ("%c1", Priority::Normal)]);
}

#[test]
fn a_group_in_a_disabled_usage_is_off() {
    let r = solve(
        r#"
        $PAIR = @DEFINE(roles=["a", "b"]) { %pair = {?a ?b} @GROUP; %row = {?a ?b} @ALIGN("y") };
        %u1 = #R1(a) #R2(b) @AS(priority="disabled") $PAIR;
        %u2 = #R3(a) #R4(b) @AS $PAIR;
        %u3 = #R5(a) #R6(b) @AS $PAIR;
        %u2 @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.groups(), [["#R5", "#R6"]]);
    assert_eq!(r.constraints(), [("%row", Priority::Normal)]);
}

#[test]
fn a_local_group_is_turned_off_from_outside() {
    let r = solve(
        r#"
        $PAIR = @DEFINE(roles=["a", "b"]) { %pair = {?a ?b} @GROUP; %row = {?a ?b} @ALIGN("y") };
        %u1 = #R1(a) #R2(b) @AS $PAIR;
        %u2 = #R3(a) #R4(b) @AS $PAIR;
        %u1::pair @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    // only the group expanded for u1 is off; its ALIGN stays
    assert_eq!(r.groups(), [["#R3", "#R4"]]);
    assert_eq!(r.constraints(), [("%row", Priority::Normal), ("%row", Priority::Normal)]);
}

#[test]
fn a_group_follows_the_order_of_overriding() {
    // the body turns its group off; the priority of a usage overrides that, and a statement from
    // outside through `::` overrides the priority of the usage (14.8)
    let r = solve(
        r#"
        $PAIR = @DEFINE(roles=["a", "b"]) { %pair = {?a ?b} @GROUP; %pair @PRIORITY("disabled") };
        %u1 = #R1(a) #R2(b) @AS $PAIR;
        %u2 = #R3(a) #R4(b) @AS(priority="high") $PAIR;
        %u3 = #R5(a) #R6(b) @AS(priority="disabled") $PAIR;
        %u3::pair @PRIORITY("normal")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.groups(), [["#R3", "#R4"], ["#R5", "#R6"]]);
}

/// Two sets with GROUP given to one PRIORITY, and a third group that stays on.
const TWO_GROUPS: &str = r#"
    %a_set = {#R1 #R2} @GROUP;
    %b_set = {#R3 #R4} @GROUP;
    {#R5 #R6} @GROUP;
    %a_set %b_set @PRIORITY("disabled")
"#;

#[test]
fn in_0_3_several_objects_are_each_taken_as_they_are() {
    // draft 0.3, 7.2: a set with GROUP among the objects is the group itself
    let r = solve_in("0.3", true, TWO_GROUPS);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert_eq!(r.groups(), [["#R5", "#R6"]]);

    // one with GROUP and one without, which is taken member by member as usual
    let r = solve_in(
        "0.3",
        true,
        r#"
        %a_set = {#R1 #R2} @GROUP;
        %c1 = {#R3 #R4} @ALIGN("y");
        %c2 = {#R5 #R6} @ALIGN("y");
        %c3 = {#R1 #R6} @ALIGN("x");
        %b_set = {%c1 %c2};
        %a_set %b_set @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(r.groups().is_empty());
    assert_eq!(r.constraints(), [("%c3", Priority::Normal)]);

    // a set of instances without GROUP fails at its first member, on its own
    let r = solve_in("0.3", true, "%a_set = {#R1 #R2} @GROUP; %s = {#R3 #R4}; %a_set %s @PRIORITY(\"disabled\")");
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains("PRIORITY takes constraints, usages and grouped sets only, not #R3 (14.8)"));
    assert!(r.groups().is_empty());

    // with a selection among them, at the geometry phase too
    let r = solve_in(
        "0.3",
        true,
        r#"
        $PAIR = @DEFINE(roles=["a", "b"]) { %row = {?a ?b} @ALIGN("y") };
        %u = #R3(a) #R4(b) @AS $PAIR;
        %a_set = {#R1 #R2} @GROUP;
        # @SELECT-USAGE $PAIR => %a_set @PRIORITY("disabled")
        "#,
    );
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(r.groups().is_empty());
    assert!(r.constraints().is_empty());
}

#[test]
fn in_0_2_several_objects_make_one_set_as_before() {
    // the sets are spread into their members, and PRIORITY fails at the first instance
    let r = solve(TWO_GROUPS);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains("PRIORITY takes constraints, usages and grouped sets only, not #R1 (14.8)"));
    assert_eq!(r.groups(), [["#R1", "#R2"], ["#R3", "#R4"], ["#R5", "#R6"]]);
    // the same with the draft read
    let draft = solve_in("0.2", true, TWO_GROUPS);
    assert_eq!((draft.errors, draft.problem), (r.errors, r.problem));
}

#[test]
fn a_file_of_0_3_without_the_draft_is_of_an_unsupported_version() {
    let r = solve_in("0.3", false, TWO_GROUPS);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains("unsupported version 0.3: no RNL of this file is interpreted"), "{:?}", r.errors);
    assert!(r.groups().is_empty());
}
