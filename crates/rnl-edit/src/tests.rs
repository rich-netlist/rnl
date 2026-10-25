use super::*;
use rnl_runtime::diag::Severity;
use rnl_runtime::markup;
use rnl_runtime::model::Attach;
use rnl_runtime::Options;
use serde_json::json;

use crate::ops::{Run, RunStyle};

fn request(dialect: &str, netlist: &str, ops: Value) -> WriteRequest {
    WriteRequest {
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
