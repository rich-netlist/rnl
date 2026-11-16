//! The diodes and the transformer of the standard series 0.4 stand upright: each is the symbol of
//! 0.3 turned. The upgrade composes the orientation that an ORIENT gives an instance after the
//! turn back, so that each pin is drawn where 0.3 drew it; an instance no ORIENT orients stays free.
use rnl_edit::upgrade::{upgrade, Upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Interpretation;

fn read(name: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(name, text);
    let options = rnl_runtime::Options { problems: true, ..rnl_runtime::Options::default() };
    rnl_view::interpret_with(name, None, &Default::default(), &files, &options).unwrap()
}

fn run(name: &str, text: &str) -> Upgrade {
    upgrade(name, &MemoryProvider::new().with(name, text), &UpgradeOptions::default()).unwrap()
}

/// A pin drawn: its terminals, its place and its direction.
type Pin = (Vec<String>, Option<(f64, f64)>, Option<u16>);

/// The pins of the instance `id`, by their terminals, each named by `name`.
fn pins_named(r: &Interpretation, id: &str, name: impl Fn(&str) -> &str) -> Vec<Pin> {
    let object = r.model.levels.iter().flat_map(|l| &l.objects).find(|o| o.id == id).expect(id);
    let mut out: Vec<Pin> =
        object.pins.iter().map(|p| (p.terminals.iter().map(|t| name(t).to_string()).collect(), p.at, p.dir)).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn pins(r: &Interpretation, id: &str) -> Vec<Pin> {
    pins_named(r, id, |t| t)
}

const SPICE: &str =
    "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\") }\n";

#[test]
fn an_oriented_diode_keeps_its_pins_and_a_free_one_stays_free() {
    let text = format!(
        "{SPICE}D1 a b dmod\nD2 b c dmod\nD3 c d dmod\nD4 d e dmod\nD5 e 0 dmod\nR1 a 0 1k\n\
         * @RNL {{\n\
         *   #D1 @PLACE(0, 0);\n\
         *   #D2 @PLACE(10, 0) => @ORIENT(\"R90\");\n\
         *   #D3 @PLACE(20, 0) => @ORIENT(code=\"MY\");\n\
         *   {{#D4 #D5}} @ORIENT(\"R180\");\n\
         *   #D4 @PLACE(30, 0); #D5 @PLACE(40, 0);\n\
         *   #R1 @PLACE(50, 0) => @ORIENT(\"R90\")\n\
         * }}\n.end\n"
    );
    let r = run("t.sp", &text);
    let written = &r.files["t.sp"];
    for expected in [
        "#D1 @PLACE(0, 0);",
        "#D2 @PLACE(10, 0) => @ORIENT(\"R180\");",
        "#D3 @PLACE(20, 0) => @ORIENT(code=\"MXR90\");",
        "{#D4 #D5} @ORIENT(\"R270\");",
        "#R1 @PLACE(50, 0) => @ORIENT(\"R90\")",
    ] {
        assert!(written.contains(expected), "{expected}: {written}");
    }
    assert!(r.report.differences.is_empty(), "{:#?}", r.report.differences);
    assert!(r.report.successful(), "{:#?}", r.report);
    let (before, after) = (read("t.sp", &text), read("t.sp", written));
    for id in ["#D2", "#D3", "#D4", "#D5", "#R1"] {
        assert_eq!(pins(&after, id), pins(&before, id), "{id}");
    }
    // the diode no ORIENT orients: the solver chooses among the same drawings
    let free = |r: &Interpretation| {
        r.problems[0]
            .objects
            .iter()
            .find(|o| matches!(&o.reference, rnl_runtime::problem::Reference::Name(id) if id == "#D1"))
            .map(|o| o.orient.clone())
    };
    assert_eq!(free(&after), free(&before));
    assert_eq!(free(&after), Some(Some(rnl_runtime::problem::OrientSlot::Free)));
}

#[test]
fn an_orient_of_a_diode_with_another_part_is_left_for_review() {
    let text = format!(
        "{SPICE}D1 a b dmod\nR1 a 0 1k\n* @RNL {{\n*   {{#D1 #R1}} @ORIENT(\"R90\");\n*   #D1 @PLACE(0, 0); #R1 @PLACE(10, 0)\n* }}\n.end\n"
    );
    let r = run("t.sp", &text);
    let written = &r.files["t.sp"];
    assert!(written.contains("{#D1 #R1} @ORIENT(\"R90\")"), "{written}");
    assert!(written.contains("rnl upgrade: this orients instances of a diode or a transformer"), "{written}");
    assert!(!r.report.successful());
}

#[test]
fn an_orient_in_a_selection_turns_the_diodes_it_selects() {
    let text = format!(
        "{SPICE}D1 a b dmod\nD2 b 0 dmod\n* @RNL {{\n*   # @SELECT-INSTANCE(name=\"D*\") => @ORIENT(\"MX\");\n*   #D1 @PLACE(0, 0); #D2 @PLACE(10, 0)\n* }}\n.end\n"
    );
    let r = run("t.sp", &text);
    let written = &r.files["t.sp"];
    assert!(written.contains("# @SELECT-INSTANCE(name=\"D*\""), "{written}");
    assert!(written.contains("=> @ORIENT(\"MYR90\");"), "{written}");
    assert!(r.report.differences.is_empty(), "{:#?}", r.report.differences);
    let (before, after) = (read("t.sp", &text), read("t.sp", written));
    for id in ["#D1", "#D2"] {
        assert_eq!(pins(&after, id), pins(&before, id), "{id}");
    }
}

#[test]
fn a_transformer_keeps_its_pins() {
    let text =
        "// t\n// @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\") }\n\
                simulator lang=spectre\nT1 (a 0 b 0) transformer\nT2 (c 0 d 0) transformer\n\
                // @RNL { #T1 @PLACE(0, 0) => @ORIENT(\"R0\"); #T2 @PLACE(10, 0) => @ORIENT(\"MX\") }\n";
    let r = run("t.scs", text);
    let written = &r.files["t.scs"];
    assert!(written.contains("#T1 @PLACE(0, 0) => @ORIENT(\"R270\")"), "{written}");
    assert!(written.contains("#T2 @PLACE(10, 0) => @ORIENT(\"MXR90\")"), "{written}");
    let (before, after) = (read("t.scs", text), read("t.scs", written));
    // the windings p_plus p_minus s_plus s_minus of 0.3 are p1 n1 p2 n2 of 0.4, node by node
    fn of_04(t: &str) -> &str {
        match t {
            "p_plus" => "p1",
            "p_minus" => "n1",
            "s_plus" => "p2",
            "s_minus" => "n2",
            t => t,
        }
    }
    for id in ["#T1", "#T2"] {
        assert_eq!(pins(&after, id), pins_named(&before, id, of_04), "{id}");
    }
    // the box of 0.4 is that of 0.3 turned too: the pins, the box and the orientations compare equal
    assert!(r.report.differences.is_empty(), "{:#?}", r.report.differences);
    assert!(r.report.successful(), "{:#?}", r.report);
}
