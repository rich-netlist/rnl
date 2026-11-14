//! What a viewer that does not solve draws of a level not solved yet, or whose numbers are out of
//! date (13.1, 13.3, 5.9): a wire some of whose points have no position
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
    if version != "0.4" {
        return format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nM1 out in vss vss nch\nR1 vdd out 10k\n\
             * @RNL {{ #M1 @PLACE(0, 0); #R1 @PLACE(10, 0) }}\n{records}.end\n"
        );
    }
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); \
         @USE-PACKAGE(name=\"rnl-std-basic\", version=\"0.4\"); \
         @USE-PACKAGE(name=\"rnl-std-semiconductors\", version=\"0.4\") }}\n\
         M1 out in vss vss nch\nR1 vdd out 10k\n\
         * @RNL {{ #M1 @SET-SYMBOL $NMOS; #R1 @SET-SYMBOL $RESISTOR; #M1 @PLACE(0, 0); #R1 @PLACE(10, 0) }}\n{records}.end\n"
    )
}

fn record(body: &str) -> String {
    format!("* @RNL {{ {body} }}\n")
}

fn level(r: &Interpretation, title: &str) -> LevelView {
    r.model.levels.iter().find(|l| l.title == title).unwrap().clone()
}

/// The ends of the flylines of a level on out.
fn ends(lv: &LevelView) -> Vec<(Pt, Pt)> {
    lv.flylines.iter().filter(|f| f.nets == ["out"]).map(|f| (f.a, f.b)).collect()
}

/// The parts of the wires on a net.
fn on_net<'a>(lv: &'a LevelView, net: &str) -> Vec<&'a WireView> {
    lv.wires.iter().filter(|w| w.nets == [net]).collect()
}

/// The pins M1:1 and R1:2 that the wire on out joins, as a wire through placed points draws them.
fn pins(version: &str) -> (Pt, Pt) {
    let lv = level(&read(&netlist(version, &record("&out @WIRE {#M1:1 (5, 5) #R1:2}")), &Options::default()), "t.sp");
    let w = &on_net(&lv, "out")[0].points;
    (w[0], w[w.len() - 1])
}

#[test]
fn a_wire_through_a_free_point_is_a_flyline() {
    let (m1, r1) = pins("0.4");
    let r = read(&netlist("0.4", &record("&out @WIRE {#M1:1 (_, _) #R1:2}")), &Options::default());
    let lv = level(&r, "t.sp");
    // no part of it has two points with a position: nothing drawn as the wire, a flyline over the
    // free point; the wire is not an object without a position, and neither is the point
    assert!(on_net(&lv, "out").is_empty(), "{:?}", lv.wires);
    assert_eq!(ends(&lv), [(m1, r1)]);
    assert!(lv.unplaced.is_empty(), "{:?}", lv.unplaced);
    // with the point placed, the wire is drawn whole, without a flyline
    let r = read(&netlist("0.4", &record("&out @WIRE {#M1:1 (5, 5) #R1:2}")), &Options::default());
    let lv = level(&r, "t.sp");
    assert_eq!((on_net(&lv, "out").len(), ends(&lv).len()), (1, 0));
    // a wire with no point that has a position draws nothing
    let r = read(&netlist("0.4", &record("&in @WIRE {(_, _) (_, 3)}")), &Options::default());
    let lv = level(&r, "t.sp");
    assert!(on_net(&lv, "in").is_empty() && lv.flylines.iter().all(|f| f.nets != ["in"]));
}

#[test]
fn a_wire_whose_middle_point_is_free_is_drawn_in_two_runs() {
    let (m1, r1) = pins("0.4");
    let wire = "&out @WIRE(mode=\"horizontal-vertical\", linestyle=\"bold\") {#M1:1 (2, 5) (_, _) (8, 5) #R1:2}";
    let r = read(&netlist("0.4", &record(wire)), &Options::default());
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
    assert_eq!(ends(&lv), [((2.0, 5.0), (8.0, 5.0))]);

    // the Document has a hit for each run, on the net, and draws the flyline when asked
    let files = MemoryProvider::new().with("/w/t.sp", &netlist("0.4", &record(wire)));
    let options = rnl_view::BuildOptions { primitives: true, flylines: true, ..rnl_view::BuildOptions::default() };
    let d = rnl_view::build("/w/t.sp", &files, &options).unwrap();
    let out = d.objects.iter().position(|o| o.kind == "net" && o.name == "out").unwrap();
    let hits: Vec<&rnl_view::Hit> = d.levels[0].hits.iter().filter(|h| h.kind == "wire").collect();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|h| h.objects == [out]));
    // the flyline over the gap is the first, and stands for its net, as the wire does: a hit of
    // its own, by the id of its group; those of the other nets follow
    let flylines: Vec<&str> = d.levels[0]
        .primitives
        .as_ref()
        .unwrap()
        .iter()
        .filter(|p| matches!(p.shape, rnl_view::Shape::Flyline { .. }))
        .map(|p| p.owner.as_str())
        .collect();
    assert_eq!(flylines.len(), lv.flylines.len());
    assert_eq!(flylines[0], "flyline-0");
    let on_out: Vec<&rnl_view::Hit> =
        d.levels[0].hits.iter().filter(|h| h.kind == "flyline" && h.objects == [out]).collect();
    let scale = d.levels[0].scale;
    assert_eq!(on_out.len(), 1);
    assert_eq!(on_out[0].id.as_deref(), Some("flyline-0"));
    assert_eq!(on_out[0].points, [[2.0 * scale, -5.0 * scale], [8.0 * scale, -5.0 * scale]]);
    // the wire, its flyline over the free point and the pins at its ends are one part of out,
    // labelled once, where the model says, in pixels
    let labels: Vec<&rnl_view::NetLabel> = d.levels[0].net_labels.iter().filter(|l| l.net == out).collect();
    let at = lv.net_labels.iter().find(|l| l.net == "out").unwrap().at;
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].at, [at.0 * scale, -at.1 * scale]);

    // in the SVG each flyline is a group of its own, which a viewer can hide; drawn without
    // flylines, as by default, the level has none, and nothing else of it changes
    let with = rnl_render::RenderOptions { flylines: true, ..rnl_render::RenderOptions::default() };
    let svg = rnl_render::svg(&lv, &with);
    assert_eq!(svg.matches("<g data-id=\"flyline-0\" class=\"flyline\"><line ").count(), 1, "{svg}");
    assert_eq!(svg.matches(" class=\"flyline\">").count(), lv.flylines.len(), "{svg}");
    let without = rnl_render::RenderOptions::default();
    let svg_without = rnl_render::svg(&lv, &without);
    assert!(!svg_without.contains("flyline"));
    let others = |svg: &str| svg.lines().filter(|l| !l.contains("flyline")).map(String::from).collect::<Vec<_>>();
    assert_eq!(others(&svg), others(&svg_without));
    assert!(!rnl_render::primitives(&lv, &without).iter().any(|p| matches!(p.kind, rnl_render::Kind::Flyline { .. })));
    // the names of the nets, when asked: one for each connected part, where the model says, over
    // everything else; and nothing else of the level changes
    let named = rnl_render::RenderOptions { net_names: true, ..rnl_render::RenderOptions::default() };
    let svg_named = rnl_render::svg(&lv, &named);
    assert!(!svg_without.contains("net-labels"));
    assert_eq!(svg_named.matches("<g class=\"net-labels\"").count(), 1, "{svg_named}");
    assert_eq!(svg_named.matches("<text x=").count() - svg_without.matches("<text x=").count(), lv.net_labels.len());
    let s = named.scale;
    let out_text = format!("<text x=\"{:.2}\" y=\"{:.2}\" text-anchor=\"middle\"", at.0 * s, -at.1 * s);
    assert!(svg_named.contains(&out_text), "{out_text} in {svg_named}");
    assert!(svg_named.contains("paint-order=\"stroke\">out</text>"), "{svg_named}");
    let unnamed = |svg: &str| svg.lines().filter(|l| !l.contains("net-labels")).map(String::from).collect::<Vec<_>>();
    assert_eq!(unnamed(&svg_named), unnamed(&svg_without));
    let options = rnl_view::BuildOptions { primitives: true, ..rnl_view::BuildOptions::default() };
    let d = rnl_view::build("/w/t.sp", &files, &options).unwrap();
    assert!(!d.levels[0].svg.as_deref().unwrap_or_default().contains("flyline"));
    assert!(!d.levels[0].hits.iter().any(|h| h.kind == "flyline"));
    assert!(!d.levels[0]
        .primitives
        .as_ref()
        .unwrap()
        .iter()
        .any(|p| matches!(p.shape, rnl_view::Shape::Flyline { .. })));
}

#[test]
fn a_stale_numbering_leaves_the_free_point_free_and_the_wire_drawn_in_part() {
    // a file of the draft whose record of the solver is out of date: the statements with numbers
    // take no effect (5.9), so the free point has no position again, the labels go where the viewer
    // puts them, and the wire through the point is drawn as far as it has positions. The pins are
    // those of the symbols of the standard series 0.2, which the file pins: a file of 0.3 uses the
    // series 0.3 otherwise (16.3)
    let authored = "*   &out @WIRE {#M1:1 (1, 3) (_, _) #R1:2};\n*   {#M1 #R1} @TEXT(\"${name}\")\n";
    let solver = "* @RNL(type=\"generated\", priority=\"high\", numbering=\"0123456789abcdef\") {\n\
                  *   #M1 @PLACE(0, 0); #R1 @PLACE(10, 0); %#3 @PLACE(6, 6); %#4 @PLACE(2, 1); %#5 @PLACE(12, 1)\n* }\n";
    let header = "@VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")";
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ {header}}}\nM1 out in vss vss nch\nR1 vdd out 10k\n* @RNL {{\n{authored}* }}\n{solver}.end\n"
    );
    let r = read(&text, &Options::default());
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
    let hints =
        r.diagnostics().into_iter().filter(|d| d.message.contains("out of date for the objects without a name"));
    assert_eq!(hints.count(), 1);
    let (m1, r1) = pins("0.2");
    let lv = level(&r, "t.sp");
    // the run from M1:1 to (1, 3) is drawn, and a flyline goes on to R1:2 over the free point
    let runs = on_net(&lv, "out");
    assert_eq!(runs.len(), 1);
    assert_eq!((runs[0].points[0], runs[0].points[runs[0].points.len() - 1]), (m1, (1.0, 3.0)));
    assert_eq!(ends(&lv), [((1.0, 3.0), r1)]);
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
fn scene_positioned_labels_keep_scene_alignment() {
    for orientation in ["MY", "MX", "R90"] {
        for position in [
            ", dx=-0.4, dy=-0.5)",
            ", dx=-0.4) => @PLACE(y=-0.5)",
            ", dy=-0.5) => @PLACE(x=-0.4)",
            ") => @PLACE(-0.4, -0.5)",
        ] {
            let body = format!(
                "#M1 @ORIENT(\"{orientation}\"); #M1 @TEXT(\"${{name}}\", key=\"name\", halign=\"end\", valign=\"top\"{position}"
            );
            let source = netlist("0.4", &record(&body)).replace("M1 out", ".model nch nmos\nM1 out");
            let r = read(&source, &Options::default());
            assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
            let lv = level(&r, "t.sp");
            let label = lv
                .texts
                .iter()
                .find(|t| t.attach.as_deref() == Some("#M1") && t.key.as_deref() == Some("name"))
                .unwrap();
            assert_eq!(label.anchor, (-0.4, -0.5));
            let prims = rnl_render::primitives(&lv, &rnl_render::RenderOptions::default());
            let prim = prims.iter().find(|p| p.owner == "#M1::name").unwrap();
            let rnl_render::Kind::Text(text) = &prim.kind else { panic!("label must be text") };
            assert_eq!(text.halign, "end", "{orientation}: {position}");
            assert_eq!(text.valign, "top", "{orientation}: {position}");
            assert!(!text.vertical, "{orientation}: {position}");
            assert!((prim.bbox[2] - label.anchor.0).abs() < 1e-10, "{:?}", prim.bbox);
        }
    }
}

#[test]
fn a_label_using_a_symbol_axis_keeps_the_symbol_orientation() {
    for offset in ["", ", dx=-0.4", ", dy=-0.5"] {
        let body = format!("#M1 @ORIENT(\"MY\"); #M1 @TEXT(\"${{name}}\", key=\"name\", halign=\"end\"{offset})");
        let source = netlist("0.4", &record(&body)).replace("M1 out", ".model nch nmos\nM1 out");
        let r = read(&source, &Options::default());
        assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
        let lv = level(&r, "t.sp");
        let prims = rnl_render::primitives(&lv, &rnl_render::RenderOptions::default());
        let prim = prims.iter().find(|p| p.owner == "#M1::name").unwrap();
        let rnl_render::Kind::Text(text) = &prim.kind else { panic!("label must be text") };
        assert_eq!(text.halign, "start", "{offset}");
    }
}

#[test]
fn a_text_on_a_wire_drawn_in_part_is_drawn_above_it() {
    let (m1, r1) = pins("0.4");
    // the wire is drawn up to (2, 5) and from (8, 5), with a flyline between: its label goes above
    // the middle of that drawing, here the flyline; a PLACE on one axis keeps its value
    let records = "%w = &out @WIRE {#M1:1 (2, 5) (_, _) (8, 5) #R1:2}; %w @TEXT(\"w\");                    %w @TEXT(\"x7\") => @PLACE(x=7); %t = %w @TEXT(\"t\"); {%t #R1} @ALIGN(\"x\")";
    let r = read(&netlist("0.4", &record(records)), &Options::default());
    let lv = level(&r, "t.sp");
    assert_eq!(text_at(&lv, "w"), Some((5.0, 5.6)));
    assert_eq!(text_at(&lv, "x7"), Some((7.0, 5.6)));
    // the place is the viewer's only: a constraint on the text still has nothing to check
    assert_eq!(lv.constraints[0].status, "unsolved");
    // a wire drawn only as a flyline: above its middle
    let r =
        read(&netlist("0.4", &record("%w = &out @WIRE {#M1:1 (_, _) #R1:2}; %w @TEXT(\"w\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "w"), Some(((m1.0 + r1.0) / 2.0, (m1.1 + r1.1) / 2.0 + 0.6)));
    // a wire with one point placed is not drawn, and neither is its label
    let r = read(&netlist("0.4", &record("%w = &out @WIRE {#M1:1 (_, _)}; %w @TEXT(\"w\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "w"), None);
}

#[test]
fn a_text_on_a_net_goes_next_to_what_is_drawn_of_it() {
    let (m1, r1) = pins("0.4");
    // no wire of out has all its points placed: the label goes at the start of the first that is
    // drawn at all, here as a flyline from M1:1
    let r =
        read(&netlist("0.4", &record("&out @WIRE {#M1:1 (_, _) #R1:2}; &out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(m1));
    // a net without a wire drawn: next to the first of its pins that is drawn (13.3)
    let r = read(&netlist("0.4", &record("&out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(m1));
    let r = read(&netlist("0.4", &record("&out @WIRE {(_, _) #R1:2}; &out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(m1));
    // a wire drawn whole comes first, as before
    let r = read(&netlist("0.4", &record("&out @WIRE {#R1:2 (12, 6)}; &out @TEXT(\"${name}\")")), &Options::default());
    assert_eq!(text_at(&level(&r, "t.sp"), "out"), Some(r1));
    // a net none of whose terminals is drawn: no label
    let text = netlist("0.4", &record("&x @TEXT(\"${name}\")")).replace(
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
    let r = read(&netlist("0.4", &record(records)), &Options::default());
    let lv = level(&r, "t.sp");
    assert_eq!(text_at(&lv, "box"), Some((21.2, 21.2)));
    assert_eq!(text_at(&lv, "line"), None);
    assert_eq!(lv.shapes.len(), 1);
}
