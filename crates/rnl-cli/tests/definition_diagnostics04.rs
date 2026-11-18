//! Static definition errors keep their original cause across uses (6.9, 7.1).
use rnl_cli::interpret_file;
use rnl_runtime::problem::{Position, Reference, Slot};
use rnl_runtime::{
    diag::{codes, Code, Diag, Severity},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

const CASES: &[(&str, Code)] = &[
    ("#R1 @PLACE(0,0)", codes::DEFINE_BODY_HOST_REF),
    ("(0,0) @PIN(terminal=1)", codes::DEFINE_BODY_PIN),
    ("?missing @PLACE(0,0)", codes::REF_HOLE_UNKNOWN),
    ("?distance @PLACE(0,0)", codes::REF_HOLE_PARAM_AS_OBJECT),
    ("?r @NOT-A-KEYWORD", codes::KEYWORD_UNKNOWN),
    ("(?missing,0)", codes::REF_HOLE_NOT_A_PARAM),
];
const TWO: &str = "%u1 = #R1 @AS $D; %u2 = #R2 @AS $D";
fn definition(body: &str) -> String {
    format!(r#"$D = @DEFINE(roles=["r"],parameters=[["distance",1]]) {{ {body}; %good=(2,0) }}"#)
}
fn source(version: &str, body: &str, calls: &str) -> String {
    format!("* definitions\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {}; {calls} }}\n",definition(body))
}
fn files(files: MemoryProvider) -> Interpretation {
    interpret_file(Path::new("main.sp"), None, &files, &Options { std: false, problems: true, ..Default::default() })
        .unwrap()
}
fn read(version: &str, body: &str, calls: &str) -> Interpretation {
    files(MemoryProvider::new().with("main.sp", &source(version, body, calls)))
}
fn errors(r: &Interpretation) -> Vec<&Diag> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect()
}

#[test]
fn static_body_errors_are_reported_once_across_uses() {
    for &(body, code) in CASES {
        let r = read("0.4", body, TWO);
        let errors = errors(&r);
        assert_eq!(errors.len(), 1, "{body}: {errors:?}");
        assert_eq!(errors[0].code, code);
        assert!(errors[0].origin.is_empty(), "{errors:?}");
        let loc = errors[0].loc.as_ref().unwrap();
        assert_eq!(loc.line, 5);
        assert!(loc.col > 1, "{errors:?}");
    }
}

#[test]
fn unused_definitions_report_the_same_static_error() {
    for &(body, code) in CASES {
        let r = read("0.4", body, "");
        let errors = errors(&r);
        assert_eq!(errors.len(), 1, "{body}: {errors:?}");
        assert_eq!(errors[0].code, code);
    }
}

#[test]
fn independent_members_of_each_usage_still_execute() {
    let r = read("0.4", "#R1 @PLACE(0,0)", &format!("{TWO}; %g1 = %u1::good @PLACE(3,4); %g2 = %u2::good @PLACE(5,6)"));
    assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
    for (name, x, y) in [("%g1", 3., 4.), ("%g2", 5., 6.)] {
        let object = r.problems[0].objects.iter().find(|o| o.reference == Reference::Name(name.into())).unwrap();
        assert!(matches!(object.position,
            Position::Own { x: Slot::Fixed { value: px, .. }, y: Slot::Fixed { value: py, .. } } if px == x && py == y));
    }
}

#[test]
fn argument_dependent_errors_still_report_each_bad_application() {
    let calls = r#"#R1 @AS(distance="bad") $D; #R2 @AS(distance="bad") $D; #R1 @AS(distance=2) $D"#;
    let r = read("0.4", "%p=(?distance,0)", calls);
    let errors = errors(&r);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors.iter().all(|d| d.code == codes::PARAM_HOLE_NOT_NUMBER && d.origin.contains("applying $D")),
        "{errors:?}"
    );
}

#[test]
fn distinct_members_and_definitions_keep_distinct_static_errors() {
    let body = "(?missing,0); (1,?missing); #R1 @PLACE(0,0); #R2 @PLACE(0,0)";
    let r = read("0.4", body, TWO);
    let diagnostics = errors(&r);
    assert_eq!(diagnostics.len(), 4, "{diagnostics:?}");
    assert_eq!(diagnostics.iter().filter(|d| d.code == codes::REF_HOLE_NOT_A_PARAM).count(), 2);
    let extra = definition("#R2 @PLACE(0,0)").replace("$D", "$E");
    let r = read("0.4", "#R1 @PLACE(0,0)", &format!("{extra}; {TWO}; #R1 @AS $E; #R2 @AS $E"));
    assert_eq!(errors(&r).len(), 2, "{:?}", r.diagnostics());
}

#[test]
fn package_errors_retain_the_definition_source() {
    let package = format!(
        "@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"bad\",version=\"1.0\") }}\n@RNL {{ {} }}\n",
        definition("#R1 @PLACE(0,0)")
    );
    let main=format!("* package use\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"bad\",version=\"1.0\",path=\"bad.rnl\") }}\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {TWO} }}\n");
    let r = files(MemoryProvider::new().with("main.sp", &main).with("bad.rnl", &package));
    let errors = errors(&r);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, codes::DEFINE_BODY_HOST_REF);
    assert!(errors[0].loc.as_ref().unwrap().file.ends_with("bad.rnl"), "{errors:?}");
}

#[test]
fn a_definition_in_an_included_file_reports_once() {
    let main = "* top\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include child.sp\nR1 a b 1k\nR2 a b 2k\n";
    let child = format!(
        "* child\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.subckt child a b\nR1 a b 1k\nR2 a b 2k\n* @RNL {{ {}; {TWO} }}\n.ends child\n",
        definition("#R1 @PLACE(0,0)")
    );
    let r = files(MemoryProvider::new().with("main.sp", main).with("child.sp", &child));
    assert_eq!(errors(&r).len(), 1, "{:?}", r.diagnostics());
}
