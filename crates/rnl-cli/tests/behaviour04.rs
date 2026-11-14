//! Rules of 0.4 that no test checked, by what they do rather than by a diagnostic alone: the
//! examples of errors of 4.4, a path of attributes through a set role (13.6), CURRENT on a net
//! (15.2), an assertion on a group (15.3), a usage taken as a black box (15.4), a malformed version
//! (16.1), and a hash whose algorithm a tool does not know (17).
use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read_with(text: &str, std: bool) -> Interpretation {
    let files = MemoryProvider::new().with("main.sp", text);
    interpret_file(Path::new("main.sp"), None, &files, &Options { std, problems: true, ..Default::default() }).unwrap()
}

/// The kinds of the reading's diagnostics, with their severity and line.
fn found(r: &Interpretation) -> Vec<(Code, Severity, u32)> {
    r.diagnostics().iter().map(|d| (d.code, d.severity, d.loc.as_ref().map_or(0, |l| l.line))).collect()
}

/// The netlist of the examples of 4.4, with the objects they name and the standard package; the
/// example goes on the last line before `.end`.
const EXAMPLES: &str = "* examples of 4.4
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }
M1 d1 g1 s 0 nch
M2 d2 g2 s 0 nch
M3 d3 g3 s 0 nch
M5 s tail 0 0 nch
R1 a b 1k
R2 b c 1k
R3 c d 1k
R4 d 0 1k
R6 vss 0 1k
XI0 D<1> D<0> buf
.model nch nmos
.subckt buf a b
R9 a b 1k
.ends buf
";

#[test]
fn each_example_of_4_4_is_an_error_for_the_reason_it_gives() {
    let line = EXAMPLES.lines().count() as u32 + 1;
    for (example, code) in [
        // a bracket after white space is a point, and PLACE has no parameters
        ("#M1 @PLACE (10, 20)", codes::PARAM_PLACE_NEEDS_X_OR_Y),
        // role brackets touch the object; a separate bracket is a point
        ("#M1 (left) #M2 (right) #M5 (tail) @AS $DIFFERENTIAL_PAIR_N", codes::SYNTAX_POINT_MALFORMED),
        // the second bracket is a point, and its content is not a point
        ("#M2 @PLACE(62, 40) (MY)", codes::SYNTAX_POINT_MALFORMED),
        // the definition of ATTACH is written first
        ("&vss @ATTACH $GROUND", codes::USAGE_NOT_A_DEFINITION),
        // ">" cannot be free text
        ("R1 -> R2 {#R1 #R2} @ORDER(\"left-to-right\")", codes::SYNTAX_NOT_FORMAL_TOKEN),
        // a comma does not separate statements: two keywords in one segment
        ("#M1 @PLACE(1, 2), #M2 @PLACE(3, 4)", codes::SYNTAX_TWO_KEYWORDS),
        // three objects, and SET-SYMBOL has two slots
        ("#M1 #M2 @SET-SYMBOL $NMOS", codes::KEYWORD_SLOT_COUNT_WITH_DEFINITION),
        // the segment after a pipe has no keyword
        ("#M1 => #M2", codes::SYNTAX_PIPE_SEGMENT_NO_KEYWORD),
        // three objects for two slots
        ("#M1 #M2 #M3 @DISTANCE(\"left-to-right\", min=1)", codes::KEYWORD_SLOT_COUNT),
        // a parameter given by position after one given by name
        ("#M1 @PLACE(x=10, 20)", codes::SYNTAX_POSITIONAL_AFTER_NAMED),
        // ORDER has no axis: the axis and the direction are one parameter
        ("{#M1 #M2} @ORDER(axis=\"x\", direction=\"increasing\")", codes::PARAM_OLD_DIRECTION),
        // brackets hold only literals
        ("#M1 @SET-SYMBOL($NMOS)", codes::SYNTAX_PARAM_OBJECT),
        // enumerations are written as strings
        ("{#M1 #M2} @ALIGN(axis=y)", codes::PARAM_BARE_WORD),
        // roles are marked on all objects or on none
        ("%ax = (_, _); #M1(left) #M2 %ax @AS $MIRRORED_LAYOUT", codes::ROLE_LABELS_MIXED),
        // the definition is written last and is not marked with a role
        ("$NMOS(symbol) #M1(target) @SET-SYMBOL", codes::KEYWORD_DEFINITION_HAS_ROLE),
        // the same role is marked twice
        ("#R1(source) #R2(chain) #R3(chain) #R4(load) @AS $CHAIN_ROW", codes::KEYWORD_POSITION_LABELLED_TWICE),
        // a pin cannot be written in the scene
        ("(0, 0) @PIN(terminal=1)", codes::KEYWORD_PIN_OUTSIDE_SYMBOL),
        // CURRENT takes only terminals
        ("#M1 @CURRENT(\"in\")", codes::CURRENT_NOT_HOST_TERMINAL),
        // a name has at most one subscript
        ("&D<1:0>[1:0]", codes::SYNTAX_TWO_SUBSCRIPTS),
        // one reference has at most one expansion
        ("#XI0<1:0>:D<1:0>", codes::SYNTAX_MULTIPLE_EXPANDING_SUBSCRIPTS),
        // a "<" directly after a name starts a subscript, and its content is not a subscript
        ("&D<_, 1>", codes::SYNTAX_SUBSCRIPT_NOT_INTEGER),
        // RNL names take no subscript; a point is written (1, 2)
        ("%p<1, 2>", codes::SYNTAX_RNL_NAME_SUBSCRIPT),
    ] {
        let r = read_with(&format!("{EXAMPLES}* @RNL {{ {example} }}\n.end\n"), true);
        let found = found(&r);
        assert!(found.contains(&(code, Severity::Error, line)), "{example}: {code:?} on line {line}: {found:?}");
    }
}

#[test]
fn a_path_through_a_set_role_shows_the_attribute_of_each_member() {
    // the usage's set role holds two pairs; `${pairs::left::w}` shows the w of both left
    // transistors, in the order of the set (13.6)
    let text = "* set role path
* @RNL(type=\"header\") { @VERSION(\"0.4\") }
M1 d1 g1 s 0 nch w=1u
M2 d2 g2 s 0 nch w=1u
M3 d3 g3 s 0 nch w=2u
M4 d4 g4 s 0 nch w=2u
.model nch nmos
* @RNL {
*   $DP = @DEFINE(roles=[\"left\", \"right\"]) {};
*   $BS = @DEFINE-SYMBOL { ? @TEXT(\"${pairs::left::w}\", key=\"widths\") };
*   $BANK = @DEFINE(roles=[\"{pairs}\"]) {};
*   %d1 = #M1(left) #M2(right) @AS $DP;
*   %d2 = #M3(left) #M4(right) @AS $DP;
*   $BANK @ATTACH {%d1 %d2}(pairs) => @SET-SYMBOL $BS => @PLACE(0, 0)
* }
.end
";
    let files = MemoryProvider::new().with("main.sp", text);
    let parts = rnl_view::Parts { drawing: false, host: false, model: true };
    let snapshot = rnl_view::snapshot("main.sp", &files, &rnl_view::BuildOptions::default(), parts).unwrap();
    assert!(!snapshot.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", snapshot.diagnostics);
    let mut texts = snapshot.levels.iter().filter_map(|l| l.model.as_ref()).flat_map(|m| m.texts.iter());
    let widths = texts.find(|t| t.id.ends_with("::widths")).expect("the text of the symbol");
    let shown: String = widths
        .lines
        .iter()
        .flatten()
        .filter_map(|run| match run {
            rnl_view::Run::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(shown, "1u,2u");
}

/// A subcircuit with the ports `inp` and `out` and R1 between them, with `statements` in it.
fn cell(statements: &str) -> String {
    format!(
        "* cell\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.subckt cell inp out\nR1 inp out 1k\n* @RNL {{ {statements} }}\n.ends cell\nX1 a b cell\n.end\n"
    )
}

fn violations(r: &Interpretation, code: Code) -> usize {
    r.diagnostics().iter().filter(|d| d.code == code && d.severity == Severity::Violation).count()
}

#[test]
fn a_port_in_and_a_device_in_balance_a_net_and_a_port_in_and_a_device_out_do_not() {
    // a port with "in" brings current into its net; a device terminal with "in" takes it out (15.2)
    let balanced = read_with(&cell("#:inp @CURRENT(\"in\"); #R1:1 @CURRENT(\"in\")"), false);
    assert_eq!(violations(&balanced, codes::VIOLATION_CURRENT_ONE_WAY), 0, "{:?}", found(&balanced));
    let one_way = read_with(&cell("#:inp @CURRENT(\"in\"); #R1:1 @CURRENT(\"out\")"), false);
    assert_eq!(violations(&one_way, codes::VIOLATION_CURRENT_ONE_WAY), 1, "{:?}", found(&one_way));
}

#[test]
fn an_assertion_on_a_group_checks_every_member() {
    // ASSERT-INSTANCE on a set marked GROUP checks its members (15.3)
    let text = |second: &str| {
        format!(
            "* assert\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nM1 d1 g s 0 nch\nM2 d2 g s 0 {second}\n.model nch nmos\n.model pch pmos\n* @RNL {{ %g = {{#M1 #M2}} @GROUP; %g @ASSERT-INSTANCE(type=\"nmos\") }}\n.end\n"
        )
    };
    let all = read_with(&text("nch"), false);
    assert!(!all.diagnostics().iter().any(|d| d.severity == Severity::Violation), "{:?}", found(&all));
    let one = read_with(&text("pch"), false);
    assert_eq!(one.diagnostics().iter().filter(|d| d.severity == Severity::Violation).count(), 1, "{:?}", found(&one));
}

#[test]
fn a_usage_is_one_member_a_black_box_of_the_nets_it_shares() {
    // the usage brings R1 and R2, from p over m to q; m is used only inside it, and p is a port
    // (15.4)
    let text = |r3: &str, topology: &str| {
        format!(
            "* topology\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.subckt cell p q\nR1 p m 1k\nR2 m q 1k\nR3 {r3}\n* @RNL {{ $B = @DEFINE(roles=[\"x\", \"y\"]) {{}}; %u = #R1(x) #R2(y) @AS $B; {{%u #R3}} @ASSERT-TOPOLOGY(\"{topology}\") }}\n.ends cell\nX1 a b cell\n.end\n"
        )
    };
    for (r3, topology, holds) in [
        // R3 beside the box, from p to q: the two are in parallel, and not in series
        ("p q 1k", "parallel", true),
        ("p q 1k", "series", false),
        // R3 after it, from q on: they share q alone, in series and as a star
        ("q z 1k", "series", true),
        ("q z 1k", "star", true),
    ] {
        let r = read_with(&text(r3, topology), false);
        assert_eq!(
            violations(&r, codes::VIOLATION_TOPOLOGY),
            usize::from(!holds),
            "R3 {r3}, {topology}: {:?}",
            found(&r)
        );
    }
}

#[test]
fn a_port_counts_for_two_black_boxes_that_bring_the_same_terminal() {
    // {R1 R2} and {R2 R3} both bring R2, the only terminal on n: n is used outside neither, so it
    // is a net of both only when it is a port of the level (15.4)
    let text = |ports: &str, outer: &str, topology: &str| {
        format!(
            "* overlap\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.subckt cell {ports}\nR1 y1 x 1k\nR2 n x 1k\nR3 x y3 1k\n* @RNL {{ {{{{#R1 #R2}} {{#R2 #R3}}}} @ASSERT-TOPOLOGY(\"{topology}\") }}\n.ends cell\nX1 {outer} cell\n.end\n"
        )
    };
    for (ports, outer, topology, holds) in [
        // n a port: the two share n and x, in parallel and neither in series nor a star
        ("y1 y3 n", "a b c", "parallel", true),
        ("y1 y3 n", "a b c", "series", false),
        ("y1 y3 n", "a b c", "star", false),
        // n inside the level: they share x alone
        ("y1 y3", "a b", "parallel", false),
        ("y1 y3", "a b", "series", true),
        ("y1 y3", "a b", "star", true),
    ] {
        let r = read_with(&text(ports, outer, topology), false);
        assert_eq!(
            violations(&r, codes::VIOLATION_TOPOLOGY),
            usize::from(!holds),
            "ports {ports}, {topology}: {:?}",
            found(&r)
        );
    }
}

#[test]
fn a_malformed_version_is_an_unsupported_one() {
    // "1" and "v1.2" are not of the form major.minor: the file is of an unsupported version, and
    // none of its RNL is read (16.1)
    for version in ["1", "v1.2"] {
        let text = format!(
            "* version\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ #R1 @PLACE(1k, 0) }}\n.end\n"
        );
        let r = read_with(&text, false);
        let found = found(&r);
        assert!(found.contains(&(codes::VERSION_UNSUPPORTED, Severity::Error, 2)), "{version}: {found:?}");
        // the number with a suffix on line 4 is not read, so it is no error
        assert!(!found.iter().any(|f| f.0 == codes::SYNTAX_NUMBER_SUFFIX), "{version}: {found:?}");
    }
}

#[test]
fn a_hash_of_an_algorithm_a_tool_does_not_know_is_not_compared() {
    // the hash carries a tag for its algorithm, and a tool that does not know it does not compare
    // it (17)
    let r = read_with("* hash\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a b 1k\n.end\n", false);
    let p = &r.problems[0];
    assert_eq!(rnl_runtime::problem::same_input(&rnl_runtime::problem::sig2(p, ""), p), Some(true));
    assert_eq!(rnl_runtime::problem::same_input("sig2:0000000000000000.0000000000000000", p), Some(false));
    assert_eq!(rnl_runtime::problem::same_input("zz9:0123456789abcdef", p), None);
    assert_eq!(rnl_runtime::problem::same_input("sig9:0123456789abcdef.0", p), None);
}
