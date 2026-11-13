use super::*;
use rnl_runtime::diag::Severity;
use rnl_runtime::markup;
use rnl_runtime::model::Attach;
use rnl_runtime::Options;
use serde_json::json;

use crate::ops::{Run, RunStyle};

fn request(dialect: &str, netlist: &str, ops: Value) -> WriteRequest {
    WriteRequest {
        language_version: None,
        dialect: dialect.into(),
        netlist: netlist.into(),
        author: "test".into(),
        ops: ops.as_array().unwrap().clone(),
        file: None,
    }
}

fn styled(text: &str, level: i32, bold: bool, italic: bool) -> Value {
    json!({"kind": "text", "text": text, "style": {"level": level, "bold": bold, "italic": italic, "overline": false}})
}

const RC: &str = "\n.subckt rc_lowpass VSS\nC1 out VSS 10n\nR1 in out 10k\n.ends rc_lowpass\n";

#[test]
fn a_named_pin_vertex_writes_its_reference_without_turning_it_into_a_terminal() {
    let source = format!("* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n{RC}");
    let ops = json!([{"level": "subckt:rc_lowpass", "op": "add-wire", "nets": ["&in"], "mode": "straight",
        "via": [{"owner": "#R1", "pin": 0, "reference": "#R1::p"}, {"at": [4, 0]}]}]);
    let written = write(&request("spice", &source, ops)).unwrap();
    assert!(written.contains("#R1::p"), "{written}");
    assert!(!written.contains("#R1:p"), "{written}");
}

#[test]
fn every_op_into_a_spice_subcircuit() {
    let level = "subckt:rc_lowpass";
    let ops = json!([
        {"level": level, "op": "use-package", "name": "rnl-std", "version": "0.3"},
        {"level": level, "op": "use-package", "name": "rnl-std-no-labels", "version": "0.3"},
        {"level": level, "op": "use-package", "name": "rnl-std", "version": "0.3"},
        {"level": level, "op": "set", "object": "#R1", "symbol": "$RES", "at": [19, -16], "orient": "R90"},
        {"level": "subckt:RC_LOWPASS", "op": "set", "object": "#C1", "at": [29, -22.0000001], "orient": "R0"},
        {"level": level, "op": "add-usage", "name": "%gnd1", "definition": "$GND", "attach": {"p": "&VSS"},
         "symbol": "$GROUND", "at": [10, -25], "orient": "R0"},
        {"level": level, "op": "add-point", "name": "%j1", "at": [11, 2], "junction": true},
        {"level": level, "op": "remove-from", "items": ["&VSS"], "set": "$PORT_FILL_SET"},
        {"level": level, "op": "add-wire", "nets": ["&in"], "mode": "straight",
         "via": [{"terminal": "#R1:1"}, {"at": [10, -16]}, {"point": "%j1"}, {"usage": "%gnd1", "role": "p"}]},
        {"level": level, "op": "add-text", "attach": "#R1", "key": "name",
         "lines": [[styled("R", 0, false, true), styled("1", -1, false, false)]],
         "offset": [1, -0.5], "halign": "start", "size": 1.5116},
        {"level": level, "op": "add-text", "at": [4.5, -5], "size": 1.5116,
         "lines": [[styled("a_b {c} $x% & # ^ \\ \"q\"", 0, true, false)]]},
        {"level": level, "op": "add-text", "attach": "&out", "at": [26, -15], "markup": "\\textit{out}", "size": 1.5116}
    ]);
    let out = write(&request("spice", RC, ops)).unwrap();
    assert_eq!(
        out,
        r#"
* @RNL(type="header") {
*   @VERSION("0.3");
*   @USE-PACKAGE(name="rnl-std", version="0.3");
*   @USE-PACKAGE(name="rnl-std-no-labels", version="0.3")
* }
.subckt rc_lowpass VSS
C1 out VSS 10n
R1 in out 10k
* @RNL(author="test") {
*   #R1 @SET-SYMBOL $RES => @PLACE(19, -16) => @ORIENT("R90");
*   #C1 @PLACE(29, -22) => @ORIENT("R0");
*   $GND @ATTACH &VSS(p) => @SET-SYMBOL $GROUND => @PLACE(10, -25) => @ORIENT("R0") => %gnd1;
*   %j1 = (11, 2);
*   $JUNCTION @ATTACH %j1;
*   &VSS @REMOVE-FROM $PORT_FILL_SET;
*   &in @WIRE(mode="straight") {#R1:1 (10, -16) %j1 %gnd1:p};
*   #R1 @TEXT("\textit{R}_{1}", key="name", dx=1, dy=-0.5, halign="start", size=1.5116);
*   (4.5, -5) @TEXT("\textbf{a\_b \{c\} \$x\% \& \# \^ \textbackslash{} ""q""}", size=1.5116);
*   &out @TEXT("\textit{out}", size=1.5116) => @PLACE(26, -15)
* }
.ends rc_lowpass
"#
    );
}

#[test]
fn spectre_with_its_carrier_and_a_short_header() {
    let net = "simulator lang=spectre\n\nsubckt top (a b)\nR1 (a b) resistor r=1k\nends top\n";
    let ops = json!([
        {"level": "subckt:top", "op": "use-package", "name": "rnl-std", "version": "0.3"},
        {"level": "subckt:top", "op": "set", "object": "#R1", "symbol": "$RES", "at": [0, 0]}
    ]);
    let out = write(&request("spectre", net, ops)).unwrap();
    assert_eq!(
        out,
        "simulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")}\n\nsubckt top (a b)\nR1 (a b) resistor r=1k\n// @RNL(author=\"test\") {\n//   #R1 @SET-SYMBOL $RES => @PLACE(0, 0)\n// }\nends top\n"
    );
    // Spectre names are case-sensitive
    let wrong = json!([{"level": "subckt:TOP", "op": "set", "object": "#R1", "at": [0, 0]}]);
    assert_eq!(write(&request("spectre", net, wrong)).unwrap_err(), "op 0 (set): the netlist has no subcircuit TOP");
}

#[test]
fn the_file_level_line_endings_and_a_missing_last_newline() {
    let net = "* title\r\nR1 a 0 1k\r\n.end";
    let ops = json!([{"level": "file:top.sp", "op": "set", "object": "#R1", "at": [1, 2]}]);
    let out = write(&request("spice", net, ops)).unwrap();
    assert_eq!(
        out,
        "* title\r\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\r\nR1 a 0 1k\r\n* @RNL(author=\"test\") {\r\n*   #R1 @PLACE(1, 2)\r\n* }\r\n.end"
    );
    // without .end the record goes at the end of the file
    let out = write(&request(
        "spice",
        "* title\nR1 a 0 1k",
        json!([{"level": "file:x", "op": "set", "object": "#R1", "at": [0, 0]}]),
    ))
    .unwrap();
    assert_eq!(out, "* title\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n* @RNL(author=\"test\") {\n*   #R1 @PLACE(0, 0)\n* }\n");
}

#[test]
fn the_header_goes_after_what_continues_line_one() {
    let out = write(&request("spice", "title\n+ more of it\nR1 a 0 1k\n", json!([]))).unwrap();
    assert_eq!(out, "title\n+ more of it\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 a 0 1k\n");
    let out =
        write(&request("spectre", "simulator lang=spectre \\\n  insensitive=no\nR1 (a b) resistor r=1k\n", json!([])))
            .unwrap();
    assert_eq!(out, "simulator lang=spectre \\\n  insensitive=no\n// @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR1 (a b) resistor r=1k\n");
    // an empty netlist still keeps line 1 for the title
    assert_eq!(write(&request("spice", "", json!([]))).unwrap(), "\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n");
}

#[test]
fn a_netlist_saved_under_a_name_is_read_as_that_file() {
    // a Spectre netlist that is not a .scs file is read in the SPICE mode up to its `simulator
    // lang=spectre` (2.2): its header goes right after that statement when it is the first, with
    // `//`, and otherwise above it with `*`; the records follow the header's marker (16.3)
    let ops = json!([{"level": "file", "op": "set", "object": "#R1", "at": [0, 0]}]);
    let saved = |netlist: &str, file: Option<&str>| {
        write(&WriteRequest { file: file.map(String::from), ..request("spectre", netlist, ops.clone()) }).unwrap()
    };
    let header = "@RNL(type=\"header\") { @VERSION(\"0.3\")}";
    let record = |m: &str| format!("{m} @RNL(author=\"test\") {{\n{m}   #R1 @PLACE(0, 0)\n{m} }}\n");
    let first = "* title\nsimulator lang=spectre\nR1 (a 0) resistor r=1k\n";
    assert_eq!(
        saved(first, Some("top.net")),
        format!("* title\nsimulator lang=spectre\n// {header}\nR1 (a 0) resistor r=1k\n{}", record("//"))
    );
    let later = "* title\n.param rr=1k\nsimulator lang=spectre\nR1 (a 0) resistor r=1k\n";
    assert_eq!(
        saved(later, Some("dir/top.net")),
        format!("* title\n* {header}\n.param rr=1k\nsimulator lang=spectre\nR1 (a 0) resistor r=1k\n{}", record("*"))
    );
    // a .scs file, or a netlist without a name, as before: in the Spectre mode from its start
    for file in [Some("top.scs"), None] {
        assert_eq!(
            saved(first, file),
            format!("* title\n// {header}\nsimulator lang=spectre\nR1 (a 0) resistor r=1k\n{}", record("//"))
        );
    }
    // read back under its name, each has its header and its record
    for net in [first, later] {
        let out = saved(net, Some("top.net"));
        let files = MemoryProvider::new().with("/rnl-edit/top.net", &out);
        let doc = rnl_view::read_host("/rnl-edit/top.net", Some("spectre"), &files, &mut Sink::new()).unwrap();
        assert_eq!(doc.levels[0].records.len(), 2, "{out}");
    }
    let named = |file: &str| entry_of(&WriteRequest { file: Some(file.into()), ..request("spectre", "", json!([])) });
    assert_eq!((named("C:\\work\\top.net"), named("")), ("/rnl-edit/top.net".into(), "/rnl-edit/netlist.scs".into()));
}

#[test]
fn long_statements_break_at_spaces_outside_strings_and_brackets() {
    let via: Vec<Value> = (0..30).map(|k| json!({"at": [k as f64 * 1.25, -(k as f64) * 0.5]})).collect();
    let ops = json!([
        {"level": "subckt:rc_lowpass", "op": "add-wire", "nets": ["&in"], "via": via, "mode": "straight"},
        {"level": "subckt:rc_lowpass", "op": "add-text", "at": [0, 0], "markup": "x ".repeat(80), "size": 1}
    ]);
    let out = write(&request("spice", RC, ops)).unwrap();
    let record: Vec<&str> = out.lines().filter(|l| l.starts_with('*') && !l.contains("type=\"header\"")).collect();
    // every line fits, but the one with the text's string, which cannot break
    for l in record.iter().filter(|l| !l.contains("x x")) {
        assert!(l.chars().count() <= LINE, "{out}");
    }
    // the wire's lines: the first, then continuations indented by four more
    assert!(record[1].starts_with("*   &in @WIRE(mode=\"straight\") {(0, 0)"), "{out}");
    assert!(record[2].starts_with("*       ("), "{out}");
    // the text's string is never broken
    assert!(record.iter().any(|l| l.contains(&format!("\"{}\"", "x ".repeat(80)))), "{out}");
    // joined again, the statements are as they were written
    let joined: String = record.iter().map(|l| l[1..].trim()).collect::<Vec<_>>().join(" ");
    assert!(joined.contains("(1.25, -0.5) (2.5, -1)"), "{joined}");
}

#[test]
fn an_edit_on_a_file_that_is_not_a_netlist_is_refused() {
    // a Verilog-A model, which the core reads as text: a viewer's edit of it carries that dialect
    let va = "module res(p, n);\n  electrical p, n;\n  analog V(p, n) <+ 1k * I(p, n);\nendmodule\n";
    let ops = json!([{"level": "file:res.va", "op": "add-text", "at": [0, 0], "markup": "x"}]);
    let r = WriteRequest { file: Some("res.va".into()), ..request("text", va, ops) };
    assert_eq!(
        write(&r).unwrap_err(),
        "dialect \"text\" is that of a file that is not a netlist; RNL is written only into a netlist"
    );
}

#[test]
fn an_edit_on_a_package_file_is_refused_and_a_netlist_named_rnl_is_written() {
    let package = "@RNL(type=\"header\") { @VERSION(\"0.3\"); @PACKAGE(name=\"acme\", version=\"1.0\")}\n";
    let ops = json!([{"level": "file:acme.pkg", "op": "add-text", "at": [0, 0], "markup": "x"}]);
    let r = WriteRequest { file: Some("acme.pkg".into()), ..request("rnl", package, ops) };
    assert_eq!(
        write(&r).unwrap_err(),
        "dialect \"rnl\" is that of a package file (2.7), which is not a netlist; RNL is written here only into a netlist"
    );
    // a netlist saved under a `.rnl` name, read as SPICE as the request says, is carried in comments
    let ops = json!([{"level": "subckt:rc_lowpass", "op": "set", "object": "#R1", "at": [0, 0]}]);
    let r = WriteRequest { file: Some("rc.rnl".into()), ..request("spice", RC, ops) };
    let out = write(&r).unwrap();
    assert!(out.contains("\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n"), "{out}");
    assert!(out.contains("\n* @RNL(author="), "{out}");
}

#[test]
fn refusals_name_the_op() {
    let refused = |ops: Value| write(&request("spice", RC, ops)).unwrap_err();
    assert_eq!(
        refused(json!([{"level": "subckt:x", "op": "set", "object": "#R1", "at": [0, 0]}])),
        "op 0 (set): the netlist has no subcircuit x"
    );
    assert_eq!(
        refused(json!([{"level": "top", "op": "set", "object": "#R1", "at": [0, 0]}])),
        "op 0 (set): level \"top\" is not subckt:NAME or file:PATH"
    );
    assert_eq!(refused(json!([{"level": "subckt:rc_lowpass", "op": "rotate"}])), "op 0: unknown op \"rotate\"");
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "remove", "object": "#R1"}])),
        "op 0 (remove): not written yet"
    );
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "set", "object": "#R1", "style": {}}])),
        "op 0 (set): nothing to set: no `symbol`, `style`, `at` or `orient`"
    );
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "set", "object": "#R1"}])),
        "op 0 (set): nothing to set: no `symbol`, `style`, `at` or `orient`"
    );
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "set", "object": "#R1", "at": [0]}])),
        "op 0 (set): `at` is not a point [x, y]"
    );
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "add-point", "name": "%p", "at": [0, 0], "colour": 1}])),
        "op 0 (add-point): unknown field `colour`"
    );
    assert_eq!(refused(json!([{"op": "set", "object": "#R1", "at": [0, 0]}])), "op 0 (set): no `level`");
    assert_eq!(
        refused(
            json!([{"level": "subckt:rc_lowpass", "op": "add-wire", "nets": ["&in"], "via": [{"terminal": "#R1:1"}, {"pin": "#R1:2"}]}])
        ),
        "op 0 (add-wire): `via` vertex 1 is not one of {terminal}, {owner, pin, reference}, {point}, {usage, role}, {at}"
    );
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "add-text", "markup": "x"}])),
        "op 0 (add-text): neither `attach` nor `at`: the text has no place"
    );
    let mut r = request("spice", RC, json!([]));
    r.dialect = "verilog".into();
    assert_eq!(write(&r).unwrap_err(), "dialect \"verilog\" is not spice or spectre");
    let with_record = "* t\nR1 a 0 1k\n* @RNL {#R1 @PLACE(0, 0)}\n";
    assert_eq!(
        write(&request("spice", with_record, json!([]))).unwrap_err(),
        "the netlist has RNL records already; this writer writes only into a netlist without them"
    );
}

#[test]
fn a_record_is_not_written_in_a_region_written_to_be_encrypted() {
    // a subcircuit written in `.prot … .unprot` has no place for a record that is not gone once the
    // region is encrypted (2.2): refused; the file level ends after the region
    let net = "* t\n.prot\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.unprot\nX1 n1 n2 cell\n.end\n";
    let refused =
        write(&request("spice", net, json!([{"level": "subckt:cell", "op": "set", "object": "#R1", "at": [0, 0]}])));
    assert_eq!(
        refused.unwrap_err(),
        "op 0 (set): cell lies in a region written to be encrypted, lines 2 to 6, where a record is gone once the region \
         is encrypted (2.2)"
    );
    let out =
        write(&request("spice", net, json!([{"level": "file", "op": "set", "object": "#X1", "at": [0, 0]}]))).unwrap();
    assert!(
        out.ends_with(".unprot\nX1 n1 n2 cell\n* @RNL(author=\"test\") {\n*   #X1 @PLACE(0, 0)\n* }\n.end\n"),
        "{out}"
    );
    // a subcircuit whose `.ends` lies in a region that begins inside it has its record before the
    // region, and the file level whose `.end` lies in it has its record before the subcircuit: the
    // levels are written from the bottom up, by these places
    let net = "* t\nX1 n1 n2 cell\n.subckt cell a b\nR1 a b 1k\n.prot\nR2 a b 1k\n.ends cell\n.end\n";
    let ops = json!([
        {"level": "subckt:cell", "op": "set", "object": "#R1", "at": [0, 0]},
        {"level": "file", "op": "set", "object": "#X1", "at": [0, 0]}
    ]);
    let out = write(&request("spice", net, ops)).unwrap();
    let record = |name: &str| format!("* @RNL(author=\"test\") {{\n*   #{name} @PLACE(0, 0)\n* }}\n");
    let (x1, r1) = (record("X1"), record("R1"));
    let want = format!("X1 n1 n2 cell\n{x1}.subckt cell a b\nR1 a b 1k\n{r1}.prot\nR2 a b 1k\n.ends cell\n.end\n");
    assert!(out.ends_with(&want), "{out}");
}

#[test]
fn in_a_spice_section_of_a_spectre_netlist_a_record_is_carried_by_a_star() {
    // after `simulator lang=spice`, `//` is no comment and only `*` carries (2.2)
    let net = "// t\nsimulator lang=spice\n.subckt cell a b\nR1 a b 1k\n.ends cell\nsimulator lang=spectre\nX1 (n1 n2) cell\n";
    let ops = json!([
        {"level": "subckt:cell", "op": "set", "object": "#R1", "at": [0, 0]},
        {"level": "file", "op": "set", "object": "#X1", "at": [0, 0]}
    ]);
    let out = write(&request("spectre", net, ops)).unwrap();
    assert_eq!(
        out,
        "// t\n// @RNL(type=\"header\") { @VERSION(\"0.3\")}\nsimulator lang=spice\n.subckt cell a b\nR1 a b 1k\n\
         * @RNL(author=\"test\") {\n*   #R1 @PLACE(0, 0)\n* }\n.ends cell\nsimulator lang=spectre\nX1 (n1 n2) cell\n\
         // @RNL(author=\"test\") {\n//   #X1 @PLACE(0, 0)\n// }\n"
    );
    // read back, each record is where it was written
    let files = MemoryProvider::new().with(entry("spectre"), &out);
    let doc = rnl_view::read_host(entry("spectre"), Some("spectre"), &files, &mut Sink::new()).unwrap();
    let records: Vec<usize> = doc.levels.iter().map(|lv| lv.records.len()).collect();
    assert_eq!(records, [2, 1]);
}

/// The runtime's runs as this crate's, to compare what the markup reads back as.
fn ours(runs: &[markup::Run]) -> Vec<Run> {
    let style = |s: &markup::RunStyle| RunStyle {
        level: s.level as i32,
        bold: s.bold,
        italic: s.italic,
        upright: s.upright,
        overline: s.overline,
    };
    runs.iter()
        .map(|r| match r {
            markup::Run::Text { text, style: s } => Run::Text { text: text.clone(), style: style(s) },
            markup::Run::Frac { num, den, style: s } => Run::Frac { num: ours(num), den: ours(den), style: style(s) },
            markup::Run::Sqrt { radicand, style: s } => Run::Sqrt { radicand: ours(radicand), style: style(s) },
        })
        .collect()
}

#[test]
fn markup_reads_back_as_the_runs_it_was_written_from() {
    let s = |level, bold, italic, overline| RunStyle { level, bold, italic, overline, upright: false };
    let t = |text: &str, st| Run::Text { text: text.into(), style: st };
    let lines = vec![
        vec![
            t("V", s(0, false, true, false)),
            t("out", s(-1, false, false, false)),
            t(" = ", s(0, false, false, false)),
        ],
        vec![
            t("g", s(0, true, true, false)),
            t("m", s(-1, true, false, false)),
            t("x", s(2, false, false, true)),
            Run::Frac {
                num: vec![t("1", s(1, false, false, false))],
                den: vec![t("s", s(1, false, true, false)), t("2", s(2, false, true, false))],
                style: s(1, false, false, false),
            },
            t(r"a\b_c{d}$e%f&g#h^{i}", s(0, false, false, false)),
        ],
    ];
    let written = form::markup(&lines);
    let read: Vec<Vec<Run>> = markup::parse(&written).iter().map(|l| ours(l)).collect();
    assert_eq!(read, lines, "{written}");
}

#[test]
fn structured_math_preserves_roots_scripts_fractions_and_upright_runs() {
    for source in [r"\(g_m=2\mathrm{V}\)", r"\(\sqrt{x^2+\frac{a}{b}}\)", r"\textbf{\(\sqrt{x_1}\)}"] {
        let lines: Vec<_> = markup::parse_with_name_formats(source, true).iter().map(|l| ours(l)).collect();
        let content = crate::ops::TextContent::Lines(lines.clone());
        form::check_math_content(&content, "0.4").unwrap();
        assert!(form::check_math_content(&content, "0.3").unwrap_err().contains("language_version 0.4"));
        assert!(markup::problems_with_name_formats(&form::markup(&lines), true).is_empty());
    }
}

#[test]
fn structured_math_refuses_to_drop_literal_spaces_or_styles() {
    let input = markup::expand_math(r"\(\sqrt{${value}}\)", |_| "A B".into());
    let lines = markup::parse_with_name_formats(&input, true).iter().map(|l| ours(l)).collect();
    let error = form::check_math_content(&crate::ops::TextContent::Lines(lines), "0.4").unwrap_err();
    assert!(error.contains("original markup"));
}

#[test]
fn structured_math_json_requires_draft_output_and_writes_a_formula() {
    let ops = json!([{"op":"add-text", "level":"file", "at":[0,0], "lines":[[
        {"kind":"sqrt", "radicand":[
            {"kind":"text", "text":"x", "style":{"italic":true}},
            {"kind":"text", "text":"2", "style":{"level":1,"upright":true}}
        ]}
    ]]}]);
    let mut req = request("spice", "\nR1 a b 1k\n", ops);
    assert!(write(&req).unwrap_err().contains("language_version 0.4"));
    req.language_version = Some("0.4".into());
    let written = write(&req).unwrap();
    assert!(written.contains(r"\(\sqrt{x^{\mathrm{2}}}\)"), "{written}");
    assert!(written.contains("@VERSION(\"0.4\")"));
}

#[test]
fn what_it_writes_interprets_without_errors() {
    let net = "\n.subckt amp in out VSS\nR1 in out 10k\nC1 out VSS 1p\n.ends amp\n";
    let level = "subckt:amp";
    let ops = json!([
        {"level": level, "op": "set", "object": "#R1", "symbol": "$RESISTOR", "at": [4, 0], "orient": "R90"},
        {"level": level, "op": "set", "object": "#C1", "at": [8, -4]},
        {"level": level, "op": "add-usage", "name": "%gnd1", "definition": "$GROUND", "attach": {"p": "&VSS"}, "at": [8, -8]},
        {"level": level, "op": "remove-from", "items": ["&VSS"], "set": "$PORT_FILL_SET"},
        {"level": level, "op": "add-wire", "nets": ["&out"], "mode": "straight", "via": [{"terminal": "#R1:2"}, {"at": [8, 0]}, {"terminal": "#C1:1"}]},
        {"level": level, "op": "add-wire", "nets": ["&VSS"], "mode": "straight", "via": [{"terminal": "#C1:2"}, {"usage": "%gnd1", "role": "p"}]},
        {"level": level, "op": "add-text", "attach": "#R1", "key": "name", "lines": [[styled("R", 0, false, true), styled("1", -1, false, false)]], "offset": [1, 0], "size": 1.2},
        {"level": level, "op": "add-text", "at": [0, 4], "lines": [[styled("gain \\frac{1}{2}", 0, true, false)]], "size": 1}
    ]);
    let out = write(&request("spice", net, ops)).unwrap();
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
        .unwrap();
    let loud: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity != Severity::Hint).map(|d| r.sink.render(d)).collect();
    assert!(loud.is_empty(), "{out}\n{}", loud.join("\n"));
}

/// How the model draws the label `name` of the usage `%p1`, a port, after the ops are written and
/// read: its alignments and rotation, and the statement that wrote it.
fn port_label(label: Option<Value>) -> (String, String, f64, String) {
    let net = "* port\nR1 a 0 1k\n.end\n";
    let mut ops = vec![
        json!({"level": "file", "op": "add-usage", "name": "%p1", "definition": "$IO_PORT", "attach": {"p": "&a"}, "at": [0, 0]}),
    ];
    ops.extend(label);
    let out = write(&request("spice", net, Value::Array(ops))).unwrap();
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
        .unwrap();
    let loud: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity != Severity::Hint).map(|d| r.sink.render(d)).collect();
    assert!(loud.is_empty(), "{out}\n{}", loud.join("\n"));
    let texts = r.model.levels.iter().flat_map(|l| &l.texts);
    let name = texts.into_iter().find(|t| t.attach.as_deref() == Some("%p1") && t.key.as_deref() == Some("name"));
    let t = name.unwrap_or_else(|| panic!("the port has no label `name`:\n{out}"));
    let written = out.lines().find(|l| l.contains("@TEXT")).unwrap_or_default().trim_start_matches('*').trim();
    (t.halign.clone(), t.valign.clone(), t.rotate, written.to_string())
}

#[test]
fn a_text_with_a_key_keeps_the_alignment_and_rotation_it_was_given() {
    // the label `name` of a port is the symbol's: right-aligned and centred vertically. A statement
    // with the key overrides it parameter by parameter, and a parameter it leaves out stays the
    // symbol's (13.3, 8.4), so a request that gives start, baseline and no rotation, which are the
    // defaults of a text without a key, has to have them written
    let label = |extra: Value| {
        let mut op = json!({"level": "file", "op": "add-text", "attach": "%p1", "key": "name", "markup": "a"});
        op.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        Some(op)
    };
    // nothing given: the symbol's
    let (h, v, r, _) = port_label(label(json!({})));
    assert_eq!((h.as_str(), v.as_str(), r), ("end", "middle", 0.0));
    let (h, v, r, written) = port_label(label(json!({"halign": "start", "valign": "baseline", "rotate": 0})));
    assert_eq!((h.as_str(), v.as_str(), r), ("start", "baseline", 0.0), "{written}");
    assert!(written.contains(r#"key="name", halign="start", valign="baseline", rotate=0)"#), "{written}");
    // each on its own: the others stay the symbol's
    let (h, v, r, _) = port_label(label(json!({"halign": "start"})));
    assert_eq!((h.as_str(), v.as_str(), r), ("start", "middle", 0.0));
    let (h, v, r, _) = port_label(label(json!({"valign": "baseline"})));
    assert_eq!((h.as_str(), v.as_str(), r), ("end", "baseline", 0.0));
    let (h, v, r, _) = port_label(label(json!({"rotate": 0})));
    assert_eq!((h.as_str(), v.as_str(), r), ("end", "middle", 0.0));
    // other values, as before
    let (h, v, r, _) = port_label(label(json!({"halign": "middle", "valign": "top", "rotate": 90})));
    assert_eq!((h.as_str(), v.as_str(), r), ("middle", "top", 90.0));
}

#[test]
fn a_text_without_a_key_leaves_out_what_is_the_default() {
    // a text without a key starts from the defaults and has nothing to inherit: start, baseline and no
    // rotation are what it is without them, so they are left out; another value is written
    let net = "* t\nR1 a 0 1k\n.end\n";
    let text = |extra: Value| {
        let mut op = json!({"level": "file", "op": "add-text", "at": [1, 2], "markup": "x"});
        op.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let out = write(&request("spice", net, json!([op]))).unwrap();
        out.lines().find(|l| l.contains("@TEXT")).unwrap_or_default().trim_start_matches('*').trim().to_string()
    };
    assert_eq!(text(json!({"halign": "start", "valign": "baseline", "rotate": 0})), r#"(1, 2) @TEXT("x")"#);
    assert_eq!(
        text(json!({"halign": "end", "valign": "top", "rotate": 90})),
        r#"(1, 2) @TEXT("x", halign="end", valign="top", rotate=90)"#
    );
}

#[test]
fn a_usage_reads_back_with_the_roles_of_the_request() {
    // by order alone the objects would fill the roles of $DIFFERENTIAL_PAIR in its own order, in_p
    // then in_n, whatever roles the request gave them (9.4); with its role, each fills its own
    let net = "* pair\nM1 a g 0 0 nch\nM2 b g 0 0 nch\n.end\n";
    let pair = |attach: Value| {
        request(
            "spice",
            net,
            json!([{"level": "file", "op": "add-usage", "name": "%m", "definition": "$DIFFERENTIAL_PAIR", "attach": attach}]),
        )
    };
    let read = |out: &str| {
        let files = MemoryProvider::new().with(entry("spice"), out);
        rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
            .unwrap()
    };
    let errors = |r: &rnl_runtime::Interpretation| -> Vec<String> {
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
    };
    for (attach, written) in [
        (json!({"in_n": "#M2", "in_p": "#M1"}), "$DIFFERENTIAL_PAIR @ATTACH #M2(in_n) #M1(in_p) => %m"),
        (json!({"in_p": "#M1", "in_n": "#M2"}), "$DIFFERENTIAL_PAIR @ATTACH #M1(in_p) #M2(in_n) => %m"),
    ] {
        let out = write(&pair(attach)).unwrap();
        assert!(out.contains(&format!("*   {written}\n")), "{out}");
        let r = read(&out);
        assert!(errors(&r).is_empty(), "{out}\n{}", errors(&r).join("\n"));
        let m = r.model.levels.iter().flat_map(|lv| &lv.objects).find(|o| o.id == "%m").unwrap();
        let filled = |role: &str| m.attach.iter().find(|(r, _)| r == role).map(|(_, a)| a.clone());
        assert_eq!(filled("in_p"), Some(Attach::Object("#M1".into())), "{out}");
        assert_eq!(filled("in_n"), Some(Attach::Object("#M2".into())), "{out}");
    }
    // a role the definition lacks fails when the netlist is read, instead of filling another
    let out = write(&pair(json!({"in": "#M1", "in_n": "#M2"}))).unwrap();
    assert!(out.contains("*   $DIFFERENTIAL_PAIR @ATTACH #M1(in) #M2(in_n) => %m\n"), "{out}");
    let r = read(&out);
    assert!(
        errors(&r).iter().any(|e| e.contains("the ordinary definition $DIFFERENTIAL_PAIR has no role in (9.4)")),
        "{out}"
    );
    // a role that is not an ASCII name is written in quotes, the name itself (16.5); one that no
    // name can be is refused
    let out = write(&pair(json!({"1st": "#M1", "the out": "#M2"}))).unwrap();
    assert!(out.contains(r#"*   $DIFFERENTIAL_PAIR @ATTACH #M1("1st") #M2("the out") => %m"#), "{out}");
    assert_eq!(write(&pair(json!({"": "#M1"}))).unwrap_err(), r#"op 0 (add-usage): `attach` role "" is not a name"#);
}

/// The netlist of the tests of roles: four transistors and nothing of RNL.
const FOUR: &str = "* roles\nM1 a g 0 0 nch\nM2 b g 0 0 nch\nM3 a g 0 0 nch\nM4 b g 0 0 nch\n.end\n";

/// Read a netlist written into FOUR, with `record` put before what was written; what the usage %m
/// is attached to, role by role.
fn read_roles(out: &str, record: &str) -> (Vec<String>, Vec<(String, Attach)>) {
    let text = out.replacen("\n* @RNL(author=", &format!("\n{record}\n* @RNL(author="), 1);
    let files = MemoryProvider::new().with(entry("spice"), &text);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
        .unwrap();
    let errors = r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    let m = r.model.levels.iter().flat_map(|lv| &lv.objects).find(|o| o.id == "%m").unwrap();
    (errors, m.attach.clone())
}

#[test]
fn a_set_role_is_written_with_its_members_in_braces() {
    // ref and out of $CURRENT_MIRROR are set roles (9.4): a list of members, none, one or several
    let mirror = |attach: Value| {
        request(
            "spice",
            FOUR,
            json!([{"level": "file", "op": "add-usage", "name": "%m", "definition": "$CURRENT_MIRROR", "attach": attach}]),
        )
    };
    let out = write(&mirror(json!({"ref": ["#M1", "#M3"], "out": ["#M2"]}))).unwrap();
    assert!(out.contains("*   $CURRENT_MIRROR @ATTACH {#M1 #M3}(ref) {#M2}(out) => %m\n"), "{out}");
    let (errors, attach) = read_roles(&out, "");
    assert!(errors.is_empty(), "{out}\n{errors:?}");
    let set = |ids: &[&str]| Attach::Set(ids.iter().map(|id| Attach::Object(id.to_string())).collect());
    assert_eq!(attach, [("ref".to_string(), set(&["#M1", "#M3"])), ("out".to_string(), set(&["#M2"]))]);
    // none, written as an empty set; a single id stays a role of one
    let out = write(&mirror(json!({"ref": [], "out": "#M2"}))).unwrap();
    assert!(out.contains("*   $CURRENT_MIRROR @ATTACH {}(ref) #M2(out) => %m\n"), "{out}");
    let (errors, attach) = read_roles(&out, "");
    assert!(errors.is_empty(), "{out}\n{errors:?}");
    assert_eq!(attach, [("ref".to_string(), set(&[])), ("out".to_string(), set(&["#M2"]))]);
    // a list holds ids or points only
    assert_eq!(
        write(&mirror(json!({"ref": ["#M1", 3]}))).unwrap_err(),
        "op 0 (add-usage): `attach` role `ref` is not an id, a point or a list of them"
    );
}

#[test]
fn roles_of_any_name_read_back_as_written() {
    // a role in Chinese, with a dot, with a quote and with a comma: each written in quotes, the
    // name itself (16.5), a quote in it doubled, and read back as the definition names it
    let roles = ["参考管", "a.b", "x\"y", "p,q"];
    let attach: serde_json::Map<String, Value> =
        roles.iter().zip(["#M1", "#M2", "#M3", "#M4"]).map(|(r, id)| (r.to_string(), json!(id))).collect();
    let ops = json!([{"level": "file", "op": "add-usage", "name": "%m", "definition": "$T", "attach": attach}]);
    let out = write(&request("spice", FOUR, ops)).unwrap();
    assert!(out.contains(r#"*   $T @ATTACH #M1("参考管") #M2("a.b") #M3("x""y") #M4("p,q") => %m"#), "{out}");
    let (errors, attach) = read_roles(&out, r#"* @RNL { $T = @DEFINE(roles=["参考管", "a.b", "x""y", "p,q"]) {} }"#);
    assert!(errors.is_empty(), "{out}\n{errors:?}");
    let want: Vec<(String, Attach)> = roles
        .iter()
        .zip(["#M1", "#M2", "#M3", "#M4"])
        .map(|(r, id)| (r.to_string(), Attach::Object(id.into())))
        .collect();
    assert_eq!(attach, want);
}

#[test]
fn the_nets_of_a_wire_on_several_nets_are_one_set() {
    // written loose, two nets and the points would be three objects for the two slots of WIRE
    // (5.2.1); in braces they are one set, which the net slot takes whole, a bus (13.1)
    let net = "\n.subckt amp in out VSS\nR1 in out 10k\n.ends amp\n";
    let ops = json!([{"level": "subckt:amp", "op": "add-wire", "nets": ["&in", "&out"], "mode": "straight",
        "via": [{"terminal": "#R1:1"}, {"terminal": "#R1:2"}]}]);
    let out = write(&request("spice", net, ops)).unwrap();
    assert!(out.contains("*   {&in &out} @WIRE(mode=\"straight\") {#R1:1 #R1:2}\n"), "{out}");
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
        .unwrap();
    let errors: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{out}\n{}", errors.join("\n"));
}

#[test]
fn graphics_and_a_symbol_in_the_file_level() {
    let net = "\n.subckt top a b\nX1 a b blk\n.ends top\n";
    let bold = json!([[{"kind": "text", "text": "OUT", "style": {"level": 0, "bold": true, "italic": false, "overline": false}}]]);
    let ops = json!([
        {"level": "file", "op": "define-symbol", "name": "$blk_SYMBOL", "box": [-3, -2, 3, 2],
         "pins": [{"terminals": [1], "at": [-3, 0], "direction": "left", "label": {"markup": "IN"}},
                  {"terminals": [2], "at": [3, 0], "direction": "right", "label": {"lines": bold}}],
         "prims": [{"shape": "rectangle", "points": [[0, 0]], "params": {"width": 4, "height": 4}},
                   {"shape": "line", "points": [[-3, 0], [-2, 0]]},
                   {"shape": "line", "points": [[2, 0], [3, 0]]}]},
        {"level": "subckt:top", "op": "set", "object": "#X1", "symbol": "$blk_SYMBOL", "at": [10, 0]},
        {"level": "subckt:top", "op": "add-shape", "shape": "circle", "points": [[0, 5]], "params": {"radius": 1.5, "fill": "fillcolor"},
         "style": {"fillcolor": "#eef4ff", "linestyle": "dashed"}, "layer": "bottom"},
        {"level": "subckt:top", "op": "add-shape", "shape": "line", "points": [[0, 0], [4, 0]], "params": {"arrow": "end"},
         "style": {"arrowsize": 1.5}},
        {"level": "subckt:top", "op": "add-shape", "shape": "bezier", "points": [[0, 0], [1, 1], [2, 1], [3, 0]]}
    ]);
    let out = write(&request("spice", net, ops)).unwrap();
    assert_eq!(
        out,
        r##"
* @RNL(type="header") { @VERSION("0.3")}
.subckt top a b
X1 a b blk
* @RNL(author="test") {
*   #X1 @SET-SYMBOL $blk_SYMBOL => @PLACE(10, 0);
*   (0, 5) @CIRCLE(radius=1.5, fill="fillcolor", fillcolor="#eef4ff", linestyle="dashed") => @LAYER("bottom");
*   @LINE(arrow="end", arrowsize=1.5) {(0, 0) (4, 0)};
*   @BEZIER {(0, 0) (1, 1) (2, 1) (3, 0)}
* }
.ends top
* @RNL(author="test") {
*   $blk_SYMBOL = @DEFINE-SYMBOL(box=[-3, -2, 3, 2]) {
*     (-3, 0) @PIN(terminal=1, label="IN", direction="left");
*     (3, 0) @PIN(terminal=2, label="\textbf{OUT}", direction="right");
*     (0, 0) @RECTANGLE(width=4, height=4);
*     @LINE {(-3, 0) (-2, 0)};
*     @LINE {(2, 0) (3, 0)}
*   }
* }
"##
    );
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
        .unwrap();
    let loud: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity != Severity::Hint).map(|d| r.sink.render(d)).collect();
    assert!(loud.is_empty(), "{out}\n{}", loud.join("\n"));
}

#[test]
fn a_bus_pin_and_names_in_a_subcircuit_symbol() {
    let ops = json!([
        {"level": "subckt:rc_lowpass", "op": "define-symbol", "name": "%rc_lowpass_SYMBOL",
         "pins": [{"terminals": ["VSS", 2], "at": [0, -2], "direction": "down"}], "prims": []},
        {"level": "subckt:rc_lowpass", "op": "set", "object": "#", "symbol": "%rc_lowpass_SYMBOL"}
    ]);
    let out = write(&request("spice", RC, ops)).unwrap();
    assert!(
        out.contains("*   %rc_lowpass_SYMBOL = @DEFINE-SYMBOL {\n*     (0, -2) @PIN(terminal=[\"VSS\", 2], direction=\"down\")\n*   };\n*   # @SET-SYMBOL %rc_lowpass_SYMBOL\n"),
        "{out}"
    );
}

#[test]
fn graphics_that_are_not_well_formed_are_refused() {
    let refused = |op: Value| write(&request("spice", RC, json!([op]))).unwrap_err();
    let level = "subckt:rc_lowpass";
    assert_eq!(
        refused(json!({"level": level, "op": "add-shape", "shape": "arc", "points": [[0, 0], [1, 1]]})),
        "op 0 (add-shape): 2 points with these params do not make the shape arc (9.2.5)"
    );
    assert_eq!(
        refused(json!({"level": level, "op": "add-shape", "shape": "rectangle", "points": [[0, 0]]})),
        "op 0 (add-shape): 1 points with these params do not make the shape rectangle (9.2.5)"
    );
    assert_eq!(
        refused(json!({"level": level, "op": "add-shape", "shape": "polygon", "points": [[0, 0], [1, 1]]})),
        "op 0 (add-shape): `shape` \"polygon\" is not line, rectangle, circle, ellipse, arc or bezier"
    );
    assert_eq!(
        refused(
            json!({"level": level, "op": "add-shape", "shape": "line", "points": [[0, 0], [1, 1]], "layer": "under"})
        ),
        "op 0 (add-shape): `layer` \"under\" is not bottom, middle or top"
    );
    assert_eq!(
        refused(json!({"level": level, "op": "define-symbol", "name": "blk", "pins": [], "prims": []})),
        "op 0 (define-symbol): `name` \"blk\" is not a $ or % name"
    );
    assert_eq!(
        refused(
            json!({"level": level, "op": "define-symbol", "name": "%b", "pins": [{"terminals": [0], "at": [0, 0]}]})
        ),
        "op 0 (define-symbol): pin 0: `terminals` is not a list of terminal numbers and names"
    );
    assert_eq!(
        refused(
            json!({"level": level, "op": "define-symbol", "name": "%b", "prims": [{"shape": "line", "points": [[0, 0], [1, 0]], "layer": "top"}]})
        ),
        "op 0 (define-symbol): prim 0: unknown field `layer`"
    );
}

#[test]
fn a_long_text_breaks_between_its_parameters() {
    let long = "x".repeat(110);
    let ops = json!([{"level": "subckt:rc_lowpass", "op": "add-text", "at": [0, 0], "markup": long, "size": 1.5116, "style": {"bold": true}}]);
    let out = write(&request("spice", RC, ops)).unwrap();
    assert!(
        out.contains(&format!("*   (0, 0)\n*       @TEXT(\"{long}\",\n*       size=1.5116, bold=true)\n")),
        "{out}"
    );
    assert!(out.lines().all(|l| l.chars().count() <= LINE), "{out}");
}

#[test]
fn several_items_of_add_to_and_remove_from_are_one_set() {
    let level = "subckt:rc_lowpass";
    let ops = json!([
        {"level": level, "op": "remove-from", "items": ["&in", "&VSS"], "set": "$PORT_FILL_SET"},
        {"level": level, "op": "add-to", "items": ["&VSS"], "set": "$GROUND_NET_SET"}
    ]);
    let out = write(&request("spice", RC, ops)).unwrap();
    assert!(out.contains("*   {&in &VSS} @REMOVE-FROM $PORT_FILL_SET;\n*   &VSS @ADD-TO $GROUND_NET_SET\n"), "{out}");
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true, ..Options::default() })
        .unwrap();
    let loud: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity != Severity::Hint).map(|d| r.sink.render(d)).collect();
    assert!(loud.is_empty(), "{out}\n{}", loud.join("\n"));
}

#[test]
fn editor_records_and_headers_stay_outside_unterminated_host_regions() {
    for (dialect, source, opening) in [
        ("spice", "* title\nR1 a 0 1k\n.verilog\nmodule unfinished;", ".verilog"),
        ("spice", "* title\nR1 a 0 1k\n.exec\nprint unfinished", ".exec"),
        ("spectre", "// title\nR1 (a 0) resistor r=1k\nrel reliability {\n", "rel reliability"),
        ("spectre", "// title\nR1 (a 0) resistor r=1k\n#ifdef EXTERNAL\n", "#ifdef"),
    ] {
        let ops = json!([{"level": "file", "op": "set", "object": "#R1", "at": [1, 2]}]);
        let out = write(&request(dialect, source, ops)).unwrap();
        assert!(out.find("@VERSION").unwrap() < out.find(opening).unwrap(), "{out}");
        assert!(out.find("@PLACE").unwrap() < out.find(opening).unwrap(), "{out}");
        let files = MemoryProvider::new().with(entry(dialect), &out);
        let doc = rnl_view::read_host(entry(dialect), Some(dialect), &files, &mut Sink::new()).unwrap();
        assert_eq!(doc.main().records.len(), 2, "{out}");
    }
}

#[test]
fn the_editor_explains_when_a_conditional_subcircuit_has_no_safe_write_place() {
    let net = "* title\n.if 1\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.endif\n";
    let ops = json!([{"level": "subckt:cell", "op": "set", "object": "#R1", "at": [0, 0]}]);
    assert_eq!(
        write(&request("spice", net, ops)).unwrap_err(),
        "op 0 (set): cell lies in a host region excluded from writing, lines 2 to 6 (2.2)"
    );
}

#[test]
fn an_orientation_given_is_written_r0_too() {
    // without an ORIENT the solver chooses the orientation (11.2): an R0 given is a decision, and is
    // written; an orientation not given is not
    let net = "* title\nR1 a 0 1k\nR2 a 0 1k\n.end\n";
    let ops = json!([
        {"level": "file", "op": "set", "object": "#R1", "at": [0, 0], "orient": "R0"},
        {"level": "file", "op": "set", "object": "#R2", "at": [4, 0]}
    ]);
    let out = write(&request("spice", net, ops)).unwrap();
    let want = "*   #R1 @PLACE(0, 0) => @ORIENT(\"R0\");\n*   #R2 @PLACE(4, 0)\n";
    assert!(out.contains(want), "{out}");
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options::default()).unwrap();
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
}

/// A package of the tests of parameters: a definition that draws nothing, with a parameter of each
/// kind of literal, and three of analog-canvas-extensions 0.2 as that package writes them.
const PARAMS_PACKAGE: &str = r#"@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="params-test", version="0.1")}
@RNL {
  $NOTE = @DEFINE(roles=[["dev", "instance"]], parameters=[["s", ""], ["n", 1], ["on", false], ["pts", []]]) {};
  $CANVAS_NAME = @DEFINE(roles=[["part", "instance"]], parameters=["name"]) {};
  $CANVAS_LABEL = @DEFINE(roles=["label"], parameters=["bind", "parameter", "show_value"]) {};
  $CANVAS_PARAMETER_TEXT = @DEFINE(roles=[["part", "instance"]], parameters=["name", "text"]) {}
}
"#;

/// What `usages` write into a netlist of R1, placed so that it has its labels, with PARAMS_PACKAGE
/// used; the errors of reading it back, and the snapshot's objects of its level.
fn written_with_params(usages: Value) -> (String, Vec<String>, Vec<Value>) {
    let mut ops = vec![
        json!({"level": "file", "op": "use-package", "name": "params-test", "version": "0.1"}),
        json!({"level": "file", "op": "set", "object": "#R1", "at": [0, 0]}),
    ];
    ops.extend(usages.as_array().unwrap().iter().cloned());
    let out = write(&request("spice", "* params\nR1 a 0 1k\n.end\n", Value::Array(ops))).unwrap();
    let mut files = MemoryProvider::new().with(entry("spice"), &out).with("pkgs/params-test.rnl", PARAMS_PACKAGE);
    files.package_dirs.push("pkgs".into());
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options::default()).unwrap();
    let errors = r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    let parts = rnl_view::Parts { drawing: false, host: false, model: true };
    let s = rnl_view::snapshot(entry("spice"), &files, &rnl_view::BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(s).unwrap();
    (out, errors, s["levels"][0]["model"]["objects"].as_array().unwrap().clone())
}

/// A JSON value with every number as a float, as the snapshot gives numbers: `7` and `7.0` are one
/// number to a reader of JSON.
fn floats(v: &Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(floats).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), floats(v))).collect()),
        v => v.clone(),
    }
}

#[test]
fn a_usage_is_written_with_its_parameters_and_reads_them_back_as_given() {
    // each kind of literal, a string with a quote and a character outside ASCII, numbers that a
    // millionth would round, and lists, nested and empty, in an order other than the definition's
    let params = json!({
        "pts": [1, "a", [true, -0.5], []],
        "s": "1MegΩ \"q\"",
        "n": 1.0e-9,
        "on": true,
    });
    let (out, errors, objects) = written_with_params(json!([
        {"level": "file", "op": "add-usage", "name": "%u", "definition": "$NOTE", "attach": {"dev": "#R1"}, "params": params},
        {"level": "file", "op": "add-usage", "name": "%big", "definition": "$NOTE", "attach": {"dev": "#R1"},
         "params": {"n": -123456789.125, "s": ""}},
    ]));
    assert!(
        out.contains(r#"*   $NOTE @ATTACH(pts=[1, "a", [true, -0.5], []], s="1MegΩ ""q""", n=0.000000001, on=true) #R1(dev) => %u"#),
        "{out}"
    );
    assert!(errors.is_empty(), "{out}\n{errors:?}");
    let object = |id: &str| objects.iter().find(|o| o["id"] == id).unwrap_or_else(|| panic!("no {id}: {objects:?}"));
    assert_eq!(object("%u")["params"], floats(&params), "{out}");
    assert_eq!(object("%big")["params"], json!({"n": -123456789.125, "s": ""}), "{out}");
}

#[test]
fn the_parameters_of_a_definition_from_a_package_read_back_as_given() {
    // the definitions of meaning only that Analog Canvas writes: one on a part, one on a label
    let label = json!({"bind": "parameter", "parameter": "w", "show_value": false});
    let (out, errors, objects) = written_with_params(json!([
        {"level": "file", "op": "add-usage", "name": "%n", "definition": "$CANVAS_NAME", "attach": {"part": "#R1"}, "params": {"name": "M1"}},
        {"level": "file", "op": "add-usage", "name": "%l", "definition": "$CANVAS_LABEL", "attach": {"label": "#R1::name"}, "params": label},
        {"level": "file", "op": "add-usage", "definition": "$CANVAS_PARAMETER_TEXT", "attach": {"part": "#R1"}, "params": {"name": "r", "text": "1MegΩ"}},
    ]));
    assert!(out.contains(r#"*   $CANVAS_NAME @ATTACH(name="M1") #R1(part) => %n"#), "{out}");
    assert!(
        out.contains(
            r#"*   $CANVAS_LABEL @ATTACH(bind="parameter", parameter="w", show_value=false) #R1::name(label) => %l"#
        ),
        "{out}"
    );
    assert!(errors.is_empty(), "{out}\n{errors:?}");
    let object = |id: &str| objects.iter().find(|o| o["id"] == id).unwrap_or_else(|| panic!("no {id}: {objects:?}"));
    assert_eq!(object("%n")["params"], json!({"name": "M1"}));
    assert_eq!(object("%l")["params"], label);
    assert_eq!(object("%l")["attach"], json!({"label": "#R1::name"}));
    let text = objects.iter().find(|o| o["definition"] == "$CANVAS_PARAMETER_TEXT").unwrap();
    assert_eq!(text["params"], json!({"name": "r", "text": "1MegΩ"}));
}

#[test]
fn a_parameter_the_definition_lacks_fails_as_a_written_one_does() {
    let usage = json!({"level": "file", "op": "add-usage", "name": "%u", "definition": "$CANVAS_NAME",
                       "attach": {"part": "#R1"}, "params": {"nmae": "M1"}});
    let (out, errors, _) = written_with_params(json!([usage]));
    assert!(out.contains(r#"*   $CANVAS_NAME @ATTACH(nmae="M1") #R1(part) => %u"#), "{out}");
    // the same statement written by hand, read the same way
    let mut files = MemoryProvider::new()
        .with(
            entry("spice"),
            "* params\nR1 a 0 1k\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"params-test\", version=\"0.1\")}\n\
             * @RNL { #R1 @PLACE(0, 0); $CANVAS_NAME @ATTACH(nmae=\"M1\") #R1(part) => %u }\n.end\n",
        )
        .with("pkgs/params-test.rnl", PARAMS_PACKAGE);
    files.package_dirs.push("pkgs".into());
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options::default()).unwrap();
    let by_hand: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    assert!(!by_hand.is_empty());
    assert_eq!(errors, by_hand, "{out}");
}

#[test]
fn parameters_that_a_record_cannot_hold_are_refused() {
    let usage = |params: Value| {
        request(
            "spice",
            "* params\nR1 a 0 1k\n.end\n",
            json!([{"level": "file", "op": "add-usage", "definition": "$NOTE", "attach": {"dev": "#R1"}, "params": params}]),
        )
    };
    let refused =
        "op 0 (add-usage): `params` `s` is not a string without a line break, a number, a boolean or a list of them";
    for params in [json!({"s": "two\nlines"}), json!({"s": null}), json!({"s": {"a": 1}}), json!({"s": [1, {"a": 1}]})]
    {
        assert_eq!(write(&usage(params.clone())).unwrap_err(), refused, "{params}");
    }
    assert_eq!(write(&usage(json!([1]))).unwrap_err(), "op 0 (add-usage): `params` is not an object");
    // none, or an empty object, writes no parentheses
    for params in [json!({}), Value::Null] {
        let out = write(&usage(params)).unwrap();
        assert!(out.contains("*   $NOTE @ATTACH #R1(dev)\n"), "{out}");
    }
}

/// What `ops` write into `net`, read back: the text, the diagnostics as rendered, and the
/// snapshot's levels.
fn written_and_snapshot(net: &str, ops: Value) -> (String, Vec<(Severity, String)>, Vec<Value>) {
    let out = write(&request("spice", net, ops)).unwrap();
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options::default()).unwrap();
    let diagnostics = r.diagnostics().iter().map(|d| (d.severity, r.sink.render(d))).collect();
    let parts = rnl_view::Parts { drawing: false, host: false, model: true };
    let s = rnl_view::snapshot(entry("spice"), &files, &rnl_view::BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(s).unwrap();
    (out, diagnostics, s["levels"].as_array().unwrap().clone())
}

/// The object `id` of the model of `level`.
fn model_object<'a>(level: &'a Value, id: &str) -> &'a Value {
    let objects = level["model"]["objects"].as_array().unwrap();
    objects.iter().find(|o| o["id"] == id).unwrap_or_else(|| panic!("no {id} in {objects:?}"))
}

#[test]
fn a_style_is_written_as_a_definition_that_set_style_gives_and_reads_back_as_given() {
    let blue = json!({"linecolor": "#2275e2", "textcolor": "#2275e2"});
    let wide = json!({"linewidth": 0.25, "linestyle": "dashed", "bold": true});
    let net = "* styles\nR1 a 0 1k\nR2 a 0 1k\nR3 a 0 1k\nR4 a 0 1k\n.end\n";
    let (out, diagnostics, levels) = written_and_snapshot(
        net,
        json!([
            {"level": "file", "op": "set", "object": "#R1", "style": blue, "at": [0, 0]},
            // the same properties in another order are the same style
            {"level": "file", "op": "set", "object": "#R2", "style": {"textcolor": "#2275e2", "linecolor": "#2275e2"}, "at": [4, 0]},
            {"level": "file", "op": "set", "object": "#R3", "symbol": "$RESISTOR", "style": wide, "at": [8, 0]},
            {"level": "file", "op": "set", "object": "#R4", "at": [12, 0]},
            {"level": "file", "op": "add-usage", "name": "%g", "definition": "$GROUND", "attach": {"p": "&0"},
             "style": blue, "at": [0, -4]},
        ]),
    );
    for line in [
        r##"*   $style1 = @DEFINE-STYLE(linecolor="#2275e2", textcolor="#2275e2") {};"##,
        r##"*   $style2 = @DEFINE-STYLE(linewidth=0.25, linestyle="dashed", bold=true) {};"##,
        "*   #R1 @SET-STYLE $style1 => @PLACE(0, 0);",
        "*   #R2 @SET-STYLE $style1 => @PLACE(4, 0);",
        "*   #R3 @SET-SYMBOL $RESISTOR => @SET-STYLE $style2 => @PLACE(8, 0);",
        "*   $GROUND @ATTACH &0(p) => @SET-STYLE $style1 => @PLACE(0, -4) => %g",
    ] {
        assert!(out.contains(line), "{line}\n{out}");
    }
    assert_eq!(out.matches("@DEFINE-STYLE").count(), 2, "{out}");
    assert!(diagnostics.iter().all(|(s, _)| *s != Severity::Error), "{out}\n{diagnostics:?}");
    let top = &levels[0];
    for id in ["#R1", "#R2", "%g"] {
        assert_eq!(model_object(top, id)["style"], blue, "{id}\n{out}");
    }
    assert_eq!(model_object(top, "#R3")["style"], wide, "{out}");
    // the object's style passes to its labels (11.3): its name is drawn in the colour, as Analog
    // Canvas colours an instance's labels with the instance
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options::default()).unwrap();
    let drawn = r.model.levels[0].texts.iter().find(|t| t.object_id == "#R1::name").unwrap();
    assert_eq!(drawn.style.get("textcolor").and_then(|v| v.str()), Some("#2275e2"), "{out}");
    // an object the document gives no style has none: the standard package's are left out
    assert!(model_object(top, "#R4").get("style").is_none(), "{}", model_object(top, "#R4"));
}

#[test]
fn each_level_defines_the_styles_it_uses() {
    // a subcircuit could see the style of the file level, but its own definition keeps it the same
    // when its file is read on its own, and with its own name it hides nothing (6.8)
    let blue = json!({"linecolor": "#2275e2"});
    let net = "* levels\n.subckt cell a b\nR1 a b 1k\n.ends\nX1 n 0 cell\nR2 n 0 1k\n.end\n";
    let (out, diagnostics, levels) = written_and_snapshot(
        net,
        json!([
            {"level": "file", "op": "set", "object": "#R2", "style": blue, "at": [0, 0]},
            {"level": "subckt:cell", "op": "set", "object": "#R1", "style": blue, "at": [0, 0]},
            {"level": "subckt:cell", "op": "set", "object": "#R1", "style": blue},
        ]),
    );
    assert!(out.contains(r##"*   $style1 = @DEFINE-STYLE(linecolor="#2275e2") {};"##), "{out}");
    assert!(out.contains(r##"*   $style2 = @DEFINE-STYLE(linecolor="#2275e2") {};"##), "{out}");
    assert_eq!(out.matches("@DEFINE-STYLE").count(), 2, "{out}");
    let loud = diagnostics.iter().filter(|(s, m)| *s != Severity::Hint || m.contains("hides"));
    assert_eq!(loud.count(), 0, "{out}\n{diagnostics:?}");
    let level = |id: &str| levels.iter().find(|l| l["id"] == id).unwrap_or_else(|| panic!("no level {id}"));
    assert_eq!(model_object(level("file:netlist.sp"), "#R2")["style"], blue);
    assert_eq!(model_object(level("subckt:cell"), "#R1")["style"], blue);
}

#[test]
fn a_style_property_the_language_lacks_fails_as_a_written_one_does() {
    let net = "* styles\nR1 a 0 1k\n.end\n";
    let ops = json!([{"level": "file", "op": "set", "object": "#R1", "style": {"colour": "#2275e2"}}]);
    let (out, diagnostics, _) = written_and_snapshot(net, ops);
    assert!(out.contains(r##"*   $style1 = @DEFINE-STYLE(colour="#2275e2") {};"##), "{out}");
    let message = |d: &[(Severity, String)]| -> Vec<String> {
        d.iter()
            .filter(|(s, _)| *s == Severity::Error)
            // what, without where: the record written by hand is on another line
            .map(|(_, m)| m.split_once(": ").unwrap().1.lines().next().unwrap().to_string())
            .collect()
    };
    let by_hand = "* styles\nR1 a 0 1k\n* @RNL { $style1 = @DEFINE-STYLE(colour=\"#2275e2\") {}; #R1 @SET-STYLE $style1 }\n.end\n";
    let files = MemoryProvider::new().with(entry("spice"), by_hand);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options::default()).unwrap();
    let by_hand: Vec<(Severity, String)> = r.diagnostics().iter().map(|d| (d.severity, r.sink.render(d))).collect();
    assert!(!message(&by_hand).is_empty());
    assert_eq!(message(&diagnostics), message(&by_hand), "{out}");
}

#[test]
fn a_style_that_a_record_cannot_hold_is_refused() {
    let set = |style: Value| {
        request(
            "spice",
            "* styles\nR1 a 0 1k\n.end\n",
            json!([{"level": "file", "op": "set", "object": "#R1", "style": style}]),
        )
    };
    assert_eq!(
        write(&set(json!({"linecolor": ["#000000"]}))).unwrap_err(),
        "op 0 (set): `style` `linecolor` is not a string, number or boolean"
    );
    // an empty style sets nothing
    assert_eq!(
        write(&set(json!({}))).unwrap_err(),
        "op 0 (set): nothing to set: no `symbol`, `style`, `at` or `orient`"
    );
}

#[test]
fn independent_pin_names_bus_shape_and_unbound_pins_round_trip_in_0_4() {
    let ops = json!([
        {"level":"file", "op":"define-symbol", "name":"$S", "prims":[], "pins":[
            {"terminals":[1],"names":["p","alternate name"],"at":[0,0]},
            {"terminals":[1],"names":["q"],"at":[1,0]},
            {"terminals":[2],"bus":true,"at":[2,0]},
            {"terminals":[],"names":["free"],"at":[3,0]}
        ]},
        {"level":"file","op":"set","object":"#R1","symbol":"$S"}
    ]);
    let mut req = request("spice", "* pins\nR1 a b 1k\n.end\n", ops);
    assert!(write(&req).unwrap_err().contains("language_version 0.4"));
    req.language_version = Some("0.4".into());
    let out = write(&req).unwrap();
    assert!(out.contains("@VERSION(\"0.4\")"), "{out}");
    assert!(out.contains("@PIN(terminal=[2])"), "{out}");
    assert!(out.contains("@PIN() => %free"), "{out}");
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let options =
        rnl_view::BuildOptions { runtime: Options { std: false, ..Default::default() }, ..Default::default() };
    let s = rnl_view::snapshot(
        entry("spice"),
        &files,
        &options,
        rnl_view::Parts { drawing: false, host: false, model: true },
    )
    .unwrap();
    assert!(!s.diagnostics.iter().any(|d| d.severity == "error"), "{out}\n{:?}", s.diagnostics);
    let model = s.levels[0].model.as_ref().unwrap();
    let pins = &model.objects.iter().find(|o| o.id == "#R1").unwrap().pins;
    assert_eq!(pins.len(), 4);
    assert_eq!(pins[0].names.as_ref().unwrap(), &["p", "alternate name"]);
    assert_eq!(pins[1].names.as_ref().unwrap(), &["q"]);
    assert_eq!(pins[2].bus, Some(true));
    assert!(pins[3].nets.is_empty());
    assert!(pins.iter().all(|p| p.at.is_none()));
    let definition = model.symbols.iter().find(|s| s.name == "$S").unwrap().definition.as_ref().unwrap();
    assert_eq!(definition.pins.len(), 4);
    assert_eq!(definition.pins[0].names.as_ref().unwrap(), &["p", "alternate name"]);
}

#[test]
fn pin_editing_rejects_unwriteable_vertices_and_invalid_metadata() {
    for pin in [
        json!({"terminals":[], "at":[0,0], "bus":true}),
        json!({"terminals":[1,2], "at":[0,0], "bus":false}),
        json!({"terminals":[1], "at":[0,0], "names":[""]}),
        json!({"terminals":[4294967296u64], "at":[0,0]}),
    ] {
        let mut req = request(
            "spice",
            "* pins\nR1 a b 1k\n",
            json!([
                {"level":"file","op":"define-symbol","name":"$S","pins":[pin],"prims":[]}
            ]),
        );
        req.language_version = Some("0.4".into());
        assert!(write(&req).is_err());
    }
    let req = request(
        "spice",
        "* pins\nR1 a b 1k\n",
        json!([
            {"level":"file","op":"add-wire","nets":["&a"],"via":[
                {"owner":"#R1","pin":0,"reference":null},{"at":[2,0]}
            ]}
        ]),
    );
    assert!(write(&req).is_err());
}

#[test]
fn upright_and_fractional_rotation_write_back_explicitly() {
    for upright in [false, true] {
        let ops =
            json!([{"level":"file", "op":"add-text", "at":[0,0], "markup":"arrow", "upright":upright, "rotate":30.5}]);
        let mut req = request("spice", "* text\n.end\n", ops);
        assert!(write(&req).is_err());
        req.language_version = Some("0.4".into());
        let out = write(&req).unwrap();
        assert!(out.contains(&format!("upright={upright}")), "{out}");
        assert!(out.contains("rotate=30.5"), "{out}");
        let files = MemoryProvider::new().with(entry("spice"), &out);
        let r =
            rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: false, ..Options::default() })
                .unwrap();
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{out}");
        assert_eq!(r.model.levels[0].texts[0].upright, Some(upright));
        assert_eq!(r.model.levels[0].texts[0].rotate, 30.5);
    }
}

#[test]
fn fixed_rotation_round_trips_explicit_false_over_a_symbol_label() {
    let net = "* text\nR1 a b 1k\n.end\n";
    let symbol = r#"* @RNL {
* $S = @DEFINE-SYMBOL { ? @TEXT("old", key="name", fix_rotate=true, halign="middle", valign="bottom") => @PLACE(2,3) };
* #R1 @SET-SYMBOL $S => @PLACE(10,20) => @ORIENT("R90")
* }
.end"#;
    for fixed in [true, false] {
        let ops = json!([{"level":"file", "op":"add-text", "attach":"#R1", "key":"name", "markup":"new", "fix_rotate":fixed}]);
        let mut req = request("spice", net, ops.clone());
        assert!(write(&req).unwrap_err().contains("language_version 0.4"));
        req.language_version = Some("0.4".into());
        let out = write(&req).unwrap();
        assert!(out.contains(&format!("fix_rotate={fixed}")), "{out}");
        // The writer creates records in a bare netlist. Supply a symbol afterwards to verify
        // that an explicit false also overrides the earlier symbol-label layer (13.3).
        let out = out.replace(".end", symbol);
        let files = MemoryProvider::new().with(entry("spice"), &out);
        let r =
            rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: false, ..Options::default() })
                .unwrap();
        assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{out}");
        let text = &r.model.levels[0].texts[0];
        assert_eq!(text.fix_rotate, Some(fixed));
        assert_eq!(text.anchor, (7.0, 22.0));
        // The owner orientation moves the box side even when the glyph direction is fixed.
        assert_eq!(text.orient, rnl_runtime::geom::Orient::R90);
        assert_eq!(text.markup, "new");
        let document = rnl_view::build(
            entry("spice"),
            &files,
            &rnl_view::BuildOptions {
                runtime: Options { std: false, ..Options::default() },
                primitives: true,
                ..Default::default()
            },
        )
        .unwrap();
        let document = serde_json::to_value(document).unwrap();
        let drawn = document["levels"][0]["primitives"]
            .as_array()
            .unwrap()
            .iter()
            .find(|primitive| primitive["kind"] == "text")
            .unwrap();
        assert_eq!(drawn["anchor"], json!([7.0, 22.0]));
        // Arbitrary-angle primitives carry a scene-space transform. Check the
        // actual glyph axes rather than the legacy local-layout vertical flag.
        let transform = drawn["transform"].as_array().unwrap();
        assert_eq!(json!([transform[0], transform[2]]), if fixed { json!([1.0, 0.0]) } else { json!([0.0, 1.0]) });
        assert_eq!(json!([transform[1], transform[3]]), if fixed { json!([0.0, 1.0]) } else { json!([-1.0, 0.0]) });
        if fixed {
            assert_eq!(drawn["halign"], "end");
            assert_eq!(drawn["valign"], "middle");
        }
    }
    assert!(ops::parse(0, &json!({"level":"file", "op":"add-text", "at":[0,0], "markup":"bad", "fix_rotate":"true"}))
        .is_err());
}

/// A call of a subcircuit whose ports are named otherwise than the symbol's terminals, and a symbol
/// that binds its pins by name: `terminals` of `set` and `add-usage` say which host terminal each
/// symbol terminal is, as `@SET-SYMBOL(terminals=…)` does (9.2.4).
const CALL: &str =
    "* call\n.subckt nm p q r t\n.ends nm\n.subckt top a b c d\nX1 a b c d nm\nX2 a b c d nm\n.ends top\n";

fn named_pins_symbol() -> Value {
    json!({"level": "file", "op": "define-symbol", "name": "$S", "pins": [
        {"terminals": ["d"], "at": [0, 2]}, {"terminals": ["g"], "at": [-3, 0]},
        {"terminals": ["s"], "at": [0, -2]}, {"terminals": ["b"], "at": [1, 0]}
    ], "prims": []})
}

fn binding_warnings(out: &str) -> usize {
    let files = MemoryProvider::new().with(entry("spice"), out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: false, ..Options::default() })
        .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{out}");
    r.diagnostics().iter().filter(|d| d.code.number().as_deref() == Some("RNL4119")).count()
}

#[test]
fn terminals_bind_the_pins_of_a_symbol_to_the_terminals_of_a_call() {
    let terminals = json!([[1, "d"], [2, "g"], [3, "s"], [4, "b"]]);
    let ops = json!([
        named_pins_symbol(),
        {"level": "subckt:top", "op": "set", "object": "#X1", "symbol": "$S", "terminals": terminals, "at": [0, 0]},
        {"level": "subckt:top", "op": "set", "object": "#X2", "symbol": "$S", "at": [10, 0]}
    ]);
    let mut req = request("spice", CALL, ops);
    req.language_version = Some("0.4".into());
    let out = write(&req).unwrap();
    assert!(
        out.contains("#X1 @SET-SYMBOL(terminals=[[1, \"d\"], [2, \"g\"], [3, \"s\"], [4, \"b\"]]) $S => @PLACE(0, 0)"),
        "{out}"
    );
    assert!(out.contains("#X2 @SET-SYMBOL $S => @PLACE(10, 0)"), "{out}");
    // X2 names no terminals, and its ports are not the symbol's: each of its four pins binds nothing.
    // X1's four read back bound.
    assert_eq!(binding_warnings(&out), 4, "{out}");
}

#[test]
fn terminals_of_a_usage_and_a_host_terminal_by_name_are_written() {
    let ops = json!([
        {"level": "file", "op": "define-symbol", "name": "$S", "pins": [{"terminals": ["d"], "at": [0, 2]}], "prims": []},
        {"level": "subckt:top", "op": "add-usage", "definition": "$FIGURE", "attach": {"p": [3, 4]},
         "symbol": "$S", "terminals": [["q", "d"]]}
    ]);
    let mut req = request("spice", CALL, ops);
    req.language_version = Some("0.4".into());
    let out = write(&req).unwrap();
    assert!(out.contains("@SET-SYMBOL(terminals=[[\"q\", \"d\"]]) $S"), "{out}");
}

#[test]
fn terminals_need_the_language_0_4_a_symbol_and_pairs() {
    let set = |extra: Value| {
        let mut op = json!({"level": "subckt:top", "op": "set", "object": "#X1", "symbol": "$S", "at": [0, 0]});
        op.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        json!([op])
    };
    let mut req = request("spice", CALL, set(json!({"terminals": [[1, "d"]]})));
    assert!(write(&req).unwrap_err().contains("terminals"), "released output refuses it");
    assert!(write(&req).unwrap_err().contains("language_version 0.4"));
    req.language_version = Some("0.4".into());
    assert!(write(&req).is_ok());
    // an empty list is no terminals
    req.ops = set(json!({"terminals": []})).as_array().unwrap().clone();
    assert!(write(&req).unwrap().contains("#X1 @SET-SYMBOL $S"));
    for bad in [
        json!({"terminals": "d"}),
        json!({"terminals": ["d"]}),
        json!({"terminals": [[1]]}),
        json!({"terminals": [[0, "d"]]}),
        json!({"terminals": [[1, ""]]}),
        json!({"terminals": [[1.5, "d"]]}),
        json!({"terminals": [[true, "d"]]}),
        // a terminal named twice reads back as an error, on either side
        json!({"terminals": [[1, "d"], [1, "s"]]}),
        json!({"terminals": [[1, "d"], [2, "d"]]}),
        json!({"terminals": [["a", "d"], ["a", "s"]]}),
        // a name with a line break would change; an all-digit name reads as a number
        json!({"terminals": [[1, "d\ns"]]}),
        json!({"terminals": [["a\nb", "d"]]}),
        json!({"terminals": [[1, "2"]]}),
        json!({"terminals": [["3", "d"]]}),
    ] {
        req.ops = set(bad.clone()).as_array().unwrap().clone();
        let err = write(&req).unwrap_err();
        assert!(err.contains("terminals"), "{bad}: {err}");
    }
    // without a symbol the terminals bind nothing
    req.ops = json!([{"level": "subckt:top", "op": "set", "object": "#X1", "at": [0, 0], "terminals": [[1, "d"]]}])
        .as_array()
        .unwrap()
        .clone();
    assert!(write(&req).unwrap_err().contains("needs a `symbol`"));
}
