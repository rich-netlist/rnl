//! The diagnostics of definitions, symbols, names and the syntax inside a statement (chapters 6-10),
//! each given where its text is, in a file of 0.4.
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use std::path::Path;

/// A netlist of 0.4: `header` in its header record, line 2, and `record` on line 9, a whole line
/// such as `* @RNL { … }`.
fn netlist(header: &str, record: &str) -> String {
    format!(
        "* diagnostics\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"){header} }}\nR1 a b 1k\nR2 b 0 1k\nX1 a b sub\n.subckt sub p n\nR9 p n 1k\n.ends sub\n{record}\n.end\n"
    )
}

/// Each diagnostic of the reading of `main.sp` and the files with it: kind, severity, file, line.
fn read(files: &[(&str, &str)]) -> Vec<(Code, Severity, String, u32)> {
    let mut provider = MemoryProvider::new();
    for (path, text) in files {
        provider.insert(path, text);
    }
    let r =
        interpret_file(Path::new("main.sp"), None, &provider, &Options { std: false, ..Default::default() }).unwrap();
    r.diagnostics()
        .iter()
        .map(|d| {
            (
                d.code,
                d.severity,
                d.loc.as_ref().map_or(String::new(), |l| l.file.clone()),
                d.loc.as_ref().map_or(0, |l| l.line),
            )
        })
        .collect()
}

/// Each case, the statements of one record on line 9, reports its kind with its severity there.
fn check(cases: &[(Code, Severity, &str)]) {
    for &(code, severity, statements) in cases {
        let found = read(&[("main.sp", &netlist("", &format!("* @RNL {{ {statements} }}")))]);
        assert!(found.iter().any(|f| (f.0, f.1, f.3) == (code, severity, 9)), "{code:?} on line 9: {found:?}");
    }
}

const SYMBOL: &str = "$S = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (0, 2) @PIN(terminal=2) }";

#[test]
fn a_symbol_body_holds_only_what_draws() {
    check(&[
        // a set is no object of a symbol body; a host object comes in through roles (9.2.2, 6.9)
        (codes::DEFINE_SYMBOL_BODY_BAD_OBJECT, Severity::Error, "$T = @DEFINE-SYMBOL { %s = {}; %s }"),
        (codes::REF_HOST_IN_BODY, Severity::Error, "$T = @DEFINE-SYMBOL { #R1 }"),
        // a constraint is no statement of a symbol body (9.2.2)
        (
            codes::DEFINE_SYMBOL_BODY_BAD_STATEMENT,
            Severity::Error,
            "$T = @DEFINE-SYMBOL { %a = (0, 0); %b = (1, 0); {%a %b} @ALIGN(\"y\") }",
        ),
        // ORIENT in a symbol body only on a text on `?` or a usage (9.2.2)
        (
            codes::KEYWORD_SYMBOL_MODIFIER_TARGET,
            Severity::Error,
            "$T = @DEFINE-SYMBOL { (0, 0) (1, 0) @LINE => @ORIENT(\"R90\") }",
        ),
        // a point of a symbol has numbers on both axes (9.2.1)
        (codes::POINT_IN_SYMBOL_FREE_COORDINATE, Severity::Error, "$T = @DEFINE-SYMBOL { %a = (0, _) }"),
    ]);
}

#[test]
fn applications_and_their_parameters_are_checked() {
    check(&[
        // two definitions that apply each other (9.4)
        (
            codes::USAGE_CYCLIC_APPLICATION,
            Severity::Error,
            "$A = @DEFINE(roles=[\"p\"]) { $B @ATTACH ?p }; $B = @DEFINE(roles=[\"p\"]) { $A @ATTACH ?p }; %x = (0, 0); $A @ATTACH %x",
        ),
        // a parameter with no value and no default, in an array or in a position (9.4)
        (
            codes::PARAM_HOLE_UNSET_ARRAY_ELEMENT,
            Severity::Error,
            "$A = @DEFINE(roles=[\"p\"], parameters=[\"w\"]) { ?p @ORIENT([?w, \"R0\"]) }; %x = (0, 0); $A @ATTACH %x",
        ),
        (
            codes::PARAM_HOLE_UNSET_POSITIONAL,
            Severity::Error,
            "$A = @DEFINE(roles=[\"p\"], parameters=[\"w\"]) { ?p @TEXT(?w) }; %x = (0, 0); $A @ATTACH %x",
        ),
    ]);
    // SET-SYMBOL's selection cannot depend on symbols (8.2, 10.1)
    let select = format!("{SYMBOL}; # @SELECT-PIN(recursive=true) => @SET-SYMBOL $S");
    check(&[(codes::SELECT_SYMBOL_DEPENDENCY, Severity::Error, &select)]);
}

#[test]
fn text_keys_cannot_be_hidden_by_other_names() {
    check(&[
        // a standard sub-object name is no text key (13.3)
        (codes::TEXT_KEY_STANDARD_SUBOBJECT, Severity::Error, "#R1 @TEXT(\"x\", key=\"position\")"),
        // a key spelled like a role of the usage: `::` takes the role (6.5, 13.3)
        (
            codes::TEXT_KEY_LIKE_USAGE_NAME,
            Severity::Error,
            "$A = @DEFINE(roles=[\"p\"]) {}; %x = (0, 0); %u = $A @ATTACH %x; %u @TEXT(\"t\", key=\"p\")",
        ),
        // the same, for a text the usage's symbol writes
        (
            codes::TEXT_SYMBOL_KEY_CLASH,
            Severity::Error,
            "$A = @DEFINE(roles=[\"p\"]) {}; $T = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); ? @TEXT(\"t\", key=\"p\") }; # @SELECT-USAGE $A => @SET-SYMBOL $T; %x = (0, 0); $A @ATTACH %x",
        ),
    ]);
}

#[test]
fn subscripts_arrays_and_roles_are_closed() {
    check(&[
        // a step of at least 1, a closed subscript (6.11)
        (codes::SYNTAX_SUBSCRIPT_ITEM, Severity::Error, "#X1[1:2:0] @PLACE(0, 0)"),
        (codes::SYNTAX_SUBSCRIPT_NOT_CLOSED, Severity::Error, "#X1[1 @PLACE(0, 0)"),
        (codes::SYNTAX_ARRAY_NOT_CLOSED, Severity::Error, "#R1 @TEXT(\"x\", size=[1, 2)"),
        (
            codes::SYNTAX_ROLE_NOT_CLOSED,
            Severity::Error,
            "$A = @DEFINE(roles=[\"p\"]) {}; %x = (0, 0); $A @ATTACH %x(p",
        ),
    ]);
}

#[test]
fn a_record_whose_braces_do_not_match_fails_whole() {
    // a `}` before the record's `{` (7.4)
    let found = read(&[("main.sp", &netlist("", "* @RNL } { #R1 @PLACE(0, 0) }"))]);
    assert!(found.iter().any(|f| (f.0, f.1, f.3) == (codes::RECORD_BRACES_MISMATCH, Severity::Error, 9)), "{found:?}");
}

#[test]
fn device_takes_the_names_of_the_terminals() {
    // the terminals of DEVICE are strings (6.12)
    let found =
        read(&[("main.sp", &netlist("; @DEVICE(name=\"R*\", terminals=[1, 2])", "* @RNL { #R1 @PLACE(0, 0) }"))]);
    assert!(
        found.iter().any(|f| (f.0, f.1, f.3) == (codes::DEVICE_TERMINALS_NOT_NAMES, Severity::Error, 2)),
        "{found:?}"
    );
}

#[test]
fn the_file_level_of_an_included_file_has_no_circuit() {
    // neither a host reference nor a statement that only draws a scene object (6.1)
    let main = "* main\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include \"child.inc\"\nR1 a b 1k\n.end\n";
    for (code, body) in [
        (codes::REF_HOST_IN_INCLUDED_FILE, "#R5 @PLACE(0, 0)"),
        (codes::KEYWORD_SCENE_ONLY_IN_INCLUDED_FILE, "(0, 0) (1, 0) @LINE"),
    ] {
        let child = format!("* child\nR5 c d 1k\n* @RNL {{ {body} }}\n");
        let found = read(&[("main.sp", main), ("child.inc", &child)]);
        assert!(
            found.iter().any(|f| f.0 == code && f.1 == Severity::Error && f.2.ends_with("child.inc") && f.3 == 3),
            "{code:?}: {found:?}"
        );
    }
}
