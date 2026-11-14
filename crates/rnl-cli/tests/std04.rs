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
*   # @SELECT-NET(port=true) => @REMOVE-FROM $PORT_FILL_SET;
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

#[test]
fn a_bundle_pin_left_to_the_fill_gets_one_label_for_the_bus() {
    // A cell repeated twice as a bundle (11.8): its pin a is on a<1> and a<0>, a bus pin; y and vss
    // are one net for both members. The fill gives each unwired pin one net label, and the label
    // of the bus pin shows the whole bus (11.8, 13.6).
    let text = r#"* bundle pins left to the fill
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
* }
.subckt unit a y vss
R1 a y 1k
R2 y vss 1k
.ends unit
XI0<1> a<1> y vss unit
XI0<0> a<0> y vss unit
* @RNL {
*   $UNIT_SYMBOL = @DEFINE-SYMBOL {
*     @RECTANGLE {(-2, -2) (2, 2)};
*     (-2, 1) @PIN(terminal="a", direction="left");
*     (2, 1) @PIN(terminal="y", direction="right");
*     (0, -2) @PIN(terminal="vss", direction="down");
*   };
*   #XI0<1:0> @BUNDLE => @SET-SYMBOL $UNIT_SYMBOL => @PLACE(0, 0);
* }
.end
"#;
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(by_severity(&r, Severity::Error).is_empty(), "{:?}", by_severity(&r, Severity::Error));
    assert!(by_severity(&r, Severity::Warning).is_empty(), "{:?}", by_severity(&r, Severity::Warning));
    let lv = level(&r);
    // one label for each pin of the bundle, the bus pin a included; a label keeps the pin it is
    // given, so that it is on all the nets of the bus pin
    let mut pins: Vec<String> = lv
        .objects
        .iter()
        .filter(|o| o.definition.as_deref() == Some("$NET_LABEL"))
        .map(|o| match o.attach.as_slice() {
            [(role, rnl_runtime::model::Attach::Object(pin))] if role == "p" => pin.clone(),
            other => panic!("{other:?}"),
        })
        .collect();
    pins.sort();
    assert_eq!(pins.len(), 3, "{pins:?}");
    pins.dedup();
    assert_eq!(pins.len(), 3, "{pins:?}");
    assert!(pins.iter().all(|p| p.starts_with("pin(")), "{pins:?}");
}

#[test]
fn a_fill_label_on_a_pin_stands_for_the_net_of_the_pin() {
    // A net label of the fill holds the pin it is on (its role p takes the pin); its text still
    // stands for the net, so that a viewer highlights it with the net and a click on it selects
    // the net as well as the label (12.1: the pin of the label is on the net of the pin).
    let text = r#"* a fill label on a pin
* @RNL(type="header") {
*   @VERSION("0.4");
*   @USE-PACKAGE(name="rnl-std-all", version="0.4");
* }
R1 a 0 1k
* @RNL {
*   #R1 @PLACE(0, 0)
* }
* @RNL(type="generated", priority="high") {
*   %ground_stub_R1_n = $GROUND_STUB @ATTACH #R1::n; %ground_stub_R1_n @PLACE(0, 0);
*   %ground_stub_R1_n::g @PLACE(0, -4);
*   %net_label_stub_R1_p = $NET_LABEL_STUB @ATTACH #R1::p; %net_label_stub_R1_p @PLACE(0, 0);
*   %net_label_stub_R1_p::l @PLACE(0, 3) => @ORIENT("R90")
* }
.end
"#;
    let files = MemoryProvider::new().with("t.sp", text);
    let doc = rnl_view::build("t.sp", &files, &rnl_view::BuildOptions::default()).unwrap();
    let level = &doc.levels[0];
    let label = level
        .hits
        .iter()
        .find(|h| h.kind == "label" && h.objects.iter().any(|&o| doc.objects[o].kind == "usage"))
        .unwrap_or_else(|| panic!("no label of a usage among {:?}", level.hits));
    let kinds: Vec<(&str, &str)> =
        label.objects.iter().map(|&o| (doc.objects[o].kind, doc.objects[o].name.as_str())).collect();
    assert!(kinds.contains(&("net", "a")), "{kinds:?}");
}

/// A netlist of 0.4 with the series and one RNL record, `body`.
fn with_std04(devices: &str, body: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\")}}\n\
         .model nch nmos\n.model pch pmos\n.subckt cell d g s\n.ends cell\n{devices}* @RNL {{\n*   {body}\n* }}\n.end\n"
    )
}

#[test]
fn n_and_p_structures_assert_the_polarity_of_known_types() {
    let pairs = "M1 x inp tail 0 nch W=2u L=1u\nM2 y inn tail 0 nch W=2u L=1u\nM3 a inp ptail ptail pch W=2u L=1u\n\
                 M4 b inn ptail ptail pch W=2u L=1u\nM5 tail bias 0 0 nch W=4u L=1u\nM6 ptail pbias vdd vdd pch W=4u L=1u\n\
                 X1 c inp t cell\nX2 e inn t cell\nX3 t bias 0 cell\n";
    let violations = |body: &str| {
        let r = read(&MemoryProvider::new().with("t.sp", &with_std04(pairs, body)), &Options::default());
        assert!(by_severity(&r, Severity::Error).is_empty(), "{body}: {:?}", by_severity(&r, Severity::Error));
        by_severity(&r, Severity::Violation)
    };
    let polarity = |v: &[String], words: &str| v.iter().filter(|v| v.contains(words)).count();
    // the right polarity: no violation of polarity
    assert_eq!(polarity(&violations("#M1(left) #M2(right) #M5(tail) @AS $DIFFERENTIAL_PAIR_N"), "-type"), 0);
    assert_eq!(polarity(&violations("#M3(left) #M4(right) #M6(tail) @AS $DIFFERENTIAL_PAIR_P"), "-type"), 0);
    // the wrong one: one violation, which names the polarity
    let wrong = violations("#M3(left) #M4(right) #M6(tail) @AS $DIFFERENTIAL_PAIR_N");
    assert_eq!(polarity(&wrong, "n-type"), 1, "{wrong:?}");
    let wrong = violations("#M1(ref) #M2(out) @AS $CURRENT_MIRROR_P");
    assert_eq!(polarity(&wrong, "p-type"), 1, "{wrong:?}");
    // devices whose type is not known, such as subcircuit instances of a process library, are not checked
    let unknown = violations("#X1(left) #X2(right) #X3(tail) @AS $DIFFERENTIAL_PAIR_N");
    assert!(unknown.iter().all(|v| !v.contains("n-type")), "{unknown:?}");
}

#[test]
fn the_mirrored_layout_is_named_as_a_layout() {
    let devices = "M1 x inp tail 0 nch W=2u L=1u\nM2 y inn tail 0 nch W=2u L=1u\n";
    let r = read(
        &MemoryProvider::new()
            .with("t.sp", &with_std04(devices, "%ax = (_, _); #M1(left) #M2(right) %ax(axis) @AS $MIRRORED_LAYOUT")),
        &Options::default(),
    );
    assert!(by_severity(&r, Severity::Error).is_empty(), "{:?}", by_severity(&r, Severity::Error));
    let old = read(
        &MemoryProvider::new()
            .with("t.sp", &with_std04(devices, "%ax = (_, _); #M1(left) #M2(right) %ax(axis) @AS $MIRRORED")),
        &Options::default(),
    );
    assert!(!by_severity(&old, Severity::Error).is_empty(), "the name of 0.3 is gone");
}

#[test]
fn signal_and_power_flows_take_sources_and_loads_and_the_bias_circuit_its_members() {
    let devices =
        "R1 a b 1k\nR2 b c 1k\nC2 c 0 1p\nM1 d c 0 0 nch W=2u L=1u\nIb vdd bias 1u\nM6 bias bias 0 0 nch W=2u L=1u\n";
    let read04 =
        |body: &str| read(&MemoryProvider::new().with("t.sp", &with_std04(devices, body)), &Options::default());
    // the number of the members of each set role, in the order of the roles
    let roles = |r: &Interpretation, id: &str| -> Vec<(String, usize)> {
        let lv = level(r);
        let o = lv.objects.iter().find(|o| o.id == id).unwrap_or_else(|| panic!("no {id}"));
        o.attach
            .iter()
            .map(|(role, attach)| {
                let n = match attach {
                    rnl_runtime::model::Attach::Set(members) => members.len(),
                    _ => 1,
                };
                (role.clone(), n)
            })
            .collect()
    };
    let named = |pairs: &[(&str, usize)]| pairs.iter().map(|(r, n)| (r.to_string(), *n)).collect::<Vec<_>>();
    // without roles marked, the whole chain is the path, as with $FLOW before
    let r = read04("%rc = {#R2 #C2} @GROUP; %f = {#R1 {%rc} #M1} @AS $SIGNAL_FLOW");
    assert!(by_severity(&r, Severity::Error).is_empty(), "{:?}", by_severity(&r, Severity::Error));
    assert_eq!(roles(&r, "%f"), named(&[("path", 4), ("source", 0), ("load", 0)]));
    // sources and loads take nets and terminals as they are
    let r = read04(
        "%f = &a(source) {#R1 #R2}(path) {#C2 &d}(load) @AS $SIGNAL_FLOW; \
         %p = &vdd(source) #Ib(path) #M6:1(load) @AS $POWER_FLOW",
    );
    assert!(by_severity(&r, Severity::Error).is_empty(), "{:?}", by_severity(&r, Severity::Error));
    assert_eq!(roles(&r, "%f"), named(&[("path", 2), ("source", 1), ("load", 2)]));
    assert_eq!(roles(&r, "%p"), named(&[("path", 1), ("source", 1), ("load", 1)]));
    // the bias circuit has its members only
    let r = read04("%b = {#Ib #M6} @AS $BIAS_CIRCUIT");
    assert!(by_severity(&r, Severity::Error).is_empty(), "{:?}", by_severity(&r, Severity::Error));
    // $SOURCE and $LOAD are gone
    for old in ["#R1 @AS $SOURCE", "#C2 @AS $LOAD", "{#R1 #M1} @AS $FLOW"] {
        assert!(!by_severity(&read04(old), Severity::Error).is_empty(), "{old}");
    }
}

#[test]
fn port_fill_puts_each_level_in_a_column_of_its_own() {
    // The fill's port tags and instances are selected with recursive=true; each level is evaluated
    // on its own (10.1, 10.3), so a level's column is kept left of its own instances only.
    let text = "* two levels with ports\n\
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n\
.subckt blk a b\nR1 a b 1k\n.ends\n\
.subckt top in out\nX1 in mid blk\nR2 mid out 1k\n.ends\n\
X0 n1 n2 top\n.end\n";
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let count = |title: &str, kind: &str| {
        let level = r.model.levels.iter().find(|l| l.title == title).unwrap();
        level.constraints.iter().filter(|c| c.kind == kind && c.priority == "low").count()
    };
    let tags = |title: &str| {
        let level = r.model.levels.iter().find(|l| l.title == title).unwrap();
        level.objects.iter().filter(|o| o.definition.as_deref() == Some("$IO_PORT")).count()
    };
    assert_eq!((tags("blk"), tags("top")), (2, 2));
    // a DISTANCE for each pair of a tag and an instance of the same level: 2 x 1 and 2 x 2
    assert_eq!(count("blk", "DISTANCE"), 2);
    assert_eq!(count("top", "DISTANCE"), 4);
}

/// A level of one subcircuit `cell` with the standard series 0.4 and `body` in its record.
fn pieces_file(ports: &str, netlist: &str, body: &str) -> String {
    format!(
        "* pieces\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }}\n\
.model nch nmos\n.subckt cell {ports}\n{netlist}* @RNL {{\n{body}* }}\n.ends\n.end\n"
    )
}

/// The pins that the usages of `definition` in the level `cell` are attached to, as their first role
/// is written, sorted.
fn attached(r: &Interpretation, definition: &str) -> Vec<String> {
    let level = r.model.levels.iter().find(|l| l.title == "cell").unwrap();
    let mut pins: Vec<String> = level
        .objects
        .iter()
        .filter(|o| o.definition.as_deref() == Some(definition))
        .map(|o| format!("{:?}", o.attach[0].1))
        .collect();
    pins.sort();
    pins
}

fn read_pieces(text: &str) -> Interpretation {
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    r
}

/// Whether one of `pins` names the instance `name`.
fn on(pins: &[String], name: &str) -> bool {
    pins.iter().any(|p| p.contains(&format!("(#{name},")))
}

#[test]
fn fill_names_each_wired_piece_of_a_net_drawn_in_pieces() {
    // Net b is drawn in two pieces by two ROUTE-OBJECT trees, neither with a name: the first pin
    // of each gets a $PIECE_LABEL. On the ground net 0 the wired piece {R7:n, R8:n} gets a ground
    // stub, as R9:n, unwired, does already (10.2, 16.3).
    let r = read_pieces(&pieces_file(
        "",
        "R1 a b 1k\nR2 b c 1k\nR3 b d 1k\nR4 b e 1k\nR7 h 0 1k\nR8 k 0 1k\nR9 m 0 1k\n",
        "*   {#R1:2 #R2:1} @ROUTE-OBJECT;\n*   {#R3:1 #R4:1} @ROUTE-OBJECT;\n\
*   {#R7:2 #R8:2} @ROUTE-OBJECT(priority=\"high\");\n",
    ));
    let pieces = attached(&r, "$PIECE_LABEL");
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    assert!(on(&pieces, "R1") && on(&pieces, "R3"), "{pieces:?}");
    let grounds = attached(&r, "$GROUND_STUB");
    assert_eq!(grounds.len(), 2, "{grounds:?}");
    assert!(on(&grounds, "R7") && on(&grounds, "R9"), "{grounds:?}");
}

#[test]
fn a_port_net_in_one_wired_piece_gets_a_piece_label_beside_its_tag() {
    // The port net p is in one piece, so it is not split; its tag comes after the pieces are
    // fixed, and the piece gets a $PIECE_LABEL too (16.3).
    let r = read_pieces(&pieces_file("p", "R5 p f 1k\nR6 p g 1k\n", "*   {#R5:1 #R6:1} @ROUTE-OBJECT;\n"));
    let pieces = attached(&r, "$PIECE_LABEL");
    assert_eq!(pieces.len(), 1, "{pieces:?}");
    assert!(on(&pieces, "R5"), "{pieces:?}");
    assert_eq!(attached(&r, "$IO_PORT").len(), 1);
}

#[test]
fn a_piece_of_a_ground_net_led_by_a_bulk_pin_gets_a_piece_label() {
    // The bulk pins of drawn four-terminal MOS get net labels rather than ground stubs; the piece
    // {M1:b, M2:b} of the ground net 0, led by M1:b, gets a $PIECE_LABEL, and no ground stub.
    let r = read_pieces(&pieces_file(
        "",
        "M1 d1 g1 s1 0 nch\nM2 d2 g2 s2 0 nch\nR1 x 0 1k\n",
        "*   {#M1 #M2} @SET-SYMBOL $NMOS_4TERM;\n*   {#M1:b #M2:b} @ROUTE-OBJECT(priority=\"high\");\n",
    ));
    let pieces = attached(&r, "$PIECE_LABEL");
    assert_eq!(pieces.len(), 1, "{pieces:?}");
    assert!(on(&pieces, "M1"), "{pieces:?}");
    let grounds = attached(&r, "$GROUND_STUB");
    assert!(!on(&grounds, "M1") && !on(&grounds, "M2"), "{grounds:?}");
}

#[test]
fn a_piece_already_marked_gets_no_second_piece_label() {
    // Net b: a written $PIECE_LABEL on R1:n joins the first piece, which then starts with its
    // label's pin. Net c: a $NET_LABEL_STUB on R3:p, still wired by a written wire. Net p: an
    // $IO_PORT_STUB on R5:p, whose tag is a piece of its own. None of them gets one more; the
    // second piece of b does.
    let r = read_pieces(&pieces_file(
        "p",
        "R1 a b 1k\nR2 b a2 1k\nR7 b a3 1k\nR8 b a4 1k\nR3 c d 1k\nR4 c e 1k\nR9 c a5 1k\nR5 p f 1k\nR6 p g 1k\n",
        "*   {#R1:2 #R2:1} @ROUTE-OBJECT;\n*   {#R7:1 #R8:1} @ROUTE-OBJECT;\n*   $PIECE_LABEL @ATTACH #R1::n;\n\
*   &c @WIRE {#R3:1 #R4:1};\n*   $NET_LABEL_STUB @ATTACH #R3::p;\n\
*   {#R5:1 #R6:1} @ROUTE-OBJECT;\n*   $IO_PORT_STUB @ATTACH #R5::p;\n",
    ));
    let pieces = attached(&r, "$PIECE_LABEL");
    assert_eq!(pieces.len(), 2, "{pieces:?}");
    assert!(on(&pieces, "R1") && on(&pieces, "R7"), "{pieces:?}");
}

#[test]
fn a_file_takes_a_pin_out_of_the_piece_fill() {
    let r = read_pieces(&pieces_file(
        "",
        "R1 a b 1k\nR2 b c 1k\nR3 b d 1k\nR4 b e 1k\n",
        "*   {#R1:2 #R2:1} @ROUTE-OBJECT;\n*   {#R3:1 #R4:1} @ROUTE-OBJECT;\n*   #R1 @SELECT-PIN => @REMOVE-FROM $PIECE_FILL_SET;\n",
    ));
    let pieces = attached(&r, "$PIECE_LABEL");
    assert_eq!(pieces.len(), 1, "{pieces:?}");
    assert!(on(&pieces, "R3"), "{pieces:?}");
}

#[test]
fn piece_labels_written_back_by_a_solver_are_not_doubled_when_read_again() {
    // A solve writes the labels back as usages; read again, each joins its piece, which then starts
    // with the label's pin, a pin of a part the fill places itself: the fill adds none (10.2, 16.3).
    use rnl_cli::solve::{solve, SolveOptions};
    let text = pieces_file(
        "p",
        "R1 a b 1k\nR2 b c 1k\nR3 b d 1k\nR4 b e 1k\nR5 p f 1k\nR6 p g 1k\n",
        "*   {#R1:2 #R2:1} @ROUTE-OBJECT;\n*   {#R3:1 #R4:1} @ROUTE-OBJECT;\n*   {#R5:1 #R6:1} @ROUTE-OBJECT;\n",
    );
    let before = attached(&read_pieces(&text), "$PIECE_LABEL");
    assert_eq!(before.len(), 3, "{before:?}");
    let files = MemoryProvider::new().with("t.sp", &text);
    let solver = rnl_solver::SolverOptions::default().with_config(r#"limit="seed""#).unwrap();
    let run = solve(Path::new("t.sp"), &files, &SolveOptions { solver, ..Default::default() }).unwrap();
    assert_eq!(run.errors, 0);
    let written = &run.files[0].text;
    assert_eq!(written.matches("$PIECE_LABEL @ATTACH").count(), 3, "{written}");
    let options = Options { generated: true, ..Options::default() };
    let back = read(&MemoryProvider::new().with("t.sp", written), &options);
    assert!(errors(&back).is_empty(), "{:?}", errors(&back));
    assert_eq!(attached(&back, "$PIECE_LABEL"), before, "{written}");
}
