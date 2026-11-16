//! `rnl upgrade` of a file of 0.3 that uses Analog Canvas's package analog-canvas-extensions 0.1–0.6:
//! its symbols that the standard series 0.4 has take their standard names, the others keep theirs
//! in the published 0.7, and each instance drawn with one gets the terminal map of the table, so
//! that each terminal keeps the place and the direction of its pin (rnl#481).

use std::path::Path;

use crate::public_packages;

use rnl_cli::interpret_file;
use rnl_edit::upgrade::canvas::{Row, PACKAGE, TABLE, TARGET_VERSION};
use rnl_edit::upgrade::{upgrade, Upgrade, UpgradeOptions};
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// The published package of a version, as its file.
fn package(version: &str) -> String {
    let dir = public_packages::version_dir(PACKAGE, version);
    std::fs::read_to_string(dir.join(format!("{PACKAGE}.rnl"))).unwrap()
}

/// `text` as `t.sp`, with a package directory that holds `versions` of the package.
fn files(text: &str, versions: &[&str]) -> MemoryProvider {
    let mut files = MemoryProvider::new().with("t.sp", text);
    for v in versions {
        files.insert(&format!("pkgs/{v}/{PACKAGE}.rnl"), &package(v));
    }
    files.package_dirs.push("pkgs".into());
    files
}

fn read(files: &MemoryProvider) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, files, &Options::default()).unwrap()
}

fn problems(r: &Interpretation) -> Vec<String> {
    r.diagnostics()
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Violation | Severity::Warning))
        .map(|d| r.sink.render(d))
        .collect()
}

fn run(text: &str, version: &str) -> Upgrade {
    upgrade("t.sp", &files(text, &[version, TARGET_VERSION]), &UpgradeOptions::default()).unwrap()
}

/// The place and the direction of the pin of each terminal of the instance `id`, by the terminal.
fn pins(r: &Interpretation, id: &str) -> Vec<(String, (f64, f64), Option<u16>)> {
    let object = r.model.levels[0].objects.iter().find(|o| o.id == id).expect(id);
    let mut out: Vec<_> = object
        .pins
        .iter()
        .flat_map(|p| p.terminals.iter().map(move |t| (t.clone(), p.at.expect("placed"), p.dir)))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Each row of the table, in 0.6: an instance of a subcircuit with five ports drawn with each
/// symbol, each at a place of its own. The upgraded instance is drawn with the symbol of the table,
/// and each terminal the earlier symbol drew is at the same place, in the same direction; no other
/// terminal is drawn. The upgrade reports no difference.
#[test]
fn each_symbol_of_the_table_keeps_the_pins_of_its_terminals() {
    if !public_packages::available() {
        return;
    }
    let symbols: Vec<&Row> = TABLE.iter().filter(|r| !r.terminals.is_empty()).collect();
    assert_eq!(symbols.len(), 44, "the symbols drawn on instances");
    assert!(symbols.iter().flat_map(|r| r.terminals).all(|(n, _)| *n <= 5));
    let instances: String = (1..=symbols.len()).map(|k| format!("X{k} t1 t2 t3 t4 t5 part\n")).collect();
    let drawn: Vec<String> = symbols
        .iter()
        .enumerate()
        .map(|(k, r)| format!("*   #X{} @SET-SYMBOL ${} => @PLACE({}, 0) => @ORIENT(\"R0\")", k + 1, r.old, 20 * k))
        .collect();
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\"); \
         @USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.6\") }}\n.subckt part t1 t2 t3 t4 t5\n.ends part\n\
         {instances}* @RNL {{\n{}\n* }}\n.end\n",
        drawn.join(";\n")
    );
    let before = read(&files(&text, &["0.6"]));
    assert_eq!(problems(&before), Vec::<String>::new());
    let up = run(&text, "0.6");
    let written = &up.files["t.sp"];
    assert!(written.contains(&format!("@USE-PACKAGE(name=\"{PACKAGE}\", version=\"{TARGET_VERSION}\")")));
    assert!(up.report.differences.is_empty(), "{:#?}", up.report.differences);
    assert!(up.report.successful(), "{:#?} {:#?}", up.report.diagnostics, up.report.files);
    let after = read(&files(written, &[TARGET_VERSION]));
    assert_eq!(problems(&after), Vec::<String>::new(), "{written}");
    for (k, row) in symbols.iter().enumerate() {
        let id = format!("#X{}", k + 1);
        let pairs: Vec<String> = row.terminals.iter().map(|(n, t)| format!("[{n}, \"{t}\"]")).collect();
        let call = format!("{id} @SET-SYMBOL(terminals=[{}]) ${} ", pairs.join(", "), row.new);
        // the diodes of 0.4 stand upright: R0 of 0.3 is R90 of 0.4 (the turn back, then R0)
        let upright = ["LED", "SCHOTTKY_DIODE", "SHOCKLEY_DIODE", "PUT"].contains(&row.new);
        let orient = if upright { "R90" } else { "R0" };
        // the UJT of 0.4 has its origin one unit left: its PLACE moves right by it
        let x = 20 * k + usize::from(row.new == "UJT");
        assert!(
            written.contains(&format!("{call}=> @PLACE({x}, 0) => @ORIENT(\"{orient}\")")),
            "{}: {written}",
            row.old
        );
        assert_eq!(pins(&after, &id), pins(&before, &id), "{}", row.old);
        assert_eq!(pins(&before, &id).len(), row.terminals.len(), "{}: every drawn pin is in the table", row.old);
        // a symbol that moves to the standard series is found there, here through rnl-std-all
        if row.standard.is_some() {
            assert!(!written.contains(&format!("${} ", row.old)), "{written}");
        }
    }
}

/// A file of the UJT in each orientation and in none, wired, as `text` with the statements `extra`.
fn ujts(extra: &str) -> String {
    let codes = ["R0", "R90", "R180", "R270", "MY", "MX", "MYR90", "MXR90"];
    let mut drawn: Vec<String> = codes
        .iter()
        .enumerate()
        .map(|(k, c)| format!("*   #X{} @SET-SYMBOL $CANVAS_UJT => @PLACE({}, 2.5) => @ORIENT(\"{c}\")", k + 1, 10 * k))
        .collect();
    drawn.push("*   #X9 @SET-SYMBOL $CANVAS_UJT => @PLACE(90, 2.5)".into());
    drawn.push("*   &b @WIRE {#X9:2 (91, -5)}".into());
    let instances: String = (1..=9).map(|k| format!("X{k} e b c part\n")).collect();
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\"); \
         @USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.6\") }}\n.subckt part t1 t2 t3\n.ends part\n\
         {instances}* @RNL {{\n{}{extra}\n* }}\n.end\n",
        drawn.join(";\n")
    )
}

/// The UJT of 0.4 has its origin on the column of b1 and b2, one unit left of where the UJT of the
/// package had it. Each PLACE of its origin moves by that unit, turned as the instance is, so each
/// pin stays where it was; one that no ORIENT turns moves as at R0, as a viewer draws it, and the
/// wire to it still ends on its pin. The upgrade reports no difference.
#[test]
fn the_ujt_moves_with_its_origin_and_keeps_its_pins() {
    if !public_packages::available() {
        return;
    }
    let text = ujts("");
    let before = read(&files(&text, &["0.6"]));
    assert_eq!(problems(&before), Vec::<String>::new());
    let up = run(&text, "0.6");
    let written = &up.files["t.sp"];
    assert!(up.report.differences.is_empty(), "{:#?}", up.report.differences);
    assert!(up.report.successful(), "{:#?} {:#?}", up.report.diagnostics, up.report.files);
    let ujt = "@SET-SYMBOL(terminals=[[1, \"e\"], [2, \"b1\"], [3, \"b2\"]]) $UJT";
    for (id, place) in [
        ("X1", "@PLACE(1, 2.5) => @ORIENT(\"R0\")"),
        ("X2", "@PLACE(10, 3.5) => @ORIENT(\"R90\")"),
        ("X3", "@PLACE(19, 2.5) => @ORIENT(\"R180\")"),
        ("X4", "@PLACE(30, 1.5) => @ORIENT(\"R270\")"),
        ("X5", "@PLACE(39, 2.5) => @ORIENT(\"MY\")"),
        ("X9", "@PLACE(91, 2.5)"),
    ] {
        assert!(written.contains(&format!("#{id} {ujt} => {place}")), "{id}: {written}");
    }
    let after = read(&files(written, &[TARGET_VERSION]));
    assert_eq!(problems(&after), Vec::<String>::new(), "{written}");
    for k in 1..=9 {
        let id = format!("#X{k}");
        assert_eq!(pins(&after, &id), pins(&before, &id), "{id}");
    }
}

/// What measures from the origin of the UJT along the axis it moves on cannot move with it, and is
/// left for review; along the other axis it says what it said. (A PLACE of a set, which could place
/// it with others, is left for review in any case: `set-placement`.)
#[test]
fn what_measures_from_the_origin_of_the_ujt_is_left_for_review() {
    if !public_packages::available() {
        return;
    }
    let review = |extra: &str| {
        let text = format!("{}\nR1 e b 1k\n", ujts(extra)).replace(".end\n\nR1 e b 1k\n", "R1 e b 1k\n.end\n");
        assert_eq!(problems(&read(&files(&text, &["0.6"]))), Vec::<String>::new(), "{text}");
        let up = run(&text, "0.6");
        up.report.files.iter().flat_map(|f| &f.unconverted).map(|u| u.rule.clone()).collect::<Vec<_>>()
    };
    assert_eq!(review(";\n*   {#X1 #R1} @ALIGN(\"y\")"), Vec::<String>::new());
    assert_eq!(review(";\n*   {#X1 #R1} @ALIGN(\"x\")"), ["moved-symbol-origin"]);
    // turned a quarter, it moves along y
    assert_eq!(review(";\n*   {#X2 #R1} @ALIGN(\"x\")"), Vec::<String>::new());
    assert_eq!(review(";\n*   {#X2 #R1} @ALIGN(\"y\")"), ["moved-symbol-origin"]);
}

/// The names of meaning only, and the switch with three ways and the filled port, keep their names.
#[test]
fn the_names_of_meaning_only_and_the_figures_stay() {
    let kept: Vec<&str> = TABLE.iter().filter(|r| r.terminals.is_empty()).map(|r| r.old).collect();
    assert_eq!(kept.len(), 12, "{kept:?}");
    assert!(TABLE.iter().filter(|r| r.terminals.is_empty()).all(|r| r.standard.is_none() && r.old == r.new));
    assert_eq!(TABLE.iter().filter(|r| r.standard.is_some()).count(), 25);
}

/// A file of 0.1, the first version, with the chopper and a controlled source, as Analog Canvas
/// wrote them: on an E card, whose controlling terminals no pin draws.
#[test]
fn a_file_of_the_first_version_upgrades_too() {
    if !public_packages::available() {
        return;
    }
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\"); \
         @USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.1\") }}\n.subckt chop ip in op on ck\n.ends chop\n\
         X1 a b c d k chop\nE1 o 0 a b 2\n\
         * @RNL {{ #X1 @SET-SYMBOL $CANVAS_CHOPPER => @PLACE(0, 0); #E1 @SET-SYMBOL $CANVAS_VCVS_DIAMOND => @PLACE(10, 0); \
         &o @WIRE {{#E1:1 (10, 5)}} }}\n.end\n"
    );
    assert_eq!(problems(&read(&files(&text, &["0.1"]))), Vec::<String>::new());
    let up = run(&text, "0.1");
    let written = &up.files["t.sp"];
    assert!(
        written.contains(
            "#X1 @SET-SYMBOL(terminals=[[1, \"inp\"], [2, \"inn\"], [3, \"outp\"], [4, \"outn\"]]) $CHOPPER_SYMBOL"
        ),
        "{written}"
    );
    assert!(
        written.contains("#E1 @SET-SYMBOL(terminals=[[1, \"p\"], [2, \"n\"]]) $CONTROLLED_VOLTAGE_SOURCE"),
        "{written}"
    );
    assert!(up.report.successful(), "{:#?} {:#?}", up.report.differences, up.report.diagnostics);
}

/// The label of a port, which Analog Canvas binds by `%port1::name`: the key is `netname` in 0.4,
/// and a wire to the port's role reaches its pin by name, on the standard port and on the filled
/// one of the package alike.
#[test]
fn the_label_of_a_port_and_a_wire_to_it_follow_the_key_netname() {
    if !public_packages::available() {
        return;
    }
    for symbol in ["$IO_PORT_SYMBOL", "$CANVAS_PORT_FILLED"] {
        let text = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\"); \
             @USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.6\") }}\n.subckt top a\nR1 a 0 1k\n\
             * @RNL {{\n*   #R1 @PLACE(0, 0);\n\
             *   $IO_PORT @ATTACH &a(p) => @SET-SYMBOL {symbol} => @PLACE(-6, 2) => %port1;\n\
             *   %port1 @TEXT(\"A\", key=\"name\", dx=-2);\n\
             *   $CANVAS_LABEL @ATTACH(bind=\"port\") %port1::name(label);\n\
             *   &a @WIRE {{%port1:p #R1:1}}\n* }}\n.ends top\n.end\n"
        );
        assert_eq!(problems(&read(&files(&text, &["0.6"]))), Vec::<String>::new(), "{symbol}");
        let up = run(&text, "0.6");
        let written = &up.files["t.sp"];
        assert!(written.contains("%port1 @TEXT(\"A\", key=\"netname\", dx=-2)"), "{written}");
        assert!(written.contains("$CANVAS_LABEL @ATTACH(bind=\"port\") %port1::netname(label)"), "{written}");
        assert!(written.contains("&a @WIRE {%port1::pin #R1:1}"), "{written}");
        assert!(
            up.report.successful(),
            "{symbol}: {:#?} {:#?} {:#?}",
            up.report.differences,
            up.report.diagnostics,
            up.report.files
        );
    }
}

/// A `$CANVAS_*` that the file defines itself is its own, and keeps its name and its binding.
#[test]
fn a_name_the_file_defines_itself_is_left_alone() {
    if !public_packages::available() {
        return;
    }
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\"); \
         @USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.6\") }}\nD1 a 0 dm\nD2 a 0 dm\n\
         * @RNL {{ $CANVAS_LED = @DEFINE-SYMBOL {{ (0, 0) @PIN(terminal=1); (2, 0) @PIN(terminal=2) }}; \
         #D1 @SET-SYMBOL $CANVAS_LED => @PLACE(0, 0); #D2 @SET-SYMBOL $CANVAS_SCHOTTKY_DIODE => @PLACE(10, 0) }}\n.end\n"
    );
    let up = run(&text, "0.6");
    let written = &up.files["t.sp"];
    assert!(written.contains("#D1 @SET-SYMBOL $CANVAS_LED"), "{written}");
    assert!(written.contains("#D2 @SET-SYMBOL(terminals=[[1, \"a\"], [2, \"k\"]]) $SCHOTTKY_DIODE"), "{written}");
}

/// A package given by `path=`: the published file is known by its bytes, its path left out (0.7 is
/// read from a package directory), and a file that differs changes nothing and is reported.
#[test]
fn a_package_given_by_path_is_compared_with_the_published_one() {
    if !public_packages::available() {
        return;
    }
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\"); \
         @USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.6\", path=\"lib/ace.rnl\") }}\nD1 a 0 dm\n\
         * @RNL {{ #D1 @SET-SYMBOL $CANVAS_LED => @PLACE(0, 0) }}\n.end\n"
    );
    let mut published = files(&text, &[TARGET_VERSION]);
    published.insert("lib/ace.rnl", &package("0.6"));
    let up = upgrade("t.sp", &published, &UpgradeOptions::default()).unwrap();
    let written = &up.files["t.sp"];
    assert!(written.contains(&format!("@USE-PACKAGE(name=\"{PACKAGE}\", version=\"0.7\")")), "{written}");
    assert!(written.contains("$LED"), "{written}");
    assert!(!up.files.contains_key("lib/ace.rnl"), "the published package is not the file group's to upgrade");
    assert!(up.report.files[0].notices.iter().any(|n| n.rule == "canvas-package"), "{:#?}", up.report.files);
    assert!(up.report.successful(), "{:#?} {:#?}", up.report.differences, up.report.diagnostics);

    let mut changed = files(&text, &[TARGET_VERSION]);
    changed.insert("lib/ace.rnl", &format!("{}\n/* edited */\n", package("0.6")));
    let up = upgrade("t.sp", &changed, &UpgradeOptions::default()).unwrap();
    let written = &up.files["t.sp"];
    assert!(written.contains("$CANVAS_LED"), "{written}");
    assert!(
        up.report.files.iter().flat_map(|f| &f.unconverted).any(|u| u.rule == "canvas-package-changed"),
        "{:#?}",
        up.report.files
    );
    assert!(!up.report.successful());
}
