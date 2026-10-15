//! What a viewer that does not solve draws of a level not solved yet, or whose numbers are out of
//! date (the draft 0.3, section 9; 13.1, 13.3, 5.9): a wire some of whose points have no position
//! is drawn in its runs of points that have one, with flylines over the gaps, and not dropped.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::Pt;
use rnl_runtime::model::{LevelView, WireView};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(text: &str, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), options).unwrap()
}

/// A file of `version` where M1 and R1 are placed and `records` follow.
fn netlist(version: &str, records: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nM1 out in vss vss nch\nR1 vdd out 10k\n\
         * @RNL {{ #M1 @PLACE(0, 0); #R1 @PLACE(10, 0) }}\n{records}.end\n"
    )
}

fn record(body: &str) -> String {
    format!("* @RNL {{ {body} }}\n")
}

fn level(r: &Interpretation, title: &str) -> LevelView {
    r.model.levels.iter().find(|l| l.title == title).unwrap().clone()
}

/// The parts of the wires on a net.
fn on_net<'a>(lv: &'a LevelView, net: &str) -> Vec<&'a WireView> {
    lv.wires.iter().filter(|w| w.nets == [net]).collect()
}

/// The pins M1:1 and R1:2 that the wire on out joins, as a wire through placed points draws them.
fn pins() -> (Pt, Pt) {
    let lv = level(&read(&netlist("0.2", &record("&out @WIRE {#M1:1 (5, 5) #R1:2}")), &Options::default()), "t.sp");
    let w = &on_net(&lv, "out")[0].points;
    (w[0], w[w.len() - 1])
}

#[test]
fn a_wire_through_a_free_point_is_a_flyline() {
    let (m1, r1) = pins();
    let r = read(&netlist("0.2", &record("&out @WIRE {#M1:1 (_, _) #R1:2}")), &Options::default());
    let lv = level(&r, "t.sp");
    // no part of it has two points with a position: nothing drawn as the wire, a flyline over the
    // free point; the wire is not an object without a position, and neither is the point
    assert!(on_net(&lv, "out").is_empty(), "{:?}", lv.wires);
    assert_eq!(lv.flylines, [(m1, r1)]);
    assert!(lv.unplaced.is_empty(), "{:?}", lv.unplaced);
    // with the point placed, the wire is drawn whole, without a flyline
    let r = read(&netlist("0.2", &record("&out @WIRE {#M1:1 (5, 5) #R1:2}")), &Options::default());
    let lv = level(&r, "t.sp");
    assert_eq!((on_net(&lv, "out").len(), lv.flylines.len()), (1, 0));
    // a wire with no point that has a position draws nothing
    let r = read(&netlist("0.2", &record("&in @WIRE {(_, _) (_, 3)}")), &Options::default());
    let lv = level(&r, "t.sp");
    assert!(on_net(&lv, "in").is_empty() && lv.flylines.is_empty());
}

#[test]
fn a_wire_whose_middle_point_is_free_is_drawn_in_two_runs() {
    let (m1, r1) = pins();
    let wire = "&out @WIRE(mode=\"horizontal-vertical\", linestyle=\"bold\") {#M1:1 (2, 5) (_, _) (8, 5) #R1:2}";
    let r = read(&netlist("0.2", &record(wire)), &Options::default());
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error));
    let lv = level(&r, "t.sp");
    let runs = on_net(&lv, "out");
    assert_eq!(runs.len(), 2);
    // each run is the wire, with its mode and style, through its points that have a position
    assert_eq!(runs[0].points, [m1, (2.0, m1.1), (2.0, 5.0)]);
    assert_eq!(runs[1].points, [(8.0, 5.0), (r1.0, 5.0), r1]);
    for w in &runs {
        assert_eq!((w.id.as_str(), w.mode.as_str()), ("a wire", "horizontal-vertical"));
        assert_eq!(w.style.get("linestyle").and_then(|v| v.str()), Some("bold"));
    }
    assert_eq!(lv.flylines, [((2.0, 5.0), (8.0, 5.0))]);

    // the Document has a hit for each run, on the net, and draws the flyline
    let files = MemoryProvider::new().with("/w/t.sp", &netlist("0.2", &record(wire)));
    let options = rnl_view::BuildOptions { primitives: true, ..rnl_view::BuildOptions::default() };
    let d = rnl_view::build("/w/t.sp", &files, &options).unwrap();
    let out = d.objects.iter().position(|o| o.kind == "net" && o.name == "out").unwrap();
    let hits: Vec<&rnl_view::Hit> = d.levels[0].hits.iter().filter(|h| h.kind == "wire").collect();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|h| h.objects == [out]));
    let flylines =
        d.levels[0].primitives.as_ref().unwrap().iter().filter(|p| matches!(p.shape, rnl_view::Shape::Flyline { .. }));
    assert_eq!(flylines.count(), 1);
}

#[test]
fn a_stale_numbering_leaves_the_free_point_free_and_the_wire_drawn_in_part() {
    // a file of the draft whose record of the solver is out of date: the statements with numbers
    // take no effect (5.9), so the free point has no position again, the labels go where the viewer
    // puts them, and the wire through the point is drawn as far as it has positions
    let authored = "*   &out @WIRE {#M1:1 (1, 3) (_, _) #R1:2};\n*   {#M1 #R1} @TEXT(\"${name}\")\n";
    let solver = "* @RNL(type=\"generated\", priority=\"high\", numbering=\"0123456789abcdef\") {\n\
                  *   #M1 @PLACE(0, 0); #R1 @PLACE(10, 0); %#3 @PLACE(6, 6); %#4 @PLACE(2, 1); %#5 @PLACE(12, 1)\n* }\n";
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\")}}\nM1 out in vss vss nch\nR1 vdd out 10k\n* @RNL {{\n{authored}* }}\n{solver}.end\n"
    );
    let r = read(&text, &Options { draft: true, ..Options::default() });
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    let hints =
        r.diagnostics().into_iter().filter(|d| d.message.contains("out of date for the objects without a name"));
    assert_eq!(hints.count(), 1);
    let (m1, r1) = pins();
    let lv = level(&r, "t.sp");
    // the run from M1:1 to (1, 3) is drawn, and a flyline goes on to R1:2 over the free point
    let runs = on_net(&lv, "out");
    assert_eq!(runs.len(), 1);
    assert_eq!((runs[0].points[0], runs[0].points[runs[0].points.len() - 1]), (m1, (1.0, 3.0)));
    assert_eq!(lv.flylines, [((1.0, 3.0), r1)]);
    // the labels fall back beside M1 and R1, which keep their places by name
    let label =
        |name: &str| lv.texts.iter().find(|t| t.id == "a text" && rnl_runtime::markup::plain(&t.lines[0]) == name);
    assert_eq!(label("M1").map(|t| t.anchor), Some((1.2, 1.2)));
    assert_eq!(label("R1").map(|t| t.anchor), Some((11.2, 1.2)));
}
