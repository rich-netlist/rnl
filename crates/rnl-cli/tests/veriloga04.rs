//! Verilog-A files (2.9): in a file of 0.4, the Verilog-A file that Spectre's `ahdl_include` or
//! HSPICE's `.hdl` brings in is read for its `//` records. Each module is a level, known by its name
//! or by the name `-master` gives it, whose terminals are its ports, a vector port one terminal for
//! each bit from the left bound of its range to the right; its `# @SET-SYMBOL` gives the instances of
//! it their block symbol, and it holds nothing else: DEFINE-SYMBOL, DEFINE-STYLE and `# @SET-SYMBOL`;
//! outside its modules, a Verilog-A file holds only header records. A compiled model is not read,
//! and a file of 0.3 reads no Verilog-A file.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// The module `vco` of the example of 2.9, with its header record.
const VCO: &str = "// @RNL(type=\"header\") { @VERSION(\"0.4\")}\n`include \"disciplines.vams\"\n\
    module vco(vin, out, vdd, vss);\n  inout vin, out, vdd, vss;\n  electrical vin, out, vdd, vss;\n  parameter real f0 = 1e9;\n\
    // @RNL {\n//   %vco = @DEFINE-SYMBOL(box=[-3, -3, 3, 3]) {\n//     @RECTANGLE {(-2, -2) (2, 2)};\n\
    //     (-3, 0) @PIN(terminal=\"vin\", direction=\"left\");\n//     (3, 0) @PIN(terminal=\"out\", direction=\"right\");\n\
    //     (0, 3) @PIN(terminal=\"vdd\", direction=\"up\");\n//     (0, -3) @PIN(terminal=\"vss\", direction=\"down\")\n\
    //   };\n//   # @SET-SYMBOL %vco\n// }\n  analog begin\n    V(out) <+ 0;\n  end\nendmodule\n";

/// A module with vector ports of both directions, and no header record: it takes the version of the
/// file that brings it in.
const DAC: &str = "module dac(d, e, out);\n  inout [3:0] d;\n  inout [0:2] e;\n  inout out;\n  electrical [3:0] d;\n\
    electrical [0:2] e;\n  electrical out;\n// @RNL {\n//   %dac = @DEFINE-SYMBOL(box=[-3, -3, 3, 3]) {\n\
    //     @RECTANGLE {(-2, -2) (2, 2)};\n//     (-3, 1) @PIN(terminal=[\"d[3]\", \"d[2]\", \"d[1]\", \"d[0]\"], direction=\"left\");\n\
    //     (-3, -1) @PIN(terminal=[\"e[0]\", \"e[1]\", \"e[2]\"], direction=\"left\");\n\
    //     (3, 0) @PIN(terminal=\"out\", direction=\"right\")\n//   };\n//   # @SET-SYMBOL %dac\n// }\nendmodule\n";

fn read(main: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(main, text).with("vco.va", VCO).with("dac.va", DAC);
    interpret_file(Path::new(main), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

/// A Spectre netlist of `version`, with `lines` after its header record and `body` as its last record.
fn spectre(version: &str, lines: &str, body: &str) -> Interpretation {
    read(
        "t.scs",
        &format!("// t\n// @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n{lines}// @RNL {{ {body} }}\n"),
    )
}

fn of(r: &Interpretation, severity: Severity) -> Vec<(Code, String)> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).map(|d| (d.code, r.sink.render(d))).collect()
}

/// The name of the symbol each instance of the main level is drawn with, "-" for none.
fn symbol(r: &Interpretation, name: &str) -> String {
    let level = r.model.levels.iter().find(|l| l.title == "t.scs" || l.title == "t.sp").unwrap();
    let o = level.objects.iter().find(|o| o.kind == "instance" && o.id == format!("#{name}")).unwrap();
    o.symbol.as_ref().map_or("-".into(), |s| s.name.clone())
}

/// No error and no violation.
fn holds(r: &Interpretation) -> bool {
    of(r, Severity::Error).is_empty() && of(r, Severity::Violation).is_empty()
}

#[test]
fn a_module_that_ahdl_include_brings_in_gives_its_instances_its_block_symbol() {
    let lines = "ahdl_include \"vco.va\"\nI1 (ctl out vdd 0) vco f0=2e9\n";
    let body = "#I1:vin @ASSERT-TERMINAL(net=\"ctl\"); #I1:vss @ASSERT-TERMINAL(net=\"0\")";
    let r = spectre("0.4", lines, body);
    assert!(holds(&r), "{:?}", r.diagnostics().iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
    assert_eq!(symbol(&r, "I1"), "%vco");
    // the file level of the Verilog-A file is below the main file's, and the module below it
    let file = r.doc.levels.iter().position(|l| l.file == "vco.va" && l.kind == rnl_runtime::host::LevelKind::File);
    let module = r.doc.levels.iter().position(|l| l.name == "vco").unwrap();
    assert_eq!(r.doc.levels[module].parent, file);
    assert_eq!(r.doc.levels[file.unwrap()].parent, Some(0));
    assert!(!r.doc.levels[module].circuit);
    assert!(r.doc.is_module(module));
    assert_eq!(r.doc.levels[module].ports, ["vin", "out", "vdd", "vss"]);
    // the wrong net does not hold
    assert!(!holds(&spectre("0.4", lines, "#I1:vin @ASSERT-TERMINAL(net=\"out\")")));
}

#[test]
fn hdl_brings_in_a_module_with_one_terminal_for_each_bit_of_a_vector_port() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.hdl 'dac.va'\nX1 a3 a2 a1 a0 b0 b1 b2 y DAC\n\
                * @RNL { #X1:d[3] @ASSERT-TERMINAL(net=\"a3\"); #X1:d[0] @ASSERT-TERMINAL(net=\"a0\"); \
                #X1:e[0] @ASSERT-TERMINAL(net=\"b0\"); #X1:e[2] @ASSERT-TERMINAL(net=\"b2\"); #X1:out @ASSERT-TERMINAL(net=\"y\") }\n.end\n";
    let r = read("t.sp", text);
    assert!(holds(&r), "{:?}", r.diagnostics().iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
    // an instance of HSPICE names the module in any case; the names of its terminals keep theirs
    assert_eq!(symbol(&r, "X1"), "%dac");
    let module = r.doc.levels.iter().position(|l| l.name == "dac").unwrap();
    assert_eq!(r.doc.levels[module].ports, ["d[3]", "d[2]", "d[1]", "d[0]", "e[0]", "e[1]", "e[2]", "out"]);
    let r = read("t.sp", &text.replace("\"a3\"); #X1:d[0]", "\"a0\"); #X1:d[0]"));
    assert!(!holds(&r));
}

#[test]
fn master_gives_the_module_the_name_the_netlist_knows_it_by() {
    let lines = "ahdl_include \"vco.va\" -master myvco\nI1 (ctl out vdd 0) myvco\nI2 (a b c d) vco\n";
    let r = spectre("0.4", lines, "");
    assert_eq!(symbol(&r, "I1"), "%vco");
    assert_ne!(symbol(&r, "I2"), "%vco");
    let hints = of(&r, Severity::Hint);
    assert!(hints.iter().any(|(c, m)| *c == codes::SYMBOL_SUBCKT_UNDEFINED && m.contains("I2")), "{hints:?}");
}

#[test]
fn a_compiled_model_is_not_read_and_a_file_not_found_gives_a_warning() {
    let lines = "ahdl_include \"model.osdi\"\nahdl_include \"missing.va\"\nI1 (a b) model\n";
    let r = spectre("0.4", lines, "");
    assert_eq!(of(&r, Severity::Error), Vec::new());
    let warnings = of(&r, Severity::Warning);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].0, codes::HOST_VERILOGA_NOT_FOUND);
    assert!(warnings[0].1.contains("t.scs:4"), "{warnings:?}");
    assert_eq!(r.doc.sources.len(), 1);
}

#[test]
fn a_file_of_0_3_reads_no_verilog_a_file() {
    let lines = "ahdl_include \"vco.va\"\nI1 (ctl out vdd 0) vco f0=2e9\n";
    let r = spectre("0.3", lines, "");
    assert_eq!(r.doc.sources.len(), 1);
    assert!(r.doc.levels.iter().all(|l| l.file != "vco.va"));
    assert_ne!(symbol(&r, "I1"), "%vco");
    let r = read(
        "t.sp",
        "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.hdl 'dac.va'\nX1 a3 a2 a1 a0 b0 b1 b2 y dac\n.end\n",
    );
    assert_eq!(r.doc.sources.len(), 1);
}

/// The codes of the errors of `r` with their lines in the Verilog-A file `file`.
fn errors_in(r: &Interpretation, file: &str) -> Vec<(Code, u32)> {
    r.diagnostics()
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.code, d.loc.as_ref().filter(|l| l.file == file).map_or(0, |l| l.line)))
        .collect()
}

/// The module `m` of `m.va`, whose record, on lines 4 to 9, writes `body` on line 8, after a style,
/// its symbol and its block symbol; `before` is written before the module, and `after` after its
/// record, from line 10.
fn module(before: &str, body: &str, after: &str) -> Interpretation {
    let va = format!(
        "// @RNL(type=\"header\") {{ @VERSION(\"0.4\")}}\n{before}module m(a, b);\n  inout a, b;\n// @RNL {{\n\
         //   $ST = @DEFINE-STYLE(linecolor=\"#123456\") {{}};\n\
         //   %s = @DEFINE-SYMBOL(box=[-2, -2, 2, 2]) {{ @RECTANGLE {{(-1, -1) (1, 1)}}; \
         (-2, 0) @PIN(terminal=\"a\", direction=\"left\"); (2, 0) @PIN(terminal=\"b\", direction=\"right\") }};\n\
         //   # @SET-SYMBOL %s;\n//   {body}\n// }}\n{after}endmodule\n"
    );
    let files = MemoryProvider::new()
        .with("t.scs", "// t\n// @RNL(type=\"header\") { @VERSION(\"0.4\")}\nahdl_include \"m.va\"\nI1 (x y) m\n")
        .with("m.va", &va);
    interpret_file(Path::new("t.scs"), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

#[test]
fn a_module_holds_only_symbols_styles_and_its_block_symbol() {
    let r = module("", "%t = @DEFINE-SYMBOL { (0, 0) @CIRCLE(radius=1) }", "");
    assert_eq!(errors_in(&r, "m.va"), Vec::new());
    assert_eq!(symbol(&r, "I1"), "%s");
    // anything else fails, with an error at the statement, and the block symbol still holds
    for body in [
        "{#R1 #R2} @ALIGN(\"y\")",
        "(0, 0) @TEXT(\"x\")",
        "%u = %s",
        "@USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\")",
        "# @SELECT-INSTANCE(name=\"*\") => @SET-SYMBOL %s",
        "&a @CURRENT(\"in\")",
    ] {
        let r = module("", body, "");
        assert_eq!(errors_in(&r, "m.va"), [(codes::KEYWORD_NOT_IN_MODULE, 8)], "{body}");
        assert_eq!(symbol(&r, "I1"), "%s", "{body}");
    }
}

#[test]
fn a_record_of_a_module_with_scope_or_of_the_solver_fails() {
    for header in ["(scope=\"m\")", "(type=\"generated\")"] {
        let r = module("", "", &format!("// @RNL{header} {{ $X = @DEFINE-STYLE {{}} }}\n"));
        assert_eq!(errors_in(&r, "m.va"), [(codes::KEYWORD_NOT_IN_MODULE, 10)], "{header}");
    }
}

#[test]
fn outside_its_modules_a_verilog_a_file_holds_only_header_records() {
    let r = module("// @RNL { $S0 = @DEFINE-STYLE {} }\n", "", "");
    assert_eq!(errors_in(&r, "m.va"), [(codes::HOST_VERILOGA_FILE_RECORD, 2)]);
    assert_eq!(symbol(&r, "I1"), "%s");
}
