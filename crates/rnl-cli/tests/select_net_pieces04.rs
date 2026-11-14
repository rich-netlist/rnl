//! The pieces of a net in selections (10.1): `SELECT-NET(split=true)` takes a net drawn in two or
//! more pieces, `SELECT-PIN(piece_first=true)` the first pin of each piece. The pins that one tree
//! of a ROUTE joins (13.7), or that written wires join, are one piece; any other pin is one by
//! itself.
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::Filler;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

/// Four resistors on the net `n` and the lines `extra`, all of the symbol $S, with `body`; the
/// usages of $PROBE are what a selection took.
fn read(version: &str, extra: &str, body: &str) -> Interpretation {
    let text = format!(
        "* pieces\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a n 1k\nR2 n b 1k\nR3 c n 1k\nR4 n d 1k\n{extra}* @RNL {{\n*   $PROBE = @DEFINE(roles=[\"x\"]) {{}};\n*   $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) }};\n*   # @SELECT-INSTANCE => @SET-SYMBOL $S;\n*   {body}\n* }}\n.end\n"
    );
    interpret_file(
        Path::new("main.sp"),
        None,
        &MemoryProvider::new().with("main.sp", &text),
        &Options { std: false, ..Options::solver() },
    )
    .unwrap()
}

fn selected(extra: &str, body: &str, selection: &str) -> Vec<String> {
    let r = read("0.4", extra, &format!("{body}; {selection} => @AS $PROBE"));
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    let p = &r.problems[0];
    p.usages
        .iter()
        .filter(|u| u.definition == "$PROBE")
        .flat_map(|u| u.roles[0].members.iter())
        .map(|m| match m {
            Filler::Net(name) => name.clone(),
            Filler::Pin { object, pin } => format!("{}.{pin}", p.ids.objects[*object]),
            other => format!("{other:?}"),
        })
        .collect()
}

fn split(body: &str) -> Vec<String> {
    selected("", body, "# @SELECT-NET(split=true)")
}

fn firsts(extra: &str, body: &str) -> Vec<String> {
    selected(extra, body, "&n @SELECT-PIN(piece_first=true)")
}

#[test]
fn a_net_whose_pins_nothing_joins_is_in_as_many_pieces_as_pins() {
    assert_eq!(split(""), ["n"]);
    let mut all = selected("", "", "# @SELECT-NET(split=false)");
    all.sort();
    assert_eq!(all, ["a", "b", "c", "d"], "a net of one pin is one piece");
    assert_eq!(firsts("", "").len(), 4);
}

#[test]
fn one_tree_over_the_net_makes_one_piece_and_two_trees_two() {
    assert!(split("{#R1 #R2 #R3 #R4} @ROUTE-OBJECT").is_empty());
    assert_eq!(firsts("", "{#R1 #R2 #R3 #R4} @ROUTE-OBJECT"), ["#R1.1"]);
    assert_eq!(split("{#R1 #R2} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT"), ["n"]);
    assert_eq!(firsts("", "{#R4 #R3} @ROUTE-OBJECT; {#R2 #R1} @ROUTE-OBJECT"), ["#R1.1", "#R3.1"]);
}

#[test]
fn trees_that_share_a_pin_and_written_wires_join_their_pieces() {
    assert!(split("{#R1 #R2} @ROUTE-OBJECT; {#R2 #R3 #R4} @ROUTE-OBJECT").is_empty());
    assert!(split("{#R1 #R2} @ROUTE-OBJECT; &n @WIRE {#R2:1 (3,5) #R3:2}; {#R3 #R4} @ROUTE-OBJECT").is_empty());
    let wire = "&n @WIRE {#R1:2 (3,0) #R2:1}";
    assert_eq!(split(wire), ["n"]);
    assert_eq!(firsts("", wire), ["#R1.1", "#R3.1", "#R4.0"]);
    // the fill names the first pin of each wired piece only
    assert_eq!(selected("", wire, "&n @SELECT-PIN(piece_first=true, wired=true)"), ["#R1.1"]);
}

#[test]
fn the_first_pin_of_an_instance_is_the_one_of_its_lowest_terminal() {
    // #R5 draws its terminal 2 before its terminal 1, both on n
    let extra = "R5 n n 1k\n";
    let body = "$T = @DEFINE-SYMBOL { (0,0) @PIN(terminal=2); (2,0) @PIN(terminal=1) }; #R5 @SET-SYMBOL $T; {#R5} @ROUTE-OBJECT";
    assert_eq!(firsts(extra, body), ["#R1.1", "#R2.0", "#R3.1", "#R4.0", "#R5.1"]);
}

#[test]
fn the_pins_of_virtual_parts_come_first() {
    let body = "$L = @DEFINE(roles=[[\"p\", \"net\"]]) {}; $LS = @DEFINE-SYMBOL { (0,0) @PIN(terminal=\"p\") }; %l = $L @ATTACH &n; %l @SET-SYMBOL $LS; {#R1 #R2} @ROUTE-OBJECT; {#R2 %l} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT";
    // the label's pin, not #R1's, starts the piece {#R1 #R2 %l}
    let mut got = firsts("", body);
    got.sort();
    assert_eq!(got, ["#R3.1", "%l.0"]);
}

#[test]
fn split_is_a_word_of_0_4() {
    // SELECT-PIN, and with it piece_first, is new in 0.4 as a whole
    let r = read("0.3", "", "# @SELECT-NET(split=true) => @AS $PROBE");
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
}
