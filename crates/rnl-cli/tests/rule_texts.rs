//! A TEXT that a rule gives what a selection reaches (10.4), landing on a usage without a symbol or
//! a terminal without a pin: it does nothing, and says nothing, as SET-STYLE does on a semantic usage
//! (8.3). Written for the object itself, it fails (12.1, 13.3).
//!
//! The case is rnl-std-no-labels, which every file of Analog Canvas uses: its `# @SELECT-USAGE`
//! rules reach the usages of structures, $MIRRORED_LAYOUT, $SIGNAL_FLOW and the rest, which have no
//! symbol, and a file that writes its intent as structures still passes `rnl check`.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use rnl_view::{BuildOptions, Parts};

use crate::public_packages;

/// A five-transistor amplifier with structures of the standard package written as usages, a net
/// label of the standard package drawn with its symbol, and the `extra` statements, in a file that
/// uses rnl-std-no-labels when `no_labels`.
fn netlist(no_labels: bool, extra: &str) -> String {
    let package = if no_labels { "; @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.4\")" } else { "" };
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"){package} }}\n\
         M1 d1 g1 s 0 nch\nM2 d2 g2 s 0 nch\nM3 s b 0 0 nch\n\
         M4 d1 d1 vdd vdd pch\nM5 d2 d1 vdd vdd pch\n.model nch nmos\n.model pch pmos\n* @RNL {{\n\
         *   #M1(left) #M2(right) #M3(axis) @AS $MIRRORED_LAYOUT => %m;\n*   {{#M1 #M2 #M3}} @AS $SIGNAL_FLOW;\n\
         *   #M1(left) #M2(right) #M3(tail) @AS $DIFFERENTIAL_PAIR_N;\n*   #M4(ref) #M5(out) @AS $CURRENT_MIRROR_P;\n\
         *   {{#M4 #M1}} @AS $BRANCH;\n*   $NET_LABEL @ATTACH &d1 => @PLACE(0, 0) => %p{extra}\n* }}\n.end\n"
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
        let want = "error t.sp:17:5: RNL4414 text-usage-no-symbol: %m has no symbol and no drawing meaning: this TEXT fails (12.1, 13.3)";
        assert_eq!(errors(&r), [want], "{statement}");
    }
}

/// Whether the text of `key` attached to `object` in the main level is drawn, as the snapshot
/// gives it: `Some(false)` when a style hides it, none when nothing says.
fn visible(net: &str, object: &str, key: &str) -> Option<bool> {
    let files = MemoryProvider::new().with("/w/t.sp", net);
    let parts = Parts { drawing: false, host: false, model: true };
    let s = rnl_view::snapshot("/w/t.sp", &files, &BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(s).unwrap();
    let texts = s["levels"][0]["model"]["texts"].as_array().unwrap().clone();
    let text = texts.into_iter().find(|t| t["attach"] == object && t["key"] == key).unwrap();
    text["visible"].as_bool()
}

#[test]
fn a_usage_with_a_symbol_still_has_its_labels_hidden() {
    // the label's symbol shows its net name; rnl-std-no-labels hides it with textvisible=false and
    // keeps its content
    assert_eq!(label(&netlist(false, ""), "%p", "netname").as_deref(), Some("${p}"));
    assert_eq!(visible(&netlist(false, ""), "%p", "netname"), None);
    assert_eq!(label(&netlist(true, ""), "%p", "netname").as_deref(), Some("${p}"));
    assert_eq!(visible(&netlist(true, ""), "%p", "netname"), Some(false));
}

#[test]
fn an_analog_canvas_export_with_structures_checks_without_errors() {
    // a netlist as Analog Canvas writes one, with its extensions, and the structures of its intent
    if !public_packages::available() {
        return;
    }
    let net = "* export\n* @RNL(type=\"header\") {\n*   @VERSION(\"0.4\");\n*   @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\");\n\
               *   @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.4\");\n\
               *   @USE-PACKAGE(name=\"analog-canvas-extensions\", version=\"0.7\")\n* }\n.subckt dut VDD VSS A B Vo\n\
               M1 x A VDD VDD pch\nM2 Vo B x VDD pch\nM3 Vo A VSS VSS nch\nM4 Vo B VSS VSS nch\n.model nch nmos\n.model pch pmos\n\
               * @RNL(author=\"analog-canvas\") {\n*   #M1 @SET-SYMBOL $PMOS => @PLACE(29, -19);\n\
               *   #M2 @SET-SYMBOL $PMOS => @PLACE(29, -23);\n*   #M3 @SET-SYMBOL $NMOS => @PLACE(25, -29);\n\
               *   #M4 @SET-SYMBOL $NMOS => @PLACE(33, -29);\n\
               *   $IO_PORT @ATTACH &A(p) => @SET-SYMBOL $CANVAS_PORT_FILLED => @PLACE(22, -29) => %port1;\n\
               *   $GROUND @ATTACH &VSS(p) => @SET-SYMBOL $GROUND_SYMBOL => @PLACE(29, -32) => %gnd1;\n\
               *   %port1 @TEXT(\"A\", key=\"name\", dx=-1.5, dy=-0.5, halign=\"end\")\n* }\n\
               * @RNL { {#M1 #M2} @AS $BRANCH; {#M3 #M4} @AS $SIGNAL_FLOW; #M3(left) #M4(right) %gnd1(axis) @AS $MIRRORED_LAYOUT }\n.ends dut\n";
    // the package as the other tests of it give it: its file in a package directory of its own
    let package = public_packages::version_dir("analog-canvas-extensions", "0.7").join("analog-canvas-extensions.rnl");
    let mut files = MemoryProvider::new()
        .with("export.sp", net)
        .with("pkgs/analog-canvas-extensions.rnl", &std::fs::read_to_string(package).unwrap());
    files.package_dirs.push("pkgs".into());
    let r = interpret_file(Path::new("export.sp"), None, &files, &Options::default()).unwrap();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
}

// A rule's TEXT on a usage without a symbol and on one with a symbol, with a definition of its own.
fn netlist_v04(suppress: bool, extra: &str) -> String {
    let rule = if suppress { "# @SELECT-USAGE => @TEXT(\"\",key=\"name\");" } else { "" };
    format!(
        "* rules\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nR1 a b 1k\n* @RNL {{\n\
         * $D = @DEFINE(roles=[\"x\"]) {{}};\n\
         * $S = @DEFINE-SYMBOL {{ ? @TEXT(\"label\",key=\"name\") }};\n\
         * $P = @DEFINE(roles=[\"p\"]) {{ ? @SET-SYMBOL $S }};\n\
         * #R1(x) @AS $D => %sem;\n\
         * &a(p) @AS $P => %drawn => @PLACE(0,0);\n\
         * {rule}\n* {extra}\n* }}\n.end\n"
    )
}

#[test]
fn v04_rule_text_skips_semantic_usage_and_empties_drawn_usage_label() {
    let before = netlist_v04(false, "");
    let after = netlist_v04(true, "");
    assert!(errors(&interpret(&before)).is_empty());
    assert!(errors(&interpret(&after)).is_empty());
    assert_eq!(label(&before, "%drawn", "name").as_deref(), Some("label"));
    assert_eq!(label(&after, "%drawn", "name"), None);
}

#[test]
fn v04_direct_text_on_semantic_usage_still_fails() {
    for statement in ["%sem @TEXT(\"mirror\",key=\"name\")", "%sem @TEXT(\"mirror\")"] {
        let got = errors(&interpret(&netlist_v04(true, statement)));
        assert_eq!(got.len(), 1, "{got:?}");
        assert!(got[0].contains("RNL4414 text-usage-no-symbol"), "{got:?}");
    }
}

/// A 0.4 file that uses rnl-std-no-labels, with a drawn usage whose symbol draws a name, a value, a
/// model and a net name, and the parameter textvisible each of them ends with in the problem.
fn hidden_v04(no_labels: bool) -> Vec<(String, Option<bool>)> {
    let package = if no_labels { "; @USE-PACKAGE(name=\"rnl-std-no-labels\", version=\"0.4\")" } else { "" };
    let net = format!(
        "* rules\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"){package} }}\nR1 a b 1k\n* @RNL {{\n\
         * $S = @DEFINE-SYMBOL {{ (0, 0) @PIN(terminal=\"p\"); ? @TEXT(\"n\", key=\"name\"); \
         ? @TEXT(\"v\", key=\"value\"); ? @TEXT(\"m\", key=\"model\"); ? @TEXT(\"a\", key=\"netname\") }};\n\
         * $P = @DEFINE(roles=[\"p\"]) {{ ? @SET-SYMBOL $S }};\n\
         * &a(p) @AS $P => %drawn => @PLACE(0, 0);\n* }}\n.end\n"
    );
    let files = MemoryProvider::new().with("t.sp", &net);
    let options = Options { problems: true, ..Default::default() };
    let r = interpret_file(Path::new("t.sp"), None, &files, &options).unwrap();
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let objects = r.problems[0].objects.iter().filter(|o| o.label.starts_with("%drawn::"));
    let visible = |o: &rnl_runtime::problem::ProblemObject| {
        let style = o.text.as_ref().map(|t| t.style.clone()).unwrap_or_default();
        style.iter().find(|(k, _)| k == "textvisible").and_then(|(_, v)| v.boolean())
    };
    let mut got: Vec<_> = objects.map(|o| (o.label.clone(), visible(o))).collect();
    got.sort();
    got
}

/// rnl-std-no-labels hides the name, value, model and net name a drawn usage's symbol draws, as it does
/// on an instance: its SELECT-USAGE rules reach the labels attached to the usage.
#[test]
fn v04_no_labels_hides_the_default_labels_of_a_drawn_usage() {
    let label = |key: &str, visible: Option<bool>| (format!("%drawn::{key}"), visible);
    let without = [label("model", None), label("name", None), label("netname", None), label("value", None)];
    assert_eq!(hidden_v04(false), without);
    let with = [
        label("model", Some(false)),
        label("name", Some(false)),
        label("netname", Some(false)),
        label("value", Some(false)),
    ];
    assert_eq!(hidden_v04(true), with);
}
