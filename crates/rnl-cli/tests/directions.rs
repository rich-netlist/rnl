//! Positional facts written with their direction (14.1, 0.4): ORDER, DISTRIBUTE and DISTANCE take
//! one of four directions, DISTRIBUTE as ORDER does, and "x" or "y" alone is an error; ORIENT takes a
//! list of codes and the words for the four turns (11.2), and ORIENT-RELATION a list of relations
//! and relates one object to one object (14.9). Files of 0.2 and 0.3 read as they did.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::geom::Orient;
use rnl_runtime::problem::{input_digest, Axis, FactKind, LevelProblem, Sense};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// A netlist of `version` with R1 to R4 and M1 to M4, and `body` as its last record.
fn netlist(version: &str, body: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nR1 a b 1k\nR2 b c 1k\nR3 c d 1k\nR4 d 0 1k\n\
         .model nch nmos level=1\nM1 a b c 0 nch\nM2 b c d 0 nch\nM3 c d a 0 nch\nM4 d a b 0 nch\n* @RNL {{ {body} }}\n.end\n"
    )
}

fn read(version: &str, body: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", &netlist(version, body));
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn errors(r: &Interpretation) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| (d.code, r.sink.render(d))).collect()
}

/// The problem of the level, with no error.
fn problem(version: &str, body: &str) -> LevelProblem {
    let r = read(version, body);
    assert_eq!(errors(&r), Vec::new(), "{body}");
    r.problems.into_iter().next().unwrap()
}

/// The one error of `body` in a file of `version`, of the kind `code`; its text.
fn error(version: &str, body: &str, code: Code) -> String {
    let e = errors(&read(version, body));
    assert_eq!(e.len(), 1, "{body}: {e:?}");
    assert_eq!(e[0].0, code, "{body}: {e:?}");
    e[0].1.clone()
}

#[test]
fn distribute_takes_a_direction_in_0_4() {
    let p = problem("0.4", "{#R1 #R2 #R3} @DISTRIBUTE(\"right-to-left\")");
    assert!(
        p.facts.iter().any(|f| matches!(&f.kind, FactKind::Distribute { axis: Axis::X, sense: Sense::Decreasing, members } if members.len() == 3)),
        "{:?}",
        p.facts
    );
    let p = problem("0.4", "{#R1 #R2 #R3} @DISTRIBUTE(direction=\"bottom-to-top\")");
    assert!(p
        .facts
        .iter()
        .any(|f| matches!(&f.kind, FactKind::Distribute { axis: Axis::Y, sense: Sense::Increasing, .. })));
    // 0.3 takes the axis, with no sense; a direction is not its value
    let p = problem("0.3", "{#R1 #R2 #R3} @DISTRIBUTE(\"x\")");
    assert!(p.facts.iter().any(|f| matches!(&f.kind, FactKind::Distribute { sense: Sense::Either, .. })));
    error("0.3", "{#R1 #R2 #R3} @DISTRIBUTE(\"left-to-right\")", codes::PARAM_BAD_ENUM);
}

#[test]
fn an_axis_without_a_direction_is_an_error_in_0_4() {
    for body in [
        "{#R1 #R2} @ORDER(\"x\")",
        "{#R1 #R2} @ORDER(direction=\"y\")",
        "{#R1 #R2 #R3} @DISTRIBUTE(\"y\")",
        "{#R1 #R2 #R3} @DISTRIBUTE(axis=\"x\")",
        "#R1 #R2 @DISTANCE(\"x\", min=1)",
    ] {
        let e = error("0.4", body, codes::PARAM_AXIS_WITHOUT_DIRECTION);
        assert!(e.contains("takes a direction, one of \"left-to-right\""), "{body}: {e}");
    }
    // 0.3 reads them as it did
    for body in ["{#R1 #R2} @ORDER(\"x\")", "#R1 #R2 @DISTANCE(\"x\", min=1)"] {
        problem("0.3", body);
    }
}

#[test]
fn orient_takes_a_list_of_codes_and_the_words_for_the_turns_in_0_4() {
    let p = problem("0.4", "#M1 @ORIENT([\"MY\", \"R0\"]); #M2 @ORIENT(\"right\"); #M3 @ORIENT([\"down\"])");
    let orient = |name: &str| {
        let k = p.objects.iter().position(|o| o.label == name).unwrap();
        p.facts.iter().position(|f| matches!(f.kind, FactKind::Orient { object, .. } if object == k)).unwrap()
    };
    // the fact has the first code, which a renderer that does not solve draws, and the list is beside it
    assert!(matches!(p.facts[orient("#M1")].kind, FactKind::Orient { code: Orient::MY, .. }));
    assert_eq!(p.orient_lists, vec![(orient("#M1"), vec![Orient::MY, Orient::R0])]);
    // a word is another spelling of its code; a list of one code is that code
    assert!(matches!(p.facts[orient("#M2")].kind, FactKind::Orient { code: Orient::R180, .. }));
    assert!(matches!(p.facts[orient("#M3")].kind, FactKind::Orient { code: Orient::R90, .. }));
    // 0.3 takes one code, and no word
    error("0.3", "#M1 @ORIENT([\"MY\", \"R0\"])", codes::PARAM_BAD_ENUM);
    error("0.3", "#M1 @ORIENT(\"left\")", codes::PARAM_BAD_ENUM);
    error("0.4", "#M1 @ORIENT([])", codes::PARAM_EMPTY_LIST);
    error("0.4", "#M1 @ORIENT([\"R0\", \"R45\"])", codes::PARAM_BAD_ENUM);
}

#[test]
fn orient_relation_takes_a_list_and_relates_one_object_to_one_in_0_4() {
    let p = problem("0.4", "#M1 #M2 @ORIENT-RELATION([\"R0\", \"MY\"])");
    let k = p.facts.iter().position(|f| matches!(f.kind, FactKind::OrientRelation { .. })).unwrap();
    assert!(matches!(p.facts[k].kind, FactKind::OrientRelation { relation: Orient::R0, .. }));
    assert_eq!(p.orient_lists, vec![(k, vec![Orient::R0, Orient::MY])]);
    // the words of ORIENT are not relations
    error("0.4", "#M1 #M2 @ORIENT-RELATION(\"left\")", codes::PARAM_BAD_ENUM);
    // a set in one slot is done for each member; in both, it is an error
    problem("0.4", "#M1 {#M2 #M3} @ORIENT-RELATION(\"R0\")");
    let e =
        error("0.4", "{#M1 #M2} {#M3 #M4} @ORIENT-RELATION(\"MY\")", codes::CONSTRAINT_ORIENT_RELATION_SETS_BOTH_SIDES);
    assert!(e.contains("both its slots have more than one member, {#M1 #M2} and {#M3 #M4}"), "{e}");
    problem("0.3", "{#M1 #M2} {#M3 #M4} @ORIENT-RELATION(\"MY\")");
    error("0.3", "#M1 #M2 @ORIENT-RELATION([\"R0\", \"MY\"])", codes::PARAM_BAD_ENUM);
}

#[test]
fn a_problem_without_lists_or_senses_has_the_digest_it_had() {
    // the same statements in 0.3 and 0.4 give the same digest when 0.4 writes nothing new: only the
    // version differs, which the digest does not cover
    let body = "{#R1 #R2} @ORDER(\"left-to-right\"); #M1 @ORIENT(\"MY\"); #M1 #M2 @ORIENT-RELATION(\"R0\")";
    assert_eq!(input_digest(&problem("0.3", body)), input_digest(&problem("0.4", body)));
    // a list, or a DISTRIBUTE with its sense, is in it
    let plain = input_digest(&problem("0.4", "#M1 @ORIENT(\"MY\")"));
    assert_ne!(plain, input_digest(&problem("0.4", "#M1 @ORIENT([\"MY\", \"R0\"])")));
}
