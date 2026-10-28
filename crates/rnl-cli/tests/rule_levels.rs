//! A rule written in a level that names host objects of that level (10.3, 10.4, 10.5). A rule is
//! evaluated in its level and in each level below, where those objects do not exist. For a
//! generation rule, AS or ATTACH, the other roles filled with them are a reason to warn that it
//! takes effect only in its own level (10.5); for any other rule, such as a DISTANCE between a
//! selection and named instances, 10.3 gives no warning: below, the statement is empty and does
//! nothing. With `recursive=false` the rule takes effect only in its own level, and gives the
//! levels below nothing at all, not even an empty fact. Before, a file with a subcircuit warned
//! for every such rule, and gave the subcircuit an empty DISTANCE.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{FactKind, ProblemFact};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const WARNING: &str = "this rule uses objects of the level it is written in, so it takes effect only there (10.5)";

/// A main file with two sources and a resistor, an instance of the subcircuit cell, and the
/// statements of one record, one a line from line 11 on.
fn interpret(statements: &[&str]) -> Interpretation {
    let record: Vec<String> = statements.iter().map(|s| format!("*   {s}\n")).collect();
    let net = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\")}}\n.subckt cell a b\nR1 a b 1k\n.ends cell\n\
         V1 n1 0 1\nV2 n2 0 1\nR3 n1 n2 1k\nX1 n1 n2 cell\n* @RNL {{\n{}* }}\n.end\n",
        record.concat()
    );
    let files = MemoryProvider::new().with("t.sp", &net);
    interpret_file(Path::new("t.sp"), None, &files, &Options { problems: true, ..Options::default() }).unwrap()
}

fn loud(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity != Severity::Hint).map(|d| d.message.clone()).collect()
}

/// The DISTANCE facts in the problem of the level `title` that the statement on `line` makes; the
/// standard package's fill makes others.
fn distances(r: &Interpretation, title: &str, line: u32) -> usize {
    let p = r.problems.iter().find(|p| p.title == title).unwrap();
    let on = |f: &&ProblemFact| f.source.as_ref().is_some_and(|l| l.line == line);
    p.facts.iter().filter(on).filter(|f| matches!(f.kind, FactKind::Distance { .. })).count()
}

#[test]
fn a_rule_that_is_no_generation_rule_does_not_warn_for_the_host_objects_it_names() {
    for recursive in ["", "(recursive=false)"] {
        let select = format!("$rest = # @SELECT-INSTANCE{recursive};");
        let r = interpret(&[
            &select,
            "{#V1 #V2} @REMOVE-FROM $rest;",
            "{#V1 #V2} $rest @DISTANCE(\"bottom-to-top\", min=3)",
        ]);
        assert!(loud(&r).is_empty(), "{recursive}: {:?}", loud(&r));
        // the top level has its DISTANCE, between the sources and the rest; the subcircuit none
        assert_eq!((distances(&r, "t.sp", 13), distances(&r, "cell", 13)), (1, 0), "{recursive}");
    }
}

#[test]
fn a_rule_of_its_own_level_only_gives_the_levels_below_no_fact() {
    // both sides selections, as the test set writes it to have no warning: below, both were empty,
    // and the subcircuit had a DISTANCE between nothing and nothing
    let r = interpret(&[
        "%clk = # @SELECT-INSTANCE(name=\"V*\", recursive=false);",
        "%rest = # @SELECT-INSTANCE(recursive=false);",
        "%clk @REMOVE-FROM %rest;",
        "%clk %rest @DISTANCE(\"bottom-to-top\", min=3)",
    ]);
    assert!(loud(&r).is_empty(), "{:?}", loud(&r));
    assert_eq!((distances(&r, "t.sp", 14), distances(&r, "cell", 14)), (1, 0));
    // with recursive selections, the rule is evaluated in the subcircuit as well
    let r = interpret(&[
        "%clk = # @SELECT-INSTANCE(name=\"V*\");",
        "%rest = # @SELECT-INSTANCE;",
        "%clk @REMOVE-FROM %rest;",
        "%clk %rest @DISTANCE(\"bottom-to-top\", min=3)",
    ]);
    assert_eq!((distances(&r, "t.sp", 14), distances(&r, "cell", 14)), (1, 1));
}

#[test]
fn a_generation_rule_whose_other_role_is_a_host_object_still_warns() {
    let r = interpret(&[
        "$PAIR = @DEFINE(roles=[[\"p\", \"net\"], [\"n\", \"net\"]]) { ?p @TEXT(\"p\") };",
        "# @SELECT-NET(name=\"n2\") => $PAIR @ATTACH &n1",
    ]);
    assert_eq!(loud(&r), [WARNING]);
}

#[test]
fn a_generation_rule_of_its_own_level_only_does_not_warn() {
    let r = interpret(&[
        "$PAIR = @DEFINE(roles=[[\"p\", \"net\"], [\"n\", \"net\"]]) { ?p @TEXT(\"p\") };",
        "# @SELECT-NET(name=\"n2\", recursive=false) => $PAIR @ATTACH &n1",
    ]);
    assert!(loud(&r).is_empty(), "{:?}", loud(&r));
}
