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
            (serde_json::json!("p1"), [-1.0, 2.0], Some("up")),
            (serde_json::json!("n1"), [-1.0, -2.0], Some("down")),
            (serde_json::json!("p2"), [1.0, 2.0], Some("up")),
            (serde_json::json!("n2"), [1.0, -2.0], Some("down")),
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
        "$OPAMP_INVERTED_SUPPLY",
        "$DIFFERENTIAL_OPAMP_SUPPLY",
        "$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY",
        "$DIFFERENTIAL_OPAMP_TWISTED_SUPPLY",
        "$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY",
        "$COMPARATOR_SUPPLY",
        "$COMPARATOR_INVERTED_SUPPLY",
        "$VOLTAGE_AMPLIFIER_SUPPLY",
        "$TRANSCONDUCTOR_SUPPLY",
        "$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY",
        "$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY",
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
    assert_eq!(pin_at("rst"), ([-2.0, -5.0], Some("down")));
    assert_eq!(pin_at("vp"), ([0.0, 4.0], Some("up")));
    assert_eq!(pin_at("vn"), ([0.0, -5.0], Some("down")));
}

#[test]
fn inverted_supply_symbols_swap_the_inputs_and_keep_the_supplies() {
    let all = rnl_view::catalog(
        &MemoryProvider::default(),
        &rnl_view::CatalogOptions { all_versions: true, ..Default::default() },
    );
    let pins = |name: &str| -> std::collections::BTreeMap<String, ([f64; 2], Option<&'static str>)> {
        let s = all
            .symbols
            .iter()
            .find(|s| s.version == "0.4" && s.symbol == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        s.pins
            .iter()
            .map(|p| (serde_json::to_value(&p.terminal).unwrap().as_str().unwrap().to_string(), (p.at, p.direction)))
            .collect()
    };
    // (the inverted symbol, its base, the terminals whose places it swaps)
    let cases = [
        ("$OPAMP_INVERTED_SUPPLY", "$OPAMP_SUPPLY", &[("inp", "inn")][..]),
        ("$COMPARATOR_INVERTED_SUPPLY", "$COMPARATOR_SUPPLY", &[("inp", "inn")][..]),
        ("$DIFFERENTIAL_OPAMP_INVERTED_SUPPLY", "$DIFFERENTIAL_OPAMP_SUPPLY", &[("inp", "inn"), ("outp", "outn")][..]),
        ("$DIFFERENTIAL_OPAMP_TWISTED_INVERTED_SUPPLY", "$DIFFERENTIAL_OPAMP_SUPPLY", &[("inp", "inn")][..]),
        ("$DIFFERENTIAL_TRANSCONDUCTOR_INVERTED_SUPPLY", "$DIFFERENTIAL_TRANSCONDUCTOR_SUPPLY", &[("inp", "inn")][..]),
    ];
    for (inverted, base, swapped) in cases {
        let (inv, mut want) = (pins(inverted), pins(base));
        for (a, b) in swapped {
            let (pa, pb) = (want[*a], want[*b]);
            want.insert(a.to_string(), pb);
            want.insert(b.to_string(), pa);
        }
        assert_eq!(inv, want, "{inverted}");
        // the inverting input above, the positive supply above as on the base symbol
        assert!(inv["inn"].0[1] > inv["inp"].0[1], "{inverted}");
        assert_eq!((inv["vp"], inv["vn"]), (want["vp"], want["vn"]), "{inverted}");
        assert_eq!(inv["vp"].1, Some("up"), "{inverted}");
    }
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
    // one column of the fill in each level with ports, none at the file level, which has none (9.4)
    let lists = |title: &str| {
        let level = r.model.levels.iter().find(|l| l.title == title).unwrap();
        level.objects.iter().filter(|o| o.definition.as_deref().is_some_and(|d| d.ends_with("port_column"))).count()
    };
    assert_eq!((lists("blk"), lists("top"), lists("t.sp")), (1, 1, 0));
    // a DISTANCE from the column's point to each instance of the same level: 1 and 2
    assert_eq!(count("blk", "DISTANCE"), 1);
    assert_eq!(count("top", "DISTANCE"), 2);
}

#[test]
fn a_written_port_list_leaves_the_fill_no_column_in_its_level() {
    // the file writes $PORT_LIST in blk: its tags, at R0, at least 6 units left of each instance of
    // the level, and no column of the fill there; top still gets the fill's column (12.4)
    let text = "* two levels with ports
* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }
.subckt blk a b
R1 a b 1k
R3 a b 1k
* @RNL { $PORT_LIST @ATTACH; }
.ends
.subckt top in out
X1 in mid blk
R2 mid out 1k
.ends
X0 n1 n2 top
.end
";
    let r = read(&MemoryProvider::new().with("t.sp", text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let level = |title: &str| r.model.levels.iter().find(|l| l.title == title).unwrap();
    let count = |title: &str, definition: &str| {
        level(title).objects.iter().filter(|o| o.definition.as_deref().is_some_and(|d| d.ends_with(definition))).count()
    };
    assert_eq!((count("blk", "$IO_PORT"), count("blk", "$PORT_LIST"), count("blk", "port_column")), (2, 1, 0));
    assert_eq!((count("top", "$IO_PORT"), count("top", "port_column")), (2, 1));
    let distances = level("blk").constraints.iter().filter(|c| c.kind == "DISTANCE" && c.priority == "low").count();
    assert_eq!(distances, 2, "from the list's point to R1 and R3");
}

#[test]
fn port_fill_of_many_ports_and_instances_stays_within_the_expansion_limit() {
    // 10 port tags and 103 instances: 103 facts from the column's point, where a fact for each tag
    // and instance would be 1030, beyond the limit of 1024 in one statement (RNL5004)
    let ports: Vec<String> = (0..10).map(|k| format!("p{k}")).collect();
    let mut text = format!(
        "* ten ports
* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }}
.subckt big {}
",
        ports.join(" ")
    );
    for k in 0..103 {
        text += &format!(
            "R{k} {} n{k} 1k
",
            ports[k % 10]
        );
    }
    text += ".ends big
.end
";
    let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
    let all: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(all.is_empty(), "{all:?}");
    let level = r.model.levels.iter().find(|l| l.title == "big").unwrap();
    let distances = level.constraints.iter().filter(|c| c.kind == "DISTANCE" && c.priority == "low").count();
    assert_eq!(distances, 103);
}

/// The net-label stubs of the level, by the pin each is attached to.
fn label_stub_pins(lv: &LevelView) -> Vec<String> {
    let stubs = lv.objects.iter().filter(|o| o.definition.as_deref() == Some("$NET_LABEL_STUB"));
    let mut pins: Vec<String> = stubs.flat_map(|o| o.attach.iter().map(|(_, a)| format!("{a:?}"))).collect();
    pins.sort();
    pins
}

#[test]
fn the_fill_stubs_a_pin_once_when_a_file_adds_it_again() {
    // the fill already labels the bulk pin and the unwired gate; adding them again changes nothing,
    // and a stub written back (as a solve writes it) is not repeated either
    let text = |written: &str| {
        format!(
            r#"* t
* @RNL(type="header") {{ @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }}
.model nch nmos level=1
M1 d g 0 b nch w=1u l=1u
R1 d 0 1k
* @RNL {{
*   {written}
*   # @SELECT-TERMINAL(terminal="b") => @SELECT-PIN => @ADD-TO $LABEL_FILL_SET;
*   # @SELECT-TERMINAL(terminal="g") => @SELECT-PIN => @ADD-TO $LABEL_FILL_SET
* }}
.end
"#
        )
    };
    let fresh = read(&MemoryProvider::new().with("t.sp", &text("")), &Options::solver());
    assert!(errors(&fresh).is_empty(), "{:?}", errors(&fresh));
    let pins = label_stub_pins(level(&fresh));
    let mut once = pins.clone();
    once.dedup();
    assert_eq!(pins, once, "a pin has two net-label stubs");
    assert_eq!(pins.len(), 4, "{pins:?}");
    let back = read(
        &MemoryProvider::new().with("t.sp", &text("%net_label_stub_M1_b = $NET_LABEL_STUB @ATTACH #M1::b;")),
        &Options::solver(),
    );
    assert!(errors(&back).is_empty(), "{:?}", errors(&back));
    assert_eq!(label_stub_pins(level(&back)), pins);
}

/// The low ALIGN facts in a level of two flat and two upright capacitors, after `body`: how many
/// there are and how many hold.
fn low_aligns(body: &str) -> (usize, usize) {
    let text = format!(
        r#"* t
* @RNL(type="header") {{ @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }}
C1 a b 1u
C2 b c 1u
C3 c 0 1u
C4 b 0 1u
R1 c 0 1k
* @RNL {{
*   {body}
*   #C1 @PLACE(0, 0) => @ORIENT("R90");
*   #C2 @PLACE(10, 0) => @ORIENT("R0");
*   #C3 @PLACE(20, 0) => @ORIENT("R0");
*   #C4 @PLACE(30, 0) => @ORIENT("R90")
* }}
.end
"#
    );
    let r = read(&MemoryProvider::new().with("t.sp", &text), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = level(&r);
    let aligns = lv.constraints.iter().filter(|c| c.kind == "ALIGN" && c.priority == "low");
    let ok = aligns.clone().filter(|c| c.status == "ok").count();
    (aligns.count(), ok)
}

#[test]
fn a_coupling_capacitor_places_nothing_and_a_decap_stands_in_one_column() {
    // C1 and C4 lie flat (R90), C2 and C3 stand upright (R0)
    let (without, without_ok) = low_aligns("#C1 #C2 #C3 #C4;");
    // $AC_COUPLE is meaning only: a coupling capacitor may lie or stand
    let (with, with_ok) = low_aligns("{#C1 #C2} @AS $AC_COUPLE;");
    assert_eq!((with, with_ok), (without, without_ok));
    // $DECAP aligns their pins in x, which holds for C3 and not for C4, and puts the usage's own
    // point in that column, which is left to a solver
    let (with, with_ok) = low_aligns("{#C3 #C4} @AS $DECAP;");
    assert_eq!((with, with_ok), (without + 4, without_ok + 1));
}

#[test]
fn the_degenerated_amplifiers_name_their_stage() {
    let devices = r#".model qn npn
M1 d g s 0 nch W=2u L=1u
RD vdd d 1k
RS s 0 1k
Q1 c b e qn
RC vdd c 1k
RE e 0 1k
"#;
    let read04 = |body: &str| {
        let r = read(&MemoryProvider::new().with("t.sp", &with_std04(devices, body)), &Options::default());
        assert!(by_severity(&r, Severity::Error).is_empty(), "{body}: {:?}", by_severity(&r, Severity::Error));
        r
    };
    let stage = |r: &Interpretation, definition: &str| {
        level(r).objects.iter().filter(|o| o.definition.as_deref() == Some(definition)).count()
    };
    // each applies its stage and the degenerated amplifier, without a violation on the right device
    let cs = read04("#M1(input) #RD(load) #RS(degenerator) @AS $DEGENERATED_COMMON_SOURCE_AMPLIFIER");
    assert!(by_severity(&cs, Severity::Violation).is_empty(), "{:?}", by_severity(&cs, Severity::Violation));
    assert_eq!(stage(&cs, "$COMMON_SOURCE"), 1);
    assert_eq!(stage(&cs, "$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER"), 1);
    let ce = read04("#Q1(input) #RC(load) #RE(degenerator) @AS $DEGENERATED_COMMON_EMITTER_AMPLIFIER");
    assert!(by_severity(&ce, Severity::Violation).is_empty(), "{:?}", by_severity(&ce, Severity::Violation));
    assert_eq!(stage(&ce, "$COMMON_EMITTER"), 1);
    assert_eq!(stage(&ce, "$DEGENERATED_TRANSCONDUCTOR_Z_AMPLIFIER"), 1);
    // the stage checks the device: a bipolar input to the common-source one is reported
    let wrong = read04("#Q1(input) #RC(load) #RE(degenerator) @AS $DEGENERATED_COMMON_SOURCE_AMPLIFIER");
    let v = by_severity(&wrong, Severity::Violation);
    assert_eq!(v.iter().filter(|v| v.contains("is a FET")).count(), 1, "{v:?}");
}

/// Whether the text of `key` on `object` shows, as the model of the file `net` has it.
fn shown(net: &str, object: &str, key: &str) -> bool {
    let files = MemoryProvider::new().with("/w/t.sp", net);
    let parts = rnl_view::Parts { drawing: false, host: false, model: true };
    let s = rnl_view::snapshot("/w/t.sp", &files, &rnl_view::BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(s).unwrap();
    let texts = s["levels"][0]["model"]["texts"].as_array().unwrap().clone();
    let text = texts.iter().find(|t| t["attach"] == object && t["key"] == key);
    let text = text.unwrap_or_else(|| panic!("no {key} on {object}: {texts:?}"));
    text.get("visible") != Some(&serde_json::json!(false))
}

#[test]
fn no_labels_hides_the_net_names_glyphs_draw_too() {
    let net = |no_labels: bool, own: &str| {
        let package = if no_labels { "; @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.4\")" } else { "" };
        format!(
            "* t
* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"){package} }}
             R1 a vdd 1k
* @RNL {{ #R1 @PLACE(0, 0) => @ORIENT(\"R90\"); $SUPPLY @ATTACH &vdd => @PLACE(0, 4) => %s;              $IO_PORT @ATTACH &a => @PLACE(-4, -2) => %p{own} }}
.end
"
        )
    };
    // without the package, every label shows
    let plain = net(false, "");
    assert!(shown(&plain, "#R1", "name") && shown(&plain, "%s", "netname") && shown(&plain, "%p", "netname"));
    // with it, the device's name and the glyphs' net names are hidden alike
    let hidden = net(true, "");
    assert!(!shown(&hidden, "#R1", "name"));
    assert!(!shown(&hidden, "%s", "netname"), "supply");
    assert!(!shown(&hidden, "%p", "netname"), "port tag");
    // a file's own net name shows when it says so
    let own = net(true, "; %p @TEXT(\"A\", key=\"netname\", textvisible=true)");
    assert!(shown(&own, "%p", "netname") && !shown(&own, "%s", "netname"));
}

#[test]
fn no_instance_labels_hides_the_labels_of_instances_only() {
    let net = |hide: bool, own: &str| {
        let package = if hide { "; @USE-PACKAGE(name=\"rnl-std-no-instance-labels\", version=\"0.4\")" } else { "" };
        format!(
            "* t
* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"){package} }}
             R1 a vdd 1k
* @RNL {{ #R1 @PLACE(0, 0) => @ORIENT(\"R90\"); $SUPPLY @ATTACH &vdd => @PLACE(0, 4) => %s;              $IO_PORT @ATTACH &a => @PLACE(-4, -2) => %p; #R1 @TEXT(\"note\", key=\"memo\"){own} }}
.end
"
        )
    };
    // without the package, every label shows
    let plain = net(false, "");
    assert!(shown(&plain, "#R1", "name") && shown(&plain, "#R1", "value"));
    // with it, the instance's name and value are hidden, the usages' net names and the file's own label stay
    let hidden = net(true, "");
    assert!(!shown(&hidden, "#R1", "name") && !shown(&hidden, "#R1", "value"));
    assert!(shown(&hidden, "%s", "netname"), "supply");
    assert!(shown(&hidden, "%p", "netname"), "port tag");
    assert!(shown(&hidden, "#R1", "memo"), "the file's own label");
    // a file shows one of them again when it says so
    let own = net(true, "; #R1 @TEXT(\"R1\", key=\"name\", textvisible=true)");
    assert!(shown(&own, "#R1", "name") && !shown(&own, "#R1", "value"));
}

#[test]
fn a_bus_pin_takes_a_rail_stub_only_when_all_its_nets_are_of_that_rail() {
    let read04 = |devices: &str, body: &str| {
        let r = read(&MemoryProvider::new().with("t.sp", &with_std04(devices, body)), &Options::default());
        assert!(by_severity(&r, Severity::Error).is_empty(), "{body}: {:?}", by_severity(&r, Severity::Error));
        r
    };
    let stubs = |r: &Interpretation, definition: &str| {
        level(r).objects.iter().filter(|o| o.definition.as_deref() == Some(definition)).count()
    };
    // the second pin of the bundle is on c and on the ground: a net label for the bus, no ground
    let mixed = read04("R1 a b 1k\nR2 b c 1k\nR3 c 0 1k\n", "{#R2 #R3} @BUNDLE;");
    assert_eq!((stubs(&mixed, "$NET_LABEL_STUB"), stubs(&mixed, "$GROUND_STUB")), (4, 0));
    // on a ground and a supply net, it is neither
    let rails = read04("R2 b 0 1k\nR3 c vdd 1k\n", "&vdd @ADD-TO $SUPPLY_NET_SET; {#R2 #R3} @BUNDLE;");
    assert_eq!(
        (stubs(&rails, "$NET_LABEL_STUB"), stubs(&rails, "$GROUND_STUB"), stubs(&rails, "$SUPPLY_STUB")),
        (2, 0, 0)
    );
    // both members on the ground: one net, so the pin takes a ground stub
    let ground = read04("R1 a b 1k\nR2 b 0 1k\nR3 c 0 1k\n", "{#R2 #R3} @BUNDLE;");
    assert_eq!((stubs(&ground, "$NET_LABEL_STUB"), stubs(&ground, "$GROUND_STUB")), (3, 1));
}

/// A transmission gate and an inverter side by side, with what `extra` adds to their record.
fn complementary_pairs(extra: &str) -> String {
    format!(
        r#"* own points of the complementary pairs
* @RNL(type="header") {{ @VERSION("0.4"); @USE-PACKAGE(name="rnl-std-all", version="0.4") }}
.model nch nmos level=1
.model pch pmos level=1
.subckt cell a b c vdd vss
MP1 a c b vdd pch
MN1 a c b vss nch
MP2 c b vdd vdd pch
MN2 c b vss vss nch
* @RNL {{
*   %tg = #MP1(pmos) #MN1(nmos) @AS $TRANSMISSION_GATE;
*   %inv = #MP2(pmos) #MN2(nmos) @AS $CMOS_INVERTER;
*   {{%tg %inv}} @ORDER("left-to-right");{extra}
* }}
.ends
.end
"#
    )
}

/// The record that a solve within a fixed work writes into the level `cell` of
/// [`complementary_pairs`]; the solve reports no error.
fn solved_pairs(extra: &str) -> String {
    let files = MemoryProvider::new().with("t.sp", &complementary_pairs(extra));
    let mut options =
        rnl_cli::solve::SolveOptions { solver: rnl_solver::SolverOptions::without_time_limit(), ..Default::default() };
    options.solver.run_limit = Some(rnl_solver::RunLimit::Work(100_000_000));
    options.solver.safety_seconds = None;
    let run = rnl_cli::solve::solve(Path::new("t.sp"), &files, &options).unwrap();
    assert_eq!(rnl_cli::solve::errors(&run), 0, "{:?}", rnl_cli::solve::lines(&run, true));
    run.levels.iter().find(|l| l.record.contains("%tg @PLACE")).expect("the level cell").record.clone()
}

/// The coordinates that a solved record places `name` at.
fn placed(record: &str, name: &str) -> (f64, f64) {
    let at = format!("{name} @PLACE(");
    let line = record.lines().find(|l| l.trim_start_matches(['*', ' ']).starts_with(&at)).expect(name);
    let (x, rest) = line.split_once(&at).unwrap().1.split_once(", ").unwrap();
    (x.parse().unwrap(), rest.split_once(')').unwrap().0.parse().unwrap())
}

/// A transmission gate and an inverter put their own point midway between their two
/// transistors, the PMOS above (libraries/standard/0.4, rnl-std-semantics).
#[test]
fn a_transmission_gate_and_an_inverter_hold_their_own_point_midway() {
    let record = solved_pairs("");
    for (usage, pmos, nmos) in [("%tg", "#MP1", "#MN1"), ("%inv", "#MP2", "#MN2")] {
        let (u, p, n) = (placed(&record, usage), placed(&record, pmos), placed(&record, nmos));
        assert_eq!(u, ((p.0 + n.0) / 2., (p.1 + n.1) / 2.), "{usage} midway between {pmos} and {nmos}:\n{record}");
        assert!(p.1 > n.1, "{pmos} above {nmos}:\n{record}");
    }
}

/// Their facts are low: a file that stacks the NMOS above the PMOS checks with no violation, and
/// the solve follows the file.
#[test]
fn a_file_may_stack_the_nmos_above_the_pmos_against_the_low_facts() {
    let extra = "\n*   {#MN1 #MP1} @ORDER(\"top-to-bottom\"); {#MN2 #MP2} @ORDER(\"top-to-bottom\");";
    let r = read(&MemoryProvider::new().with("t.sp", &complementary_pairs(extra)), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    for severity in [Severity::Warning, Severity::Violation] {
        assert!(by_severity(&r, severity).is_empty(), "{:?}", by_severity(&r, severity));
    }
    let record = solved_pairs(extra);
    for (pmos, nmos) in [("#MP1", "#MN1"), ("#MP2", "#MN2")] {
        assert!(placed(&record, nmos).1 > placed(&record, pmos).1, "{nmos} above {pmos}:\n{record}");
    }
}
