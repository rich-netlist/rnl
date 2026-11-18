//! Type conditions take `*` in 0.4, and unknown types never match (10.1, 15.3).
use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const SETUP: &str = r#"
    $PROBE = @DEFINE(roles=["x"]) {};
    $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1) };
    # @SELECT-INSTANCE => @SET-SYMBOL $S
"#;

fn spice(version: &str, body: &str) -> String {
    let device = if version == "0.4" { r#"; @DEVICE(name="X1", type="nmos")"# } else { "" };
    format!(
        "* type patterns\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){device} }}\n\
         .model nch nmos\n.model pch pmos\nM1 d g s 0 nch\nM2 d g s 0 pch\n\
         X1 a b cell\nX2 a b unknown\n* @RNL {{ {}; {} }}\n.end\n",
        SETUP.replace('\n', " "),
        body.replace('\n', " ")
    )
}

fn read(path: &str, source: &str, child: Option<&str>) -> Interpretation {
    let mut provider = MemoryProvider::new().with(path, source);
    if let Some(child) = child {
        provider = provider.with("child.sp", child);
    }
    interpret_file(Path::new(path), None, &provider, &Options { std: false, ..Options::solver() }).unwrap()
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

fn count(r: &Interpretation, definition: &str) -> usize {
    r.problems.iter().flat_map(|p| &p.usages).filter(|u| u.definition == definition).count()
}

#[test]
fn all_three_selections_match_host_and_device_types_with_star() {
    for (pattern, instances, terminals) in [
        ("*mos", 3, 10),
        ("n*", 2, 6),
        ("*o*", 3, 10),
        ("*m*s", 3, 10),
        ("*nmos", 2, 6),
        ("nmos*", 2, 6),
        ("**nmos**", 2, 6),
        ("*", 3, 10),
        ("pmos", 1, 4),
        ("n?os", 0, 0),
        ("", 0, 0),
        ("resistor", 0, 0),
    ] {
        for (keyword, expected) in
            [("SELECT-INSTANCE", instances), ("SELECT-TERMINAL", terminals), ("SELECT-PIN", instances)]
        {
            let source = spice("0.4", &format!(r#"# @{keyword}(type="{pattern}") => @AS $PROBE"#));
            let r = read("t.sp", &source, None);
            ok(&r);
            assert_eq!(count(&r, "$PROBE"), expected, "{keyword} {pattern}");
        }
    }
}

#[test]
fn unknown_types_match_neither_empty_nor_wildcard_patterns() {
    for keyword in ["SELECT-INSTANCE", "SELECT-TERMINAL", "SELECT-PIN"] {
        for pattern in ["", "*", "**", "*mos"] {
            let source = spice("0.4", &format!(r#"#X2 @{keyword}(type="{pattern}") => @AS $PROBE"#));
            let r = read("t.sp", &source, None);
            ok(&r);
            assert_eq!(count(&r, "$PROBE"), 0, "{keyword} {pattern}");
        }
    }
}

#[test]
fn instance_and_terminal_assertions_share_the_type_matcher() {
    let r = read(
        "t.sp",
        &spice(
            "0.4",
            r#"
        {#M1 #M2 #X1} @ASSERT-INSTANCE(type="*mos");
        {#M1:1 #M2:1 #X1:1} @ASSERT-TERMINAL(type="*mos");
        # @SELECT-INSTANCE(type="*") => @ASSERT-INSTANCE(type="*mos")
    "#,
        ),
        None,
    );
    ok(&r);
    for (target, keyword) in [("#X2", "ASSERT-INSTANCE"), ("#X2:1", "ASSERT-TERMINAL")] {
        for pattern in ["", "*", "*mos"] {
            let r = read("t.sp", &spice("0.4", &format!(r#"{target} @{keyword}(type="{pattern}")"#)), None);
            let bad: Vec<_> = r
                .diagnostics()
                .iter()
                .filter(|d| matches!(d.severity, Severity::Error | Severity::Violation))
                .map(|d| (d.code, r.sink.render(d)))
                .collect();
            assert_eq!(bad.len(), 1, "{bad:?}");
            assert_eq!(bad[0].0, codes::VIOLATION_CONDITION);
            assert!(bad[0].1.contains("no type"), "{bad:?}");
        }
    }
    let r = read("t.sp", &spice("0.4", r#"#M2 @ASSERT-INSTANCE(type="n*")"#), None);
    assert!(r.diagnostics().iter().any(|d| d.code == codes::VIOLATION_CONDITION));
}

#[test]
fn type_patterns_combine_with_other_conditions() {
    for keyword in ["SELECT-INSTANCE", "SELECT-TERMINAL", "SELECT-PIN"] {
        let r = read("t.sp", &spice("0.4", &format!(r#"# @{keyword}(type="*mos", name="X*") => @AS $PROBE"#)), None);
        ok(&r);
        assert_eq!(count(&r, "$PROBE"), if keyword == "SELECT-TERMINAL" { 2 } else { 1 });
    }
}

#[test]
fn spectre_device_types_use_the_same_patterns() {
    for keyword in ["SELECT-INSTANCE", "SELECT-TERMINAL", "SELECT-PIN"] {
        let source = format!(
            r#"// type patterns
simulator lang=spectre
// @RNL(type="header") {{ @VERSION("0.4"); @DEVICE(name="xn", type="nmos"); @DEVICE(name="xp", type="pmos") }}
xn (a b) cell
xp (a b) cell
xu (a b) unknown
// @RNL {{ {}; # @{keyword}(type="*mos") => @AS $PROBE; {{#xn #xp}} @ASSERT-INSTANCE(type="*mos") }}
"#,
            SETUP.replace('\n', " ")
        );
        let r = read("t.scs", &source, None);
        ok(&r);
        assert_eq!(count(&r, "$PROBE"), if keyword == "SELECT-TERMINAL" { 4 } else { 2 });
    }
}

#[test]
fn bundle_type_patterns_use_the_first_member_once() {
    for keyword in ["SELECT-INSTANCE", "SELECT-TERMINAL", "SELECT-PIN"] {
        for (pattern, selected) in [("n*", true), ("p*", false)] {
            let body = format!(
                r#"%b = {{#M1 #M2}} @BUNDLE; %b @SET-SYMBOL $S; %b @ASSERT-INSTANCE(type="n*"); %b @{keyword}(type="{pattern}") => @AS $PROBE"#
            );
            let source = spice("0.4", &body).replace("# @SELECT-INSTANCE => @SET-SYMBOL $S", "");
            let r = read("t.sp", &source, None);
            ok(&r);
            let expected = if selected {
                if keyword == "SELECT-TERMINAL" {
                    4
                } else {
                    1
                }
            } else {
                0
            };
            assert_eq!(count(&r, "$PROBE"), expected, "{keyword} {pattern}");
        }
    }
}

#[test]
fn scoped_and_included_selections_both_take_patterns() {
    let main = r#"* main
* @RNL(type="header") { @VERSION("0.4") }
.include child.sp
* @RNL(scope="cell") { $PROBE = @DEFINE(roles=["x"]) {}; # @SELECT-INSTANCE(type="*mos") => @AS $PROBE }
.end
"#;
    let child = r#"* child
* @RNL(type="header") { @VERSION("0.4") }
.subckt cell a b
.model nch nmos
M1 a a b b nch
* @RNL { $CHILD = @DEFINE(roles=["x"]) {}; # @SELECT-INSTANCE(type="*mos") => @AS $CHILD }
.ends
"#;
    let r = read("t.sp", main, Some(child));
    ok(&r);
    assert_eq!(count(&r, "$PROBE"), 1);
    assert_eq!(count(&r, "$CHILD"), 1);
}
