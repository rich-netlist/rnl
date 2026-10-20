//! The public package analog-canvas-extensions 0.1 (libraries/analog-canvas-extensions): the symbols of Analog Canvas that the series 0.3 does not
//! draw, which a file uses by naming the package in its header record (16.2).

use std::path::{Path, PathBuf};

#[path = "support/public_packages.rs"]
mod public_packages;

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
    public_packages::package_dir("analog-canvas-extensions").join("analog-canvas-extensions.rnl")
}

fn extension_files() -> MemoryProvider {
    let text = std::fs::read_to_string(package_file()).unwrap();
    let mut files = MemoryProvider::new().with("pkgs/analog-canvas-extensions.rnl", &text);
    files.package_dirs.push("pkgs".into());
    files
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
    let files = extension_files().with("t.sp", &text);
    interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap()
}

fn top(r: &Interpretation) -> &LevelView {
    r.model.levels.iter().find(|l| l.title == "t.sp").unwrap()
}

#[test]
fn a_file_that_names_the_package_finds_it_on_the_search_path() {
    if !public_packages::available() {
        return;
    }
    // The public extension must be provided explicitly.
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
    if !public_packages::available() {
        return;
    }
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
    if !public_packages::available() {
        return;
    }
    // every symbol is named $CANVAS_ and the component of Analog Canvas it draws
    let text = std::fs::read_to_string(package_file()).unwrap();
    let defined: Vec<&str> = text
        .lines()
        .filter(|l| l.contains(" = @DEFINE-SYMBOL("))
        .map(|l| l.trim_start().split(' ').next().unwrap())
        .collect();
    assert_eq!(defined.len(), 33, "{defined:?}");
    assert!(defined.iter().all(|n| n.starts_with("$CANVAS_")), "{defined:?}");
    // written in the language 0.3, it is read without any switch; its only remarks are the four
    // inputs of each of the six gates, off the integer points (9.2.4)
    let r = interpret_file(&package_file(), None, &FsProvider::new(&[]), &Options::default()).unwrap();
    let d: Vec<String> = r.diagnostics().iter().map(|d| r.sink.render(d)).collect();
    assert!(r.diagnostics().iter().all(|d| d.severity == Severity::Hint), "{d:?}");
    assert_eq!(d.len(), 24, "{d:?}");
    assert!(d.iter().all(|h| h.ends_with("pins are best placed on integer points (9.2.4)")), "{d:?}");
}

#[test]
fn the_diamond_sources_have_their_terminals_box_and_labels() {
    if !public_packages::available() {
        return;
    }
    // $CANVAS_VCVS_DIAMOND and $CANVAS_CCVS_DIAMOND, the diamonds of E and H without their control
    // leads: the output's two terminals, Analog Canvas's view box, and the name and the value at
    // the right of the diamond, as the catalog gives a frontend the symbols
    let options =
        rnl_view::CatalogOptions { packages: Some(vec!["analog-canvas-extensions".into()]), ..Default::default() };
    let c = serde_json::to_value(rnl_view::catalog(
        &FsProvider::new(&[package_file().parent().unwrap().to_path_buf()]),
        &options,
    ))
    .unwrap();
    let label = |key: &str, y: f64| serde_json::json!({"key": key, "text": format!("${{{key}}}"), "at": [1.65, y], "halign": "start", "valign": "baseline"});
    for name in ["$CANVAS_VCVS_DIAMOND", "$CANVAS_CCVS_DIAMOND"] {
        let s = c["symbols"].as_array().unwrap().iter().find(|s| s["symbol"] == name).unwrap();
        assert_eq!(
            s["pins"],
            serde_json::json!([{"terminal": 1, "at": [0.0, 2.0], "direction": "up"}, {"terminal": 2, "at": [0.0, -2.0], "direction": "down"}]),
            "{name}"
        );
        assert_eq!(s["box"], serde_json::json!([-2.4, -2.0, 1.5, 2.0]), "{name}");
        assert_eq!(s["labels"], serde_json::json!([label("name", 0.25), label("value", -0.95)]), "{name}");
    }
}

/// The four controlled sources of two terminals as the standard series 0.3 drew them until it drew
/// the diamonds (rnl 0d8e1d9), written as a package of their own, under other names.
const BEFORE_THE_DIAMONDS: &str = r#"@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="before-the-diamonds", version="0.1")}
@RNL {
  $BEFORE_VCVS = @DEFINE-SYMBOL(box=[-2.4, -2, 1.5, 2]) {
    (0, 2) @PIN(terminal=1, direction="up");
    (0, -2) @PIN(terminal=2, direction="down");
    (0, 0) @CIRCLE(radius=1.0756);
    @LINE {(-2.0058, 1.4535) (-1.1919, 1.4535)};
    @LINE {(-1.5988, 1.8605) (-1.5988, 1.0465)};
    @LINE {(-2.0058, -1.3372) (-1.1919, -1.3372)};
    @LINE {(0, 1.0756) (0, 2)};
    @LINE {(0, -1.0756) (0, -2)};
    ? @TEXT("${name}", key="name") => @PLACE(1.5, 0.25);
    ? @TEXT("${value}", key="value") => @PLACE(1.5, -0.95)
  };
  $BEFORE_VCCS = @DEFINE-SYMBOL(box=[-1.5, -2, 1.5, 2]) {
    (0, 2) @PIN(terminal=1, direction="up");
    (0, -2) @PIN(terminal=2, direction="down");
    (0, 0) @CIRCLE(radius=1.0756);
    @LINE {(0, 0.6977) (0, 0.2326)};
    @LINE(fill="linecolor", linestyle="none") {(0, -0.6977) (-0.4651, 0.2326) (0.4651, 0.2326) (0, -0.6977)};
    @LINE {(0, 1.0756) (0, 2)};
    @LINE {(0, -1.0756) (0, -2)};
    ? @TEXT("${name}", key="name") => @PLACE(1.5, 0.25);
    ? @TEXT("${value}", key="value") => @PLACE(1.5, -0.95)
  };
  $BEFORE_CCVS = @DEFINE-SYMBOL(box=[-2.4, -2, 1.5, 2]) {
    (0, 2) @PIN(terminal=1, direction="up");
    (0, -2) @PIN(terminal=2, direction="down");
    (0, 0) @CIRCLE(radius=1.0756);
    @LINE {(-2.0058, 1.4535) (-1.1919, 1.4535)};
    @LINE {(-1.5988, 1.8605) (-1.5988, 1.0465)};
    @LINE {(-2.0058, -1.3372) (-1.1919, -1.3372)};
    @LINE {(0, 1.0756) (0, 2)};
    @LINE {(0, -1.0756) (0, -2)};
    ? @TEXT("${name}", key="name") => @PLACE(1.5, 0.25);
    ? @TEXT("${value}", key="value") => @PLACE(1.5, -0.95)
  };
  $BEFORE_CCCS = @DEFINE-SYMBOL(box=[-1.5, -2, 1.5, 2]) {
    (0, 2) @PIN(terminal=1, direction="up");
    (0, -2) @PIN(terminal=2, direction="down");
    (0, 0) @CIRCLE(radius=1.0756);
    @LINE {(0, 0.6977) (0, 0.2326)};
    @LINE(fill="linecolor", linestyle="none") {(0, -0.6977) (-0.4651, 0.2326) (0.4651, 0.2326) (0, -0.6977)};
    @LINE {(0, 1.0756) (0, 2)};
    @LINE {(0, -1.0756) (0, -2)};
    ? @TEXT("${name}", key="name") => @PLACE(1.5, 0.25);
    ? @TEXT("${value}", key="value") => @PLACE(1.5, -0.95)
  }
}
"#;

#[test]
fn the_circle_sources_draw_what_the_standard_series_drew_before_its_diamonds() {
    if !public_packages::available() {
        return;
    }
    // Analog Canvas's own drawing of the four controlled sources, kept for the files it exports: the
    // same graphics, pins and box as the standard symbols had, beside the diamonds of the package
    let pairs = [
        ("$BEFORE_VCVS", "$CANVAS_VCVS_CIRCLE"),
        ("$BEFORE_VCCS", "$CANVAS_VCCS_CIRCLE"),
        ("$BEFORE_CCVS", "$CANVAS_CCVS_CIRCLE"),
        ("$BEFORE_CCCS", "$CANVAS_CCCS_CIRCLE"),
    ];
    let instances: String = (0..2 * pairs.len()).map(|k| format!("R{k} a{k} b{k} 1k\n")).collect();
    let mut record = Vec::new();
    for (k, (before, circle)) in pairs.iter().enumerate() {
        record.push(format!("*   #R{} @SET-SYMBOL {before} => @PLACE({}, 0)", 2 * k, 20 * k));
        record.push(format!("*   #R{} @SET-SYMBOL {circle} => @PLACE({}, 10)", 2 * k + 1, 20 * k));
    }
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); \
         @USE-PACKAGE(name=\"analog-canvas-extensions\", version=\"0.1\"); \
         @USE-PACKAGE(name=\"before-the-diamonds\", version=\"0.1\")}}\n{instances}* @RNL {{\n{}\n* }}\n.end\n",
        record.join(";\n")
    );
    let files = extension_files().with("t.sp", &text).with("pkgs/before-the-diamonds.rnl", BEFORE_THE_DIAMONDS);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = top(&r);
    for (before, circle) in pairs {
        let (a, b) = (
            lv.symbols.iter().find(|s| s.name == before).unwrap(),
            lv.symbols.iter().find(|s| s.name == circle).unwrap(),
        );
        assert_eq!(
            format!("{:?}", (&a.prims, &a.pins, a.bbox)),
            format!("{:?}", (&b.prims, &b.pins, b.bbox)),
            "{circle}"
        );
        // a circle, two pins
        assert_eq!(b.pins.len(), 2, "{circle}");
        assert!(b.prims.iter().any(|p| format!("{:?}", p.shape) == "Circle"), "{circle}");
    }
}

#[test]
fn the_circle_sources_are_in_the_catalog_with_their_terminals_box_and_labels() {
    if !public_packages::available() {
        return;
    }
    let options =
        rnl_view::CatalogOptions { packages: Some(vec!["analog-canvas-extensions".into()]), ..Default::default() };
    let c = serde_json::to_value(rnl_view::catalog(
        &FsProvider::new(&[package_file().parent().unwrap().to_path_buf()]),
        &options,
    ))
    .unwrap();
    let label = |key: &str, y: f64| serde_json::json!({"key": key, "text": format!("${{{key}}}"), "at": [1.5, y], "halign": "start", "valign": "baseline"});
    for (name, bbox) in [
        ("$CANVAS_VCVS_CIRCLE", [-2.4, -2.0, 1.5, 2.0]),
        ("$CANVAS_CCVS_CIRCLE", [-2.4, -2.0, 1.5, 2.0]),
        ("$CANVAS_VCCS_CIRCLE", [-1.5, -2.0, 1.5, 2.0]),
        ("$CANVAS_CCCS_CIRCLE", [-1.5, -2.0, 1.5, 2.0]),
    ] {
        let s = c["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["symbol"] == name)
            .unwrap_or_else(|| panic!("{name} is not in the catalog"));
        assert_eq!(
            s["pins"],
            serde_json::json!([{"terminal": 1, "at": [0.0, 2.0], "direction": "up"}, {"terminal": 2, "at": [0.0, -2.0], "direction": "down"}]),
            "{name}"
        );
        assert_eq!(s["box"], serde_json::json!(bbox), "{name}");
        assert_eq!(s["labels"], serde_json::json!([label("name", 0.25), label("value", -0.95)]), "{name}");
    }
    // the diamonds are still there, for the files that were exported with them
    for name in ["$CANVAS_VCVS_DIAMOND", "$CANVAS_VCCS_DIAMOND", "$CANVAS_CCVS_DIAMOND", "$CANVAS_CCCS_DIAMOND"] {
        assert!(c["symbols"].as_array().unwrap().iter().any(|s| s["symbol"] == name), "{name}");
    }
}
