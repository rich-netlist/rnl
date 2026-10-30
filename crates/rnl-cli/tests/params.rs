//! The parameters of subcircuits as the Document shows them: those a definition writes
//! (`Level::params`), those an instance writes (`Child::params`), their values for an instance
//! (`Child::effective`), and a subcircuit built with values bound to them (`BuildOptions::bind`).

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use rnl_cli::path_str;
use rnl_runtime::source::MemoryProvider;
use rnl_view::{Bind, BuildOptions, Document, SubcktParam};

const SPICE: &str = "* params
.param vdd=1.8 scale=2
.subckt inv in out vdd vss params: wn=1u wp={2*wn}
+ l=100n
M1 out in vss vss nch w=wn l=l
M2 out in vdd vdd pch w=wp l=l
.ends
.subckt buf a y vdd vss wn=2u
X1 a m vdd vss inv wn={wn*scale}
X2 m y vdd vss inv
.ends
.subckt loop p q a={b} b={a} c=1 d={a}
.ends
X1 in out vdd 0 buf wn=3u
X2 in out vdd 0 inv
X3 p q missing w=1
.model nch nmos level=1
.model pch pmos level=1
.end
";

fn build(text: &str, name: &str, bind: Option<Bind>) -> Document {
    let files = MemoryProvider::new().with(&format!("/w/{name}"), text);
    let options = BuildOptions { svg: false, bind, ..BuildOptions::default() };
    rnl_view::build(&format!("/w/{name}"), &files, &options).unwrap()
}

/// Each parameter: its name, its value as written, its number and where the value comes from.
fn said(params: &[SubcktParam]) -> Vec<(&str, Option<&str>, Option<f64>, &str)> {
    params.iter().map(|p| (p.name.as_str(), p.value.as_deref(), p.number.map(round), p.from)).collect()
}

/// A number to twelve significant digits, so that 2*1e-6 compares equal to 2e-6.
fn round(x: f64) -> f64 {
    format!("{x:.12e}").parse().unwrap()
}

fn level<'d>(d: &'d Document, title: &str) -> &'d rnl_view::Level {
    d.levels.iter().find(|l| l.title == title).unwrap()
}

#[test]
fn a_subcircuit_lists_its_parameters_with_their_defaults_and_numbers() {
    let d = build(SPICE, "params.sp", None);
    // `params:` and the `+` line after it; a default that refers to another parameter
    assert_eq!(
        said(&level(&d, "inv").params),
        [
            ("wn", Some("1u"), Some(1e-6), "default"),
            ("wp", Some("{2*wn}"), Some(2e-6), "default"),
            ("l", Some("100n"), Some(1e-7), "default")
        ]
    );
    // without `params:`, as HSPICE writes them
    assert_eq!(said(&level(&d, "buf").params), [("wn", Some("2u"), Some(2e-6), "default")]);
    let inv = &level(&d, "inv").params[1];
    assert_eq!(inv.default.as_deref(), Some("{2*wn}"));
    // parameters that refer to each other have no number, with a hint; one that only refers to
    // them has none either, and the hint does not name it
    assert_eq!(
        said(&level(&d, "loop").params),
        [
            ("a", Some("{b}"), None, "default"),
            ("b", Some("{a}"), None, "default"),
            ("c", Some("1"), Some(1.0), "default"),
            ("d", Some("{a}"), None, "default")
        ]
    );
    let hint = d.diagnostics.iter().find(|x| x.message.contains("refer to each other")).unwrap();
    assert_eq!(
        (hint.severity, hint.message.as_str(), hint.line),
        ("hint", "the parameters a, b of subcircuit loop refer to each other, so they have no number", 12)
    );
    // a file level has none
    assert!(d.levels[0].params.is_empty());
}

#[test]
fn an_instance_gives_its_values_and_the_defaults_follow_them() {
    let d = build(SPICE, "params.sp", None);
    let top = &d.levels[0].children;
    let child = |name: &str| top.iter().find(|c| c.instance == name).unwrap();
    // what the instance writes, evaluated where it is, and the subcircuit's values for it
    let x1 = child("X1");
    assert_eq!(
        x1.params.iter().map(|p| (p.name.as_str(), p.value.as_str(), p.number.map(round))).collect::<Vec<_>>(),
        [("wn", "3u", Some(3e-6))]
    );
    assert_eq!(said(&x1.effective), [("wn", Some("3u"), Some(3e-6), "instance")]);
    let x2 = child("X2");
    assert!(x2.params.is_empty());
    assert_eq!(said(&x2.effective), said(&level(&d, "inv").params));
    // in buf, the value written refers to buf's own parameter and to a value the file sets; the
    // default of wp follows wn
    let inner = &level(&d, "buf").children[0];
    assert_eq!(inner.params[0].number.map(round), Some(4e-6));
    assert_eq!(
        said(&inner.effective),
        [
            ("wn", Some("{wn*scale}"), Some(4e-6), "instance"),
            ("wp", Some("{2*wn}"), Some(8e-6), "default"),
            ("l", Some("100n"), Some(1e-7), "default")
        ]
    );
    // a subcircuit the netlist does not define: what the instance writes, and nothing more
    let x3 = child("X3");
    assert_eq!(x3.level, None);
    assert_eq!(x3.params[0].number, Some(1.0));
    assert!(x3.effective.is_empty());
    let json = serde_json::to_value(x2).unwrap();
    assert!(json.get("params").is_none() && json.get("effective").is_some());
}

#[test]
fn a_subcircuit_built_with_values_bound_to_its_parameters() {
    let bind = Bind { subckt: "buf".into(), params: BTreeMap::from([("wn".into(), "5u".into())]) };
    let d = build(SPICE, "params.sp", Some(bind));
    assert_eq!(said(&level(&d, "buf").params), [("wn", Some("5u"), Some(5e-6), "bound")]);
    // what is written in it is evaluated with the value bound
    assert_eq!(level(&d, "buf").children[0].params[0].number.map(round), Some(1e-5));
    // nothing else changes, and nothing is said
    assert_eq!(level(&d, "inv").params[0].from, "default");
    assert!(!d.diagnostics.iter().any(|x| x.message.contains("bound")));

    // a parameter it does not have, and a subcircuit the netlist does not have
    let bind = Bind { subckt: "BUF".into(), params: BTreeMap::from([("wx".into(), "1u".into())]) };
    let d = build(SPICE, "params.sp", Some(bind));
    let warning = d.diagnostics.iter().find(|x| x.message.starts_with("bound parameter")).unwrap();
    assert_eq!(
        (warning.severity, warning.message.as_str(), warning.line),
        ("warning", "bound parameter wx is not a parameter of subcircuit buf; it is ignored; did you mean wn?", 8)
    );
    assert_eq!(level(&d, "buf").params[0].from, "default");
    let bind = Bind { subckt: "nope".into(), params: BTreeMap::new() };
    let d = build(SPICE, "params.sp", Some(bind));
    assert!(d
        .diagnostics
        .iter()
        .any(|x| x.severity == "warning" && x.message == "no subcircuit nope to bind parameters to"));
}

#[test]
fn spectre_parameters_and_scale_factors() {
    let text = "// t
simulator lang=spectre
parameters big=1M
subckt amp (in out)
parameters w=1u m=2 r=big
r1 (in out) resistor r=1k
ends amp
x1 (a b) amp w=3u
";
    let d = build(text, "amp.scs", None);
    // `M` is mega in Spectre
    assert_eq!(
        said(&level(&d, "amp").params),
        [
            ("w", Some("1u"), Some(1e-6), "default"),
            ("m", Some("2"), Some(2.0), "default"),
            ("r", Some("big"), Some(1e6), "default")
        ]
    );
    let x1 = &d.levels[0].children[0];
    assert_eq!(said(&x1.effective)[0], ("w", Some("3u"), Some(3e-6), "instance"));
}

fn rnl(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.code().unwrap_or(-1), text(&out.stdout), text(&out.stderr))
}

#[test]
fn rnl_view_binds_parameters() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-params");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("params.sp"), SPICE).unwrap();
    let file = path_str(&dir.join("params.sp"));
    let (code, out, err) = rnl(&["view", &file, "--no-svg", "--bind", "buf", "--param", "wn=5u"]);
    assert_eq!((code, err.as_str()), (0, ""));
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let buf = v["levels"].as_array().unwrap().iter().find(|l| l["title"] == "buf").unwrap();
    assert_eq!(buf["params"][0]["from"], "bound");
    assert_eq!(buf["params"][0]["value"], "5u");
    let (code, _, err) = rnl(&["view", &file, "--param", "wn=5u"]);
    assert_eq!((code, err.trim()), (2, "--param needs --bind SUBCKT, the subcircuit whose parameters it gives"));
    let (code, _, err) = rnl(&["check", &file, "--bind", "buf"]);
    assert_eq!((code, err.trim()), (2, "--bind and --param are options of view"));
}
