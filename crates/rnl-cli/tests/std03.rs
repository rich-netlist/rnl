//! The standard series 0.3 (libraries/standard/0.3), bundled beside the series 0.2, and the
//! standard package that a file uses implicitly (16.3 of the language 0.3): the newest version
//! that the tool has of the standard series written in the language version of the file, so that
//! a file of 0.2 keeps the series 0.2 and a file of 0.3 uses the series 0.3; a version pinned in
//! the header record of the file, or of a file that includes it, comes first. A main file without
//! a header record is read in the tool's default version, 0.3 (16.1), and so uses the series 0.3.

use std::path::{Path, PathBuf};

use rnl_cli::solve::{report, solve, SolveOptions};
use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::diag::Severity;
use rnl_runtime::interp::obj::Priority;
use rnl_runtime::model::LevelView;
use rnl_runtime::problem::{FactKind, LevelProblem, ObjectKind, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use rnl_solver::SolverOptions;

/// The packages of the series 0.2 at the root, as rnl-std 0.2 brings them in.
const SERIES_02: [&str; 8] = [
    "rnl-std-style",
    "rnl-std-symbols",
    "rnl-std-virtual",
    "rnl-std-core",
    "rnl-std-fill",
    "rnl-std-semantics",
    "rnl-std-layout",
    "rnl-std",
];

/// The packages of the series 0.3 at the root, as rnl-std 0.3 brings them in: rnl-std-core's
/// first, the rule packages of SPICE before Spectre's (16.3).
const SERIES_03: [&str; 10] = [
    "rnl-std-style",
    "rnl-std-basic",
    "rnl-std-semiconductors",
    "rnl-std-spice",
    "rnl-std-spectre",
    "rnl-std-core",
    "rnl-std-fill",
    "rnl-std-semantics",
    "rnl-std-layout",
    "rnl-std",
];

fn read(files: &MemoryProvider, entry: &str, options: &Options) -> Interpretation {
    interpret_file(Path::new(entry), None, files, options).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

fn level<'a>(r: &'a Interpretation, title: &str) -> &'a LevelView {
    r.model.levels.iter().find(|l| l.title == title).unwrap()
}

/// The packages at the root of a level, in queue order, with their versions.
fn root(lv: &LevelView) -> Vec<(&str, &str)> {
    lv.packages.iter().filter(|p| p.at.is_none()).map(|p| (p.name.as_str(), p.version.as_str())).collect()
}

fn names<'a>(of: &[(&'a str, &str)]) -> Vec<&'a str> {
    of.iter().map(|(n, _)| *n).collect()
}

/// A netlist whose header record writes `header` after its version.
fn netlist(version: &str, header: &str, body: &str) -> String {
    let header = if header.is_empty() { String::new() } else { format!("; {header}") };
    format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){header}}}\nR1 a 0 1k\n{body}.end\n")
}

#[test]
fn a_file_of_0_2_keeps_the_series_0_2() {
    // with the series 0.3 bundled beside it
    let files = MemoryProvider::new().with("t.sp", &netlist("0.2", "", ""));
    let r = read(&files, "t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let at_root = root(level(&r, "t.sp"));
    assert_eq!(names(&at_root), SERIES_02);
    assert!(at_root.iter().all(|(_, v)| *v == "0.2"), "{at_root:?}");
    // the series 0.3 is not even read
    assert!(
        !r.sources.iter().any(|(f, _)| f == "rnl-std-basic.rnl"),
        "{:?}",
        r.sources.iter().map(|s| &s.0).collect::<Vec<_>>()
    );
}

#[test]
fn a_file_of_0_3_uses_the_series_0_3() {
    // so does a main file without a header record, read in the tool's default version (16.1)
    for text in [netlist("0.3", "", ""), "* t\nR1 a 0 1k\n.end\n".to_string()] {
        let files = MemoryProvider::new().with("t.sp", &text);
        let r = read(&files, "t.sp", &Options::default());
        assert!(errors(&r).is_empty(), "{:?}", errors(&r));
        let at_root = root(level(&r, "t.sp"));
        // a file of the SPICE dialect drops the rule package of Spectre (16.3)
        let want: Vec<&str> = SERIES_03.iter().copied().filter(|n| *n != "rnl-std-spectre").collect();
        assert_eq!(names(&at_root), want, "{text}");
        assert!(at_root.iter().all(|(_, v)| *v == "0.3"), "{at_root:?}");
    }
}

#[test]
fn a_pinned_version_comes_first() {
    // a file of 0.3 that pins the series 0.2: the series 0.2, in its own order
    let files =
        MemoryProvider::new().with("t.sp", &netlist("0.3", "@USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")", ""));
    let r = read(&files, "t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&root(level(&r, "t.sp"))), SERIES_02);
    // a file of 0.2 that pins the series 0.3, written in the language 0.3: the series 0.3 in its
    // order at the root, not after the packages of 0.2 that it has too; a file of 0.2 drops no
    // rule package
    let files =
        MemoryProvider::new().with("t.sp", &netlist("0.2", "@USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")", ""));
    let r = read(&files, "t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let at_root = root(level(&r, "t.sp"));
    assert_eq!(names(&at_root), SERIES_03);
    assert!(at_root.iter().all(|(_, v)| *v == "0.3"), "{at_root:?}");
}

#[test]
fn a_file_that_includes_another_pins_its_series_too() {
    let lib = "* lib\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.subckt cell a b\nR9 a b 1k\n.ends\n";
    let main = |header: &str| {
        let header = if header.is_empty() { String::new() } else { format!("; {header}") };
        format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"){header}}}\n.include \"lib.sp\"\nX1 a 0 cell\n.end\n")
    };
    // a file of 0.3 that pins rnl-std 0.3 gives it to the file of 0.2 it includes
    let files = MemoryProvider::new()
        .with("lib.sp", lib)
        .with("t.sp", &main("@USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")"));
    let r = read(&files, "t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let cell = root(level(&r, "cell"));
    assert!(cell.iter().all(|(_, v)| *v == "0.3") && names(&cell).contains(&"rnl-std-basic"), "{cell:?}");
    // unpinned, each file uses the series of its own language version
    let files = MemoryProvider::new().with("lib.sp", lib).with("t.sp", &main(""));
    let r = read(&files, "t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&root(level(&r, "cell"))), SERIES_02);
    assert!(names(&root(level(&r, "t.sp"))).contains(&"rnl-std-basic"));
}

#[test]
fn a_package_directory_hides_the_bundled_series_of_the_same_version() {
    // a frontend's own rnl-std 0.3 written in the language 0.2, as Analog Canvas's preview of the
    // series is, in a package directory, which comes before the bundled packages: it is the rnl-std
    // 0.3 found (16.2), and the bundled one is not read. A file of 0.2 uses it, the newest written
    // in 0.2; so does a file of 0.3, for which no series written in 0.3 is left
    let own = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"rnl-std\", version=\"0.3\")}\n";
    for version in ["0.2", "0.3"] {
        let mut files = MemoryProvider::new().with("pkgs/rnl-std.rnl", own).with("t.sp", &netlist(version, "", ""));
        files.package_dirs.push("pkgs".into());
        let r = read(&files, "t.sp", &Options::default());
        assert!(errors(&r).is_empty(), "{version}: {:?}", errors(&r));
        assert_eq!(root(level(&r, "t.sp")), [("rnl-std", "0.3")], "{version}");
        assert!(!r.sources.iter().any(|(f, _)| f == "rnl-std-basic.rnl"), "{version}");
    }
}

#[test]
fn a_package_that_is_not_read_uses_no_series() {
    // a package of a version that the toolkit does not read, checked on its own, is not read
    // (16.1): its version is the one error, and no standard series is used
    let package = "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\", version=\"1.0\")}\n";
    let r = read(&MemoryProvider::new().with("p.rnl", package), "p.rnl", &Options::default());
    let e = errors(&r);
    assert!(e.len() == 1 && e[0].contains("unsupported version 0.4"), "{e:?}");
    assert!(!r.sources.iter().any(|(f, _)| f.starts_with("rnl-std")), "{:?}", r.sources);
}

/// The cases of 0.3 of the corpus.
fn case(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases-0.3").join(name)
}

/// The symbol each drawn instance of the top level is drawn with.
fn symbols(path: &Path) -> Vec<(String, String)> {
    let r = interpret_file(path, None, &FsProvider::new(&[]), &Options::default()).unwrap();
    let errors = errors(&r);
    assert!(errors.is_empty(), "{}: {errors:?}", path.display());
    let lv = &r.model.levels[0];
    lv.items
        .iter()
        .filter(|i| i.kind == "instance")
        .map(|i| (i.id.clone(), lv.symbols[i.symbol].name.clone()))
        .collect()
}

#[test]
fn every_native_element_gets_its_symbol() {
    // the checks of rnl-dev's std-0.3/check-preview.mjs: every element each adapter reads natively,
    // a SPICE file by element letter and model type, a Spectre file by master and model type
    let spice = [
        ("#R1", "$RESISTOR"),
        ("#C1", "$CAPACITOR"),
        ("#L1", "$INDUCTOR"),
        ("#V1", "$VOLTAGE_SOURCE"),
        ("#I1", "$CURRENT_SOURCE"),
        ("#E1", "$VCVS_4TERM"),
        ("#G1", "$VCCS_4TERM"),
        ("#F1", "$CCCS"),
        ("#H1", "$CCVS"),
        ("#B1", "$BEHAVIORAL_SOURCE"),
        ("#S1", "$VOLTAGE_CONTROLLED_SWITCH"),
        ("#W1", "$SWITCH"),
        ("#D1", "$DIODE"),
        ("#Q1", "$NPN"),
        ("#Q2", "$PNP"),
        ("#M1", "$NMOS_4TERM"),
        ("#M2", "$PMOS_4TERM"),
        ("#J1", "$NJFET"),
        ("#J2", "$PJFET"),
        ("#Z1", "$NJFET"),
        ("#Z2", "$PJFET"),
        ("#T1", "$TRANSMISSION_LINE"),
        ("#O1", "$TRANSMISSION_LINE"),
        ("#U1", "$RC_LINE"),
        ("#R2", "$RESISTOR"),
    ];
    let spectre = [
        ("#r1", "$RESISTOR"),
        ("#c1", "$CAPACITOR"),
        ("#l1", "$INDUCTOR"),
        ("#v1", "$VOLTAGE_SOURCE"),
        ("#i1", "$CURRENT_SOURCE"),
        ("#e1", "$VCVS_4TERM"),
        ("#g1", "$VCCS_4TERM"),
        ("#f1", "$CCCS"),
        ("#h1", "$CCVS"),
        ("#b1", "$BEHAVIORAL_SOURCE"),
        ("#sw1", "$SWITCH"),
        ("#p1", "$RF_PORT"),
        ("#d1", "$DIODE"),
        ("#q1", "$NPN"),
        ("#q2", "$PNP"),
        ("#m1", "$NMOS_4TERM"),
        ("#m2", "$PMOS_4TERM"),
        ("#j1", "$NJFET"),
        ("#j2", "$PJFET"),
        ("#rp1", "$RESISTOR"),
        ("#rel1", "$VOLTAGE_CONTROLLED_SWITCH"),
        ("#t1", "$TRANSFORMER"),
        // the letter M would draw a MOS in SPICE; Spectre goes by the master
        ("#M3", "$RESISTOR"),
    ];
    for (file, want) in [("05-native-spice.sp", &spice[..]), ("06-native-spectre.scs", &spectre[..])] {
        let got = symbols(&case(file));
        let want: Vec<(String, String)> = want.iter().map(|(i, s)| (i.to_string(), s.to_string())).collect();
        assert_eq!(got, want, "{file}");
    }
}

#[test]
fn a_subcircuit_instance_named_i0_is_no_current_source() {
    // in a Spectre netlist of 0.3 as Virtuoso writes it, I0 and I1 call a subcircuit: the rule
    // package of SPICE, which would draw them as current sources by their letter, is dropped (16.3)
    let got = symbols(&case("07-virtuoso-netlist.scs"));
    let of = |id: &str| got.iter().find(|(i, _)| i == id).map(|(_, s)| s.as_str()).unwrap();
    assert_eq!((of("#I0"), of("#I1")), ("(viewer default)", "(viewer default)"));
    assert_eq!((of("#I2"), of("#V0")), ("$CURRENT_SOURCE", "$VOLTAGE_SOURCE"));
}

#[test]
fn a_relay_of_a_model_card_goes_by_its_type() {
    // the Spectre adapter reads switch, relay and transformer natively: an instance of a relay's
    // model card goes by its model type, sw, as S of SPICE does; a switch of three throws is still
    // drawn as $SWITCH, its pins on t0 and t1 (Annex C)
    let text = "// t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\")}\nmodel rl relay\n\
                K1 (a b c d) rl\nS1 (a b c d) switch position=3\nT1 (a b c d) transformer\n\
                // @RNL { #K1 @PLACE(0, 0); #S1 @PLACE(10, 0); #T1 @PLACE(20, 0) }\n";
    let files = MemoryProvider::new().with("t.scs", text);
    let r = read(&files, "t.scs", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = &r.model.levels[0];
    let got: Vec<(&str, &str)> = lv
        .items
        .iter()
        .filter(|i| i.kind == "instance")
        .map(|i| (i.id.as_str(), lv.symbols[i.symbol].name.as_str()))
        .collect();
    assert_eq!(got, [("#K1", "$VOLTAGE_CONTROLLED_SWITCH"), ("#S1", "$SWITCH"), ("#T1", "$TRANSFORMER")]);
}

#[test]
fn every_package_of_the_series_names_what_it_uses_through_its_references() {
    // without any standard package in effect, each package of the series checked on its own sees
    // only itself and what it references (16.2), and finds every name it uses
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/standard/0.3");
    let mut checked = 0;
    for f in std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()) {
        let options = Options { std: false, ..Options::default() };
        let r = interpret_file(&f, None, &FsProvider::new(&[]), &options).unwrap();
        assert!(errors(&r).is_empty(), "{}: {:?}", f.display(), errors(&r));
        checked += 1;
    }
    assert_eq!(checked, 17);
}

#[test]
fn the_fill_puts_no_port_tag_on_ground_and_supply_nets() {
    // the plan's PK-1: a port that is a ground or supply net has its stubs and no port tag; the
    // grounds of a ground net other than 0 show its name. PK-2: a stub gives only its least length
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.subckt amp in out vdd vss\nM1 out in vss vss nch\n\
                R1 vdd out 10k\n* @RNL { &vdd @ADD-TO $SUPPLY_NET_SET; &vss @ADD-TO $GROUND_NET_SET }\n.ends\n\
                X1 a b vdd 0 amp\nV1 vdd 0 1.8\n.model nch nmos\n.end\n";
    let files = MemoryProvider::new().with("t.sp", text);
    let r = read(&files, "t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let amp = level(&r, "amp");
    let usages = |def: &str| -> Vec<(String, String)> {
        amp.objects
            .iter()
            .filter(|o| o.definition.as_deref() == Some(def))
            .map(|o| {
                let at = o.attach.iter().map(|(_, a)| format!("{a:?}")).collect::<String>();
                (at, o.symbol.as_ref().map(|s| s.name.clone()).unwrap_or_default())
            })
            .collect()
    };
    let ports: Vec<String> = usages("$IO_PORT").into_iter().map(|(at, _)| at).collect();
    assert_eq!(ports, ["Object(\"&in\")", "Object(\"&out\")"]);
    let grounds = usages("$GROUND");
    assert_eq!(grounds.len(), 2, "{grounds:?}");
    assert!(grounds.iter().all(|(at, s)| at.contains("&vss") && s == "$NAMED_GROUND_SYMBOL"), "{grounds:?}");
    let top = r.model.levels.iter().find(|l| l.title == "t.sp").unwrap();
    let on_zero = top.objects.iter().filter(|o| o.definition.as_deref() == Some("$GROUND"));
    assert!(
        on_zero.clone().count() > 0 && on_zero.into_iter().all(|o| o.symbol.as_ref().unwrap().name == "$GROUND_SYMBOL")
    );
    // the stubs' distances have a least value and no greatest
    let r = read(&files, "t.sp", &Options::solver());
    let distances: Vec<(Option<f64>, Option<f64>)> = r
        .problems
        .iter()
        .flat_map(|p| &p.facts)
        .filter_map(|f| match &f.kind {
            FactKind::Distance { min, max, .. } if !f.chain.is_empty() => Some((*min, *max)),
            _ => None,
        })
        .collect();
    assert!(!distances.is_empty() && distances.iter().all(|d| *d == (Some(2.0), None)), "{distances:?}");
}

#[test]
fn a_reading_can_ask_for_a_version_of_the_standard_package() {
    // the tool's default version (16.3), whatever the language version of the file: a file of 0.3
    // read with the series 0.2, and a file of 0.2 with the series 0.3, both rule packages and all
    let with = |v: &str| Options { std_version: Some(v.into()), ..Options::default() };
    let files = MemoryProvider::new().with("t.sp", &netlist("0.3", "", ""));
    let r = read(&files, "t.sp", &with("0.2"));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&root(level(&r, "t.sp"))), SERIES_02);
    let files = MemoryProvider::new().with("t.sp", &netlist("0.2", "", ""));
    let r = read(&files, "t.sp", &with("0.3"));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&root(level(&r, "t.sp"))), SERIES_03);
    // a version that a header record pins still comes first
    let pinned = netlist("0.3", "@USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")", "");
    let r = read(&MemoryProvider::new().with("t.sp", &pinned), "t.sp", &with("0.2"));
    assert!(root(level(&r, "t.sp")).iter().all(|(_, v)| *v == "0.3"));
    // a version that there is not: once said, and no standard package
    let files = MemoryProvider::new().with("t.sp", &netlist("0.3", "", ""));
    let r = read(&files, "t.sp", &with("9.9"));
    let e = errors(&r);
    assert!(e.len() == 1 && e[0].contains("rnl-std 9.9 that the reading asks for cannot be found"), "{e:?}");
    assert!(root(level(&r, "t.sp")).is_empty());
    // a file that pins a version of it, or drops it with an inline package of its name, does not
    // need the version asked for: nothing is said, and the version pinned is in effect (16.3)
    for (header, series) in
        [("@USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")", true), ("@USE-PACKAGE(name=\"rnl-std\") {}", false)]
    {
        let files = MemoryProvider::new().with("t.sp", &netlist("0.3", header, ""));
        let r = read(&files, "t.sp", &with("9.9"));
        assert!(errors(&r).is_empty(), "{header}: {:?}", errors(&r));
        // the series pinned, without the rule package of the other dialect, which a file of 0.3
        // drops (16.3)
        let at_root = root(level(&r, "t.sp"));
        if series {
            assert!(!at_root.is_empty() && at_root.iter().all(|(_, v)| *v == "0.3"), "{at_root:?}");
        }
    }
    // one that does not pin it, which includes one that does, needs it: said once
    let child = "* c\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\") }\n\
                 .subckt cell a\nR1 a 0 1k\n.ends\n";
    let files =
        MemoryProvider::new().with("t.sp", &netlist("0.3", "", ".include c.inc\nX1 a cell\n")).with("c.inc", child);
    let e = errors(&read(&files, "t.sp", &with("9.9")));
    assert!(e.len() == 1 && e[0].contains("rnl-std 9.9 that the reading asks for cannot be found"), "{e:?}");
}

#[test]
fn a_name_of_the_series_0_2_is_told_its_name_in_the_series_0_3() {
    let record = "* @RNL {\n*   $GND @ATTACH &a;\n*   #R1 @SET-SYMBOL $RES;\n*   #R1 @SET-SYMBOL $OPAMP;\n\
                  *   #R1 @SET-SYMBOL $GROUND\n* }\n";
    let files = MemoryProvider::new().with("t.sp", &netlist("0.3", "", record));
    let e = errors(&read(&files, "t.sp", &Options::default()));
    let has = |s: &str| e.iter().any(|m| m.contains(s));
    assert!(
        has("$GND cannot be found: the standard series 0.3 calls it $GROUND; write that, or pin the series 0.2"),
        "{e:?}"
    );
    assert!(has("$RES cannot be found: the standard series 0.3 calls it $RESISTOR"), "{e:?}");
    assert!(has("$OPAMP cannot be found: the standard series 0.3 has it in rnl-std-functional"), "{e:?}");
    assert!(
        has("not $GROUND (11.3): in the standard series 0.3 $GROUND is a definition, and $GROUND_SYMBOL the symbol"),
        "{e:?}"
    );
    // pinning the series 0.2 makes them names that are found
    let pinned = netlist("0.3", "@USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")", record);
    let e = errors(&read(&MemoryProvider::new().with("t.sp", &pinned), "t.sp", &Options::default()));
    assert!(!e.iter().any(|m| m.contains("cannot be found") || m.contains("series 0.3")), "{e:?}");
    // without the series 0.3 in effect, nothing is said of it
    let e = errors(&read(&files, "t.sp", &Options { std: false, ..Options::default() }));
    assert!(e.iter().any(|m| m.contains("$GND cannot be found (6.6)")), "{e:?}");
    assert!(!e.iter().any(|m| m.contains("series 0.3")), "{e:?}");
}

/// The transistors of the tests of the semantic definitions: a pair, two tails, and the four of a
/// mirror; and R2 to R4 of a chain.
const PARTS: &str = ".model nch nmos level=1\n.model pch pmos level=1\nM1 o1 g1 t 0 nch\nM2 o2 g2 t 0 nch\n\
                     M5 t b 0 0 nch\nM8 t b 0 0 nch\nM3 d3 d3 vdd vdd pch\nM4 d4 d3 vdd vdd pch\n\
                     M6 d6 d3 vdd vdd pch\nM9 d3 d3 vdd vdd pch\nR2 a m 1k\nR3 m n 1k\nR4 n 0 1k\n";

/// A netlist of 0.3 whose header record writes `header` after its version, with PARTS and a
/// record of `statements`.
fn text_with_record(header: &str, statements: &[&str]) -> String {
    let lines: Vec<String> = statements.iter().map(|s| format!("*   {s}")).collect();
    netlist("0.3", header, &format!("{PARTS}* @RNL {{\n{}\n* }}\n", lines.join(";\n")))
}

/// That netlist, without a header beyond its version, read with the problems of a solver.
fn with_record(statements: &[&str]) -> Interpretation {
    read(&MemoryProvider::new().with("t.sp", &text_with_record("", statements)), "t.sp", &Options::solver())
}

/// The usages of `definition` in the problem of t.sp, each as what fills its roles, as the text
/// writes it, role by role in the definition's order; "" for a set role left empty.
fn fills(r: &Interpretation, definition: &str) -> Vec<Vec<String>> {
    let p = r.problems.iter().find(|p| p.title == "t.sp").unwrap();
    p.usages
        .iter()
        .filter(|u| u.definition == definition)
        .map(|u| u.roles.iter().map(|f| format!("{}={}", f.role, f.written.clone().unwrap_or_default())).collect())
        .collect()
}

fn problem(r: &Interpretation) -> &LevelProblem {
    r.problems.iter().find(|p| p.title == "t.sp").unwrap()
}

#[test]
fn a_differential_pair_has_a_tail_that_may_be_left_empty() {
    // tail is a set role, so it may be left empty and takes several tails (9.4)
    let r = with_record(&[
        "#M1(in_p) #M2(in_n) @AS $DIFFERENTIAL_PAIR",
        "#M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR",
        "#M1 #M2 @AS $DIFFERENTIAL_PAIR",
        "#M1 #M2 #M5 @AS $DIFFERENTIAL_PAIR",
        "#M1(in_p) #M2(in_n) {#M5 #M8}(tail) @AS $DIFFERENTIAL_PAIR",
    ]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let pairs = fills(&r, "$DIFFERENTIAL_PAIR");
    assert!(pairs.iter().all(|u| u[..2] == ["in_p=#M1", "in_n=#M2"]), "{pairs:?}");
    let tails: Vec<&str> = pairs.iter().map(|u| u[2].as_str()).collect();
    assert_eq!(tails, ["tail={}", "tail={#M5}", "tail={}", "tail={#M5}", "tail={#M5 #M8}"]);
}

#[test]
fn the_series_puts_the_tail_of_each_differential_pair_under_its_middle_as_a_low_default() {
    // for each differential pair, a rule of rnl-std-layout makes a usage of its own %TAIL_CENTER,
    // of priority low (14.8): a DISTRIBUTE of in_p, the tails as one set counted by its centre
    // (14.1), and in_n. The file's own facts come first
    let r = with_record(&[
        "#M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR",
        "#M1 #M2 @AS $DIFFERENTIAL_PAIR",
        "#M1(in_p) #M2(in_n) {#M5 #M8}(tail) @AS $DIFFERENTIAL_PAIR",
        "#M5 @PLACE(20, -10)",
        "{#M1 #M2} @ALIGN(\"y\")",
    ]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let p = problem(&r);
    let from_layout =
        |f: &&rnl_runtime::problem::ProblemFact| f.source.as_ref().is_some_and(|s| s.file == "rnl-std-layout.rnl");
    fn written(p: &LevelProblem, m: &PointRef) -> String {
        match m {
            PointRef::Origin(k) => p.objects[*k].label.clone(),
            PointRef::Centre(ms) => format!("{{{}}}", ms.iter().map(|m| written(p, m)).collect::<Vec<_>>().join(" ")),
            other => format!("{other:?}"),
        }
    }
    let mut low: Vec<Vec<String>> = p
        .facts
        .iter()
        .filter(from_layout)
        .map(|f| {
            assert_eq!(f.priority, Priority::Low, "{}", f.label);
            let made = &p.usages[f.chain[0]];
            assert!(made.generated && made.priority == Some(Priority::Low), "{made:?}");
            let FactKind::Distribute { members, .. } = &f.kind else { panic!("{:?}", f.kind) };
            members.iter().map(|m| written(p, m)).collect()
        })
        .collect();
    low.sort();
    // an empty set is no member (14.1): without a tail, two members, which hold by themselves (14.4)
    assert_eq!(low, [vec!["#M1", "#M2"], vec!["#M1", "{#M5 #M8}", "#M2"], vec!["#M1", "{#M5}", "#M2"]]);
    // and no point of its own
    assert!(p.objects.iter().all(|o| o.kind != ObjectKind::Point), "{:?}", p.objects);
    // in the order of resolution of 8.5, after every fact of the file's own
    let first = p.facts.iter().position(|f| from_layout(&f)).unwrap();
    assert!(p.facts[..first].iter().all(|f| f.priority == Priority::Normal), "{:?}", p.facts);
    assert!(p.facts[first..].iter().all(|f| from_layout(&f)), "{:?}", p.facts);
}

#[test]
fn a_tail_placed_elsewhere_lets_go_of_the_low_middle() {
    // the pair and its tail placed so that the tail is not under the middle: the solver discards
    // a fact of the low centering, and lists what it conflicts with only with --report (8.5)
    let text = text_with_record(
        "",
        &[
            "#M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR",
            "#M1 @PLACE(0, 0); #M2 @PLACE(10, 0); #M5 @PLACE(20, -10)",
        ],
    );
    let files = MemoryProvider::new().with("/w/t.sp", &text);
    let options = SolveOptions { solver: SolverOptions::without_time_limit(), ..SolveOptions::default() };
    let run = solve(Path::new("/w/t.sp"), &files, &options).unwrap();
    let lines = report(&run.levels[0], false);
    let discarded: Vec<&String> = lines.iter().filter(|l| l.trim_start().starts_with("discarded [")).collect();
    assert!(!discarded.is_empty(), "{lines:#?}");
    for l in &discarded {
        assert!(l.contains("(low, rnl-std-layout.rnl:") && l.contains("--report lists what it conflicts with"), "{l}");
    }
    let all = report(&run.levels[0], true);
    assert!(all.iter().any(|l| l.contains("kept [") && l.contains("the x of #M5 = 20 (normal")), "{all:#?}");
}

#[test]
fn a_solve_puts_the_tails_under_the_middle_and_leaves_nothing_unsolved() {
    // three pairs, nothing placed: one with a tail, one without, one with two tails
    let body = ".model nch nmos level=1\nM1 o1 g1 t 0 nch\nM2 o2 g2 t 0 nch\nM5 t b 0 0 nch\n\
                M3 p1 h1 u 0 nch\nM4 p2 h2 u 0 nch\nM6 q1 k1 w 0 nch\nM7 q2 k2 w 0 nch\nM8 w c 0 0 nch\n\
                M10 w c 0 0 nch\n* @RNL {\n*   #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR;\n\
                *   #M3(in_p) #M4(in_n) @AS $DIFFERENTIAL_PAIR;\n\
                *   #M6(in_p) #M7(in_n) {#M8 #M10}(tail) @AS $DIFFERENTIAL_PAIR\n* }\n";
    let files = MemoryProvider::new().with("/w/t.sp", &netlist("0.3", "", body));
    let options = SolveOptions { solver: SolverOptions::without_time_limit(), ..SolveOptions::default() };
    let run = solve(Path::new("/w/t.sp"), &files, &options).unwrap();
    let lv = &run.levels[0];
    assert!(lv.written.unsolved.is_empty(), "{:?}", lv.written.unsolved);
    let lines = report(lv, true);
    assert!(!lines.iter().any(|l| l.contains("discarded [") || l.contains("unsolved")), "{lines:#?}");
    // read again: every constraint holds, the one of the pair without a tail among them
    let solved = MemoryProvider::new().with("/w/t.sp", &run.files[0].text);
    let r = read(&solved, "/w/t.sp", &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let status = rnl_view::status_lines(&r);
    assert!(status.iter().all(|l| !l.contains("unsolved") && !l.contains("dropped")), "{status:?}");
    let x = |id: &str| level(&r, "t.sp").objects.iter().find(|o| o.id == id).and_then(|o| o.at).unwrap().0;
    // the tail's origin in the middle of the pair's, and the centre of two tails
    assert!((x("#M5") - (x("#M1") + x("#M2")) / 2.0).abs() < 1e-6, "{}", run.files[0].text);
    let (lo, hi) = (x("#M8").min(x("#M10")), x("#M8").max(x("#M10")));
    assert!(((lo + hi) / 2.0 - (x("#M6") + x("#M7")) / 2.0).abs() < 1e-6, "{}", run.files[0].text);
}

#[test]
fn a_current_mirror_is_one_usage_whatever_its_transistors() {
    // ref and out are set roles: one usage of each mirror (9.4)
    let r = with_record(&[
        "#M3 #M4 @AS $CURRENT_MIRROR",
        "{#M3 #M9}(ref) {#M4 #M6}(out) @AS $CURRENT_MIRROR",
        "{#M3 #M9} {#M4 #M6} @AS $CURRENT_MIRROR",
        "#M4(out) @AS $CURRENT_MIRROR",
    ]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(
        fills(&r, "$CURRENT_MIRROR"),
        [
            ["ref={#M3}", "out={#M4}"],
            ["ref={#M3 #M9}", "out={#M4 #M6}"],
            ["ref={#M3 #M9}", "out={#M4 #M6}"],
            ["ref={}", "out={#M4}"],
        ]
    );
    let e = errors(&with_record(&["#M3 #M4 #M6 @AS $CURRENT_MIRROR"]));
    assert!(e.iter().any(|m| m.contains("$CURRENT_MIRROR has 2 roles, 3 objects are written (9.4)")), "{e:?}");
    let e = errors(&with_record(&["#M3(ref) #M4(out) #M6(out) @AS $CURRENT_MIRROR"]));
    assert!(e.iter().any(|m| m.contains("a role is labelled only once (4.2 rule 6)")), "{e:?}");
}

#[test]
fn a_chain_takes_its_chain_as_a_set() {
    let r = with_record(&["#R1 {#R2 #R3} #R4 @AS $CHAIN"]);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(fills(&r, "$CHAIN"), [["source=#R1", "chain={#R2 #R3}", "load=#R4"]]);
    // the role of the series 0.2 is told its name in the series 0.3
    let e = errors(&with_record(&["#R1(source) #R2(series) #R4(load) @AS $CHAIN"]));
    assert!(
        e.iter().any(|m| m.contains(
            "the ordinary definition $CHAIN has no role series (9.4); the standard series 0.3 calls it chain (16.3)"
        )),
        "{e:?}"
    );
}

#[test]
fn a_definition_that_the_series_0_3_left_out_is_told_what_to_write() {
    let e = errors(&with_record(&[
        "#M1 #M2 #M5 @AS $DIFFPAIR_TAIL",
        "#M1 #M5 @AS $INPUT_STAGE",
        "#M1 #M2 #M5 @AS $TAIL_CENTER",
    ]));
    let has = |s: &str| e.iter().any(|m| m.contains(s));
    assert!(
        has("$DIFFPAIR_TAIL cannot be found: the tail is the role tail of $DIFFERENTIAL_PAIR in the standard series 0.3, which may be left empty; write that, or pin the series 0.2"),
        "{e:?}"
    );
    assert!(
        has("$INPUT_STAGE cannot be found: the standard series 0.3 has no input stage; write the tail into the role tail of the differential pair, as in #M1(in_p) #M2(in_n) #M5(tail) @AS $DIFFERENTIAL_PAIR"),
        "{e:?}"
    );
    assert!(
        has("$TAIL_CENTER cannot be found: the standard series 0.3 puts the tail of each differential pair under the middle of the pair itself, as a low default (14.8)"),
        "{e:?}"
    );
    // pinning the series 0.2 makes them definitions that are found
    let pinned = text_with_record(
        "@USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")",
        &[
            "#M1 #M2 #M5 @AS $DIFFPAIR_TAIL",
            "%dp = #M1 #M2 @AS $DIFFPAIR",
            "%dp #M5 @AS $INPUT_STAGE",
            "#M1 #M2 #M5 @AS $TAIL_CENTER",
        ],
    );
    let e = errors(&read(&MemoryProvider::new().with("t.sp", &pinned), "t.sp", &Options::default()));
    assert!(e.is_empty(), "{e:?}");
}

#[test]
fn the_chinese_aliases_of_the_series_0_3_name_its_symbols_definitions_and_roles() {
    // rnl-zh 0.3, which a file loads with USE-ALIASES (16.5): the standard words as rnl-zh 0.2 has
    // them, and the names of the series 0.3 and of the roles of its definitions
    let text = text_with_record(
        "@USE-ALIASES(name=\"rnl-zh\", version=\"0.3\")",
        &[
            "#R2 @使用符号 $\"电阻\"",
            "#M5 @使用符号 $\"四端子NMOS\"",
            "#M1(正输入) #M2(负输入) #M5(尾管) @视作 $\"差分对\"",
            "{#M3 #M9}(参考管) {#M4 #M6}(输出管) @视作 $\"电流镜\"",
            "#R1(源) {#R2 #R3}(链条) #R4(负载) @视作 $\"信号链\"",
        ],
    );
    let r = read(&MemoryProvider::new().with("t.sp", &text), "t.sp", &Options::solver());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(fills(&r, "$DIFFERENTIAL_PAIR"), [["in_p=#M1", "in_n=#M2", "tail={#M5}"]]);
    assert_eq!(fills(&r, "$CURRENT_MIRROR"), [["ref={#M3 #M9}", "out={#M4 #M6}"]]);
    assert_eq!(fills(&r, "$CHAIN"), [["source=#R1", "chain={#R2 #R3}", "load=#R4"]]);
    let lv = level(&r, "t.sp");
    let symbol =
        |id: &str| lv.objects.iter().find(|o| o.id == id).and_then(|o| o.symbol.as_ref()).unwrap().name.clone();
    assert_eq!((symbol("#R2"), symbol("#M5")), ("$RESISTOR".to_string(), "$NMOS_4TERM".to_string()));
    // the aliases of the definitions that the series 0.3 left out are not in it
    let gone =
        text_with_record("@USE-ALIASES(name=\"rnl-zh\", version=\"0.3\")", &["#M1 #M2 #M5 @视作 $\"带尾管差分对\""]);
    let e = errors(&read(&MemoryProvider::new().with("t.sp", &gone), "t.sp", &Options::default()));
    assert!(e.iter().any(|m| m.contains("带尾管差分对")), "{e:?}");
}
