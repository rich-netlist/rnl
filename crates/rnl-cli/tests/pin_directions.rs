//! Pin direction facts retain the owner's mirror freedom (11.2, 14.9).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::FactKind;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use rnl_solver::{resolve, SolverOptions};

fn read(version: &str, body: &str) -> Interpretation {
    let body = body.replace('\n', "\n* ");
    let source = format!("* directions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {body} }}\n.end\n");
    interpret_file(
        Path::new("directions.sp"),
        None,
        &MemoryProvider::new().with("directions.sp", &source),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn checked(body: &str) -> Interpretation {
    let r = read("0.4", body);
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

fn symbol(direction: &str, statements: &str) -> String {
    format!(
        r#"$S = @DEFINE-SYMBOL {{ %p = (2,0) @PIN(terminal=1,direction="{direction}") => %alias; %free = (0,2) @PIN(terminal=2); (0,0) (2,0) @LINE }}; {{#R1 #R2}} @SET-SYMBOL $S; #R1 @PLACE(3,4); {statements}"#
    )
}

#[test]
fn pin_orientation_is_a_fact_but_does_not_turn_the_unsolved_preview() {
    let before = checked(&symbol("left", ""));
    let after = checked(&symbol("left", "#R1::p @ORIENT(\"up\")"));
    assert_eq!(before.model.levels[0].items[0].orient, after.model.levels[0].items[0].orient);
    let fact = after.problems[0]
        .facts
        .iter()
        .find_map(|f| match &f.kind {
            FactKind::PinOrient { pin, codes } => Some((pin, codes)),
            _ => None,
        })
        .unwrap();
    assert_eq!(fact.0.pin, 0);
    assert_eq!(fact.0.direction, 180);
    assert_eq!(fact.1, &[Orient::R270]);
    assert_eq!(before.model.levels[0].items[0].at, after.model.levels[0].items[0].at);
}

#[test]
fn each_pin_direction_accepts_exactly_two_owner_orientations() {
    for local in ["left", "down", "right", "up"] {
        for wanted in ["left", "down", "right", "up"] {
            let mut accepted = Vec::new();
            for owner in Orient::ALL {
                let r = checked(&symbol(
                    local,
                    &format!("#R1::p @ORIENT(\"{wanted}\"); #R1 @ORIENT(\"{}\")", owner.as_str()),
                ));
                let p = &r.problems[0];
                let solved = resolve(p, &SolverOptions::default());
                assert!(solved.failed.is_empty(), "{local}/{wanted}/{owner:?}");
                if solved.discarded.is_empty() {
                    accepted.push(owner);
                }
            }
            assert_eq!(accepted.len(), 2, "{local}/{wanted}: {accepted:?}");
        }
    }
}

#[test]
fn a_pin_and_its_owner_can_choose_the_other_mirror_without_conflict() {
    let r = checked(&symbol("left", "#R1::p @ORIENT(\"left\"); #R1 @ORIENT(\"MX\")"));
    let p = &r.problems[0];
    let result = resolve(p, &SolverOptions::default());
    assert!(result.discarded.is_empty() && result.failed.is_empty(), "{result:?}");
    let owner = p.objects.iter().position(|object| object.label == "#R1").unwrap();
    assert_eq!(result.orientation(owner), Some(Orient::MX));
}

#[test]
fn a_pin_direction_list_is_one_fact_and_keeps_later_alternatives() {
    let r = checked(&symbol("left", "#R1::p @ORIENT([\"left\",\"down\"]); #R1 @ORIENT(\"R90\")"));
    let p = &r.problems[0];
    assert_eq!(p.facts.iter().filter(|f| matches!(f.kind, FactKind::PinOrient { .. })).count(), 1);
    let result = resolve(p, &SolverOptions::default());
    assert!(result.discarded.is_empty() && result.failed.is_empty());
}

#[test]
fn terminal_targets_use_the_last_bound_pin_and_mirrored_codes_use_left() {
    let r = checked(
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1,direction="up"); (2,0) @PIN(terminal=1,direction="left") };
        #R1 @SET-SYMBOL $S; #R1:1 @ORIENT("MY"); #R1 @ORIENT("MY")
    "#,
    );
    let p = &r.problems[0];
    let pin = p
        .facts
        .iter()
        .find_map(|f| match &f.kind {
            FactKind::PinOrient { pin, .. } => Some(pin),
            _ => None,
        })
        .unwrap();
    assert_eq!((pin.pin, pin.direction), (1, 180));
    assert!(resolve(p, &SolverOptions::default()).discarded.is_empty());
}

#[test]
fn a_pin_without_direction_fails_without_losing_other_members() {
    let r = read("0.4", &symbol("left", "{#R1::free #R1::p #R2} @ORIENT(\"down\")"));
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::PIN_DIRECTION_MISSING).count(), 1);
    assert_eq!(r.problems[0].facts.iter().filter(|f| matches!(f.kind, FactKind::PinOrient { .. })).count(), 1);
    assert_eq!(r.problems[0].facts.iter().filter(|f| matches!(f.kind, FactKind::Orient { .. })).count(), 1);
}

#[test]
fn pin_relations_compare_directions_and_reject_mixed_kinds() {
    let r = checked(&symbol(
        "left",
        "#R1::p #R2::p @ORIENT-RELATION(\"R180\"); #R1 @ORIENT(\"R90\"); #R2 @ORIENT(\"R270\")",
    ));
    let p = &r.problems[0];
    assert_eq!(p.facts.iter().filter(|f| matches!(f.kind, FactKind::PinOrientRelation { .. })).count(), 1);
    assert!(resolve(p, &SolverOptions::default()).discarded.is_empty());
    let bad = read("0.4", &symbol("left", "#R1::p #R2 @ORIENT-RELATION(\"R0\")"));
    assert!(bad.diagnostics().iter().any(|d| d.code == codes::CONSTRAINT_ORIENT_RELATION_MIXED_KINDS));
    let missing = read("0.4", &symbol("left", "#R1::p #R2::free @ORIENT-RELATION(\"R0\")"));
    assert!(missing.diagnostics().iter().any(|d| d.code == codes::PIN_DIRECTION_MISSING));
}

#[test]
fn a_mirrored_relation_on_one_pin_can_hold_and_a_quarter_turn_cannot() {
    let r = checked(&symbol("left", "#R1::p #R1::alias @ORIENT-RELATION(\"MY\")"));
    let result = resolve(&r.problems[0], &SolverOptions::default());
    assert!(result.failed.is_empty() && result.discarded.is_empty());
    let r = checked(&symbol("left", "#R1::p #R1::alias @ORIENT-RELATION(\"R90\")"));
    let result = resolve(&r.problems[0], &SolverOptions::default());
    assert_eq!(result.failed.len(), 1);
}

#[test]
fn a_relation_list_keeps_later_choices_and_expands_a_set_member_by_member() {
    let r = checked(&symbol(
        "left",
        r#"#R1::p {#R1::alias #R2::p} @ORIENT-RELATION(["R90","MX"]);
        {#R1 #R2} @ORIENT("R0")"#,
    ));
    let p = &r.problems[0];
    assert_eq!(p.facts.iter().filter(|f| matches!(f.kind, FactKind::PinOrientRelation { .. })).count(), 2);
    let result = resolve(p, &SolverOptions::without_time_limit());
    assert!(result.failed.is_empty() && result.discarded.is_empty(), "{result:?}");
}

#[test]
fn solving_and_writing_back_preserves_pin_directions() {
    let body = symbol(
        "left",
        r#"
        #R2 @PLACE(-3,4);
        #R1::p @ORIENT("up");
        #R1::p #R2::p @ORIENT-RELATION("R180")
    "#,
    );
    let r = checked(&body);
    let p = &r.problems[0];
    let result = rnl_solver::solve(p, &SolverOptions::without_time_limit());
    assert!(result.report.failed.is_empty() && result.report.discarded.is_empty(), "{:?}", result.report);
    let written = rnl_runtime::solution::write_record(p, &result.solution, "test", "");
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    let contents = format!(
        "* directions\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {} }}\n* {}\n.end\n",
        body.replace('\n', "\n* "), written.record.replace('\n', "\n* "),
    );
    let reread = interpret_file(
        Path::new("directions.sp"),
        None,
        &MemoryProvider::new().with("directions.sp", &contents),
        &Options { std: false, ..Options::default() },
    )
    .unwrap();
    assert!(!reread.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", reread.diagnostics());
    for (name, allowed) in [("#R1", [Orient::R270, Orient::MYR90]), ("#R2", [Orient::R90, Orient::MXR90])] {
        let item = reread.model.levels[0].items.iter().find(|item| item.object_id == name).unwrap();
        assert!(allowed.contains(&item.orient), "{name}: {:?}\n{}", item.orient, written.record);
    }
    assert!(
        reread.model.levels[0].constraints.iter().all(|c| c.status == "ok"),
        "{:?}",
        reread.model.levels[0].constraints
    );
}
