//! The box that the viewer of 0.3 draws for an instance with no symbol (9.2.6), kept by the upgrade
//! as a symbol of the file, `$BOX_<cell>`: the block symbol of a subcircuit the file defines, or
//! the symbol of each instance of a cell it does not. The upgraded instance is drawn as before.
use rnl_edit::upgrade::{upgrade, Upgrade, UpgradeOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::Interpretation;

fn read(text: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", text);
    rnl_view::interpret_with("t.sp", None, &Default::default(), &files, &rnl_runtime::Options::default()).unwrap()
}

fn run(text: &str) -> Upgrade {
    upgrade("t.sp", &MemoryProvider::new().with("t.sp", text), &UpgradeOptions::default()).unwrap()
}

/// The drawing of the symbol of the object `id`: its box, pins (place, way, label), primitives
/// (shape, points, style) and labels (key, text, place, alignment).
fn drawing(r: &Interpretation, id: &str) -> String {
    for level in &r.model.levels {
        if let Some(object) = level.objects.iter().find(|o| o.id == id) {
            let s = &level.symbols[object.symbol_view.expect("placed")];
            let pins: Vec<_> = s.pins.iter().map(|p| format!("{:?} {:?} {:?}", p.pos, p.dir, p.label)).collect();
            let prims: Vec<_> = s.prims.iter().map(|p| format!("{:?} {:?} {:?}", p.shape, p.points, p.style)).collect();
            let labels: Vec<_> = s
                .labels
                .iter()
                .map(|l| format!("{:?} {} {:?} {:?} {:?}", l.key, l.text, l.at, l.halign, l.valign))
                .collect();
            return format!("{:?}\n{pins:?}\n{prims:?}\n{labels:?}", s.bbox);
        }
    }
    panic!("no {id}")
}

fn header(version: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\") }}\n"
    )
}

/// A subcircuit `cell` of `ports` ports and an instance X1 of it in `top`, at the origin.
fn one_cell(ports: usize) -> String {
    let ports: Vec<String> = (1..=ports).map(|k| format!("p{k}")).collect();
    let nets: Vec<String> = (1..=ports.len()).map(|k| format!("n{k}")).collect();
    format!(
        "{}.subckt cell {}\n.ends cell\n.subckt top {}\nX1 {} cell\n* @RNL {{ #X1 @PLACE(0, 0) }}\n.ends top\n.end\n",
        header("0.3"),
        ports.join(" "),
        nets.join(" "),
        nets.join(" ")
    )
}

#[test]
fn the_box_of_0_3_is_the_block_symbol_of_the_subcircuit_for_any_count_of_ports() {
    for ports in [0, 1, 2, 3, 5, 10] {
        let text = one_cell(ports);
        let r = run(&text);
        let written = &r.files["t.sp"];
        assert!(written.contains("*   $BOX_cell = @DEFINE-SYMBOL {"), "{written}");
        assert!(written.contains("*   # @SET-SYMBOL $BOX_cell\n* }\n.ends cell"), "{written}");
        assert!(r.report.successful(), "{ports}: {:#?}", r.report);
        let notices: Vec<_> = r.report.files[0].notices.iter().filter(|n| n.rule == "default-box").collect();
        assert_eq!(notices.len(), 1, "{ports}");
        assert!(notices[0].reason.contains("delete this record"), "{}", notices[0].reason);
        assert_eq!(drawing(&read(written), "#X1"), drawing(&read(&text), "#X1"), "{ports} ports: {written}");
    }
}

#[test]
fn every_instance_of_a_cell_has_one_box() {
    let text = format!(
        "{}.subckt cell a b c\n.ends cell\n.subckt top x y z\nX1 x y z cell\nX2 z y x cell\n\
         * @RNL {{ #X1 @PLACE(0, 0); #X2 @PLACE(10, 0) }}\n.ends top\nX3 n1 n2 n3 cell\n* @RNL {{ #X3 @PLACE(0, 0) }}\n.end\n",
        header("0.3")
    );
    let r = run(&text);
    let written = &r.files["t.sp"];
    assert_eq!(written.matches("@DEFINE-SYMBOL").count(), 1, "{written}");
    assert!(r.report.successful(), "{:#?}", r.report);
    let (before, after) = (read(&text), read(written));
    for id in ["#X1", "#X2", "#X3"] {
        assert_eq!(drawing(&after, id), drawing(&before, id), "{id}");
    }
}

#[test]
fn a_cell_the_file_does_not_define_has_its_box_in_the_level_of_its_instances() {
    // pdk_cell is no subcircuit of the file: 0.3 draws its instances with the box, by node number
    let text = format!(
        "{}X3 a b c pdk_cell\nX4 c b a pdk_cell\nV1 a 0 1\n* @RNL {{ #X3 @PLACE(0, 0); #X4 @PLACE(10, 0); #V1 @PLACE(20, 0) }}\n.end\n",
        header("0.3")
    );
    let r = run(&text);
    let written = &r.files["t.sp"];
    assert_eq!(written.matches("$BOX_pdk_cell = @DEFINE-SYMBOL").count(), 1, "{written}");
    assert!(written.contains("*   #X3 @SET-SYMBOL $BOX_pdk_cell;\n*   #X4 @SET-SYMBOL $BOX_pdk_cell\n"), "{written}");
    assert!(r.report.successful(), "{:#?}", r.report);
    let (before, after) = (read(&text), read(written));
    for id in ["#X3", "#X4"] {
        assert_eq!(drawing(&after, id), drawing(&before, id), "{id}");
    }
}

#[test]
fn a_name_the_file_writes_is_not_taken() {
    let text = one_cell(2).replace("* t\n", "* t: $BOX_cell is mentioned here\n");
    let written = &run(&text).files["t.sp"];
    assert!(written.contains("$BOX_cell_2 = @DEFINE-SYMBOL"), "{written}");
    assert!(written.contains("# @SET-SYMBOL $BOX_cell_2"), "{written}");
}

#[test]
fn a_wire_to_a_pin_of_the_box_keeps_ending_at_it() {
    // ten ports: the default block symbol of 0.4 would have the pins at x ±5, the box of 0.3 has them at ±3
    let text = one_cell(10)
        .replace("#X1 @PLACE(0, 0)", "#X1 @PLACE(0, 0); &n1 @WIRE {#X1:1 (-6, 4)}; &n6 @WIRE {#X1:6 (6, 4)}");
    let r = run(&text);
    assert!(r.report.differences.is_empty(), "{:#?}", r.report.differences);
    assert!(r.report.successful(), "{:#?}", r.report);
    let written = &r.files["t.sp"];
    assert_eq!(drawing(&read(written), "#X1"), drawing(&read(&text), "#X1"));
}

#[test]
fn an_instance_given_a_symbol_has_no_box() {
    let text = format!(
        "{}.subckt cell a b\n.ends cell\nX1 n1 n2 cell\n* @RNL {{ #X1 @SET-SYMBOL $RESISTOR => @PLACE(0, 0) }}\n.end\n",
        header("0.3")
    );
    let r = run(&text);
    assert!(!r.files["t.sp"].contains("$BOX_"), "{}", r.files["t.sp"]);
    assert!(!r.report.files[0].notices.iter().any(|n| n.rule == "default-box"));
}
