//! The symbols of the series 0.3 that the review of the devices changed: the P type JFET stands
//! as `$PMOS` and `$PNP` do, its source up and its drain down, and the controlled sources of two
//! terminals are the diamonds of Analog Canvas.

#[path = "support/public_packages.rs"]
mod public_packages;

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::model::{LevelView, SymbolView};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(net: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", net);
    interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

fn top(r: &Interpretation) -> &LevelView {
    r.model.levels.iter().find(|l| l.title == "t.sp").unwrap()
}

fn symbol<'a>(lv: &'a LevelView, name: &str) -> &'a SymbolView {
    lv.symbols.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{name} is not drawn"))
}

/// Where the pin of a terminal stands in a symbol, and the way out of it.
fn pin(s: &SymbolView, terminal: &str) -> ((f64, f64), Option<u16>) {
    let p = s
        .pins
        .iter()
        .find(|p| p.terminals == [terminal])
        .unwrap_or_else(|| panic!("{} has no terminal {terminal}", s.name));
    (p.pos, p.dir)
}

#[test]
fn a_p_jfet_has_its_source_up_and_its_drain_down() {
    // d g s = 1 2 3: the N type has its drain up and its source down, the P type the other way
    let r = read(
        "* t\n.model nj njf\n.model pj pjf\nJ1 d g s nj\nJ2 d g s pj\nZ1 d g s nz\n.model nz nmf\n\
         * @RNL { #J1 @PLACE(0, 0); #J2 @PLACE(10, 0) }\n.end\n",
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    let n = symbol(lv, "$NJFET");
    assert_eq!(
        (pin(n, "1"), pin(n, "2"), pin(n, "3")),
        (((0.0, 2.0), Some(90)), ((-3.0, 0.0), Some(180)), ((0.0, -2.0), Some(270)))
    );
    let p = symbol(lv, "$PJFET");
    assert_eq!(
        (pin(p, "1"), pin(p, "2"), pin(p, "3")),
        (((0.0, -2.0), Some(270)), ((-3.0, 0.0), Some(180)), ((0.0, 2.0), Some(90)))
    );
    // as $PMOS and $PNP: the drain, or the collector, at the bottom
    let r = read("* t\n.model pm pmos\n.model pn pnp\nM1 d g s s pm\nQ1 c b e pn\n* @RNL { #M1 @PLACE(0, 0); #Q1 @PLACE(10, 0) }\n.end\n");
    let lv = top(&r);
    assert_eq!(pin(symbol(lv, "$PMOS_4TERM"), "1").0, (0.0, -2.0));
    assert_eq!(pin(symbol(lv, "$PNP"), "1").0, (0.0, -2.0));
}

#[test]
fn the_controlled_sources_of_two_terminals_are_the_diamonds_of_analog_canvas() {
    if !public_packages::available() {
        return;
    }
    // the symbols of the standard series draw what $CANVAS_*_DIAMOND of analog-canvas-extensions
    // draw: the same graphics, pins and box, for each of the four kinds
    let pairs = [
        ("$VCVS", "$CANVAS_VCVS_DIAMOND"),
        ("$VCCS", "$CANVAS_VCCS_DIAMOND"),
        ("$CCVS", "$CANVAS_CCVS_DIAMOND"),
        ("$CCCS", "$CANVAS_CCCS_DIAMOND"),
    ];
    let instances: String = (0..2 * pairs.len()).map(|k| format!("R{k} a{k} b{k} 1k\n")).collect();
    let mut record = Vec::new();
    for (k, (std, canvas)) in pairs.iter().enumerate() {
        record.push(format!("*   #R{} @SET-SYMBOL {std} => @PLACE({}, 0)", 2 * k, 20 * k));
        record.push(format!("*   #R{} @SET-SYMBOL {canvas} => @PLACE({}, 10)", 2 * k + 1, 20 * k));
    }
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); \
         @USE-PACKAGE(name=\"analog-canvas-extensions\", version=\"0.1\")}}\n{instances}* @RNL {{\n{}\n* }}\n.end\n",
        record.join(";\n")
    );
    let mut files = MemoryProvider::new().with("t.sp", &text).with(
        "pkgs/canvas.rnl",
        &std::fs::read_to_string(
            public_packages::package_dir("analog-canvas-extensions").join("analog-canvas-extensions.rnl"),
        )
        .unwrap(),
    );
    files.package_dirs.push("pkgs".into());
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    for (std, canvas) in pairs {
        let (a, b) = (symbol(lv, std), symbol(lv, canvas));
        assert_eq!(format!("{:?}", (&a.prims, &a.pins, a.bbox)), format!("{:?}", (&b.prims, &b.pins, b.bbox)), "{std}");
        // two terminals, the diamond's four lines among the graphics and no circle
        assert_eq!(a.pins.len(), 2, "{std}");
        assert!(a.prims.iter().all(|p| format!("{:?}", p.shape) != "Circle"), "{std}");
    }
}
