use super::*;
use rnl_runtime::diag::Severity;
use rnl_runtime::markup;
use rnl_runtime::Options;
use serde_json::json;

use crate::ops::{Run, RunStyle};

fn request(dialect: &str, netlist: &str, ops: Value) -> WriteRequest {
    WriteRequest {
        dialect: dialect.into(),
        netlist: netlist.into(),
        author: "test".into(),
        ops: ops.as_array().unwrap().clone(),
    }
}

fn styled(text: &str, level: i32, bold: bool, italic: bool) -> Value {
    json!({"kind": "text", "text": text, "style": {"level": level, "bold": bold, "italic": italic, "overline": false}})
}

const RC: &str = "\n.subckt rc_lowpass VSS\nC1 out VSS 10n\nR1 in out 10k\n.ends rc_lowpass\n";

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
*   @VERSION("0.2");
*   @USE-PACKAGE(name="rnl-std", version="0.3");
*   @USE-PACKAGE(name="rnl-std-no-labels", version="0.3")
* }
.subckt rc_lowpass VSS
C1 out VSS 10n
R1 in out 10k
* @RNL(author="test") {
*   #R1 @SET-SYMBOL $RES => @PLACE(19, -16) => @ORIENT("R90");
*   #C1 @PLACE(29, -22);
*   $GND @ATTACH &VSS => @SET-SYMBOL $GROUND => @PLACE(10, -25) => %gnd1;
*   %j1 = (11, 2);
*   $JUNCTION @ATTACH %j1;
*   &VSS @REMOVE-FROM $PORT_FILL_SET;
*   &in @WIRE(mode="straight") {#R1:1 (10, -16) %j1 %gnd1:p};
*   #R1 @TEXT("\textit{R}_{1}", key="name", dx=1, dy=-0.5, size=1.5116);
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
        "simulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.2\"); @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")}\n\nsubckt top (a b)\nR1 (a b) resistor r=1k\n// @RNL(author=\"test\") {\n//   #R1 @SET-SYMBOL $RES => @PLACE(0, 0)\n// }\nends top\n"
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
        "* title\r\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\r\nR1 a 0 1k\r\n* @RNL(author=\"test\") {\r\n*   #R1 @PLACE(1, 2)\r\n* }\r\n.end"
    );
    // without .end the record goes at the end of the file
    let out = write(&request(
        "spice",
        "* title\nR1 a 0 1k",
        json!([{"level": "file:x", "op": "set", "object": "#R1", "at": [0, 0]}]),
    ))
    .unwrap();
    assert_eq!(out, "* title\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\n* @RNL(author=\"test\") {\n*   #R1 @PLACE(0, 0)\n* }\n");
}

#[test]
fn the_header_goes_after_what_continues_line_one() {
    let out = write(&request("spice", "title\n+ more of it\nR1 a 0 1k\n", json!([]))).unwrap();
    assert_eq!(out, "title\n+ more of it\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 a 0 1k\n");
    let out =
        write(&request("spectre", "simulator lang=spectre \\\n  insensitive=no\nR1 (a b) resistor r=1k\n", json!([])))
            .unwrap();
    assert_eq!(out, "simulator lang=spectre \\\n  insensitive=no\n// @RNL(type=\"header\") { @VERSION(\"0.2\")}\nR1 (a b) resistor r=1k\n");
    // an empty netlist still keeps line 1 for the title
    assert_eq!(write(&request("spice", "", json!([]))).unwrap(), "\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n");
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
        "op 0 (set): `style` is not written yet"
    );
    assert_eq!(
        refused(json!([{"level": "subckt:rc_lowpass", "op": "set", "object": "#R1"}])),
        "op 0 (set): nothing to set: no `symbol`, `at` or `orient`"
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
        "op 0 (add-wire): `via` vertex 1 is not one of {terminal}, {point}, {usage, role}, {at}"
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

/// The runtime's runs as this crate's, to compare what the markup reads back as.
fn ours(runs: &[markup::Run]) -> Vec<Run> {
    let style =
        |s: &markup::RunStyle| RunStyle { level: s.level as i32, bold: s.bold, italic: s.italic, overline: s.overline };
    runs.iter()
        .map(|r| match r {
            markup::Run::Text { text, style: s } => Run::Text { text: text.clone(), style: style(s) },
            markup::Run::Frac { num, den, style: s } => Run::Frac { num: ours(num), den: ours(den), style: style(s) },
        })
        .collect()
}

#[test]
fn markup_reads_back_as_the_runs_it_was_written_from() {
    let s = |level, bold, italic, overline| RunStyle { level, bold, italic, overline };
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
fn what_it_writes_interprets_without_errors() {
    let net = "\n.subckt amp in out VSS\nR1 in out 10k\nC1 out VSS 1p\n.ends amp\n";
    let level = "subckt:amp";
    let ops = json!([
        {"level": level, "op": "set", "object": "#R1", "symbol": "$RES", "at": [4, 0], "orient": "R90"},
        {"level": level, "op": "set", "object": "#C1", "at": [8, -4]},
        {"level": level, "op": "add-usage", "name": "%gnd1", "definition": "$GND", "attach": {"p": "&VSS"}, "at": [8, -8]},
        {"level": level, "op": "remove-from", "items": ["&VSS"], "set": "$PORT_FILL_SET"},
        {"level": level, "op": "add-wire", "nets": ["&out"], "mode": "straight", "via": [{"terminal": "#R1:2"}, {"at": [8, 0]}, {"terminal": "#C1:1"}]},
        {"level": level, "op": "add-wire", "nets": ["&VSS"], "mode": "straight", "via": [{"terminal": "#C1:2"}, {"usage": "%gnd1", "role": "p"}]},
        {"level": level, "op": "add-text", "attach": "#R1", "key": "name", "lines": [[styled("R", 0, false, true), styled("1", -1, false, false)]], "offset": [1, 0], "size": 1.2},
        {"level": level, "op": "add-text", "at": [0, 4], "lines": [[styled("gain \\frac{1}{2}", 0, true, false)]], "size": 1}
    ]);
    let out = write(&request("spice", net, ops)).unwrap();
    let files = MemoryProvider::new().with(entry("spice"), &out);
    let r = rnl_view::interpret(entry("spice"), Some("spice"), &files, &Options { std: true }).unwrap();
    let loud: Vec<String> =
        r.diagnostics().iter().filter(|d| d.severity != Severity::Hint).map(|d| r.sink.render(d)).collect();
    assert!(loud.is_empty(), "{out}\n{}", loud.join("\n"));
}
