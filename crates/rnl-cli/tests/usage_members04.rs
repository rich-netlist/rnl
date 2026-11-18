//! A usage without a symbol among the items of a constraint stands for its members (6.5, 11.5,
//! 14.1).

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::problem::{FactKind, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

const SYMBOL: &str = "$S = @DEFINE-SYMBOL { (-1,-1) (1,1) @RECTANGLE }; # @SELECT-INSTANCE => @SET-SYMBOL $S;";

fn read(body: &str) -> Interpretation {
    let instances: String = (1..=12).map(|i| format!("R{i} n{i} 0 1k\n")).collect();
    let text = format!(
        "* members\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n{instances}* @RNL {{ {SYMBOL} {body} }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}
fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}
fn count(r: &Interpretation, code: Code) -> usize {
    r.diagnostics().iter().filter(|d| d.code == code).count()
}
/// The members of each ALIGN, ORDER and DISTRIBUTE, by their labels, with the fact's label.
fn facts(r: &Interpretation) -> Vec<(String, Vec<String>)> {
    let p = &r.problems[0];
    let name = |m: &PointRef| match m {
        PointRef::Origin(i) => p.objects[*i].label.clone(),
        other => format!("{other:?}"),
    };
    p.facts
        .iter()
        .filter_map(|f| match &f.kind {
            FactKind::Align { members, .. }
            | FactKind::Order { members, .. }
            | FactKind::Distribute { members, .. } => Some((f.label.clone(), members.iter().map(name).collect())),
            _ => None,
        })
        .collect()
}
fn rows(r: &Interpretation) -> Vec<Vec<String>> {
    facts(r).into_iter().map(|(_, m)| m).collect()
}
fn strs(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter().map(|r| r.iter().map(|s| s.to_string()).collect()).collect()
}

const PAIR: &str = "$D = @DEFINE(roles=[\"a\", \"b\"]) {};";

#[test]
fn a_usage_without_a_symbol_stands_for_each_member_in_turn() {
    let r = read(&format!("{PAIR} %u = #R2(a) #R3(b) @AS $D; {{#R1 %u #R4}} @ORDER(\"left-to-right\")"));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(rows(&r), strs(&[&["#R1", "#R2", "#R4"], &["#R1", "#R3", "#R4"]]));
    assert_eq!(count(&r, codes::USAGE_POINT_IDLE_04), 0, "{:?}", r.diagnostics());
}

#[test]
fn a_usage_with_a_symbol_stays_itself() {
    let r = read(
        "$D = @DEFINE(roles=[\"a\", \"b\"]) { ? @SET-SYMBOL $S }; %u = #R2(a) #R3(b) @AS $D; {#R1 %u} @ALIGN(\"x\")",
    );
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(rows(&r), strs(&[&["#R1", "%u"]]));
}

#[test]
fn nested_usages_stand_for_their_own_members_and_nets_are_left_out() {
    let r = read(&format!(
        "{PAIR} %inner = #R2(a) &n1(b) @AS $D; %outer = %inner(a) #R3(b) @AS $D; {{#R1 %outer}} @ALIGN(\"y\")"
    ));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(rows(&r), strs(&[&["#R1", "#R2"], &["#R1", "#R3"]]));
}

#[test]
fn a_combination_with_an_object_twice_is_left_out() {
    let r = read(&format!("{PAIR} %u = #R1(a) #R2(b) @AS $D; {{#R1 %u}} @ALIGN(\"x\")"));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(rows(&r), strs(&[&["#R1", "#R2"]]));
    // none left: the statement holds trivially, and says so
    let r = read("$ONE = @DEFINE(roles=[\"a\"]) {}; %v = #R1(a) @AS $ONE; {#R1 %v} @ALIGN(\"x\")");
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(rows(&r), Vec::<Vec<String>>::new());
    assert_eq!(count(&r, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES), 1, "{:?}", r.diagnostics());
}

/// The members of each DISTANCE, by their labels, `from` then `to`.
fn distances(r: &Interpretation) -> Vec<Vec<String>> {
    let p = &r.problems[0];
    let name = |m: &PointRef| match m {
        PointRef::Origin(i) => p.objects[*i].label.clone(),
        other => format!("{other:?}"),
    };
    p.facts
        .iter()
        .filter_map(|f| match &f.kind {
            FactKind::Distance { from, to, .. } => Some(from.iter().chain(to).map(name).collect()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_usage_written_on_both_sides_stands_for_its_members_on_each() {
    // each side stands for #R1 and #R2: the pairs of two different members are kept, as when the
    // members are written (14.1, 14.5)
    let written = read("{#R1 #R2} {#R1 #R2} @DISTANCE(\"left-to-right\", exact=0)");
    let r = read(&format!("{PAIR} %u = #R1(a) #R2(b) @AS $D; %u %u @DISTANCE(\"left-to-right\", exact=0)"));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    let sorted = |r: &Interpretation| {
        let mut d = distances(r);
        d.sort();
        d
    };
    assert_eq!(sorted(&r), strs(&[&["#R1", "#R2"], &["#R2", "#R1"]]));
    assert_eq!(sorted(&r), sorted(&written));
    assert_eq!(count(&r, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES), 0, "{:?}", r.diagnostics());
    // its own point on one side, its members on the other
    let r = read(&format!("{PAIR} %u = #R1(a) #R2(b) @AS $D; %u::position %u @DISTANCE(\"left-to-right\", exact=0)"));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(distances(&r).len(), 2, "{:?}", distances(&r));
}

#[test]
fn a_usage_with_a_symbol_written_twice_relates_nothing() {
    let r = read(
        "$D = @DEFINE(roles=[\"a\", \"b\"]) { ? @SET-SYMBOL $S }; %u = #R2(a) #R3(b) @AS $D; %u %u @DISTANCE(\"left-to-right\", exact=0)",
    );
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    assert_eq!(distances(&r), Vec::<Vec<String>>::new());
    assert_eq!(count(&r, codes::CONSTRAINT_EXPANSION_TOO_FEW_PLACES), 1, "{:?}", r.diagnostics());
}

#[test]
fn the_constraints_split_off_a_named_one_keep_its_name_and_priority() {
    let r = read(&format!(
        "{PAIR} %u = #R2(a) #R3(b) @AS $D; %c = {{#R1 %u}} @ORDER(\"left-to-right\"); %c @PRIORITY(\"low\")"
    ));
    assert_eq!(errors(&r), [], "{:?}", r.diagnostics());
    let p = &r.problems[0];
    let split: Vec<_> = p.facts.iter().filter(|f| matches!(f.kind, FactKind::Order { .. })).collect();
    assert_eq!(split.len(), 2);
    for f in split {
        assert_eq!(f.label, "%c");
        assert_eq!(f.priority, rnl_runtime::interp::obj::Priority::Low, "{f:?}");
    }
}

#[test]
fn the_limit_of_expanded_constraints_holds_once_usages_stand_for_their_members() {
    let set = (1..=11).map(|i| format!("#R{i}")).collect::<Vec<_>>().join(" ");
    let r = read(&format!(
        "$ONE = @DEFINE(roles=[\"a\"]) {{}}; %u = {{{set}}}(a) @AS $ONE; %v = {{{set}}}(a) @AS $ONE; %w = {{{set}}}(a) @AS $ONE; {{%u %v %w}} @ALIGN(\"x\")"
    ));
    assert_eq!(errors(&r), [codes::CONSTRAINT_EXPANSION_TOO_MANY], "{:?}", r.diagnostics());
}

/// An ALIGN over a usage whose body places its members apart cannot hold for each member: the hint
/// says that `::position` names the usage's own point (6.5, 14.1).
#[test]
fn aligning_members_the_body_places_apart_has_the_hint() {
    let body = "$ROW = @DEFINE(roles=[\"a\", \"b\"]) { ?a @PLACE(0,0); ?b @PLACE(10,0) }; %row = #R2(a) #R3(b) @AS $ROW; #R1 @PLACE(0,5); %row @PLACE(20,0);";
    for (tail, hints) in [
        ("{#R1 %row} @ALIGN(\"x\")", 1),
        ("{#R1 %row} @ALIGN(\"y\")", 0),
        ("{#R1 %row::position} @ALIGN(\"x\")", 0),
        ("{#R1 %row #R4} @DISTRIBUTE(\"left-to-right\")", 1),
        ("{#R1 %row} @ORDER(\"left-to-right\")", 0),
    ] {
        let r = read(&format!("{body} {tail}"));
        assert_eq!(errors(&r), [], "{tail}: {:?}", r.diagnostics());
        assert_eq!(count(&r, codes::USAGE_MEMBERS_PLACED_APART), hints, "{tail}: {:?}", r.diagnostics());
    }
    let r = read(&format!("{body} {{#R1 %row}} @ALIGN(\"x\")"));
    let hint = r.diagnostics().into_iter().find(|d| d.code == codes::USAGE_MEMBERS_PLACED_APART).unwrap();
    assert_eq!(
        hint.message,
        "ALIGN stands for each member of %row, and the definition of %row places them apart in x, so its constraints \
         cannot all hold; to align its own point write %row::position (6.5, 14.1)"
    );
}
