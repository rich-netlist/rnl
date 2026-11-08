//! The geometry hints of `rnl check` and of a Document asked for them (`rnl_view::hints`): read-only,
//! each on a small drawing written for it, and none on a clean one.

use std::path::Path;
use std::process::Command;

use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Options;
use rnl_view::hints::{interpretation_hints, Hint, Kind};

/// A file of 0.3 with these devices and this record, read as `rnl check` reads it.
fn hints(devices: &str, record: &str) -> Vec<Hint> {
    hints_of(&format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n{devices}{}.end\n", records(record)))
}

/// [`hints`] of a subcircuit whose ports are `ports`, with these devices and this record; the top
/// level only calls it.
fn hints_in_cell(ports: &str, devices: &str, record: &str) -> Vec<Hint> {
    hints_of(&format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n.subckt cell {ports}\n{devices}{}.ends\nX1 {ports} cell\n.end\n",
        records(record)
    ))
}

/// `record` as a record of comment lines.
fn records(record: &str) -> String {
    format!("* @RNL {{\n{}\n* }}\n", record.lines().map(|l| format!("*   {}", l.trim())).collect::<Vec<_>>().join("\n"))
}

/// The hints of `text` read as `rnl check` reads it, which must read without errors.
fn hints_of(text: &str) -> Vec<Hint> {
    let files = MemoryProvider::new().with("/w/t.sp", text);
    let r =
        rnl_cli::interpret_file(Path::new("/w/t.sp"), None, &files, &Options { problems: true, ..Options::default() })
            .unwrap();
    let errors: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}\n{text}");
    interpretation_hints(&r).into_iter().flat_map(|l| l.hints).collect()
}

fn has(hs: &[Hint], kind: Kind, words: &[&str]) -> bool {
    hs.iter().any(|h| h.kind == kind && words.iter().all(|w| h.message.contains(w)))
}

fn kinds(hs: &[Hint]) -> Vec<Kind> {
    hs.iter().map(|h| h.kind).collect()
}

#[test]
fn a_clean_drawing_has_no_hints() {
    // every net wired
    let hs = hints(
        "R1 a b 1k
R2 b a 1k
",
        "#R1 @PLACE(0, 4);
#R2 @PLACE(0, -4);
&b @WIRE(mode=\"straight\") {#R1:2 #R2:1};
         &a @WIRE(mode=\"straight\") {#R1:1 (0, 8) (4, 8) (4, -8) (0, -8) #R2:2}",
    );
    assert!(hs.is_empty(), "{hs:#?}");
}

#[test]
fn a_wire_over_a_symbol_against_a_pin_and_over_another_nets_pin() {
    let hs = hints(
        "R1 a b 1k\nR2 c d 1k\nR3 b e 1k\nR4 f g 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(4, 0);\n#R3 @PLACE(8, -2);\n#R4 @PLACE(12, 0);\n\
         &b @WIRE(mode=\"straight\") {#R1:2 (0, -3) (2, -3) (2, 0) #R3:1};\n\
         &e @WIRE(mode=\"straight\") {#R3:2 (8, -6) (12, -6) (12, -2)}",
    );
    // the wire of b runs through R2, names it and the line of its PLACE
    assert!(has(&hs, Kind::WireOverSymbol, &["the wire of &b", "the symbol of #R2 (line 9)"]), "{hs:#?}");
    // it comes into R3's pin 1, which points up, from the left
    assert!(has(&hs, Kind::WrongExit, &["#R3:p", "going left", "points up"]), "{hs:#?}");
    // the wire of e ends on R4's pin of g: a warning, it looks joined
    let joined = hs.iter().find(|h| h.kind == Kind::LooksJoined).expect("looks joined");
    assert_eq!(joined.severity, Severity::Warning);
    assert!(joined.message.contains("#R4:n") && joined.message.contains("&g"), "{joined:?}");
    // each hint is placed at a line of the record
    assert!(hs.iter().all(|h| h.loc.as_ref().is_some_and(|l| l.line > 0)), "{hs:#?}");
}

#[test]
fn two_nets_on_one_line_one_step_apart_and_crossing() {
    let hs = hints(
        "R1 a b 1k
R2 c d 1k
R3 e f 1k
",
        "#R1 @PLACE(0, 0);
#R2 @PLACE(10, 0);
#R3 @PLACE(20, 0);
         &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) (6, 4)};
         &c @WIRE(mode=\"straight\") {#R2:1 (10, 4) (4, 4)};
         &b @WIRE(mode=\"straight\") {#R1:2 (0, -4) (6, -4)};
         &d @WIRE(mode=\"straight\") {#R2:2 (10, -5) (2, -5)};
         &e @WIRE(mode=\"straight\") {#R3:1 (20, 6) (8, 6) (8, 2)}",
    );
    assert!(has(&hs, Kind::Overlap, &["&a", "&c", "on one line for 2"]), "{hs:#?}");
    assert!(hs.iter().filter(|h| h.kind == Kind::Overlap).all(|h| h.severity == Severity::Warning));
    assert!(has(&hs, Kind::Close, &["&b", "&d", "1 apart for 4"]), "{hs:#?}");
    // the wire of e comes down across the wire of c: the summary line counts it
    assert!(has(&hs, Kind::Crossings, &["1 crossing of wires", "&e", "&c", "at (8, 4)"]), "{hs:#?}");
}

#[test]
fn a_t_join_needs_a_dot_and_a_dot_on_a_corner_is_the_authors() {
    let hs = hints(
        "R1 a b 1k\nR2 a c 1k\nR3 a d 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n#R3 @PLACE(12, 0);\n%j = (6, 4);\n%k = (0, 4);\n\
         &a @WIRE(mode=\"straight\") {#R1:1 %k %j (12, 4) #R3:1};\n&a @WIRE(mode=\"straight\") {%j #R2:1};\n\
         $JUNCTION @ATTACH %k",
    );
    assert!(has(&hs, Kind::NoDot, &["&a joins at (6, 4) without a junction dot"]), "{hs:#?}");
    // a dot where only two wires meet is the author's to draw (13.2): nothing is said of it
    assert_eq!(kinds(&hs), [Kind::NoDot], "{hs:#?}");
    // with the dot where the wires meet, and none on the corner, nothing is said
    let hs = hints(
        "R1 a b 1k\nR2 a c 1k\nR3 a d 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n#R3 @PLACE(12, 0);\n%j = (6, 4);\n\
         &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) %j (12, 4) #R3:1};\n&a @WIRE(mode=\"straight\") {%j #R2:1};\n\
         $JUNCTION @ATTACH %j",
    );
    assert!(!kinds(&hs).contains(&Kind::NoDot), "{hs:#?}");
}

#[test]
fn a_wire_end_on_its_net_that_it_does_not_name_is_a_silent_open() {
    let hs = hints(
        "R1 a b 1k\nR2 a b 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n\
         &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) (3, 4)};\n&a @WIRE(mode=\"straight\") {(3, 4) (6, 4) #R2:1};\n\
         &b @WIRE(mode=\"straight\") {#R1:2 (0, -4) (6, -4) (6, -2)}",
    );
    let open: Vec<&Hint> = hs.iter().filter(|h| h.kind == Kind::SilentOpen).collect();
    // the two wires of a meet at (3, 4) by their coordinates alone: said once
    assert_eq!(open.iter().filter(|h| h.message.contains("at (3, 4)")).count(), 1, "{open:#?}");
    // the wire of b ends on R2's pin 2 without naming it
    assert!(open.iter().any(|h| h.message.contains("#R2:2") && h.message.contains("write #R2:2")), "{open:#?}");
    assert!(open.iter().all(|h| h.severity == Severity::Hint));
}

#[test]
fn a_port_enclosed_or_facing_into_the_drawing() {
    // e and m are ports of the subcircuit, and the usages attached to them stand for them
    let hs = hints_in_cell(
        "e m",
        "R1 a b 1k
R2 a c 1k
R3 f g 1k
R4 h k 1k
R5 m e 1k
",
        "#R1 @PLACE(0, 0);
#R2 @PLACE(8, 0);
#R3 @PLACE(3, 4) => @ORIENT(\"R90\");
         #R4 @PLACE(3, -4) => @ORIENT(\"R90\");
%p = $IO_PORT @ATTACH &e;
%p @PLACE(4, 0);
         #R5 @PLACE(-10, 3) => @ORIENT(\"R90\");
%q = $IO_PORT @ATTACH &m;
%q @PLACE(-15, 3) => @ORIENT(\"MY\");
         &m @WIRE(mode=\"straight\") {%q:p #R5:1}",
    );
    assert!(has(&hs, Kind::PortEnclosed, &["the port %p", "&e", "is enclosed"]), "{hs:#?}");
    assert!(has(&hs, Kind::PortInside, &["the port %q", "faces right into the drawing", "#R5"]), "{hs:#?}");
}

#[test]
fn a_net_partly_wired_and_a_pin_joined_by_name_alone() {
    // no ROUTE, which leaves the pins to a solver; the fill of the series 0.3 labels no pin
    let devices = "R1 a b 1k\nR2 b c 1k\nR3 b d 1k\n";
    let placed = "#R1 @PLACE(0, 0);\n#R2 @PLACE(4, 0) => @ORIENT(\"R180\");\n#R3 @PLACE(8, 0);\n\
                  &b @WIRE(mode=\"straight\") {#R1:2 (0, -3) (4, -3) #R2:1}";
    let hs = hints(devices, placed);
    assert!(has(&hs, Kind::PartlyNamed, &["&b is drawn in 2 pieces", "#R3:p"]), "{hs:#?}");
    // the fix it gives names the pin, and no definition of a package
    assert!(
        has(&hs, Kind::NameOnly, &["#R3:p is joined to &b by its name alone", "give it a label at #R3:1"]),
        "{hs:#?}"
    );
    // a net-name text at the pin names it: no longer headless, though the net is still in pieces
    let hs = hints(devices, &format!("{placed};\n#R3:1 @TEXT(\"${{net}}\", key=\"netname\")"));
    assert!(!kinds(&hs).contains(&Kind::NameOnly), "{hs:#?}");
    assert!(has(&hs, Kind::PartlyNamed, &["{#R3:p} a label"]), "{hs:#?}");
}

#[test]
fn a_segment_of_no_length_and_a_loop() {
    let hs = hints(
        "R1 a b 1k\n",
        "#R1 @PLACE(0, 0);\n%p = (0, -4);\n%q = (3, -4);\n\
         &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) (0, 4) (3, 4)};\n\
         &b @WIRE(mode=\"straight\") {#R1:2 %p %q};\n&b @WIRE(mode=\"straight\") {%p (0, -6) (3, -6) %q}",
    );
    assert!(has(&hs, Kind::ZeroLength, &["the wire of &a", "no length at (0, 4)"]), "{hs:#?}");
    assert!(has(&hs, Kind::Loop, &["&b close a loop"]), "{hs:#?}");
}

#[test]
fn texts_over_texts_and_wires_over_texts() {
    let hs = hints(
        "R1 a b 1k\n",
        "#R1 @PLACE(0, 0);\n(-6, 6) @TEXT(\"hello\");\n(-5.5, 6.2) @TEXT(\"world\");\n\
         &a @WIRE(mode=\"straight\") {#R1:1 (0, 6.5) (-8, 6.5)}",
    );
    assert!(has(&hs, Kind::TextOverText, &["\"hello\"", "\"world\""]), "{hs:#?}");
    assert!(has(&hs, Kind::WireOverText, &["the wire of &a", "\"hello\""]), "{hs:#?}");
}

#[test]
fn check_prints_the_hints_and_counts_them_but_does_not_fail() {
    let dir = std::env::temp_dir().join(format!("rnl-hints-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("t.sp");
    std::fs::write(
        &f,
        "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a b 1k\nR2 c d 1k\n* @RNL {\n\
         *   #R1 @PLACE(0, 0); #R2 @PLACE(10, 0);\n\
         *   &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) (6, 4)};\n\
         *   &c @WIRE(mode=\"straight\") {#R2:1 (10, 4) (4, 4)}\n* }\n.end\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(["check", &f.to_string_lossy()]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.lines().any(|l| l.starts_with("warning t.sp:7:") && l.contains("run on one line")), "{text}");
    let last = text.lines().last().unwrap();
    assert!(last.starts_with("0 errors, ") && !last.contains(" 0 warnings"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------- regressions

/// Ports in a column at the edge, each facing out: none is buried or enclosed by its neighbours.
#[test]
fn ports_side_by_side_at_the_edge_are_not_buried() {
    let hs = hints_in_cell(
        "a b c",
        "R1 a o 1k\nR2 b o 1k\nR3 c o 1k\n",
        "%pa = $IO_PORT @ATTACH &a;\n%pb = $IO_PORT @ATTACH &b;\n%pc = $IO_PORT @ATTACH &c;\n\
         %pa @PLACE(-6, 6);\n%pb @PLACE(-6, 3);\n%pc @PLACE(-6, 0);\n\
         #R1 @PLACE(2, 6) => @ORIENT(\"R90\");\n#R2 @PLACE(2, 3) => @ORIENT(\"R90\");\n#R3 @PLACE(2, 0) => @ORIENT(\"R90\");\n\
         &a @WIRE(mode=\"straight\") {%pa:p #R1:1};\n&b @WIRE(mode=\"straight\") {%pb:p #R2:1};\n&c @WIRE(mode=\"straight\") {%pc:p #R3:1};\n\
         %o = (6, 3);\n&o @WIRE(mode=\"straight\") {#R1:2 (6, 6) %o (6, 0) #R3:2};\n&o @WIRE(mode=\"straight\") {#R2:2 %o};\n$JUNCTION @ATTACH %o",
    );
    assert!(!kinds(&hs).iter().any(|k| matches!(k, Kind::PortInside | Kind::PortEnclosed)), "{hs:#?}");
}

/// A wire that writes its pin's place and then the pin has one point there: no T-join, no
/// segment of no length.
#[test]
fn a_pin_written_twice_is_one_point() {
    let hs = hints(
        "R1 a b 1k\nR2 a c 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n&a @WIRE(mode=\"straight\") {#R1:1 (0, 4) (6, 4) (6, 2) #R2:1}",
    );
    assert!(!kinds(&hs).iter().any(|k| matches!(k, Kind::NoDot | Kind::ZeroLength)), "{hs:#?}");
}

/// Two wires that share a named point are joined through it, whatever lies at its place.
#[test]
fn wires_joined_through_a_shared_point_are_no_silent_open() {
    let hs = hints(
        "R1 a b 1k\nR2 a c 1k\nR3 a d 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n#R3 @PLACE(12, 0);\n%j = (0, 2);\n\
         &a @WIRE(mode=\"straight\") {#R1:1 %j (0, 4) (6, 4) #R2:1};\n\
         &a @WIRE(mode=\"straight\") {%j (-3, 2) (-3, 6) (12, 6) #R3:1};\n$JUNCTION @ATTACH %j",
    );
    assert!(!kinds(&hs).contains(&Kind::SilentOpen), "{hs:#?}");
}

/// The name over the value of a two-pin part, as the package stacks them, never lies over itself,
/// whatever the orientation and however short the value.
#[test]
fn the_stack_of_a_name_and_a_value_is_no_text_over_text() {
    for value in ["5", "2p", "1k"] {
        for o in ["R0", "R90", "R180", "R270", "MX", "MY", "MXR90", "MYR90"] {
            let hs = hints(&format!("R1 a b {value}\n"), &format!("#R1 @PLACE(0, 0) => @ORIENT(\"{o}\")"));
            assert!(!kinds(&hs).contains(&Kind::TextOverText), "{value} {o}: {hs:#?}");
        }
    }
}

/// A net a ROUTE is to wire, drawn before it is solved, is left to the solver: not joined by name.
#[test]
fn a_net_a_route_is_to_wire_is_not_joined_by_name() {
    let hs = hints(
        "R1 a b 1k\nR2 b c 1k\nR3 c a 1k\n",
        "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n#R3 @PLACE(12, 0);\n&b @ROUTE;\n&c @ROUTE;\n\
         &c @WIRE(mode=\"straight\") {#R2:2 (6, -4) (12, -4) (12, -6)}",
    );
    // a, which no ROUTE names and no wire draws, is joined by name alone; b and c are not
    let by_name = |net: &str| {
        hs.iter()
            .any(|h| matches!(h.kind, Kind::NameOnly | Kind::PartlyNamed) && h.message.contains(&format!("&{net} ")))
    };
    assert!(by_name("a") && !by_name("b") && !by_name("c"), "{hs:#?}");
}

/// The hints of a long chain take time in proportion to it: 800 resistors well within seconds.
#[test]
fn the_hints_of_a_long_chain_are_quick() {
    let n = 800;
    let mut devices = String::new();
    let mut record = String::new();
    for k in 1..=n {
        devices.push_str(&format!("R{k} n{k} n{} 1k\n", k + 1));
        record.push_str(&format!("#R{k} @PLACE({}, 0);\n", 6 * k));
    }
    // each wire from the bottom of one resistor down, across, up past it and down to the top of the next
    for k in 1..n {
        let (x, m, y) = (6 * k, 6 * k + 3, 6 * (k + 1));
        record.push_str(&format!(
            "&n{} @WIRE(mode=\"straight\") {{#R{k}:2 ({x}, -4) ({m}, -4) ({m}, 4) ({y}, 4) #R{}:1}};\n",
            k + 1,
            k + 1
        ));
    }
    let start = std::time::Instant::now();
    let hs = hints(&devices, record.trim_end().trim_end_matches(';'));
    assert!(start.elapsed().as_secs_f64() < 20.0, "{:?}", start.elapsed());
    assert!(!kinds(&hs).contains(&Kind::NoDot), "{:#?}", &hs[..hs.len().min(5)]);
}

/// A wire of several nets, such as a bus, and a wire or a pin of one of them are not two nets: the
/// taps on the buses of the conformance case of buses and bundles look joined to nothing.
#[test]
fn a_bus_and_its_taps_do_not_look_joined() {
    let case = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases/09-buses-and-bundles.sp");
    let text = std::fs::read_to_string(case).unwrap();
    let files = MemoryProvider::new().with("/w/t.sp", &text);
    let r =
        rnl_cli::interpret_file(Path::new("/w/t.sp"), None, &files, &Options { problems: true, ..Options::default() })
            .unwrap();
    // the case draws wires of several nets, which wires and pins of one of them meet
    assert!(r.model.levels.iter().any(|lv| lv.wires.iter().any(|w| w.nets.len() > 1)));
    let hs: Vec<Hint> = interpretation_hints(&r).into_iter().flat_map(|l| l.hints).collect();
    assert!(!kinds(&hs).contains(&Kind::LooksJoined), "{hs:#?}");
}

/// A Document has the hints of each level when asked for them, as `rnl check` prints them, with the
/// file and line of each; not asked, it has none, and its levels have no `hints`.
#[test]
fn a_document_has_the_hints_when_asked() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a b 1k\nR2 c d 1k\n* @RNL {\n\
                *   #R1 @PLACE(0, 0); #R2 @PLACE(10, 0);\n\
                *   &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) (6, 4)};\n\
                *   &c @WIRE(mode=\"straight\") {#R2:1 (10, 4) (4, 4)}\n* }\n.end\n";
    let files = MemoryProvider::new().with("/w/t.sp", text);
    let asked = rnl_view::BuildOptions { hints: true, ..rnl_view::BuildOptions::default() };
    let d = rnl_view::build("/w/t.sp", &files, &asked).unwrap();
    let overlap = d.levels[0].hints.iter().find(|h| h.kind == "overlap").expect("overlap");
    assert_eq!(
        (overlap.severity, overlap.file.as_str(), overlap.file_index, overlap.line),
        ("warning", "t.sp", Some(0), 7)
    );
    assert!(overlap.message.contains("run on one line"), "{overlap:?}");
    let d = rnl_view::build("/w/t.sp", &files, &rnl_view::BuildOptions::default()).unwrap();
    assert!(d.levels[0].hints.is_empty());
    assert!(serde_json::to_value(&d).unwrap()["levels"][0].get("hints").is_none());
}

/// The hints tell objects apart by their structure, not by the names of a package's definitions: a
/// usage of a definition of the file's own, with its pin at its own point, marks the point it sits
/// on, as a junction dot of the standard series does; attached to a net, it stands for the net.
#[test]
fn a_junction_dot_or_a_net_symbol_of_any_name() {
    let own = "$DOT_SYM = @DEFINE-SYMBOL {(0, 0) @PIN(terminal=\"p\"); (0, 0) @CIRCLE(radius=0.2)};\n\
               $DOT = @DEFINE(roles=[[\"p\", [\"point\", \"net\", \"terminal\"]]]) {};\n\
               # @SELECT-USAGE $DOT => @SET-SYMBOL $DOT_SYM;\n";
    let tee = "#R1 @PLACE(0, 0);\n#R2 @PLACE(6, 0);\n#R3 @PLACE(12, 0);\n%j = (6, 4);\n\
               &a @WIRE(mode=\"straight\") {#R1:1 (0, 4) %j (12, 4) #R3:1};\n&a @WIRE(mode=\"straight\") {%j #R2:1}";
    let devices = "R1 a b 1k\nR2 a c 1k\nR3 a d 1k\n";
    // without a dot, the T-join wants one; with one of the file's own definition, it has one
    assert!(has(&hints(devices, &format!("{own}{tee}")), Kind::NoDot, &["&a joins at (6, 4)"]));
    let hs = hints(devices, &format!("{own}{tee};\n$DOT @ATTACH %j"));
    assert!(!kinds(&hs).contains(&Kind::NoDot), "{hs:#?}");
    // attached to a net, the same usage stands for it: its net is left out of the joins by name
    let pieces = "#R1 @PLACE(0, 0);\n#R2 @PLACE(4, 0) => @ORIENT(\"R180\");\n#R3 @PLACE(8, 0);\n\
                  &b @WIRE(mode=\"straight\") {#R1:2 (0, -3) (4, -3) #R2:1}";
    let devices = "R1 a b 1k\nR2 b c 1k\nR3 b d 1k\n";
    assert!(has(&hints(devices, &format!("{own}{pieces}")), Kind::PartlyNamed, &["&b"]));
    let hs = hints(devices, &format!("{own}{pieces};\n%n = $DOT @ATTACH &b;\n%n @PLACE(8, -3)"));
    assert!(!kinds(&hs).iter().any(|k| matches!(k, Kind::NameOnly | Kind::PartlyNamed)), "{hs:#?}");
    // attached to a terminal, its pin at its own point on the terminal's net, it stands for the
    // net too, as a ground or a supply drawn on a pin does
    let hs = hints(devices, &format!("{own}{pieces};\n$DOT @ATTACH #R3:1"));
    assert!(!kinds(&hs).iter().any(|k| matches!(k, Kind::NameOnly | Kind::PartlyNamed)), "{hs:#?}");
}
