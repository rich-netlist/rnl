//! DEVICE names and types are independent, source-file-scoped items (6.12).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(path: &str, text: &str, child: Option<&str>) -> Interpretation {
    let mut files = MemoryProvider::new().with(path, text);
    if let Some(child) = child {
        files = files.with("child.sp", child);
    }
    interpret_file(Path::new(path), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn spice(version: &str, header: &str, host: &str, body: &str) -> String {
    let header = header.replace('\n', " ");
    let body = body.replace('\n', " ");
    format!("* devices\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"); {header} }}\n{host}\n* @RNL {{ {body} }}\n.end\n")
}

fn ok(r: &Interpretation) {
    let bad: Vec<_> = r
        .diagnostics()
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Violation))
        .map(|d| r.sink.render(d))
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
}

fn names(r: &Interpretation, name: &str) -> Vec<Option<String>> {
    r.model
        .levels
        .iter()
        .find_map(|lv| {
            r.doc.levels[lv.level].instances.iter().position(|i| i.name == name).map(|i| lv.terminal_names[i].clone())
        })
        .unwrap()
}

fn named(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|v| Some((*v).to_string())).collect()
}

#[test]
fn names_and_types_override_separately_and_all_consumers_see_the_type() {
    let text = spice(
        "0.4",
        r#"
        @DEVICE(model="cell*", terminals=["d","g","s","b"], type="pmos");
        @DEVICE(name="X*", type="nmos");
        @DEVICE(name="X1", terminals=["out","in","src","bulk"])
    "#,
        "X1 d g s 0 cell_a\nX2 d g s 0 cell_b",
        r#"
        $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
        # @SELECT-INSTANCE(type="nmos") => @SET-SYMBOL $S;
        {#X1 #X2} @ASSERT-INSTANCE(type="nmos", same=["type"]);
        #X1:out @ASSERT-TERMINAL(type="nmos");
        #X1 @PLACE(0,0) => @TEXT("${type}", key="kind"); #X2 @PLACE(10,0)
    "#,
    );
    let r = read("t.sp", &text, None);
    ok(&r);
    assert_eq!(names(&r, "X1"), named(&["out", "in", "src", "bulk"]));
    assert_eq!(names(&r, "X2"), named(&["d", "g", "s", "b"]));
    assert_eq!(r.model.levels[0].objects.iter().filter(|i| i.symbol.is_some()).count(), 2);
    assert!(r.model.levels[0]
        .texts
        .iter()
        .any(|t| t.lines.iter().any(|line| rnl_runtime::markup::plain(line) == "nmos")));
}

#[test]
fn incomplete_device_statements_fail_without_erasing_earlier_items() {
    for invalid in [r#"@DEVICE(type="pmos")"#, r#"@DEVICE(model="cell")"#, r#"@DEVICE()"#] {
        let text = spice("0.4", &format!(r#"@DEVICE(name="X*", terminals=["a","b"]); {invalid}"#), "X1 p n cell", "");
        let r = read("t.sp", &text, None);
        assert!(r.diagnostics().iter().any(|d| d.code == codes::DEVICE_MISSING_PARAMS));
        assert_eq!(names(&r, "X1"), named(&["a", "b"]));
    }
}

#[test]
fn invalid_terminal_names_fail_the_whole_declaration() {
    for invalid in [r#"["d","D"]"#, r#"["1","b"]"#, r#"["a","a"]"#] {
        let header = format!(
            r#"@DEVICE(name="X*", terminals=["p","n"], type="nmos"); @DEVICE(name="X*", terminals={invalid}, type="pmos")"#
        );
        let r = read("t.sp", &spice("0.4", &header, "X1 a b cell", r#"#X1 @ASSERT-INSTANCE(type="nmos")"#), None);
        assert_eq!(r.diagnostics().iter().filter(|d| d.code == codes::DEVICE_TERMINAL_NAMES_INVALID).count(), 1);
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
        assert_eq!(names(&r, "X1"), named(&["p", "n"]));
    }
}

#[test]
fn spectre_names_are_case_sensitive() {
    let text="// devices\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.4\"); @DEVICE(model=\"cell\", terminals=[\"d\",\"D\"], type=\"nmos\") }\nX1 (a b) cell\n";
    let r = read("t.scs", text, None);
    ok(&r);
    assert_eq!(names(&r, "X1"), named(&["d", "D"]));
}

#[test]
fn a_header_carrier_does_not_change_the_files_case_rules() {
    let text = "// device names\nsimulator lang=spice\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @DEVICE(model=\"cell\", terminals=[\"d\",\"D\"]) }\nsimulator lang=spectre\nX1 (a b) cell\n";
    let r = read("t.scs", text, None);
    ok(&r);
    assert_eq!(names(&r, "X1"), named(&["d", "D"]));
}

#[test]
fn host_types_and_defined_ports_win_but_other_items_still_apply() {
    let text = spice(
        "0.4",
        r#"
        @DEVICE(name="M1", terminals=["out","in","source","bulk"], type="pmos");
        @DEVICE(name="X1", terminals=["renamed","ports"], type="nmos")
    "#,
        ".model nch nmos\nM1 d g s 0 nch\nX1 a b cell\n.subckt cell left right\nR1 left right 1k\n.ends cell",
        r#"
        #M1 @ASSERT-INSTANCE(type="nmos"); #X1 @ASSERT-INSTANCE(type="nmos");
        #M1:out @ASSERT-TERMINAL(type="nmos"); #X1:left @ASSERT-TERMINAL(type="nmos")
    "#,
    );
    let r = read("t.sp", &text, None);
    ok(&r);
    assert_eq!(names(&r, "X1"), named(&["left", "right"]));
    assert_eq!(names(&r, "M1"), named(&["out", "in", "source", "bulk"]));
    for code in [codes::DEVICE_HOST_TYPE_CONFLICT, codes::DEVICE_SUBCIRCUIT_PORTS_CONFLICT] {
        assert_eq!(r.diagnostics().iter().filter(|d| d.code == code).count(), 1);
    }
}

#[test]
fn included_headers_act_only_on_their_own_host_lines() {
    let main = spice(
        "0.4",
        r#"@DEVICE(name="X*", terminals=["outer","return"], type="nmos")"#,
        "X1 a b cell\n.include child.sp",
        r#"#X1 @ASSERT-INSTANCE(type="nmos"); #X2 @ASSERT-INSTANCE(type="pmos")"#,
    );
    let child="* child\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @DEVICE(name=\"X*\", terminals=[\"inner\",\"return\"], type=\"pmos\") }\nX2 a b cell\n";
    let r = read("t.sp", &main, Some(child));
    ok(&r);
    assert_eq!(names(&r, "X1"), named(&["outer", "return"]));
    assert_eq!(names(&r, "X2"), named(&["inner", "return"]));
}

#[test]
fn packages_precede_file_items_and_check_names_without_case() {
    let package = r#"@RNL(type="header") { @VERSION("0.4"); @PACKAGE(name="devices", version="1.0"); @DEVICE(model="cell", terminals=["package","return"], type="pmos"); @DEVICE(model="cell", terminals=["d","D"], type="bad") }"#;
    let main = spice(
        "0.4",
        r#"@DEVICE(name="X*", terminals=["file","return"]); @USE-PACKAGE(name="devices", path="devices.rnl")"#,
        "X1 a b cell",
        r#"#X1 @ASSERT-INSTANCE(type="pmos")"#,
    );
    let files = MemoryProvider::new().with("t.sp", &main).with("devices.rnl", package);
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let errors: Vec<_> = r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect();
    assert_eq!(errors, [codes::DEVICE_TERMINAL_NAMES_INVALID]);
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
    assert_eq!(names(&r, "X1"), named(&["file", "return"]));
}

#[test]
fn both_written_patterns_must_match_and_short_lists_keep_numeric_positions() {
    let main = spice(
        "0.4",
        r#"@DEVICE(name="X1", model="cell_*", terminals=["first"]); @DEVICE(name="X2", model="other", terminals=["wrong"])"#,
        "X1 a b cell_a\nX2 a b cell_a",
        r#"$D = @DEFINE(roles=["{ts}"]) {}; {#X1:first #X1:2} @AS $D"#,
    );
    let r = read("t.sp", &main, None);
    ok(&r);
    assert_eq!(names(&r, "X1"), vec![Some("first".into()), None]);
    assert_eq!(names(&r, "X2"), vec![None, None]);
    assert_eq!(r.problems[0].usages[0].roles[0].members.len(), 2);
}

#[test]
fn topology_exclusion_reads_device_types() {
    let main = spice(
        "0.4",
        r#"@DEVICE(name="X*", terminals=["left","right","bulk"], type="nmos")"#,
        "X1 a b 0 cell\nX2 b c 0 cell",
        r#"{#X1 #X2} @ASSERT-TOPOLOGY("series", exclude=[["nmos","bulk"]])"#,
    );
    let r = read("t.sp", &main, None);
    ok(&r);
    let r = read("t.sp", &main.replace("exclude=[[\"nmos\",\"bulk\"]]", "exclude=[[\"pmos\",\"bulk\"]]"), None);
    assert!(r.diagnostics().iter().any(|d| d.severity == Severity::Violation));
}
