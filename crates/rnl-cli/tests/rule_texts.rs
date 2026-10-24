//! A TEXT that a rule gives what a selection reaches (10.4), landing on a usage without a symbol or
//! a terminal without a pin: it does nothing, and says nothing, as SET-STYLE does on a semantic usage
//! (8.3). Written for the object itself, it fails as before (12.1, 13.3).
//!
//! The case is rnl-std-no-labels, which every file of Analog Canvas uses: its `# @SELECT-USAGE =>
//! @TEXT("", key=…)` reaches the usages of structures, $MIRRORED, $FLOW and the rest, which have no
//! symbol, and each failed with an error, so a file that wrote its intent as structures could not
//! pass `rnl check`. A deviation from the text of 0.3, which the user accepted for core 0.4.1.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use rnl_view::{BuildOptions, Parts};

#[path = "support/public_packages.rs"]
mod public_packages;

/// A five-transistor amplifier with every structure of the standard package written as a usage, a
/// port of the standard package drawn with its symbol, and the `extra` statements, in a file that
/// uses rnl-std-no-labels when `no_labels`.
fn netlist(no_labels: bool, extra: &str) -> String {
    let package = if no_labels { "; @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.3\")" } else { "" };
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\"){package}}}\nM1 d1 g1 s 0 nch\nM2 d2 g2 s 0 nch\nM3 s b 0 0 nch\n\
         M4 d1 d1 vdd vdd pch\nM5 d2 d1 vdd vdd pch\n.model nch nmos\n.model pch pmos\n* @RNL {{\n\
         *   #M1(a) #M3(axis) #M2(b) @AS $MIRRORED => %m;\n*   {{#M1 #M2 #M3}} @AS $FLOW;\n\
         *   #M1(in_p) #M2(in_n) #M3(tail) @AS $DIFFERENTIAL_PAIR;\n*   #M4(ref) #M5(out) @AS $CURRENT_MIRROR;\n\
         *   {{#M4 #M1}} @AS $BRANCH;\n*   $IO_PORT @ATTACH &d1 => @PLACE(0, 0) => %p{extra}\n* }}\n.end\n"
    )
}

fn interpret(net: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", net);
    interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    let errors = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error);
    errors.map(|d| r.sink.render(d)).collect()
}

/// What the text of `key` attached to `object` in the main level shows, as the model has it; none
/// when it shows nothing, which the snapshot leaves out.
fn label(net: &str, object: &str, key: &str) -> Option<String> {
    let files = MemoryProvider::new().with("/w/t.sp", net);
    let parts = Parts { drawing: false, host: false, model: true };
    let s = rnl_view::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(s).unwrap();
    let texts = s["levels"][0]["model"]["texts"].as_array().unwrap().clone();
    let text = texts.into_iter().find(|t| t["attach"] == object && t["key"] == key)?;
    text["markup"]["value"].as_str().map(String::from)
}

#[test]
fn the_structures_of_a_file_without_default_labels_are_no_error() {
    let r = interpret(&netlist(true, ""));
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    // and without the package, as before
    assert!(errors(&interpret(&netlist(false, ""))).is_empty());
}

#[test]
fn a_text_written_for_a_usage_without_a_symbol_still_fails() {
    // with a key, which rnl-std-no-labels gives the same usage a layer of, and without one
    for statement in ["%m @TEXT(\"mirror\", key=\"name\")", "%m @TEXT(\"mirror\")"] {
        let r = interpret(&netlist(true, &format!(";\n*   {statement}")));
        let want = "error t.sp:17:5: %m has no symbol and no drawing meaning: this TEXT fails (12.1, 13.3)";
        assert_eq!(errors(&r), [want], "{statement}");
    }
}

#[test]
fn a_usage_with_a_symbol_still_has_its_labels_emptied() {
    // the port's symbol labels it with its net; rnl-std-no-labels empties it, as before
    assert!(label(&netlist(false, ""), "%p", "name").is_some_and(|m| !m.is_empty()));
    assert_eq!(label(&netlist(true, ""), "%p", "name"), None);
}

#[test]
fn an_analog_canvas_export_with_structures_checks_without_errors() {
    // a netlist as Analog Canvas writes one, with its extensions, and the structures of its intent
    if !public_packages::available() {
        return;
    }
    let net = "* export\n* @RNL(type=\"header\") {\n*   @VERSION(\"0.3\");\n*   @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\");\n\
               *   @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.3\");\n\
               *   @USE-PACKAGE(name=\"analog-canvas-extensions\", version=\"0.1\")\n* }\n.subckt dut VDD VSS A B Vo\n\
               M1 x A VDD VDD pch\nM2 Vo B x VDD pch\nM3 Vo A VSS VSS nch\nM4 Vo B VSS VSS nch\n.model nch nmos\n.model pch pmos\n\
               * @RNL(author=\"analog-canvas\") {\n*   #M1 @SET-SYMBOL $PMOS => @PLACE(29, -19);\n\
               *   #M2 @SET-SYMBOL $PMOS => @PLACE(29, -23);\n*   #M3 @SET-SYMBOL $NMOS => @PLACE(25, -29);\n\
               *   #M4 @SET-SYMBOL $NMOS => @PLACE(33, -29);\n\
               *   $IO_PORT @ATTACH &A(p) => @SET-SYMBOL $CANVAS_PORT_FILLED => @PLACE(22, -29) => %port1;\n\
               *   $GROUND @ATTACH &VSS(p) => @SET-SYMBOL $GROUND_SYMBOL => @PLACE(29, -32) => %gnd1;\n\
               *   %port1 @TEXT(\"A\", key=\"name\", dx=-1.5, dy=-0.5, halign=\"end\")\n* }\n\
               * @RNL { {#M1 #M2} @AS $BRANCH; {#M3 #M4} @AS $FLOW; #M3(a) #M4(b) %gnd1(axis) @AS $MIRRORED }\n.ends dut\n";
    // the package as the other tests of it give it: its file in a package directory of its own
    let package = public_packages::package_dir("analog-canvas-extensions").join("analog-canvas-extensions.rnl");
    let mut files = MemoryProvider::new()
        .with("export.sp", net)
        .with("pkgs/analog-canvas-extensions.rnl", &std::fs::read_to_string(package).unwrap());
    files.package_dirs.push("pkgs".into());
    let r = interpret_file(Path::new("export.sp"), None, &files, &Options::default()).unwrap();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
}
