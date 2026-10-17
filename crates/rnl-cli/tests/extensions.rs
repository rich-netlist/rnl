//! The package analog-canvas-extensions 0.1 (libraries/analog-canvas-extensions), bundled with the
//! core beside the standard series: the symbols of Analog Canvas that the series 0.3 does not
//! draw, which a file uses by naming the package in its header record (16.2).

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::diag::Severity;
use rnl_runtime::model::LevelView;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// Analog Canvas's gates with four inputs.
const GATES: [&str; 6] = [
    "$CANVAS_AND_4INPUT",
    "$CANVAS_NAND_4INPUT",
    "$CANVAS_OR_4INPUT",
    "$CANVAS_NOR_4INPUT",
    "$CANVAS_XOR_4INPUT",
    "$CANVAS_XNOR_4INPUT",
];

fn package_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/analog-canvas-extensions/analog-canvas-extensions.rnl")
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

/// A file of 0.3 that names the package, as Analog Canvas writes it, and draws each of its gates
/// with four inputs on an instance of a subcircuit with five terminals, a b c d y.
fn read_gates() -> Interpretation {
    let instances: String = (0..GATES.len()).map(|k| format!("X{k} a b c d y g4\n")).collect();
    let record: Vec<String> =
        GATES.iter().enumerate().map(|(k, g)| format!("*   #X{k} @SET-SYMBOL {g} => @PLACE({}, 0)", 10 * k)).collect();
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); \
         @USE-PACKAGE(name=\"analog-canvas-extensions\", version=\"0.1\")}}\n\
         .subckt g4 a b c d y\n.ends g4\n{instances}* @RNL {{\n{}\n* }}\n.end\n",
        record.join(";\n")
    );
    let files = MemoryProvider::new().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap()
}

fn top(r: &Interpretation) -> &LevelView {
    r.model.levels.iter().find(|l| l.title == "t.sp").unwrap()
}

#[test]
fn a_file_that_names_the_package_finds_it_bundled() {
    // no package directory is given: the package is the core's own
    let r = read_gates();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    let p = lv.packages.iter().find(|p| p.name == "analog-canvas-extensions").unwrap();
    assert_eq!(p.version, "0.1");
    assert!(
        r.sources.iter().any(|(f, _)| f == "analog-canvas-extensions.rnl"),
        "{:?}",
        r.sources.iter().map(|s| &s.0).collect::<Vec<_>>()
    );
    let drawn: Vec<&str> =
        lv.items.iter().filter(|i| i.kind == "instance").map(|i| lv.symbols[i.symbol].name.as_str()).collect();
    assert_eq!(drawn, GATES);
}

#[test]
fn its_gates_keep_the_inputs_where_analog_canvas_puts_them() {
    // 0.8 apart, not on integer points, unlike the standard gates with four inputs
    let r = read_gates();
    let lv = top(&r);
    for gate in GATES {
        let s = lv.symbols.iter().find(|s| s.name == gate).unwrap();
        let inputs: Vec<(f64, f64)> = s.pins.iter().take(4).map(|p| p.pos).collect();
        assert_eq!(inputs, [(-3.0, 1.2), (-3.0, 0.4), (-3.0, -0.4), (-3.0, -1.2)], "{gate}");
    }
}

#[test]
fn checked_on_its_own_it_gives_a_hint_for_those_pins_alone() {
    // every symbol is named $CANVAS_ and the component of Analog Canvas it draws
    let text = std::fs::read_to_string(package_file()).unwrap();
    let defined: Vec<&str> = text
        .lines()
        .filter(|l| l.contains(" = @DEFINE-SYMBOL("))
        .map(|l| l.trim_start().split(' ').next().unwrap())
        .collect();
    assert_eq!(defined.len(), 29, "{defined:?}");
    assert!(defined.iter().all(|n| n.starts_with("$CANVAS_")), "{defined:?}");
    // written in the language 0.3, it is read without any switch; its only remarks are the four
    // inputs of each of the six gates, off the integer points (9.2.4)
    let r = interpret_file(&package_file(), None, &FsProvider::new(&[]), &Options::default()).unwrap();
    let d: Vec<String> = r.diagnostics().iter().map(|d| r.sink.render(d)).collect();
    assert!(r.diagnostics().iter().all(|d| d.severity == Severity::Hint), "{d:?}");
    assert_eq!(d.len(), 24, "{d:?}");
    assert!(d.iter().all(|h| h.ends_with("pins are best placed on integer points (9.2.4)")), "{d:?}");
}
