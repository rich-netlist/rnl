//! What a viewer that does not solve draws of a level not solved yet, or whose numbers are out of
//! date (the draft 0.3, section 9; 13.1, 13.3, 5.9): a wire some of whose points have no position
//! is drawn in its runs of points that have one, with flylines over the gaps, and not dropped; a
//! text without a position whose attached object is drawn is drawn beside it.

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

/// The texts of a level by what they show.
fn text_at(lv: &LevelView, shown: &str) -> Option<Pt> {
    lv.texts.iter().find(|t| rnl_runtime::markup::plain(&t.lines[0]) == shown).map(|t| t.anchor)
}

#[test]
fn a_text_on_a_wire_drawn_in_part_is_drawn_above_it() {
    let (m1, r1) = pins();
    // the wire is drawn up to (2, 5) and from (8, 5), with a flyline between: its label goes above
    // the middle of that drawing, here the flyline; a PLACE on one axis keeps its value
    let records = "%w = &out @WIRE {#M1:1 (2, 5) (_, _) (8, 5) #R1:2}; %w @TEXT(\"w\");                    %w @TEXT(\"x7\") => @PLACE(x=7); %t = %w @TEXT(\"t\"); {%t #R1} @ALIGN(\"x\")";
    let r = read(&netlist("0.2", &record(records)), &Options::default());
    let lv = level(&r, "t.sp");
    assert_eq!(text_at(&lv, "w"), Some((5.0, 5.6)));
    assert_eq!(text_at(&lv, "x7"), Some((7.0, 5.6)));
    // the place is the viewer's only: a constraint on the text still has nothing to check
    assert_eq!(lv.constraints[0].status, "unsolved");
    // a wire drawn only as a flyline: above its middle
    let r =
        read(&netlist("0.2", &record("%w = &out @WIRE {#M1:1 (_, _) #R1:2}; %w @TEXT(\"w\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "w"), Some(((m1.0 + r1.0) / 2.0, (m1.1 + r1.1) / 2.0 + 0.6)));
    // a wire with one point placed is not drawn, and neither is its label
    let r = read(&netlist("0.2", &record("%w = &out @WIRE {#M1:1 (_, _)}; %w @TEXT(\"w\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "w"), None);
}

#[test]
fn a_text_on_a_net_goes_next_to_what_is_drawn_of_it() {
    let (m1, r1) = pins();
    // no wire of out has all its points placed: the label goes at the start of the first that is
    // drawn at all, here as a flyline from M1:1
    let r =
        read(&netlist("0.2", &record("&out @WIRE {#M1:1 (_, _) #R1:2}; &out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(m1));
    // a net without a wire drawn: next to the first of its pins that is drawn (13.3)
    let r = read(&netlist("0.2", &record("&out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(m1));
    let r = read(&netlist("0.2", &record("&out @WIRE {(_, _) #R1:2}; &out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(m1));
    // a wire drawn whole comes first, as before
    let r = read(&netlist("0.2", &record("&out @WIRE {#R1:2 (12, 6)}; &out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(r1));
    // a net none of whose terminals is drawn: no label
    let text = netlist("0.2", &record("&x @TEXT(\"${name}\")")).replace(
        "R1 vdd out 10k
",
        "R1 vdd out 10k
R2 x 0 1k
",
    );
    assert_eq!(text_at(&level(&read(&text, &Options::default()), "t.sp"), "x"), None);
}

#[test]
fn a_text_on_a_graphic_is_drawn_beside_it() {
    // a label on a graphic in the scene is drawn beside its first point, as one on an object beside
    // its origin (13.3, 13.4); on a graphic that is not drawn, as one point has no position, it is not
    let records = "%r = @RECTANGLE {(20, 20) (24, 22)}; %r @TEXT(\"box\");                    %s = @LINE {(30, 30) (_, _)}; %s @TEXT(\"line\")";
    let r = read(&netlist("0.2", &record(records)), &Options::default());
    let lv = level(&r, "t.sp");
    assert_eq!(text_at(&lv, "box"), Some((21.2, 21.2)));
    assert_eq!(text_at(&lv, "line"), None);
    assert_eq!(lv.shapes.len(), 1);
}
