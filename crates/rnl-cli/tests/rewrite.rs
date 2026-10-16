//! WIRE given a wire rewrites it (the draft 0.3, 13.1), in a file that declares 0.3: the
//! statement is read as a new WIRE, its list of points and its mode replace those of the wire,
//! the net stays, its style parameters are direct style of the wire, and of several rewrites the
//! one written last takes effect (5.8, 8.4). A rewrite that fails, also only once the symbols are
//! known, is an error, and the wire keeps its earlier list and mode. The model, the Document and
//! the solver's view draw and give the wire as rewritten. A file of 0.2 reads a wire in the net
//! slot as of the wrong kind, as before.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::Pt;
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::model::WireView;
use rnl_runtime::problem::{ObjectKind, PointRef, Reference};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use rnl_view::{BuildOptions, Segment, Shape};

fn read(text: &str, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), options).unwrap()
}

/// A file of `version` where R1 and C1 are placed and the wire %w joins their first terminals,
/// then `records`.
fn netlist(version: &str, records: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nR1 a b 1k\nC1 a 0 1p\n\
         * @RNL {{ #R1 @PLACE(0, 0); #C1 @PLACE(10, 0); %w = &a @WIRE {{#R1:1 #C1:1}} }}\n{records}.end\n"
    )
}

/// One authored record on one carrier line.
fn record(body: &str) -> String {
    format!("* @RNL {{ {body} }}\n")
}

fn diagnostics(r: &Interpretation, severity: Severity) -> Vec<(u32, String)> {
    let found = r.diagnostics().into_iter().filter(|d| d.severity == severity);
    found.map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone())).collect()
}

/// The wire %w as the model draws it.
fn drawn(r: &Interpretation) -> &WireView {
    r.model.levels[0].wires.iter().find(|w| w.id == "%w").unwrap_or_else(|| panic!("{:?}", r.model.levels[0].wires))
}

/// The pins that %w joins, R1:1 and C1:1, where the file places them.
fn pins() -> (Pt, Pt) {
    let w = drawn(&read(&netlist("0.3", ""), &Options::default())).points.clone();
    (w[0], w[w.len() - 1])
}

/// The vertices of a wire from `a` through `via` to `b` with `"horizontal-vertical"` (13.1).
fn horizontal_vertical(a: Pt, via: Pt, b: Pt) -> Vec<Pt> {
    vec![a, (via.0, a.1), via, (b.0, via.1), b]
}

#[test]
fn an_authored_record_rewrites_a_wire() {
    let (a, c) = pins();
    let text = netlist(
        "0.3",
        &record("%w @WIRE(mode=\"horizontal-vertical\", linestyle=\"bold\") {#R1:1 (4, 4) #C1:1} => @LAYER(\"top\")"),
    );
    let r = read(&text, &Options::default());
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    // one wire still, on its own net, drawn by the rewrite: its points and its mode
    let wires = &r.model.levels[0].wires;
    assert_eq!(wires.iter().filter(|w| w.nets == ["a"]).count(), 1);
    let w = drawn(&r);
    assert_eq!(w.mode, "horizontal-vertical");
    assert_eq!(w.points, horizontal_vertical(a, (4.0, 4.0), c));
    // the style parameter is direct style of the wire, and the handle passed on is the wire
    assert_eq!(w.style.get("linestyle").and_then(Lit::str), Some("bold"));
    assert_eq!(w.layer.as_str(), "top");

    // a solver reads the wire as the authored record rewrites it: its mode, its points, the point
    // written as coordinates being an object of the authored record, with a number
    let r = read(&text, &Options::solver());
    let p = &r.problems[0];
    let wire = p.wires.iter().find(|w| w.label == "%w").unwrap();
    assert_eq!(wire.mode, "horizontal-vertical");
    assert_eq!(wire.anchors.len(), 3);
    let PointRef::Origin(k) = wire.anchors[1] else { panic!("{:?}", wire.anchors) };
    assert_eq!((p.objects[k].kind, p.objects[k].label.as_str()), (ObjectKind::Point, "(4, 4)"));
    assert_eq!(p.objects[k].reference, Reference::Number { number: 1, path: Vec::new() });
    assert!(matches!(wire.anchors[0], PointRef::Pin { .. }) && matches!(wire.anchors[2], PointRef::Pin { .. }));

    // in a file of 0.2 a wire in the net slot is of the wrong kind
    let text = netlist("0.2", &record("%w @WIRE(mode=\"horizontal-vertical\") {#R1:1 (4, 4) #C1:1}"));
    let r = read(&text, &Options::default());
    let e = diagnostics(&r, Severity::Error);
    assert_eq!(e, [(6, "the net position of a wire received %w, the wrong kind (13.1)".to_string())]);
    assert_eq!(drawn(&r).mode, "auto");
}

#[test]
fn the_rewrite_written_last_takes_effect() {
    let (a, c) = pins();
    let bold = "%w @WIRE(mode=\"straight\", linestyle=\"bold\") {#R1:1 (5, 5) #C1:1}";
    let hv = "%w @WIRE(mode=\"horizontal-vertical\") {#R1:1 (4, 4) #C1:1}";
    // by the position of the statements (8.4), whichever record is read first
    let r = read(&netlist("0.3", &(record(bold) + &record(hv))), &Options::default());
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert_eq!(drawn(&r).points, horizontal_vertical(a, (4.0, 4.0), c));
    // the style written with the earlier rewrite stays: it is direct style of the wire (8.3), not
    // part of the rewrite that the later one replaces
    assert!(drawn(&r).style.contains_key("linestyle"));
    let r = read(&netlist("0.3", &(record(hv) + &record(bold))), &Options::default());
    assert_eq!((drawn(&r).mode.as_str(), drawn(&r).points.clone()), ("straight", vec![a, (5.0, 5.0), c]));
    // in one pipe, the later segment
    let r = read(
        &netlist("0.3", &record(&format!("{bold} => @WIRE(mode=\"manhattan\") {{#R1:1 #C1:1}}"))),
        &Options::default(),
    );
    assert_eq!(drawn(&r).mode, "manhattan");
    // a later rewrite that fails leaves the earlier one (5.8): a wire has two points at least
    let r = read(&netlist("0.3", &(record(hv) + &record("%w @WIRE {#R1:1}"))), &Options::default());
    let e = diagnostics(&r, Severity::Error);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].1.contains("a wire has at least two points"), "{e:?}");
    assert_eq!(drawn(&r).points, horizontal_vertical(a, (4.0, 4.0), c));
}

#[test]
fn the_net_slot_takes_a_net_a_set_of_nets_or_one_wire() {
    // a set that holds a wire is of the wrong kind
    let r = read(&netlist("0.3", &record("{%w} @WIRE {#R1:1 #C1:1}")), &Options::default());
    let e = diagnostics(&r, Severity::Error);
    assert_eq!(e, [(6, "the net position of a wire received %w, the wrong kind (13.1)".to_string())]);
    // a wire passed on by a pipe is rewritten as soon as it is made: one wire on the net b
    let r = read(
        &netlist("0.3", &record("&b @WIRE {#R1:2 (3, 3)} => @WIRE(mode=\"straight\") {#R1:2 (3, -3)}")),
        &Options::default(),
    );
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    let on_b: Vec<&WireView> = r.model.levels[0].wires.iter().filter(|w| w.nets == ["b"]).collect();
    assert_eq!(on_b.len(), 1);
    assert_eq!((on_b[0].mode.as_str(), on_b[0].points[1]), ("straight", (3.0, -3.0)));
}

#[test]
fn a_rewritten_wire_reaches_the_terminals_of_its_rewrite() {
    // C1:2 is on the net 0 without a wire, so the standard package draws a ground at it; rewritten
    // to reach C1:2 as well, %w makes it wired, and the terminal off the net of the wire is warned
    // about at the rewrite (10.2, 13.1)
    let r = read(&netlist("0.3", ""), &Options::default());
    assert_eq!(r.model.levels[0].unplaced, ["%g"]);
    let r = read(&netlist("0.3", &record("%w @WIRE {#R1:1 #C1:1 #C1:2}")), &Options::default());
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert!(r.model.levels[0].unplaced.is_empty(), "{:?}", r.model.levels[0].unplaced);
    let w = diagnostics(&r, Severity::Warning);
    assert_eq!(w, [(6, "the wire is marked on net a, but the terminal #C1:n it reaches is on 0 (13.1)".to_string())]);
}

/// The netlist with R2 as well, drawn with a symbol that has no pin, so that a wire that lists one
/// of its terminals fails (9.2.4); the records come after the one that gives R2 its symbol, line 7.
fn pinless(records: &str) -> String {
    let symbol = record("$BOX = @DEFINE-SYMBOL { @RECTANGLE {(0, 0) (2, 2)} }; #R2 @SET-SYMBOL $BOX");
    netlist("0.3", &(symbol + records)).replace("C1 a 0 1p\n", "C1 a 0 1p\nR2 a 0 1k\n")
}

#[test]
fn a_rewrite_that_fails_once_the_symbols_are_known_takes_no_effect() {
    let (a, c) = pins();
    let as_made = drawn(&read(&netlist("0.3", ""), &Options::default())).points.clone();
    let hv = "%w @WIRE(mode=\"horizontal-vertical\") {#R1:1 (4, 4) #C1:1}";
    let to_r2 = "%w @WIRE(mode=\"straight\") {#R1:1 (6, 6) #R2:1}";
    let fails = "#R2:p has no pin in its symbol: this rewrite of %w fails and has no effect, its style parameters included; the wire keeps the list and mode of the latest rewrite that holds, or those it was made with (9.2.4, 5.8, 13.1)";
    let errors = |r: &Interpretation| {
        let mut e = diagnostics(r, Severity::Error);
        e.sort();
        e
    };
    // a later rewrite that fails is an error, and the wire keeps the earlier rewrite (5.8)
    let text = pinless(&(record(hv) + &record(to_r2)));
    let r = read(&text, &Options::default());
    assert_eq!(errors(&r), [(9, fails.to_string())]);
    let w = drawn(&r);
    assert_eq!((w.mode.as_str(), &w.points), ("horizontal-vertical", &horizontal_vertical(a, (4.0, 4.0), c)));
    // the rewrite is text that takes no effect, and a solver reads the wire as the earlier rewrite
    assert!(r.failed.iter().any(|f| f.start.line == 9 && !f.conflict), "{:?}", r.failed);
    let r = read(&text, &Options::solver());
    assert_eq!(r.problems[0].wires.iter().find(|w| w.label == "%w").unwrap().mode, "horizontal-vertical");

    // a first rewrite that fails leaves the list and the mode the wire was made with
    let r = read(&pinless(&record(to_r2)), &Options::default());
    assert_eq!(errors(&r), [(8, fails.to_string())]);
    assert_eq!((drawn(&r).mode.as_str(), &drawn(&r).points), ("auto", &as_made));

    // of several, the latest that holds; each that fails is an error, also before one that holds
    let r = read(&pinless(&(record(hv) + &record(to_r2) + &record("%w @WIRE {#R2:1 #C1:1}"))), &Options::default());
    assert_eq!(errors(&r).iter().map(|(line, _)| *line).collect::<Vec<_>>(), [9, 10]);
    assert_eq!(drawn(&r).mode, "horizontal-vertical");
    let r = read(&pinless(&(record(to_r2) + &record(hv))), &Options::default());
    assert_eq!(errors(&r), [(8, fails.to_string())]);
    assert_eq!(drawn(&r).mode, "horizontal-vertical");

    // the wire fails as a whole only when the list it was made with fails as well
    let made = "%v = &a @WIRE {#R1:1 #R2:1}";
    let r = read(&pinless(&(record(made) + &record("%v @WIRE {#R2:1 #C1:1}"))), &Options::default());
    let e = errors(&r);
    assert_eq!(e.len(), 2, "{e:?}");
    assert_eq!(e[0], (8, "#R2:p has no pin in its symbol: the wire fails (9.2.4)".to_string()));
    assert!(e[1].0 == 9 && e[1].1.contains("this rewrite of %v fails and has no effect"), "{e:?}");
    assert!(r.model.levels[0].wires.iter().all(|w| w.id != "%v"));
    // and a wire made to a terminal without a pin is drawn by a rewrite that holds
    let r = read(&pinless(&(record(made) + &record("%v @WIRE {#R1:1 #C1:1}"))), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(r.model.levels[0].wires.iter().any(|w| w.id == "%v" && w.points == [a, c]));
}

#[test]
fn a_rewrite_that_fails_gives_the_wire_no_style() {
    // a rewrite that fails has no effect, its style parameters included (1.4.2, 13.1)
    let hv = "%w @WIRE(mode=\"horizontal-vertical\") {#R1:1 (4, 4) #C1:1}";
    let red = "linecolor=\"#ff0000\", linestyle=\"bold\"";
    let colour = |r: &Interpretation| drawn(r).style.get("linecolor").and_then(Lit::str).map(String::from);
    let before = read(&pinless(&record(hv)), &Options::default());
    assert_ne!(colour(&before).as_deref(), Some("#ff0000"));
    // a later rewrite that fails: the wire keeps the earlier rewrite and the style it had
    let fails = format!("%w @WIRE(mode=\"straight\", {red}) {{#R1:1 (6, 6) #R2:1}}");
    let r = read(&pinless(&(record(hv) + &record(&fails))), &Options::default());
    assert_eq!(diagnostics(&r, Severity::Error).len(), 1);
    assert_eq!(drawn(&r).mode, "horizontal-vertical");
    assert_eq!(drawn(&r).style, drawn(&before).style);
    // the first rewrite, failing, leaves the style the wire was made with
    let made = read(&pinless(""), &Options::default());
    let r = read(&pinless(&record(&fails)), &Options::default());
    assert_eq!((drawn(&r).mode.as_str(), &drawn(&r).style), ("auto", &drawn(&made).style));
    // the same style written with a rewrite that holds takes effect, also when a later rewrite fails
    let holds = format!("%w @WIRE(mode=\"straight\", {red}) {{#R1:1 (6, 6) #C1:1}}");
    let r =
        read(&pinless(&(record(&holds) + &record(&fails.replace(red, "linecolor=\"#00ff00\"")))), &Options::default());
    assert_eq!((drawn(&r).mode.as_str(), colour(&r).as_deref()), ("straight", Some("#ff0000")));
    assert_eq!(drawn(&r).style.get("linestyle").and_then(Lit::str), Some("bold"));
}

#[test]
fn the_document_draws_the_rewrite() {
    let (a, c) = pins();
    let text = netlist("0.3", &record("%w @WIRE(mode=\"horizontal-vertical\") {#R1:1 (4, 4) #C1:1}"));
    let files = MemoryProvider::new().with("/w/t.sp", &text);
    let options = BuildOptions { primitives: true, ..BuildOptions::default() };
    let d = rnl_view::build("/w/t.sp", &files, &options).unwrap();
    let level = &d.levels[0];
    let hit = level.hits.iter().find(|h| h.kind == "wire" && h.points.len() == 5).unwrap();
    let id = hit.id.clone().unwrap();
    let path = level.primitives.as_ref().unwrap().iter().find(|p| p.owner == id).unwrap();
    let Shape::Path { segments, .. } = &path.shape else { panic!("{:?}", path.shape) };
    let points: Vec<Pt> = segments
        .iter()
        .map(|s| match s {
            Segment::Move { to } | Segment::Line { to } => (to[0], to[1]),
            s => panic!("{s:?}"),
        })
        .collect();
    assert_eq!(points, horizontal_vertical(a, (4.0, 4.0), c));
}
