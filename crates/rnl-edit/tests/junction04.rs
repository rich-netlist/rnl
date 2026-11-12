use rnl_edit::{write_diagnosed, write_with_options_diagnosed, WriteRequest};
use rnl_runtime::{source::MemoryProvider, Options};
use serde_json::json;
fn request(version: &str, packages: bool) -> WriteRequest {
    let mut ops = vec![
        json!({"op":"add-point","level":"file","name":"%j","at":[1,2],"junction":true}),
        json!({"op":"add-point","level":"subckt:child","name":"%k","at":[3,4],"junction":true}),
    ];
    if packages {
        ops.push(json!({"op":"use-package","name":"dots","version":"1.0"}));
    }
    serde_json::from_value(json!({"language_version":version,"dialect":"spice","author":"test",
        "netlist":"title\n.subckt child a b\nR2 a b 1k\n.ends child\nR1 a 0 1k\n.end\n","ops":ops}))
    .unwrap()
}
fn packages(value: &str) -> MemoryProvider {
    let mut f=MemoryProvider::new().with("/packages/dots.rnl",&format!(
        "@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"dots\",version=\"1.0\"); @SOLVER-CONFIG(junction={value})}}\n@RNL {{$DOT = @DEFINE(roles=[[\"p\",\"point\"]]) {{}}}}\n"));
    f.package_dirs.push("/packages".into());
    f
}
#[test]
fn modern_writer_does_not_add_an_unconfigured_dot() {
    let w = write_diagnosed(&request("0.4", false)).unwrap();
    assert!(w.text.contains("%j = (1, 2)"));
    assert!(w.text.contains("%k = (3, 4)"));
    assert!(!w.text.contains("@ATTACH"));
    assert!(w.diagnostics.is_empty());
}
#[test]
fn package_configuration_is_used_in_each_target_level_and_reads_back() {
    let f = packages(r#""$DOT""#);
    let req = request("0.4", true);
    let w = write_with_options_diagnosed(&req, &f, &Options::default()).unwrap();
    assert!(w.text.contains("$DOT @ATTACH %j"));
    assert!(w.text.contains("$DOT @ATTACH %k"));
    assert!(w.diagnostics.is_empty(), "{:?}", w.diagnostics);
    let memory = MemoryProvider::new().with(&rnl_edit::entry_of(&req), &w.text);
    let both = rnl_runtime::source::Layered { layers: vec![&memory, &f] };
    let read = rnl_view::interpret_with(
        &rnl_edit::entry_of(&req),
        Some("spice"),
        &Default::default(),
        &both,
        &Options::default(),
    )
    .unwrap();
    assert!(
        !read.diagnostics().iter().any(|d| d.severity == rnl_runtime::diag::Severity::Error),
        "{:?}",
        read.diagnostics()
    );
    for name in ["%j", "%k"] {
        assert!(read.model.levels.iter().flat_map(|l| &l.points).find(|p| p.id == name).unwrap().junction);
    }
}
#[test]
fn invalid_setting_returns_one_numbered_warning_and_keeps_points() {
    let w = write_with_options_diagnosed(&request("0.4", true), &packages("false"), &Options::default()).unwrap();
    assert!(!w.text.contains("@ATTACH"));
    assert!(w.text.contains("%j ="));
    assert_eq!(w.diagnostics.len(), 1, "{:?}", w.diagnostics);
    assert_eq!(w.diagnostics[0].severity, "warning");
    assert_eq!(w.diagnostics[0].code.as_deref(), Some("RNL8118"));
    assert!(w.diagnostics[0].full_path.ends_with("dots.rnl"));
}
#[test]
fn explicit_empty_configuration_disables_dots_without_warning() {
    let w = write_with_options_diagnosed(&request("0.4", true), &packages(r#""""#), &Options::default()).unwrap();
    assert!(!w.text.contains("@ATTACH"));
    assert!(w.diagnostics.is_empty());
}
#[test]
fn include_standard_is_persisted_before_junction_resolution() {
    let mut f = MemoryProvider::new().with(
        "/packages/std.rnl",
        r#"@RNL(type="header") {
        @VERSION("0.4"); @PACKAGE(name="rnl-std-all",version="0.4"); @SOLVER-CONFIG(junction="$MARK")}
        @RNL {$MARK = @DEFINE(roles=[["p","point"]]) {}}"#,
    );
    f.package_dirs.push("/packages".into());
    let w =
        write_with_options_diagnosed(&request("0.4", false), &f, &Options { include_std: true, ..Default::default() })
            .unwrap();
    assert!(w.text.contains("name=\"rnl-std-all\", version=\"0.4\""), "{}", w.text);
    assert!(w.text.contains("$MARK @ATTACH %j"));
    assert!(w.text.contains("$MARK @ATTACH %k"));
    assert!(w.diagnostics.is_empty());
}
#[test]
fn released_writer_keeps_existing_standard_dot_output() {
    let w = write_diagnosed(&request("0.3", false)).unwrap();
    assert!(w.text.contains("$JUNCTION @ATTACH %j"));
    assert!(w.text.contains("$JUNCTION @ATTACH %k"));
    assert!(w.diagnostics.is_empty());
}
