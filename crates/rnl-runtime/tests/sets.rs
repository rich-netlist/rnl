//! Cycles of sets (5.2.2). A set depends on the sets written in the statement that makes it and on
//! the sets that ADD-TO and REMOVE-FROM add to it or remove from it; taking the statements in order
//! of position (8.5), the one that closes a cycle fails, with the whole cycle in its diagnostic,
//! and the others take effect. Once a cycle through a removal made the whole interpretation fail,
//! a cycle of additions was interpreted, and one through sets holding a selection (10.3)
//! overflowed the stack.

use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::host::{Dialect, HostDocument, Instance, Level, LevelKind};
use rnl_runtime::records::{package_lines, scan_run};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{check_package, interpret, Interpretation, Options};

fn resistor(name: &str, a: &str, b: &str, order: usize) -> Instance {
    Instance {
        name: name.into(),
        letter: Some('R'),
        nodes: vec![a.into(), b.into()],
        value: Some("1k".into()),
        file: "sets.sp".into(),
        line: 1,
        order,
        default_terminals: Some(vec!["p".into(), "n".into()]),
        ..Instance::default()
    }
}

/// What a document gives: the errors shown, and the instances of each level left without a
/// position.
struct Outcome {
    errors: Vec<String>,
    unplaced: Vec<Vec<String>>,
}

impl Outcome {
    fn of(r: &Interpretation) -> Outcome {
        assert!(r.model.failed.is_none(), "no case makes the whole interpretation fail (7.5)");
        Outcome {
            errors: r
                .diagnostics()
                .into_iter()
                .filter(|d| d.severity == Severity::Error)
                .map(|d| r.sink.render(d))
                .collect(),
            unplaced: r.model.levels.iter().map(|l| l.unplaced.clone()).collect(),
        }
    }
}

/// R1 n1 n2, R2 n2 n3 and R3 n3 n4 with `statements` written in the main file, whose record is
/// line 2, and the subcircuits (name, ports, statements), each of R1 between its two ports,
/// instantiated once, with its record on line 1.
fn interpret_in(statements: &str, subckts: &[(&str, [&str; 2], &str)]) -> Outcome {
    interpret_with("", statements, subckts, &MemoryProvider::new())
}

/// `interpret_in`, with more in the header record and the files of `provider`.
fn interpret_with(
    header: &str,
    statements: &str,
    subckts: &[(&str, [&str; 2], &str)],
    provider: &MemoryProvider,
) -> Outcome {
    let mut sink = Sink::new();
    let mut main = Level::new(LevelKind::File, "sets.sp", "sets.sp", "sets.sp");
    main.circuit = true;
    for (k, (name, a, b)) in [("R1", "n1", "n2"), ("R2", "n2", "n3"), ("R3", "n3", "n4")].into_iter().enumerate() {
        main.instances.push(resistor(name, a, b, k));
    }
    for (k, (name, _, _)) in subckts.iter().enumerate() {
        main.instances.push(Instance {
            name: format!("X{k}"),
            letter: Some('X'),
            nodes: vec![format!("a{k}"), format!("b{k}")],
            model: Some(name.to_string()),
            file: "sets.sp".into(),
            line: 2,
            order: k + 3,
            is_subckt: true,
            ..Instance::default()
        });
    }
    let records = format!("@RNL(type=\"header\") {{ @VERSION(\"0.2\"){header}}}\n@RNL {{ {statements} }}");
    main.records = scan_run(&package_lines(&records), "sets.sp", &mut sink);
    let dialect = Dialect { name: "spice".into(), case_sensitive: false, ground_aliases: vec!["gnd".into()] };
    let mut doc = HostDocument::new(main, dialect);
    for &(name, [a, b], statements) in subckts {
        let mut lv = Level::new(LevelKind::Subckt, name, "sets.sp", "sets.sp");
        lv.circuit = true;
        lv.ports = vec![a.to_string(), b.to_string()];
        lv.instances.push(resistor("R1", a, b, 0));
        lv.records = scan_run(&package_lines(&format!("@RNL {{ {statements} }}")), "sets.sp", &mut sink);
        let id = doc.add_level(lv, Some(0));
        doc.subckts.insert(doc.key(name), id);
    }
    Outcome::of(&interpret(doc, provider, &Options { std: false }, sink))
}

fn interpret_main(statements: &str) -> Outcome {
    interpret_in(statements, &[])
}

/// Where `stmt`, written once in `statements`, is in a record `@RNL { statements }` on `line`.
fn at(line: u32, statements: &str, stmt: &str) -> String {
    let k = statements.find(stmt).unwrap_or_else(|| panic!("{stmt} is not in {statements}"));
    assert_eq!(statements.matches(stmt).count(), 1, "{stmt} is written more than once");
    format!("sets.sp:{line}:{}", statements[..k].chars().count() + 8)
}

/// The error of a statement that closes a cycle of sets, written at `place`, with its cycle.
fn closes(errors: &[String], place: &str, cycle: &str) {
    let want = format!(
        "error {place}: a cycle of sets: {cycle}; taken in order of position, this statement closes it and fails (5.2.2)"
    );
    assert!(errors.contains(&want), "{want}\nnot in {errors:#?}");
}

#[test]
fn a_cycle_of_additions_fails_at_the_statement_that_closes_it() {
    let s = "%a = {#R1}; %b = {#R2}; %a @ADD-TO %b; %b @ADD-TO %a; %a @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle = format!("%a depends on %b here, and %b on %a at {}", at(2, s, "%a @ADD-TO %b"));
    closes(&r.errors, &at(2, s, "%b @ADD-TO %a"), &cycle);
    // %a keeps R1 only, and %b has R2 and R1
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
    let r = interpret_main("%a = {#R1}; %b = {#R2}; %a @ADD-TO %b; %b @ADD-TO %a; %b @PLACE(0, 0)");
    assert_eq!(r.unplaced[0], ["#R3"]);
}

#[test]
fn a_cycle_through_a_removal_fails_at_the_removal_and_the_rest_takes_effect() {
    let s = "%a = {#R1 #R2}; %b = {%a}; %b @REMOVE-FROM %a; %a @PLACE(0, 0); #R3 @PLACE(4, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle = format!("%a depends on %b here, and %b on %a at {}", at(2, s, "{%a}"));
    closes(&r.errors, &at(2, s, "%b @REMOVE-FROM %a"), &cycle);
    assert!(r.unplaced[0].is_empty(), "{:?}", r.unplaced);
}

#[test]
fn a_set_added_to_itself_fails() {
    let s = "%a = {#R1}; %a @ADD-TO %a; {%a} @ADD-TO %a; %a @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 2, "{:?}", r.errors);
    closes(&r.errors, &at(2, s, "%a @ADD-TO %a"), "%a depends on itself here");
    let braces = at(2, s, "{%a} @ADD-TO");
    closes(&r.errors, &braces, &format!("%a depends on {{…}} here, and {{…}} on %a at {braces}"));
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
}

#[test]
fn a_cycle_through_braces() {
    let s = "%a = {#R1}; %b = {%a #R2}; %c = {%b}; %c @ADD-TO %a; %a @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle =
        format!("%a depends on %c here, %c on %b at {}, and %b on %a at {}", at(2, s, "{%b}"), at(2, s, "{%a #R2}"));
    closes(&r.errors, &at(2, s, "%c @ADD-TO %a"), &cycle);
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
}

#[test]
fn a_cycle_through_a_union() {
    // the set of the two objects of UNION is not written: %u depends on %b there
    let s = "%a = {#R1}; %b = {#R2}; %u = %a %b @UNION; %u @ADD-TO %b; %b @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let union = at(2, s, " %a %b @UNION");
    let cycle = format!("%b depends on %u here, and %u on %b at {union}");
    closes(&r.errors, &at(2, s, "%u @ADD-TO %b"), &cycle);
    assert_eq!(r.unplaced[0], ["#R1", "#R3"]);
}

#[test]
fn a_cycle_through_the_scope_of_a_selection() {
    let s = "%a = {#R1}; %t = %a @SELECT-TERMINAL(name=\"p\"); %t @ADD-TO %a; %a @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle = format!("%a depends on %t here, and %t on %a at {}", at(2, s, " %a @SELECT-TERMINAL"));
    closes(&r.errors, &at(2, s, "%t @ADD-TO %a"), &cycle);
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
}

#[test]
fn a_cycle_of_sets_holding_selections_fails_and_does_not_recurse() {
    // whether such a set is lazy (10.3) went round the sources of its edits without end
    let s = "%a = {# @SELECT-INSTANCE(name=\"R1\")}; %b = {# @SELECT-INSTANCE(name=\"R2\")}; %a @ADD-TO %b; %b @ADD-TO %a; %a @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle = format!("%a depends on %b here, and %b on %a at {}", at(2, s, "%a @ADD-TO %b"));
    closes(&r.errors, &at(2, s, "%b @ADD-TO %a"), &cycle);
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
}

#[test]
fn a_cycle_across_two_levels_fails_in_the_level_where_it_closes() {
    // the rule of the level above takes effect in the subcircuit too (10.3) and comes first there
    // (8.5); in the level above there is no cycle
    for (a, b) in [("{}", "{}"), ("{# @SELECT-NET(name=\"p\")}", "{# @SELECT-NET(name=\"n1\")}")] {
        let main = format!("@USE-PACKAGE(name=\"house\") {{ $A = {a}; $B = {b} }}; $A @ADD-TO $B; #R1 @PLACE(0, 0)");
        let sub = "$B @ADD-TO $A; #R1 @PLACE(0, 0)";
        let r = interpret_in(&main, &[("core", ["p", "q"], sub)]);
        assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
        let cycle = format!("$A depends on $B here, and $B on $A at {}", at(2, &main, "$A @ADD-TO $B"));
        closes(&r.errors, &at(1, sub, "$B @ADD-TO $A"), &cycle);
    }
}

#[test]
fn edits_of_package_sets_in_two_levels_are_not_a_cycle() {
    // one level adds $SUPPLIES to $RAILS, the other $RAILS to $SUPPLIES: each level sees its own
    for (rails, supplies) in [("{}", "{}"), ("{# @SELECT-NET(name=\"a*\")}", "{# @SELECT-NET(name=\"v*\")}")] {
        let r = interpret_in(
            &format!("@USE-PACKAGE(name=\"house\") {{ $RAILS = {rails}; $SUPPLIES = {supplies} }}; &n1 @ADD-TO $SUPPLIES; $SUPPLIES @ROUTE"),
            &[
                ("core", ["vdd", "out"], "$SUPPLIES @ADD-TO $RAILS; $RAILS @ROUTE"),
                ("pad", ["vio", "x"], "&vio @ADD-TO $RAILS; $RAILS @ADD-TO $SUPPLIES; $SUPPLIES @ROUTE"),
            ],
        );
        assert!(r.errors.is_empty(), "{:?}", r.errors);
    }
}

#[test]
fn a_package_used_after_an_edit_comes_after_it() {
    // the rules of a package are evaluated first, but the package is at its USE-PACKAGE (8.5)
    let s = "$A @ADD-TO $B; @USE-PACKAGE(name=\"p\") { $A = {}; $B = {}; $B @ADD-TO $A }; #R1 @ADD-TO $A; #R2 @ADD-TO $B; $B @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle = format!("$A depends on $B here, and $B on $A at {}", at(2, s, "$A @ADD-TO $B"));
    closes(&r.errors, &at(2, s, "$B @ADD-TO $A"), &cycle);
    assert_eq!(r.unplaced[0], ["#R3"]);
}

#[test]
fn a_set_of_a_package_comes_where_the_package_is_used() {
    // used at the root, the package comes before the level's statements (8.5), although its set
    // is made before any level: the level's edit closes the cycle
    let pk = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"pk\", version=\"1.0\") }\n@RNL { $B = {}; $A = {$B} }\n";
    let provider = MemoryProvider::new().with("pk.rnl", pk);
    let header = "; @USE-PACKAGE(name=\"pk\", version=\"1.0\", path=\"pk.rnl\")";
    let s = "$A @ADD-TO $B; #R1 @ADD-TO $B; $B @PLACE(0, 0)";
    let r = interpret_with(header, s, &[], &provider);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    closes(&r.errors, &at(2, s, "$A @ADD-TO $B"), "$B depends on $A here, and $A on $B at pk.rnl:2:22");
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
    // used after the edit, the package's statement that makes $A closes it, and the edit fails
    // with it (7.2)
    let s = "$A @ADD-TO $B; @USE-PACKAGE(name=\"p\") { $B = {}; $A = {$B} }; #R1 @ADD-TO $B; $B @PLACE(0, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 2, "{:?}", r.errors);
    let cycle = format!("$A depends on $B here, and $B on $A at {}", at(2, s, "$A @ADD-TO $B"));
    closes(&r.errors, &at(2, s, "{$B}"), &cycle);
    assert!(r.errors.iter().any(|e| e.starts_with(&format!("error {}: $A failed", at(2, s, "$A @ADD-TO $B")))));
    assert_eq!(r.unplaced[0], ["#R2", "#R3"]);
}

#[test]
fn a_set_made_after_an_edit_that_names_it_closes_the_cycle() {
    // a name can be used before the statement that binds it (1.5 item 1): the statement that makes
    // %b comes after the edit, so it fails, and the edit fails with it (7.2)
    let s = "%a = {#R1}; %b @ADD-TO %a; %b = {%a #R2}; %a @PLACE(0, 0); #R3 @PLACE(4, 0)";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 2, "{:?}", r.errors);
    let cycle = format!("%b depends on %a here, and %a on %b at {}", at(2, s, "%b @ADD-TO %a"));
    closes(&r.errors, &at(2, s, "{%a #R2}"), &cycle);
    assert!(r.errors[1].starts_with(&format!("error {}: %b failed", at(2, s, "%b @ADD-TO %a"))), "{:?}", r.errors);
    assert_eq!(r.unplaced[0], ["#R2"]);
}

#[test]
fn an_edit_generated_at_an_earlier_place_comes_before_a_later_statement() {
    // what a rule generates in step 5 takes the position of the rule (8.5, 10.5)
    let s = "$A = {#R1}; $B = {#R2}; $D = @DEFINE(roles=[\"x\"]) { $A @ADD-TO $B }; # @SELECT-INSTANCE(name=\"R1\") => $D @AS; $B @ADD-TO $A";
    let r = interpret_main(s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    let cycle = format!("$A depends on $B here, and $B on $A at {}", at(2, s, "$A @ADD-TO $B"));
    closes(&r.errors, &at(2, s, "$B @ADD-TO $A"), &cycle);
}

#[test]
fn a_package_checked_on_its_own_has_its_cycles() {
    let text = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"cyc\", version=\"1.0\") }\n@RNL {\n  $A = {# @SELECT-NET(name=\"a\")};\n  $B = {};\n  $A @ADD-TO $B;\n  $B @REMOVE-FROM $A\n}\n";
    let r = check_package("cyc.rnl", &MemoryProvider::new().with("cyc.rnl", text), &Options { std: false });
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    closes(&errors, "cyc.rnl:6:3", "$A depends on $B here, and $B on $A at cyc.rnl:5:3");
}

#[test]
fn a_long_chain_of_sets_is_no_cycle() {
    // each set holds a selection and is added to the next, and also to a set made of the next
    let n = 1000;
    let mut s = String::new();
    for k in 0..=n {
        s.push_str(&format!("%s{k} = {{# @SELECT-INSTANCE(name=\"R{}\")}}; %t{k} = {{%s{k}}}; ", k % 3 + 1));
    }
    for k in 0..n {
        s.push_str(&format!("%s{k} @ADD-TO %s{}; %s{k} @ADD-TO %t{}; ", k + 1, k + 1));
    }
    s.push_str(&format!("%s{n} @PLACE(0, 0)"));
    let r = interpret_main(&s);
    assert!(r.errors.is_empty(), "{:?}", r.errors);
    assert!(r.unplaced[0].is_empty(), "{:?}", r.unplaced);
    // closing it fails at the last statement
    s.push_str(&format!("; %s{n} @ADD-TO %s0"));
    let r = interpret_main(&s);
    assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
    assert!(r.errors[0].contains(&format!("a cycle of sets: %s0 depends on %s{n} here, %s{n} on %s{} at", n - 1)));
}
