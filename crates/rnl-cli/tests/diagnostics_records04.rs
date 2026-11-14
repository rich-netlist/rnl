//! The diagnostics of the carriers, the record heads and the syntax of a statement (chapters 2-5),
//! each given where its text is, in a file of 0.4.
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use std::path::Path;

/// A netlist of 0.4 with `top` on line 9, after `.ends`, and `sub` on line 8, in the subcircuit.
fn netlist(top: &str, sub: &str) -> String {
    format!(
        "* diagnostics\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\nR2 b 0 1k\nX1 a b sub\n.subckt sub p n\nR9 p n 1k\n{sub}.ends sub\n{top}.end\n"
    )
}

/// Each diagnostic of the reading of `text`: its kind, severity and line.
fn read(text: &str) -> Vec<(Code, Severity, u32)> {
    let files = MemoryProvider::new().with("main.sp", text);
    let r = interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, ..Default::default() }).unwrap();
    r.diagnostics().iter().map(|d| (d.code, d.severity, d.loc.as_ref().map_or(0, |l| l.line))).collect()
}

/// Each case reports its kind, with its severity, on the line of its text.
fn check(cases: &[(Code, Severity, &str, &str)]) {
    for &(code, severity, top, sub) in cases {
        let line = if sub.is_empty() { 9 } else { 8 };
        let found = read(&netlist(top, sub));
        assert!(found.contains(&(code, severity, line)), "{code:?} on line {line}: {found:?}");
    }
}

#[test]
fn a_carrier_and_a_record_entry_need_a_space_before_them() {
    check(&[
        // `*@RNL`: no space after the comment marker, so the line carries no record (2.2)
        (codes::HOST_CARRIER_NO_SPACE, Severity::Warning, "*@RNL { %a = (0, 0) }\n", ""),
        // `x@RNL`: the entry is not after a space (2.3)
        (codes::RECORD_AT_NOT_AFTER_SPACE, Severity::Warning, "* x@RNL { %a = (0, 0) }\n", ""),
        // `#R1@PLACE`: the `@` of a keyword follows a space (2.3)
        (codes::SYNTAX_KEYWORD_AT_NO_SPACE, Severity::Error, "* @RNL { #R1@PLACE(0, 0) }\n", ""),
    ]);
}

#[test]
fn the_fields_of_a_record_head_are_checked() {
    check(&[
        // scope= in a record inside a subcircuit (2.6, 2.8)
        (codes::RECORD_HEADER_SCOPE_IN_SUBCIRCUIT, Severity::Error, "", "* @RNL(scope=\"sub\") { #R9 @PLACE(0, 0) }\n"),
        // a value a field does not take, a value that is not a string, a value without a name
        (codes::RECORD_HEADER_FIELD_BAD_VALUE, Severity::Error, "* @RNL(type=\"nonsense\") { #R1 @PLACE(0, 0) }\n", ""),
        (codes::RECORD_HEADER_FIELD_NOT_STRING, Severity::Error, "* @RNL(author=3) { #R1 @PLACE(0, 0) }\n", ""),
        (codes::RECORD_HEADER_POSITIONAL, Severity::Error, "* @RNL(\"x\") { #R1 @PLACE(0, 0) }\n", ""),
        // an unknown field is ignored, with a warning (2.8)
        (codes::RECORD_HEADER_FIELD_UNKNOWN, Severity::Warning, "* @RNL(colour=\"red\") { #R1 @PLACE(0, 0) }\n", ""),
    ]);
}

#[test]
fn literals_and_names_are_written_as_the_syntax_says() {
    check(&[
        // a character glued to a reference is free text (3.3)
        (codes::SYNTAX_GLUED_CHARACTER, Severity::Warning, "* @RNL { #R1! @PLACE(0, 0) }\n", ""),
        // a number has no suffix, an enumeration is a string (3.7)
        (codes::SYNTAX_NUMBER_SUFFIX, Severity::Error, "* @RNL { #R1 @PLACE(1k, 0) }\n", ""),
        (codes::PARAM_BARE_WORD, Severity::Error, "* @RNL { #R1 @ORIENT(R90) }\n", ""),
        // parameters hold literals, not objects (4.4)
        (codes::SYNTAX_PARAM_OBJECT, Severity::Error, "* @RNL { %a = (0, 0); %a @PLACE(0, #R1) }\n", ""),
        // a name after `::` (16.5)
        (codes::SYNTAX_SUB_NAME_MISSING, Severity::Error, "* @RNL { #R1:: @PLACE(0, 0) }\n", ""),
        // subscripts: none on an RNL name, one only, unsigned integers (6.11)
        (codes::SYNTAX_RNL_NAME_SUBSCRIPT, Severity::Error, "* @RNL { %p[0] = (0, 0) }\n", ""),
        (codes::SYNTAX_TWO_SUBSCRIPTS, Severity::Error, "* @RNL { #X1[0][1] @PLACE(0, 0) }\n", ""),
        (codes::SYNTAX_SUBSCRIPT_NOT_INTEGER, Severity::Error, "* @RNL { #X1[a] @PLACE(0, 0) }\n", ""),
    ]);
}

#[test]
fn segments_roles_and_member_places_are_checked() {
    check(&[
        // a segment is not empty (4.2 rule 1)
        (codes::SYNTAX_EMPTY_SEGMENT, Severity::Error, "* @RNL { #R1 @PLACE(0, 0) => => @ORIENT(\"R0\") }\n", ""),
        // roles only on the objects of a segment with a keyword (4.2 rule 5)
        (codes::SYNTAX_ROLE_WITHOUT_KEYWORD, Severity::Error, "* @RNL { #R1(p) }\n", ""),
        // member places (5.2.3)
        (codes::SYNTAX_PLACES_INVALID, Severity::Error, "* @RNL { %s = {#R1 #R2}; %s{0} @PLACE(0, 0) }\n", ""),
        // an empty set to BUNDLE (5.2.1, 11.8)
        (codes::KEYWORD_BUNDLE_EMPTY_SET, Severity::Error, "* @RNL { {} @BUNDLE }\n", ""),
        // two single roles each given a set of several members (5.2.1)
        (
            codes::USAGE_SEVERAL_SET_ROLES,
            Severity::Error,
            "* @RNL { $D = @DEFINE(roles=[\"a\", \"b\"]) {}; %p = (0, 0); %q = (1, 0); %r = (2, 0); %s = (3, 0); $D @ATTACH {%p %q}(a) {%r %s}(b) }\n",
            "",
        ),
    ]);
}
