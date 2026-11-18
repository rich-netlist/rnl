//! Symbols of the standard series that the review of the devices settled: the P type JFET stands
//! as `$PMOS` and `$PNP` do, its source up and its drain down, and the controlled sources of two
//! terminals are the diamonds of Analog Canvas.

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

const HEADER: &str =
    "* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }";

#[test]
fn a_p_jfet_has_its_source_up_and_its_drain_down() {
    // the N type has its drain up and its source down, the P type the other way
    let r = read(&format!(
        "* t\n{HEADER}\n.model nj njf\n.model pj pjf\nJ1 d g s nj\nJ2 d g s pj\nZ1 d g s nz\n.model nz nmf\n\
         * @RNL {{ #J1 @PLACE(0, 0); #J2 @PLACE(10, 0) }}\n.end\n"
    ));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    let n = symbol(lv, "$NJFET");
    assert_eq!(
        (pin(n, "d"), pin(n, "g"), pin(n, "s")),
        (((0.0, 2.0), Some(90)), ((-3.0, 0.0), Some(180)), ((0.0, -2.0), Some(270)))
    );
    let p = symbol(lv, "$PJFET");
    assert_eq!(
        (pin(p, "d"), pin(p, "g"), pin(p, "s")),
        (((0.0, -2.0), Some(270)), ((-3.0, 0.0), Some(180)), ((0.0, 2.0), Some(90)))
    );
    // as $PMOS and $PNP: the drain, or the collector, at the bottom
    let r = read(&format!(
        "* t\n{HEADER}\n.model pm pmos\n.model pn pnp\nM1 d g s s pm\nQ1 c b e pn\n\
         * @RNL {{ #M1 @PLACE(0, 0); #Q1 @PLACE(10, 0) }}\n.end\n"
    ));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    assert_eq!(pin(symbol(lv, "$PMOS_4TERM"), "d").0, (0.0, -2.0));
    assert_eq!(pin(symbol(lv, "$PNP"), "c").0, (0.0, -2.0));
}

#[test]
fn the_controlled_sources_of_two_terminals_are_diamonds() {
    // the standard series draws them as Analog Canvas does, so analog-canvas-extensions 0.7 no
    // longer has its diamonds: two terminals, the diamond among the graphics and no circle
    let text = format!(
        "* t\n{HEADER}\nR1 a b 1k\nR2 c d 1k\n* @RNL {{\n\
         *   #R1 @SET-SYMBOL $CONTROLLED_VOLTAGE_SOURCE => @PLACE(0, 0);\n\
         *   #R2 @SET-SYMBOL $CONTROLLED_CURRENT_SOURCE => @PLACE(10, 0)\n* }}\n.end\n"
    );
    let r = read(&text);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    let diamond = [(0.0, 1.25), (1.25, 0.0), (0.0, -1.25), (-1.25, 0.0), (0.0, 1.25)];
    for name in ["$CONTROLLED_VOLTAGE_SOURCE", "$CONTROLLED_CURRENT_SOURCE"] {
        let s = symbol(lv, name);
        assert_eq!(s.pins.len(), 2, "{name}");
        assert_eq!((pin(s, "p").0, pin(s, "n").0), ((0.0, 2.0), (0.0, -2.0)), "{name}");
        assert!(s.prims.iter().all(|p| format!("{:?}", p.shape) != "Circle"), "{name}");
        assert!(
            s.prims.iter().any(|p| p.points.iter().map(|q| (q.0, q.1)).eq(diamond.iter().copied())),
            "{name}: {:?}",
            s.prims
        );
    }
}
