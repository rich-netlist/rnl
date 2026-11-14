//! A net drawn in pieces that only its name joins (RNL7008 partly-named), with the pieces as a
//! solver reads the level: the pins that one tree of a ROUTE joins are one piece (13.7), drawn or not.
use rnl_cli::interpret_file;
use rnl_runtime::{diag::Severity, problem::net_pieces, source::MemoryProvider, Interpretation, Options};
use rnl_view::hints::{interpretation_hints, Hint, Kind};
use std::path::Path;

/// Four resistors on the net `n`, placed, with `body` and then `generated`, a record of a solver.
fn read(body: &str, generated: &str) -> Interpretation {
    let text = format!(
        "* pieces\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a n 1k\nR2 n b 1k\nR3 c n 1k\nR4 n d 1k\n* @RNL {{\n*   $S = @DEFINE-SYMBOL {{ (0,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) }};\n*   {{#R1 #R2 #R3 #R4}} @SET-SYMBOL $S;\n*   #R1 @PLACE(0,0); #R2 @PLACE(4,0); #R3 @PLACE(0,10); #R4 @PLACE(4,10);\n*   {body}\n* }}\n{generated}.end\n"
    );
    let files = MemoryProvider::new().with("main.sp", &text);
    let r = interpret_file(
        Path::new("main.sp"),
        None,
        &files,
        &Options { std: false, problems: true, ..Default::default() },
    )
    .unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    r
}

fn partly_named(r: &Interpretation) -> Vec<Hint> {
    interpretation_hints(r).into_iter().flat_map(|l| l.hints).filter(|h| h.kind == Kind::PartlyNamed).collect()
}

/// The number of pieces of the net `n`, as a solver reads the level.
fn pieces_of_n(r: &Interpretation) -> usize {
    let p = &r.problems[0];
    let n = p.nets.iter().position(|net| net.name == "n").expect("the net n");
    net_pieces(p)[n].len()
}

const TWO_TREES: &str = "{#R1 #R2} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT";

#[test]
fn two_trees_drawn_by_a_solver_are_two_pieces_joined_by_name() {
    // a ROUTE no longer hides the net from the hint once its trees are drawn
    let record = "* @RNL(type=\"generated\", priority=\"high\", author=\"test\", hash=\"test\") { &n @WIRE(mode=\"straight\") {#R1:2 (3,0) #R2:1}; &n @WIRE(mode=\"straight\") {#R3:2 (3,10) #R4:1} }\n";
    let r = read(TWO_TREES, record);
    let hints = partly_named(&r);
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert!(hints[0].message.contains("&n is drawn in 2 pieces"), "{}", hints[0].message);
    assert!(hints[0].message.contains("{#R1:n, #R2:p} wired, {#R3:n, #R4:p} wired"), "{}", hints[0].message);
}

#[test]
fn two_trees_not_drawn_yet_are_two_pieces_already() {
    let r = read(TWO_TREES, "");
    assert_eq!(pieces_of_n(&r), 2);
    let hints = partly_named(&r);
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert!(hints[0].message.contains("{#R1:n, #R2:p} routed, {#R3:n, #R4:p} routed"), "{}", hints[0].message);
    // a pin in a tree is not joined by its name alone
    let all: Vec<_> = interpretation_hints(&r).into_iter().flat_map(|l| l.hints).collect();
    assert!(!all.iter().any(|h| h.kind == Kind::NameOnly), "{all:?}");
}

#[test]
fn one_tree_over_the_net_is_one_piece() {
    let r = read("{#R1 #R2 #R3 #R4} @ROUTE-OBJECT", "");
    assert_eq!(pieces_of_n(&r), 1);
    assert!(partly_named(&r).is_empty());
}

#[test]
fn trees_that_share_a_pin_and_written_wires_join_pieces() {
    // the trees share #R2; the wire joins #R3 to #R4
    let r = read("{#R1 #R2} @ROUTE-OBJECT; {#R2 #R3} @ROUTE-OBJECT", "");
    assert_eq!(pieces_of_n(&r), 2, "#R4 is a piece by itself");
    let r = read("{#R1 #R2} @ROUTE-OBJECT; &n @WIRE(mode=\"straight\") {#R3:2 (3,10) #R4:1}", "");
    assert_eq!(pieces_of_n(&r), 2);
    let r = read("", "");
    assert_eq!(pieces_of_n(&r), 4, "no tree and no wire: each pin is a piece");
}
