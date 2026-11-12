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
