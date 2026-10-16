//! A later segment of a pipe is written later, wherever the place of writing matters (5.5, as the
//! draft 0.3 makes it clear, in every version): in the appearance queues (8.3), in override by key
//! (8.4), in the order of resolution (8.5, in `solver.rs`), in the drawing order (11.6), and for
//! PRIORITY (14.8) and CURRENT (15.2), between the PLACE of a text and its offset (13.3), and for the
//! statement that closes a cycle of sets (5.2.2); the rewrites of a wire are in `rewrite.rs`. The later
//! segment comes after all that the earlier one holds, such as the statements nested in it: in
//! `{#R2 @LAYER("top")} => @LAYER("bottom")`, the outer LAYER is the later. Before, the segments
//! of one statement shared its position, so the earlier of two could win, and a statement nested
//! in an earlier segment won over a later segment.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::model::{ItemView, Layer};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// The main level of a netlist of R1 to R4, drawn with `$TALL` in a row, and `statements` after it
/// in the same record, from line 14; there are two symbols and two styles to give.
fn read(statements: &[&str]) -> Interpretation {
    let r = read_with(statements, &Options { std: false, ..Options::default() });
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    r
}

/// The same netlist read with `options`, whatever it reports.
fn read_with(statements: &[&str], options: &Options) -> Interpretation {
    let mut text = String::from("* segments\nR1 a b 1k\nR2 c d 1k\nR3 e f 1k\nR4 g h 1k\n");
    text += "* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n* @RNL {\n";
    let mut all = vec![
        "$TALL = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (0, 10) @PIN(terminal=2) }",
        "$WIDE = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1); (10, 0) @PIN(terminal=2) }",
        "$red = @DEFINE-STYLE(linecolor=\"#ff0000\") {}",
        "$blue = @DEFINE-STYLE(linecolor=\"#0000ff\") {}",
        "{#R1 #R2 #R3 #R4} @SET-SYMBOL $TALL",
        "#R1 @PLACE(0, 0); #R2 @PLACE(10, 0); #R3 @PLACE(20, 0); #R4 @PLACE(30, 0)",
    ];
    all.extend_from_slice(statements);
    text += &all.iter().map(|s| format!("*   {s}")).collect::<Vec<_>>().join(";\n");
    text += "\n* }\n.end\n";
    let files = MemoryProvider::new().with("s.sp", &text);
    interpret_file(Path::new("s.sp"), None, &files, options).unwrap()
}

fn item<'a>(r: &'a Interpretation, id: &str) -> &'a ItemView {
    r.model.levels[0].items.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("no {id}"))
}

#[test]
fn the_later_symbol_is_drawn() {
    // the appearance queue (8.3), as 5.5 has it: in `#M1 @SET-SYMBOL $A => @SET-SYMBOL $B`, M1 is
    // drawn with $B
    let r = read(&["#R1 @SET-SYMBOL $TALL => @SET-SYMBOL $WIDE", "{#R2 @SET-SYMBOL $TALL} => @SET-SYMBOL $WIDE"]);
    for id in ["#R1", "#R2"] {
        assert_eq!(r.model.levels[0].symbols[item(&r, id).symbol].name, "$WIDE", "{id}");
    }
}

#[test]
fn the_later_style_is_drawn() {
    // the style queue (8.3), property by property
    let r = read(&["#R1 @SET-STYLE $red => @SET-STYLE $blue", "{#R2 @SET-STYLE $red} => @SET-STYLE $blue"]);
    for id in ["#R1", "#R2"] {
        assert_eq!(item(&r, id).style.get("linecolor").and_then(Lit::str), Some("#0000ff"), "{id}");
    }
}

#[test]
fn the_later_layer_is_kept() {
    // override by key (8.4); before, of two LAYER at one position the earlier stayed
    let r = read(&["#R1 @LAYER(\"top\") => @LAYER(\"bottom\")", "{#R2 @LAYER(\"top\")} => @LAYER(\"bottom\")"]);
    for id in ["#R1", "#R2"] {
        assert_eq!(item(&r, id).layer, Layer::Bottom, "{id}");
    }
}

#[test]
fn what_a_later_segment_makes_is_drawn_over() {
    // the drawing order (11.6): on one layer, the text that the later segment makes is drawn after
    // the circle of the earlier one, which it covers; before, the two had the same order
    let r = read(&["(0, 0) @CIRCLE(radius=2) => @TEXT(\"c\")"]);
    let lv = &r.model.levels[0];
    assert_eq!(lv.shapes.len(), 1);
    let (circle, text) = (&lv.shapes[0], lv.texts.iter().find(|t| t.markup == "c").unwrap());
    assert_eq!((circle.layer, text.layer), (Layer::Middle, Layer::Middle));
    assert!(text.order > circle.order, "{:?} {:?}", text.order, circle.order);
}

#[test]
fn the_later_priority_holds() {
    // of two PRIORITY statements on one constraint, the later (14.8)
    let r = read(&[
        "%c1 = {#R1 #R2} @ALIGN(\"y\")",
        "%c2 = {#R3 #R4} @ALIGN(\"y\")",
        "%c1 @PRIORITY(\"high\") => @PRIORITY(\"low\")",
        "{%c2 @PRIORITY(\"high\")} => @PRIORITY(\"low\")",
    ]);
    for label in ["%c1", "%c2"] {
        let c = r.model.levels[0].constraints.iter().find(|c| c.label == label).unwrap();
        assert_eq!(c.priority, "low", "{label}");
    }
}

#[test]
fn the_later_current_holds() {
    // the direction of the current that selections read (15.2); before, of two CURRENT at one
    // position the earlier held
    let r = read(&[
        "#R1:1 @CURRENT(\"in\") => @CURRENT(\"out\")",
        "{#R2:1 @CURRENT(\"in\")} => @CURRENT(\"out\")",
        "# @SELECT-TERMINAL(current=\"out\") => @TEXT(\"o\")",
    ]);
    let texts = &r.model.levels[0].texts;
    let mut at: Vec<&str> = texts.iter().filter(|t| t.markup == "o").filter_map(|t| t.attach.as_deref()).collect();
    at.sort();
    assert_eq!(at, ["#R1:1", "#R2:1"]);
}

#[test]
fn in_a_symbol_body_the_later_pin_and_place_win() {
    // a symbol body is read in order, so its later segments win too: the later pin of a terminal
    // (9.2.4) and the later PLACE of a text on `?` (9.2.2)
    let r = read(&[
        "$PINS = @DEFINE-SYMBOL { (0, 0) @PIN(terminal=1, label=\"a\") => @PIN(terminal=1, label=\"b\"); \
         (0, 10) @PIN(terminal=2); ? @TEXT(\"${name}\", key=\"name\") => @PLACE(2, 1) => @PLACE(2, 5) }",
        "#R4 @SET-SYMBOL $PINS => @PLACE(20, 0)",
    ]);
    let lv = &r.model.levels[0];
    let pins = &lv.symbols[item(&r, "#R4").symbol].pins;
    assert_eq!(pins.iter().find(|p| p.terminals == ["1"]).unwrap().label.as_deref(), Some("b"));
    let name =
        lv.texts.iter().find(|t| t.key.as_deref() == Some("name") && t.attach.as_deref() == Some("#R4")).unwrap();
    assert_eq!(name.anchor, (22.0, 5.0));
}

#[test]
fn a_later_place_of_a_text_replaces_an_earlier_offset() {
    // 13.3 compares where the PLACE of a text and its offset are written in the order of writing
    // too: the PLACE of the later segment replaces the dx of a text made in a statement nested in the
    // earlier one, for a text and for a text with a key alike; before, the nested offset won
    let statements = ["{#R1 @TEXT(\"a\", dx=2)} => @PLACE(x=5)", "{#R2 @TEXT(\"b\", key=\"k\", dx=2)} => @PLACE(x=15)"];
    let r = read(&statements);
    let x = |m: &str| r.model.levels[0].texts.iter().find(|t| t.markup == m).unwrap().anchor.0;
    assert_eq!((x("a"), x("b")), (5.0, 15.0));
    // the problem of a solver has the PLACE of each, not the offset
    let r = read_with(&statements, &Options { std: false, ..Options::solver() });
    let kinds: Vec<&str> = r.problems[0]
        .facts
        .iter()
        .filter(|f| f.label.contains("\"a\"") || f.label.contains("::k"))
        .map(|f| f.kind.name())
        .collect();
    assert!(!kinds.is_empty() && kinds.iter().all(|k| k.starts_with("PLACE")), "{kinds:?}");
}

#[test]
fn the_later_segment_closes_a_cycle_of_sets() {
    // of the statements of a cycle of sets, the last in order of position fails (5.2.2), in the
    // order of writing: the ADD-TO of the later segment, not the one nested in the earlier segment.
    // So %y still holds %x, and R1 is blue with it; before, the nested ADD-TO failed instead
    let r = read_with(
        &["%x = {#R1}", "%y = {#R2}", "{%x @ADD-TO %y} => @ADD-TO %x", "%y @SET-STYLE $blue"],
        &Options { std: false, ..Options::default() },
    );
    let errors: Vec<(u32, String)> = r
        .diagnostics()
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone()))
        .collect();
    assert!(errors.len() == 1 && errors[0].0 == 16 && errors[0].1.contains("cycle"), "{errors:?}");
    for id in ["#R1", "#R2"] {
        assert_eq!(item(&r, id).style.get("linecolor").and_then(Lit::str), Some("#0000ff"), "{id}");
    }
}
