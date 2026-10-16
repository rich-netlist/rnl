//! Loose objects match the slots of a keyword one to one (5.2.1 item 3, in every version, as the
//! draft 0.3 keeps it): a keyword with several slots, where a definition takes a slot too, fails
//! when the number of objects is not the number of slots. So `#M1 #M2 @SET-SYMBOL $NMOS` fails;
//! to give M1 and M2 the symbol, write `{#M1 #M2} @SET-SYMBOL $NMOS`. Before, a keyword with a
//! definition and one other slot put the loose objects together into one set, while one with two
//! other slots failed. Roles marked and a lone set split one level (items 1 and 2) are unchanged.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// R1 to R3, two definitions, a style, a set, and `statements` in the same record, from line 11.
fn read(statements: &[&str]) -> Interpretation {
    let mut text = String::from("* slots\nR1 a b 1k\nR2 b c 1k\nR3 c 0 1k\n");
    text += "* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n* @RNL {\n";
    let mut all = vec![
        "$ONE = @DEFINE(roles=[\"a\"]) { ?a @PLACE(x=0) }",
        "$TALL = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (0, 10) @PIN(terminal=2) }",
        "$grey = @DEFINE-STYLE(linecolor=\"#888888\") {}",
        "%s = {#R3}",
    ];
    all.extend_from_slice(statements);
    text += &all.iter().map(|s| format!("*   {s}")).collect::<Vec<_>>().join(";\n");
    text += "\n* }\n.end\n";
    let files = MemoryProvider::new().with("s.sp", &text);
    interpret_file(Path::new("s.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

/// The errors, each by its line and message.
fn errors(statements: &[&str]) -> Vec<(u32, String)> {
    let r = read(statements);
    let found = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error);
    found.map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone())).collect()
}

#[test]
fn a_definition_takes_a_slot() {
    // SET-SYMBOL, SET-STYLE, SELECT-USAGE and SELECT-ANCESTOR-USAGE have a definition and one
    // other slot: two loose objects before the definition are three objects for two slots
    for (kw, def) in
        [("SET-SYMBOL", "$TALL"), ("SET-STYLE", "$grey"), ("SELECT-USAGE", "$ONE"), ("SELECT-ANCESTOR-USAGE", "$ONE")]
    {
        let message = format!(
            "@{kw} has 2 slots, and 3 objects are written (5.2.1); for several objects, write {{#R1 #R2}} @{kw} {def}"
        );
        assert_eq!(errors(&[&format!("#R1 #R2 @{kw} {def}")]), [(11, message)], "{kw}");
        // in braces they are one object, the set, which the slot takes
        let braced = errors(&[&format!("{{#R1 #R2}} @{kw} {def}")]);
        assert!(braced.is_empty(), "{kw}: {braced:?}");
    }
}

#[test]
fn a_usage_takes_one_object_for_each_role() {
    // AS and ATTACH: the roles of the definition and the definition are the slots (9.4)
    let one = "the ordinary definition $ONE has 1 roles, 2 objects are written (9.4)".to_string();
    assert_eq!(errors(&["#R1 #R2 @AS $ONE", "$ONE @ATTACH #R1 #R2"]), [(11, one.clone()), (12, one)]);
    let braced = errors(&["{#R1 #R2} @AS $ONE", "$ONE @ATTACH {#R1 #R2}"]);
    assert!(braced.is_empty(), "{braced:?}");
}

#[test]
fn two_slots_take_two_objects() {
    // ADD-TO, REMOVE-FROM, DISTANCE, ORIENT-RELATION and WIRE have two slots and no definition
    let cases = [
        ("ADD-TO", "#R1 #R2 @ADD-TO %s", "{#R1 #R2} @ADD-TO %s"),
        ("REMOVE-FROM", "#R1 #R2 @REMOVE-FROM %s", "{#R1 #R2} @REMOVE-FROM %s"),
        (
            "DISTANCE",
            "#R1 #R2 #R3 @DISTANCE(\"left-to-right\", min=1)",
            "{#R1 #R2} #R3 @DISTANCE(\"left-to-right\", min=1)",
        ),
        ("ORIENT-RELATION", "#R1 #R2 #R3 @ORIENT-RELATION(\"R0\")", "{#R1 #R2} #R3 @ORIENT-RELATION(\"R0\")"),
        ("WIRE", "&a &b @WIRE {#R1:1 #R1:2}", "{&a &b} @WIRE {#R1:1 #R1:2}"),
    ];
    for (kw, loose, braced) in cases {
        let message = format!(
            "@{kw} has 2 slots, and 3 objects are written (5.2.1); the objects of one slot are written in {{…}}"
        );
        assert_eq!(errors(&[loose]), [(11, message)], "{kw}");
        let found = errors(&[braced]);
        assert!(found.is_empty(), "{kw}: {found:?}");
    }
    // fewer objects than slots fail as well
    let one = "@DISTANCE has 2 slots, and 1 object is written (5.2.1)".to_string();
    assert_eq!(errors(&["#R1 @DISTANCE(\"left-to-right\", min=1)"]), [(11, one)]);
}

#[test]
fn roles_marked_and_a_lone_set_are_as_before() {
    // 5.2.1 items 1 and 2: objects marked with roles, and a lone set split one level
    let found = errors(&["#R1(a) @AS $ONE", "{#R1 #R2} @DISTANCE(\"left-to-right\", min=1)", "#R1 #R2 @ALIGN(\"y\")"]);
    assert!(found.is_empty(), "{found:?}");
}
