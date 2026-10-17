//! The standard series 0.3 (libraries/standard/0.3), bundled beside the series 0.2, and the
//! standard package that a file uses implicitly (16.3 of the language 0.3): the newest version
//! that the tool has of the standard series written in the language version of the file, so that
//! a file of 0.2 keeps the series 0.2 and a file of 0.3 uses the series 0.3; a version pinned in
//! the header record of the file, or of a file that includes it, comes first. A main file without
//! a header record is read in the tool's default version, 0.3 (16.1), and so uses the series 0.3.

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::diag::Severity;
use rnl_runtime::model::LevelView;
use rnl_runtime::problem::FactKind;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

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
    assert_eq!(checked, 16);
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
