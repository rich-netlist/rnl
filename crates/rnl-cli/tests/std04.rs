//! The draft language 0.4 and the standard series 0.4 (libraries/standard/0.4), bundled beside
//! the series 0.2 and 0.3. A file of 0.4 is read (16.1) and explicitly uses the series 0.4
//! (16.3). 0.4 is newer than the tool's default version, 0.3, so a
//! USE-PACKAGE that names no version finds a package written in 0.4 only in a file of 0.4 (16.2):
//! the files of 0.2 and 0.3 find what they found before the series 0.4 was bundled.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{Diag, Severity};
use rnl_runtime::model::LevelView;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(files: &MemoryProvider, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, files, options).unwrap()
}

/// The errors of the file itself, not of the packages it uses.
fn errors(r: &Interpretation) -> Vec<String> {
    let own = |d: &&Diag| d.severity == Severity::Error && d.loc.as_ref().is_none_or(|l| l.file == "t.sp");
    r.diagnostics().into_iter().filter(own).map(|d| r.sink.render(d)).collect()
}

fn level(r: &Interpretation) -> &LevelView {
    r.model.levels.iter().find(|l| l.title == "t.sp").unwrap()
}

/// The packages at the root of the level, in queue order, with their versions.
fn root(lv: &LevelView) -> Vec<(&str, &str)> {
    lv.packages.iter().filter(|p| p.at.is_none()).map(|p| (p.name.as_str(), p.version.as_str())).collect()
}

/// A netlist whose header record writes `header` after its version.
fn netlist(version: &str, header: &str) -> String {
    let header = if header.is_empty() { String::new() } else { format!("; {header}") };
    format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){header}}}\nR1 a 0 1k\n.end\n")
}

#[test]
fn a_file_of_0_4_uses_the_series_0_4_when_requested() {
    let without = read(&MemoryProvider::new().with("t.sp", &netlist("0.4", "")), &Options::default());
    assert!(root(level(&without)).is_empty());
    let r = read(
        &MemoryProvider::new().with("t.sp", &netlist("0.4", "@USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\")")),
        &Options::default(),
    );
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(r.model.language, "0.4");
    let at_root = root(level(&r));
    assert!(at_root.contains(&("rnl-std-all", "0.4")), "{at_root:?}");
    assert!(at_root.iter().all(|(_, v)| *v == "0.4"), "{at_root:?}");
    // the files of 0.2 and 0.3 keep their series
    for version in ["0.2", "0.3"] {
        let r = read(&MemoryProvider::new().with("t.sp", &netlist(version, "")), &Options::default());
        let at_root = root(level(&r));
        assert!(at_root.contains(&("rnl-std", version)), "{version}: {at_root:?}");
        assert!(at_root.iter().all(|(_, v)| *v == version), "{version}: {at_root:?}");
    }
}

#[test]
fn a_package_that_names_no_version_is_found_in_the_series_the_file_can_use() {
    // without the standard package, the one package at the root is the one the header names
    let options = Options { std: false, ..Options::default() };
    let rf = |version: &str, header: &str| {
        let r = read(&MemoryProvider::new().with("t.sp", &netlist(version, header)), &options);
        assert!(errors(&r).is_empty(), "{version} {header}: {:?}", errors(&r));
        root(level(&r)).iter().map(|(n, v)| format!("{n} {v}")).collect::<Vec<_>>()
    };
    let unnamed = "@USE-PACKAGE(name=\"rnl-std-rf\")";
    // The published 0.2/0.3 lookup stays; 0.4 uses its own series.
    for (version, want) in [("0.2", "0.3"), ("0.3", "0.3"), ("0.4", "0.4")] {
        assert_eq!(rf(version, unnamed), [format!("rnl-std-rf {want}")], "{version}");
    }
    // An explicit package version cannot cross the language boundary either.
    let named = "@USE-PACKAGE(name=\"rnl-std-rf\", version=\"0.4\")";
    for (version, header) in [("0.2", named), ("0.3", named)] {
        let r = read(&MemoryProvider::new().with("t.sp", &netlist(version, header)), &options);
        assert!(errors(&r).iter().any(|e| e.contains("RNL2308")), "{:?}", errors(&r));
        assert!(root(level(&r)).is_empty());
    }
}

#[test]
fn the_rules_that_0_3_changes_hold_in_0_4() {
    // `numbering`, a word that 0.3 adds (2.8), is a standard word in a file of 0.4 too, which no
    // alias may be spelled as (16.5)
    let options = Options { std: false, ..Options::default() };
    for (version, error) in [("0.2", false), ("0.3", true), ("0.4", true)] {
        let r =
            read(&MemoryProvider::new().with("t.sp", &netlist(version, "@ALIAS(\"numbering\", \"axis\")")), &options);
        let said = errors(&r).iter().any(|e| e.contains("the alias numbering is the name of a standard word (16.5)"));
        assert_eq!(said, error, "{version}: {:?}", errors(&r));
    }
    // T has terminal names by default (6.12)
    let t = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\nT1 a b c d z0=50 td=1n\n\
             * @RNL { #T1:p1 @TEXT(\"first port\") }\n.end\n";
    let r = read(&MemoryProvider::new().with("t.sp", t), &options);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
}

fn by_severity(r: &Interpretation, severity: Severity) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| r.sink.render(d)).collect()
}

#[test]
fn unknown_instance_types_give_no_structure_violations() {
    let text = r#"* type="*" guard on structure type assertions (10.1, 13.2-13.4)
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
*   @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4");
*   @USE-PACKAGE(name="rnl-std-spice", version="0.4");
*   @USE-PACKAGE(name="rnl-std-semantics", version="0.4");
* }
.model nch nmos
.subckt nfet d g s b
.ends
.subckt pfet d g s b
.ends

.subckt cell_unknown_type vdd vss in out d1 d2 d3
M1   d1  in  vss vss nch l=100n w=1u
XM1  d2  in  vss vss nfet l=100n w=1u
XM2  d3  in  vss vss nfet l=100n w=1u
XMP1 out in  in2 vdd pfet
M3   in2 inb out vss nch l=100n w=1u
XMP2 out in  vdd vdd pfet
M4   out in  vss vss nch l=100n w=1u
* @RNL {
*   &vss @ADD-TO $GROUND_NET_SET;
*   &vdd @ADD-TO $SUPPLY_NET_SET;
*   {#XM1 #XM2 #XMP1 #XMP2} @SET-SYMBOL(terminals=["d", "g", "s", "b"]) $NMOS_4TERM;
*   {#M1 #XM1} @AS $MATCHED;
*   {#XM1 #XM2} @AS $MATCHED_MOS;
*   #M1 @AS $COMMON_SOURCE;
*   #XM1 @AS $COMMON_SOURCE;
*   #XMP1(pmos) #M3(nmos) @AS $TRANSMISSION_GATE;
*   #XMP2(pmos) #M4(nmos) @AS $CMOS_INVERTER;
* }
.ends
.end
"#;
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(by_severity(&r, Severity::Warning).is_empty(), "{:?}", by_severity(&r, Severity::Warning));
    assert!(by_severity(&r, Severity::Violation).is_empty(), "{:?}", by_severity(&r, Severity::Violation));
}

#[test]
fn mismatched_known_instance_types_report_expected_structure_violations() {
    let text = r#"* known wrong-type devices still violate structure assertions (10.1, 13.2-13.4)
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
*   @USE-PACKAGE(name="rnl-std-semiconductors", version="0.4");
*   @USE-PACKAGE(name="rnl-std-spice", version="0.4");
*   @USE-PACKAGE(name="rnl-std-semantics", version="0.4");
* }
.model qnpn npn
.subckt nfet d g s b
.ends

.subckt cell_wrong_type a b vss
Q1  a b vss qnpn
XM2 a b vss vss nfet
Q2  a b vss qnpn
Q3  a b vss qnpn
* @RNL {
*   #XM2 @SET-SYMBOL(terminals=["d", "g", "s", "b"]) $NMOS_4TERM;
*   {#Q1 #Q2} @AS $MATCHED_MOS;
*   #Q2 @AS $COMMON_SOURCE;
*   #Q3(pmos) #XM2(nmos) @AS $TRANSMISSION_GATE;
* }
.ends
.end
"#;
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(by_severity(&r, Severity::Warning).is_empty(), "{:?}", by_severity(&r, Severity::Warning));
    let violations = by_severity(&r, Severity::Violation);
    assert_eq!(violations.len(), 3, "{violations:?}");
    assert!(
        violations.iter().any(|v| v.contains("@ASSERT-IN")
            && v.contains("$MATCHED_MOS")
            && v.contains("#Q1 and #Q2 are not members of mos_set")),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.contains("@ASSERT-IN")
            && v.contains("$COMMON_SOURCE")
            && v.contains("#Q2 is not members of fet_set")),
        "{violations:?}"
    );
    assert!(
        violations.iter().any(|v| v.contains("@ASSERT-INSTANCE")
            && v.contains("$TRANSMISSION_GATE")
            && v.contains("#Q3 has type \"npn\"")),
        "{violations:?}"
    );
}

#[test]
fn transformer_pins_follow_dotted_first_order() {
    let all = rnl_view::catalog(
        &MemoryProvider::default(),
        &rnl_view::CatalogOptions { all_versions: true, ..Default::default() },
    );
    let xfmr = all.symbols.iter().find(|s| s.version == "0.4" && s.symbol == "$TRANSFORMER").unwrap();
    let pins: Vec<_> =
        xfmr.pins.iter().map(|p| (serde_json::to_value(&p.terminal).unwrap(), p.at, p.direction)).collect();
    assert_eq!(
        pins,
        vec![
            (serde_json::json!("p1"), [2.0, 1.0], Some("right")),
            (serde_json::json!("n1"), [-2.0, 1.0], Some("left")),
            (serde_json::json!("p2"), [2.0, -1.0], Some("right")),
            (serde_json::json!("n2"), [-2.0, -1.0], Some("left")),
        ]
    );
}

#[test]
fn comment_attaches_to_any_kind_or_whole_level_without_drawing() {
    let text = r#"* non-drawn $COMMENT definition (rnl-std-basic 0.4)
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
* }
R1 a b 1k
R2 b 0 2k
* @RNL {
*   #R1 @PLACE(0, 4);
*   #R2 @PLACE(0, -4);
*   %w = &b @WIRE {#R1::n #R2::p};
*   %pt = (4, 0);
*   #R1 @AS(text="precision input resistor") $COMMENT;
*   #R1:p @AS(text="kelvin sense node") $COMMENT;
*   &b @AS(text="mid-rail divider node") $COMMENT;
*   %w @AS(text="short matched interconnect") $COMMENT;
*   %pt @AS(text="probe pad") $COMMENT;
*   {#R1 #R2} @AS(text="0.1% ratio pair") $COMMENT;
*   $COMMENT @ATTACH(text="sheet-level review comment");
* }
.end
"#;
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(by_severity(&r, Severity::Warning).is_empty(), "{:?}", by_severity(&r, Severity::Warning));
    assert!(by_severity(&r, Severity::Violation).is_empty(), "{:?}", by_severity(&r, Severity::Violation));
    let lv = level(&r);
    let comments: Vec<_> = lv.objects.iter().filter(|o| o.definition.as_deref() == Some("$COMMENT")).collect();
    assert_eq!(comments.len(), 7, "{comments:?}");
    assert!(comments.iter().all(|o| o.symbol.is_none() && o.symbol_view.is_none() && !o.usage_has_drawing));
    assert!(comments.iter().any(|o| {
        o.attach == vec![("target".into(), rnl_runtime::model::Attach::Set(vec![]))]
            && o.params.iter().any(|(k, v)| k == "text" && v.str().is_some_and(|s| s.contains("sheet-level")))
    }));

    let zh = r#"* alias-zh $备注
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
*   @USE-ALIASES(name="alias-zh", version="0.4");
* }
R1 a 0 1k
* @RNL {
*   #R1(对象) @视作(文本="低温漂电阻") $"备注";
*   $"备注" @附着(文本="整图说明");
* }
.end
"#;
    let r_zh = read(&MemoryProvider::new().with("t.sp", zh), &Options::default());
    assert!(errors(&r_zh).is_empty(), "{:?}", errors(&r_zh));
    assert_eq!(level(&r_zh).objects.iter().filter(|o| o.definition.as_deref() == Some("$COMMENT")).count(), 2);
}

#[test]
fn no_connect_retains_bus_bit_terminal_and_suppresses_fill() {
    let text = r#"* $NO_CONNECT on bus bit terminal (12.6)
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-style", version="0.4");
*   @USE-PACKAGE(name="rnl-std-basic", version="0.4");
*   @USE-PACKAGE(name="rnl-std-fill", version="0.4");
* }
.subckt reg4 clk d3 d2 d1 d0 q0 q1
.ends
.subckt top clk q0
XU1 clk nc_d3 bus_d2 bus_d1 bus_d0 q0 nc_q1 reg4
* @RNL {
*   $REG4_SYM = @DEFINE-SYMBOL(box=[-4, -3, 4, 3]) {
*     %clk = (-4, -1) @PIN(terminal="clk", direction="left");
*     %d = (-4, 1) @PIN(terminal=["d3", "d2", "d1", "d0"], direction="left");
*     %q0 = (4, 1) @PIN(terminal="q0", direction="right");
*     %q1 = (4, -1) @PIN(terminal="q1", direction="right");
*     @RECTANGLE {(-3, -3) (3, 3)};
*     @LINE {(-4, -1) (-3, -1)};
*     @LINE {(-4, 1) (-3, 1)};
*     @LINE {(3, 1) (4, 1)};
*     @LINE {(3, -1) (4, -1)};
*   };
*   #XU1 @SET-SYMBOL $REG4_SYM => @PLACE(0, 0);
*   &clk @WIRE {(-6, -1) #XU1::clk};
*   {&bus_d2 &bus_d1 &bus_d0} @WIRE {(-8, 1) #XU1::d};
*   &q0 @WIRE {#XU1::q0 (6, 1)};
*   %nc_msb = #XU1:d3 @AS $NO_CONNECT;
*   %nc_q1 = #XU1:q1 @AS $NO_CONNECT;
* }
.ends
.end
"#;
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::solver());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(by_severity(&r, Severity::Warning).is_empty(), "{:?}", by_severity(&r, Severity::Warning));
    assert!(by_severity(&r, Severity::Violation).is_empty(), "{:?}", by_severity(&r, Severity::Violation));
    let sub = r.model.levels.iter().find(|l| l.title.contains("top")).unwrap();
    let nc_msb = sub.objects.iter().find(|o| o.id == "%nc_msb").unwrap();
    assert_eq!(nc_msb.attach, vec![("t".into(), rnl_runtime::model::Attach::Object("#XU1:2".into()))]);
    assert_eq!(
        sub.constraints.iter().filter(|c| c.kind == "ALIGN" && c.status == "unsolved").count(),
        4,
        "{:?}",
        sub.constraints
    );
    // nc_q1 suppresses automatic $NET_LABEL_STUB fill on the unwired scalar pin q1
    assert!(!sub.objects.iter().any(|o| o.definition.as_deref() == Some("$NET_LABEL_STUB")), "{:?}", sub.objects);
}

#[test]
fn supply_variants_cover_functional_and_logic_on_integer_grid() {
    let all = rnl_view::catalog(
        &MemoryProvider::default(),
        &rnl_view::CatalogOptions { all_versions: true, ..Default::default() },
    );
    let syms_04: Vec<_> = all.symbols.iter().filter(|s| s.version == "0.4").collect();
    assert!(!syms_04.iter().any(|s| s.symbol == "$OPAMP_5TERM" || s.symbol == "$COMPARATOR_5TERM"));

    let expected_func = [
        "$OPAMP_SUPPLY",
        "$DIFFERENTIAL_OPAMP_SUPPLY",
        "$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY",
        "$COMPARATOR_SUPPLY",
        "$VOLTAGE_AMPLIFIER_SUPPLY",
        "$TRANSCONDUCTOR_SUPPLY",
        "$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY",
        "$ADC_SUPPLY",
        "$DAC_SUPPLY",
    ];
    let expected_logic = [
        "$AND_2INPUT_SUPPLY",
        "$AND_3INPUT_SUPPLY",
        "$AND_4INPUT_SUPPLY",
        "$NAND_2INPUT_SUPPLY",
        "$NAND_3INPUT_SUPPLY",
        "$NAND_4INPUT_SUPPLY",
        "$OR_2INPUT_SUPPLY",
        "$OR_3INPUT_SUPPLY",
        "$OR_4INPUT_SUPPLY",
        "$NOR_2INPUT_SUPPLY",
        "$NOR_3INPUT_SUPPLY",
        "$NOR_4INPUT_SUPPLY",
        "$XOR_2INPUT_SUPPLY",
        "$XOR_3INPUT_SUPPLY",
        "$XOR_4INPUT_SUPPLY",
        "$XNOR_2INPUT_SUPPLY",
        "$XNOR_3INPUT_SUPPLY",
        "$XNOR_4INPUT_SUPPLY",
        "$BUFFER_SUPPLY",
        "$INVERTER_SUPPLY",
        "$DELAY_CELL_SUPPLY",
        "$D_FLIP_FLOP_SUPPLY",
        "$D_FLIP_FLOP_Q_ONLY_SUPPLY",
        "$D_FLIP_FLOP_WITH_RESET_SUPPLY",
    ];

    for (pkg, names) in [("rnl-std-functional", &expected_func[..]), ("rnl-std-logic", &expected_logic[..])] {
        for &name in names {
            let s = syms_04
                .iter()
                .find(|s| s.symbol == name && s.package == pkg)
                .unwrap_or_else(|| panic!("missing {name} in {pkg} 0.4"));
            let [x1, y1, x2, y2] = s.bbox;
            for p in &s.pins {
                assert_eq!(p.at[0].fract(), 0.0, "{name} pin {:?} off x grid", p);
                assert_eq!(p.at[1].fract(), 0.0, "{name} pin {:?} off y grid", p);
                assert!(p.at[0] >= x1 && p.at[0] <= x2 && p.at[1] >= y1 && p.at[1] <= y2, "{name} pin outside box");
            }
            let n = s.pins.len();
            assert!(n >= 4, "{name} should have at least 4 pins");
            assert_eq!(
                (serde_json::to_value(&s.pins[n - 2].terminal).unwrap(), s.pins[n - 2].direction),
                (serde_json::json!("vp"), Some("up")),
                "{name} vp"
            );
            assert_eq!(
                (serde_json::to_value(&s.pins[n - 1].terminal).unwrap(), s.pins[n - 1].direction),
                (serde_json::json!("vn"), Some("down")),
                "{name} vn"
            );
        }
    }

    let dffr = syms_04.iter().find(|s| s.symbol == "$D_FLIP_FLOP_WITH_RESET_SUPPLY").unwrap();
    let pin_at = |term: &str| {
        dffr.pins
            .iter()
            .find(|p| serde_json::to_value(&p.terminal).unwrap() == serde_json::json!(term))
            .map(|p| (p.at, p.direction))
            .unwrap()
    };
    assert_eq!(pin_at("rst"), ([-1.0, -5.0], Some("down")));
    assert_eq!(pin_at("vp"), ([1.0, 4.0], Some("up")));
    assert_eq!(pin_at("vn"), ([1.0, -5.0], Some("down")));
}

#[test]
fn expanded_0_4_symbols_and_chinese_aliases_load_cleanly() {
    let all = rnl_view::catalog(
        &MemoryProvider::default(),
        &rnl_view::CatalogOptions { all_versions: true, ..Default::default() },
    );
    let syms_04: Vec<_> = all.symbols.iter().filter(|s| s.version == "0.4").collect();

    let expected: &[(&str, &str, &[&str])] = &[
        ("rnl-std-basic", "$POTENTIOMETER", &["n1", "nw", "n2"]),
        ("rnl-std-basic", "$PHOTORESISTOR", &["p", "n"]),
        ("rnl-std-basic", "$POLARIZED_CAPACITOR", &["p", "n"]),
        ("rnl-std-basic", "$CRYSTAL", &["n1", "n2"]),
        ("rnl-std-basic", "$SPEAKER", &["p", "n"]),
        ("rnl-std-basic", "$SINE_VOLTAGE_SOURCE", &["p", "n"]),
        ("rnl-std-basic", "$PULSE_VOLTAGE_SOURCE", &["p", "n"]),
        ("rnl-std-basic", "$MICROPHONE", &["p", "n"]),
        ("rnl-std-basic", "$AMMETER", &["p", "n"]),
        ("rnl-std-semiconductors", "$LED", &["a", "k"]),
        ("rnl-std-semiconductors", "$SCHOTTKY_DIODE", &["a", "k"]),
        ("rnl-std-semiconductors", "$SHOCKLEY_DIODE", &["a", "k"]),
        ("rnl-std-semiconductors", "$PUT", &["a", "g", "k"]),
        ("rnl-std-semiconductors", "$UJT", &["e", "b1", "b2"]),
        ("rnl-std-functional", "$CHOPPER_SYMBOL", &["inp", "inn", "outp", "outn"]),
    ];

    for &(pkg, sym, terms) in expected {
        let s = syms_04
            .iter()
            .find(|s| s.package == pkg && s.symbol == sym)
            .unwrap_or_else(|| panic!("missing {sym} in {pkg} 0.4"));
        let got_terms: Vec<String> =
            s.pins.iter().map(|p| serde_json::to_value(&p.terminal).unwrap().as_str().unwrap().to_string()).collect();
        let want_terms: Vec<String> = terms.iter().map(|t| (*t).to_string()).collect();
        assert_eq!(got_terms, want_terms, "{sym} terminals");
        for p in &s.pins {
            assert_eq!(p.at[0].fract(), 0.0, "{sym} pin {:?} off x grid", p);
            assert_eq!(p.at[1].fract(), 0.0, "{sym} pin {:?} off y grid", p);
        }
    }

    // Verify pin-side box alignment across every symbol in 0.4
    for s in &syms_04 {
        let [x1, y1, x2, y2] = s.bbox;
        for p in &s.pins {
            assert!(
                p.at[0] >= x1 && p.at[0] <= x2 && p.at[1] >= y1 && p.at[1] <= y2,
                "{} ({}) pin {:?} outside {:?}",
                s.symbol,
                s.package,
                p,
                s.bbox
            );
        }
        let min_px = s.pins.iter().map(|p| p.at[0]).reduce(f64::min);
        let max_px = s.pins.iter().map(|p| p.at[0]).reduce(f64::max);
        let min_py = s.pins.iter().map(|p| p.at[1]).reduce(f64::min);
        let max_py = s.pins.iter().map(|p| p.at[1]).reduce(f64::max);
        let left = s.pins.iter().filter(|p| p.direction == Some("left")).map(|p| p.at[0]).reduce(f64::min);
        let down = s.pins.iter().filter(|p| p.direction == Some("down")).map(|p| p.at[1]).reduce(f64::min);
        let right = s.pins.iter().filter(|p| p.direction == Some("right")).map(|p| p.at[0]).reduce(f64::max);
        let up = s.pins.iter().filter(|p| p.direction == Some("up")).map(|p| p.at[1]).reduce(f64::max);
        if let Some(v) = left {
            assert_eq!(x1, v.min(min_px.unwrap_or(v)), "{} left box edge", s.symbol);
        }
        if let Some(v) = down {
            assert_eq!(y1, v.min(min_py.unwrap_or(v)), "{} down box edge", s.symbol);
        }
        if let Some(v) = right {
            assert_eq!(x2, v.max(max_px.unwrap_or(v)), "{} right box edge", s.symbol);
        }
        if let Some(v) = up {
            assert_eq!(y2, v.max(max_py.unwrap_or(v)), "{} up box edge", s.symbol);
        }
    }

    let zh = r#"* alias-zh coverage for expanded 0.4 symbols
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @USE-ALIASES(name="alias-zh", version="0.4");
* }
.model dmod d
.subckt xtal2 n1 n2
.ends
.subckt chop4 inp inn outp outn
.ends
.subckt op5 inp inn out vp vn
.ends
.subckt dffr7 d ck rst q qb vp vn
.ends
Vpulse clk 0 1.8V
Dled a 0 dmod
Xxtal x1 x2 xtal2
Xchop ip in op on chop4
Xop ip in vo vdd 0 op5
Xdff d clk rst q qb vdd 0 dffr7
* @RNL {
*   #Vpulse @使用符号 $"脉冲电压源";
*   #Dled @使用符号 $"发光二极管";
*   #Xxtal @使用符号(端子数=["n1", "n2"]) $"晶振";
*   #Xchop @使用符号(端子数=["inp", "inn", "outp", "outn"]) $"斩波器符号";
*   #Xop @使用符号(端子数=["inp", "inn", "out", "vp", "vn"]) $"带电源运放";
*   #Xdff @使用符号(端子数=["d", "ck", "rst", "q", "qb", "vp", "vn"]) $"带电源带复位D触发器";
* }
.end
"#;
    let r_zh = read(&MemoryProvider::new().with("t.sp", zh), &Options::default());
    assert!(errors(&r_zh).is_empty(), "{:?}", errors(&r_zh));
    assert!(by_severity(&r_zh, Severity::Warning).is_empty(), "{:?}", by_severity(&r_zh, Severity::Warning));
    assert!(by_severity(&r_zh, Severity::Violation).is_empty(), "{:?}", by_severity(&r_zh, Severity::Violation));
}

#[test]
fn optional_45deg_package_is_isolated_from_rnl_std_all_and_stays_on_grid() {
    let without_45 = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\nR1 a 0 1k\n* @RNL { #R1 @SET-SYMBOL $RESISTOR_45 }\n.end\n";
    let r_without = read(&MemoryProvider::new().with("t.sp", without_45), &Options::default());
    assert!(!level(&r_without).packages.iter().any(|p| p.name == "rnl-std-45deg"));
    assert!(!errors(&r_without).is_empty(), "rnl-std-all must not implicitly load rnl-std-45deg");

    let all = rnl_view::catalog(
        &MemoryProvider::default(),
        &rnl_view::CatalogOptions { all_versions: true, ..Default::default() },
    );
    let syms_45: Vec<_> = all.symbols.iter().filter(|s| s.package == "rnl-std-45deg" && s.version == "0.4").collect();
    let expected_45 = [
        "$RESISTOR_45",
        "$CAPACITOR_45",
        "$INDUCTOR_45",
        "$SWITCH_45",
        "$VOLTAGE_SOURCE_45",
        "$CURRENT_SOURCE_45",
        "$DIODE_45",
        "$ZENER_DIODE_45",
        "$SCHOTTKY_DIODE_45",
        "$LED_45",
        "$NMOS_45",
        "$NMOS_4TERM_45",
        "$PMOS_45",
        "$PMOS_4TERM_45",
        "$NPN_45",
        "$PNP_45",
        "$ARROW_45_SYMBOL",
    ];
    let got_names: Vec<&str> = syms_45.iter().map(|s| s.symbol.as_str()).collect();
    assert_eq!(got_names, expected_45);

    let defs_45: Vec<&str> = all
        .definitions
        .iter()
        .filter(|d| d.package == "rnl-std-45deg" && d.version == "0.4")
        .map(|d| d.definition.as_str())
        .collect();
    assert_eq!(defs_45, ["$ARROW_45", "$CURRENT_ARROW_45"]);

    let transform = |[x, y]: [f64; 2], code: &str| -> [f64; 2] {
        match code {
            "R0" => [x, y],
            "R90" => [-y, x],
            "R180" => [-x, -y],
            "R270" => [y, -x],
            "MY" => [-x, y],
            "MX" => [x, -y],
            "MYR90" => [-y, -x],
            "MXR90" => [y, x],
            _ => unreachable!(),
        }
    };
    for s in &syms_45 {
        for p in &s.pins {
            for code in ["R0", "R90", "R180", "R270", "MY", "MX", "MYR90", "MXR90"] {
                let [tx, ty] = transform(p.at, code);
                assert_eq!(tx.fract(), 0.0, "{} {} pin {:?} off x grid", s.symbol, code, p);
                assert_eq!(ty.fract(), 0.0, "{} {} pin {:?} off y grid", s.symbol, code, p);
            }
        }
    }

    let zh_45 = r#"* rnl-std-45deg + alias-zh 0.4
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
*   @USE-PACKAGE(name="rnl-std-45deg", version="0.4");
*   @USE-ALIASES(name="alias-zh", version="0.4");
* }
.model nch nmos
R1 a b 1k
M1 d g s 0 nch
* @RNL {
*   #R1 @使用符号 $"45度电阻" => @位于(0, 0) => @朝向("R90");
*   #M1 @使用符号 $"45度四端子NMOS" => @位于(8, 0);
*   %pt = (4, 4);
*   %pt(挂点) @视作 $"45度箭头";
*   #R1:p(挂点) @视作 $"45度电流箭头";
* }
.end
"#;
    let r_zh = read(&MemoryProvider::new().with("t.sp", zh_45), &Options::default());
    assert!(errors(&r_zh).is_empty(), "{:?}", errors(&r_zh));
    assert!(by_severity(&r_zh, Severity::Warning).is_empty(), "{:?}", by_severity(&r_zh, Severity::Warning));
    assert!(by_severity(&r_zh, Severity::Violation).is_empty(), "{:?}", by_severity(&r_zh, Severity::Violation));
}
