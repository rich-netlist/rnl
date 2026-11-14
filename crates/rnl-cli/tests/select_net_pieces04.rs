//! The pieces of a net in selections (10.1): `SELECT-NET(split=true)` takes a net drawn in two or
//! more pieces, `SELECT-PIN(piece_first=true)` the first pin of each piece. The pins that one tree
//! of a ROUTE joins (13.7), or that written wires join, are one piece; any other pin is one by
//! itself.
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::{net_pieces, Filler, LevelProblem};
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

/// The pieces the selections saw agree with those of the whole level (`problem::net_pieces`):
/// every pin they saw on a net is on it in the problem (`ProblemNet::pins`), the pins of parts
/// attached to the net itself, such as `$SUPPLY`, included; and what step 6 adds may join a new
/// pin to a piece, never two pieces into one or one into two. Returns the number of pieces the
/// selections saw of each net of each level, by name.
fn agree(r: &Interpretation) -> Vec<(String, usize)> {
    assert_eq!(r.selection_pieces.len(), r.problems.len());
    r.problems.iter().zip(&r.selection_pieces).flat_map(|(p, seen)| agree_on(p, seen)).collect()
}

fn agree_on(p: &LevelProblem, seen: &[Vec<Vec<(usize, usize)>>]) -> Vec<(String, usize)> {
    use std::collections::BTreeSet;
    type Piece = BTreeSet<(usize, usize)>;
    assert_eq!(seen.len(), p.nets.len());
    let whole = net_pieces(p);
    p.nets
        .iter()
        .enumerate()
        .map(|(n, net)| {
            let listed: Piece = net.pins.iter().filter_map(|q| q.pin_key()).collect();
            let pins: Piece = seen[n].iter().flatten().copied().collect();
            if let Some(k) = pins.difference(&listed).next() {
                panic!("&{}: {}.{} is not on the net in the problem", net.name, p.ids.objects[k.0], k.1);
            }
            let mut ours: Vec<Piece> = seen[n].iter().map(|piece| piece.iter().copied().collect()).collect();
            let mut theirs: Vec<Piece> = whole[n]
                .iter()
                .map(|piece| piece.iter().filter_map(|q| q.pin_key()).filter(|k| pins.contains(k)).collect())
                .filter(|piece: &Piece| !piece.is_empty())
                .collect();
            ours.sort();
            theirs.sort();
            assert_eq!(ours, theirs, "&{}", net.name);
            (net.name.clone(), seen[n].len())
        })
        .collect()
}

#[test]
fn the_pieces_the_selections_see_are_those_of_the_level() {
    for body in [
        "",
        "{#R1 #R2 #R3 #R4} @ROUTE-OBJECT",
        "{#R1 #R2} @ROUTE-OBJECT; {#R3 #R4} @ROUTE-OBJECT",
        "{#R1 #R2} @ROUTE-OBJECT; {#R2 #R3} @ROUTE-OBJECT",
        "{#R1 #R2} @ROUTE-OBJECT; &n @WIRE {#R3:2 (3,10) #R4:1}",
        "&n @WIRE {#R1:2 (3,0) #R2:1}; &n @WIRE {#R2:1 (3,5) #R3:2}",
    ] {
        let r = read("0.4", "", body);
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
        agree(&r);
    }
}

#[test]
fn the_pieces_of_a_sampler_and_an_inverter_with_the_standard_packages() {
    // a transmission gate holds its output on a capacitor, which an inverter reads: the gate's
    // tree and the inverter's are two pieces of &x
    let text = "* sampler\n* @RNL(type=\"header\") {\n*   @VERSION(\"0.4\");\n*   @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\");\n*   @DEVICE(name=\"M*\", model=\"n*\", terminals=[\"d\", \"g\", \"s\", \"b\"], type=\"nmos\");\n*   @DEVICE(name=\"M*\", model=\"p*\", terminals=[\"d\", \"g\", \"s\", \"b\"], type=\"pmos\");\n* }\n.model n1 nmos level=1\n.model p1 pmos level=1\n.subckt hold a y ck ckb vdd gnd\nMA a ck x gnd n1\nMB a ckb x vdd p1\nC1 x gnd 1p\nMC y x vdd vdd p1\nMD y x gnd gnd n1\n* @RNL {\n*   &vdd @ADD-TO $SUPPLY_NET_SET;\n*   &gnd @ADD-TO $GROUND_NET_SET;\n*   %g = #MB(pmos) #MA(nmos) @AS $TRANSMISSION_GATE;\n*   %g(switch) #C1(capacitor) @AS $SC_SAMPLER;\n*   #MC(pmos) #MD(nmos) @AS $CMOS_INVERTER;\n*   %pa = $IO_PORT @ATTACH &a;\n*   %py = $IO_PORT @ATTACH &y;\n*   %s = $SUPPLY @ATTACH &vdd;\n*   $SUPPLY_STUB @ATTACH #MC::s;\n* }\n.ends hold\n";
    let r =
        interpret_file(Path::new("hold.sp"), None, &MemoryProvider::new().with("hold.sp", text), &Options::solver())
            .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    let counts = agree(&r);
    let x = counts.iter().find(|(name, _)| name == "x").expect("&x").1;
    assert!(x >= 2, "&x is drawn in pieces: {counts:?}");
}

#[test]
fn split_is_a_word_of_0_4() {
    // SELECT-PIN, and with it piece_first, is new in 0.4 as a whole
    let r = read("0.3", "", "# @SELECT-NET(split=true) => @AS $PROBE");
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    // a reading of 0.3 computes no pieces
    let r = read("0.3", "", "");
    assert!(r.selection_pieces.iter().flatten().all(|net| net.is_empty()));
    assert!(!read("0.4", "", "").selection_pieces.iter().flatten().all(|net| net.is_empty()));
}
