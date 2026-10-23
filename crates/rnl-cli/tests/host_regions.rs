//! Regression coverage for ordinary netlist carriers versus host-language regions (2.2).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::{append_record, record_line, replace_generated_with_header, strip_generated};
use rnl_runtime::{Interpretation, Options};

fn read(file: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(file, text);
    interpret_file(Path::new(file), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

fn record_texts(r: &Interpretation) -> Vec<&str> {
    r.doc.levels.iter().flat_map(|l| &l.records).map(|r| r.text.as_str()).collect()
}

fn warning_at(r: &Interpretation, text: &str, line: u32) -> bool {
    r.diagnostics().iter().any(|d| {
        d.severity == Severity::Warning && d.message.contains(text) && d.loc.as_ref().is_some_and(|l| l.line == line)
    })
}

#[test]
fn script_and_verilog_regions_are_not_carriers_in_either_spice_reader() {
    for (start, end) in [(".control", ".endc"), (".exec", ".endc"), (".postrun", ".endc"), (".verilog", ".endv")] {
        for (file, prefix) in [("t.sp", "* title\n"), ("t.scs", "// title\nsimulator lang=spice\n")] {
            let body = format!(
                "R1 a 0 1k\n{start}\n* @RNL {{\n{end}_extra\n* @RNL {{ (0,0) @TEXT(\"hidden\") }}\n\
                 .include missing.inc\n.subckt fake a b\nsimulator lang=spectre\n.end\n{end}\n\
                 R2 a 0 2k\n* @RNL {{ (1,2) @TEXT(\"visible\") }}\n"
            );
            let r = read(file, &format!("{prefix}{body}"));
            assert_eq!(r.doc.levels.len(), 1, "{file} {start}");
            assert_eq!(r.doc.main().instances.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["R1", "R2"]);
            let records = record_texts(&r);
            assert_eq!(records.len(), 1, "{file} {start}: {records:?}");
            assert!(records[0].contains("visible"));
            assert!(warning_at(&r, "not read", prefix.lines().count() as u32 + 3));
            assert!(!r.diagnostics().iter().any(|d| d.message.contains("cannot be found")));
        }
    }
}

#[test]
fn reliability_braces_ignore_comments_strings_and_nested_blocks() {
    for opening in ["rel reliability {", "reliability{", "rel reliability\n{", "rel \\\nreliability {"] {
        let text = format!(
            "// title\nR1 (a 0) resistor r=1k\n{opening}\n\
             /* a misleading }}\n// @RNL {{ (0,0) @TEXT(\"hidden comment\") }}\n*/\n\
             data file=\"escaped \\\" }}\"\n nested {{\n// @RNL {{\n}}\n\
             // }}\n* @RNL {{ (0,0) @TEXT(\"hidden star\") }}\n\
             include \"missing.scs\"\nsimulator lang=spice\n}}\n\
             R2 (a 0) resistor r=2k\n// @RNL {{ (1,2) @TEXT(\"visible\") }}\n"
        );
        let r = read("t.scs", &text);
        assert_eq!(
            r.doc.main().instances.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["R1", "R2"],
            "{opening}"
        );
        let records = record_texts(&r);
        assert_eq!(records.len(), 1, "{opening}: {records:?}");
        assert!(records[0].contains("visible"));
        assert!(r.diagnostics().iter().any(|d| d.message.contains("inside `reliability")));
        assert!(!r.diagnostics().iter().any(|d| d.message.contains("cannot be found")));
    }
}

#[test]
fn newjob_has_a_title_only_in_a_directly_read_file() {
    for (file, prefix) in [("t.sp", "* title\n"), ("t.scs", "// title\nsimulator lang=spice\n")] {
        let text = format!("{prefix}.newjob\n* @RNL {{ broken title\n* @RNL {{ (1,2) @TEXT(\"visible\") }}\n");
        let r = read(file, &text);
        assert_eq!(record_texts(&r).len(), 1);
        assert!(warning_at(&r, "after `.newjob`", prefix.lines().count() as u32 + 2));
    }
    // WRspice does not recognize .newjob as a job separator in include/lib files.
    let files = MemoryProvider::new()
        .with("t.sp", "* title\n.include child.sp\n")
        .with("child.sp", "* child\n.newjob\n* @RNL { (1,2) @TEXT(\"included\") }\n");
    let r = interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    assert_eq!(record_texts(&r).len(), 1);
    assert!(record_texts(&r)[0].contains("included"));
}

#[test]
fn literal_conditionals_keep_the_selected_branch_and_unknown_conditions_warn() {
    for (file, prefix, marker, if_, elif, else_, endif) in [
        ("t.sp", "* title\n", "*", ".if", ".elseif", ".else", ".endif"),
        ("t.scs", "// title\nsimulator lang=spice\n", "*", ".if", ".elif", ".else", ".endif"),
        ("t.scs", "// title\n", "//", "#if", "#elif", "#else", "#endif"),
    ] {
        let text = format!(
            "{prefix}{if_} (0)\n{marker} @RNL {{ inactive\n{elif} -1\n\
             {marker} @RNL {{ (1,2) @TEXT(\"selected\") }}\n\
             {if_} 0\n{marker} @RNL {{ nested inactive\n{endif}\n\
             {else_}\n{marker} @RNL {{ inactive alternative\n{endif}\n\
             {if_} externally_defined\n{marker} @RNL {{ unknown\n{else_}\n\
             {marker} @RNL {{ unknown alternative\n{endif}\n\
             {marker} @RNL {{ (3,4) @TEXT(\"after\") }}\n"
        );
        let r = read(file, &text);
        let records = record_texts(&r);
        assert_eq!(records.len(), 2, "{file} {if_}: {records:?}");
        assert!(records[0].contains("selected") && records[1].contains("after"));
        assert!(r.diagnostics().iter().any(|d| d.message.contains("cannot be resolved")));
        assert!(!r.diagnostics().iter().any(|d| d.message.contains("record is incomplete")));
    }
}

#[test]
fn cpp_directive_payload_and_continuations_are_not_carriers() {
    let text = "// title\n#define RECORD \\\n// @RNL { hidden payload \\\n// @RNL { still payload\n#ifdef EXTERNAL\n// @RNL { hidden conditional\n#endif\n\
                // @RNL { (1,2) @TEXT(\"after\") }\n";
    let r = read("t.scs", text);
    assert_eq!(record_texts(&r).len(), 1);
    assert!(warning_at(&r, "preprocessing directive", 3));
    assert!(warning_at(&r, "preprocessing directive", 4));
    assert!(warning_at(&r, "inactive or unresolved", 6));
}

#[test]
fn foreign_language_includes_are_not_followed() {
    let files = MemoryProvider::new()
        .with("t.scs", "// title\nahdl_include \"hidden.va\"\n// @RNL { (1,2) @TEXT(\"after\") }\n")
        .with("hidden.va", "// title\n// @RNL { hidden language\n");
    let r = interpret_file(Path::new("t.scs"), None, &files, &Options { std: false, ..Options::default() }).unwrap();
    assert_eq!(r.doc.sources.len(), 1);
    assert_eq!(record_texts(&r).len(), 1);
}

#[test]
fn writes_move_before_an_unterminated_host_region_and_read_back() {
    for (file, prefix, opening, marker) in [
        ("t.sp", "* title\nR1 a 0 1k\n", ".control", "*"),
        ("t.sp", "* title\nR1 a 0 1k\n", ".exec", "*"),
        ("t.sp", "* title\nR1 a 0 1k\n", ".postrun", "*"),
        ("t.sp", "* title\nR1 a 0 1k\n", ".verilog", "*"),
        ("t.sp", "* title\nR1 a 0 1k\n", ".if unresolved", "*"),
        ("t.sp", "* title\nR1 a 0 1k\n", ".lib unused", "*"),
        ("t.scs", "// title\nR1 (a 0) resistor r=1k\n", "rel reliability {", "//"),
        ("t.scs", "// title\nR1 (a 0) resistor r=1k\n", "#ifdef EXTERNAL", "//"),
        ("t.scs", "// title\nR1 (a 0) resistor r=1k\n", "section unused", "//"),
    ] {
        for eol in ["\n", "\r\n"] {
            let region = format!("{opening}\n{marker} @RNL(type=\"generated\") {{ (0,0) @TEXT(\"hidden\") }}")
                .replace('\n', eol);
            let text = format!("{}{region}", prefix.replace('\n', eol));
            let r = read(file, &text);
            assert_eq!(record_line(&r.doc, 0, &text), Ok(3), "{opening}");
            assert_eq!(strip_generated(&r.doc, file, &text), text, "excluded source is preserved");
            let generated = "@RNL(type=\"generated\") { (1,2) @TEXT(\"written\") }";
            for out in [
                append_record(&r.doc, 0, &text, generated),
                replace_generated_with_header(&r.doc, 0, &text, generated, "0.3"),
            ] {
                assert!(out.ends_with(&region), "{opening}: {out}");
                assert!(out.find("written").unwrap() < out.find(opening).unwrap());
                let reread = read(file, &out);
                assert_eq!(record_texts(&reread).iter().filter(|s| s.contains("written")).count(), 1, "{out}");
                assert!(!record_texts(&reread).iter().any(|s| s.contains("hidden")), "{out}");
            }
        }
    }
}

#[test]
fn records_are_written_inside_the_selected_library_section() {
    for (main, text, lib, source, closing) in [
        ("t.sp", "* title\n.lib lib.sp tt\n", "lib.sp", "* lib\n.lib tt\nR1 a 0 1k\n.endl tt\n.lib ff\nR2 a 0 2k\n.endl ff\n", ".endl tt"),
        ("t.scs", "// title\ninclude \"lib.scs\" section=tt\n", "lib.scs", "// lib\nsection tt\nR1 (a 0) resistor r=1k\nendsection tt\nsection ff\nR2 (a 0) resistor r=2k\nendsection ff\n", "endsection tt"),
    ] {
        let files = MemoryProvider::new().with(main, text).with(lib, source);
        let r = interpret_file(Path::new(main), None, &files, &Options { std: false, ..Options::default() }).unwrap();
        assert_eq!(record_line(&r.doc, 1, source), Ok(4));
        let out = append_record(&r.doc, 1, source, "@RNL { (1,2) @TEXT(\"written\") }");
        assert!(out.find("written").unwrap() < out.find(closing).unwrap());
        let files = MemoryProvider::new().with(main, text).with(lib, &out);
        let reread = interpret_file(Path::new(main), None, &files, &Options { std: false, ..Options::default() }).unwrap();
        assert_eq!(record_texts(&reread).len(), 1, "{out}");
    }
}

#[test]
fn library_section_comments_do_not_change_selection_or_writeback() {
    for (main, prefix, lib, marker, opening, closing, instance, whole, selected, comment) in [
        ("t.sp", "* title\n", "lib.sp", "*", ".lib", ".endl", "a 0 1k", ".include lib.sp", ".lib lib.sp tt", "$"),
        ("t.sp", "* title\n", "lib.sp", "*", ".lib", ".endl", "a 0 1k", ".include lib.sp", ".lib lib.sp tt", ";"),
        (
            "t.scs",
            "// title\nsimulator lang=spice\n",
            "lib.sp",
            "*",
            ".lib",
            ".endl",
            "a 0 1k",
            ".include lib.sp",
            ".lib lib.sp tt",
            "$",
        ),
        (
            "t.scs",
            "// title\nsimulator lang=spice\n",
            "lib.sp",
            "*",
            ".lib",
            ".endl",
            "a 0 1k",
            ".include lib.sp",
            ".lib lib.sp tt",
            ";",
        ),
        (
            "t.scs",
            "// title\n",
            "lib.scs",
            "//",
            "section",
            "endsection",
            "(a 0) resistor r=1k",
            "include \"lib.scs\"",
            "include \"lib.scs\" section=tt",
            "//",
        ),
    ] {
        // Both the unnamed and named closing forms may have a host comment. The first
        // section's body ends at line 5, before the second section and the file's own text.
        let source = format!(
            "{marker} library\n{opening} tt {comment} first section\nR1 {instance}\n\
             {marker} @RNL {{ (1,2) @TEXT(\"selected\") }}\n{closing} {comment} end tt\n\
             {opening} ff {comment} second section\nR2 {instance}\n\
             {marker} @RNL {{ (3,4) @TEXT(\"hidden\") }}\n{closing} ff {comment} end ff\n\
             R3 {instance}\n{marker} @RNL {{ (5,6) @TEXT(\"outside\") }}\n"
        );
        let options = Options { std: false, ..Options::default() };
        let whole_text = format!("{prefix}{whole}\n");
        let files = MemoryProvider::new().with(main, &whole_text).with(lib, &source);
        let r = interpret_file(Path::new(main), None, &files, &options).unwrap();
        let instances = r.doc.levels.iter().flat_map(|l| &l.instances).map(|i| i.name.as_str()).collect::<Vec<_>>();
        assert_eq!(instances, ["R3"], "{main} {comment}");
        let records = record_texts(&r);
        assert_eq!(records.len(), 1, "{main} {comment}: {records:?}");
        assert!(records[0].contains("outside"));

        let selected_text = format!("{prefix}{selected} {comment} selected include\n");
        let files = MemoryProvider::new().with(main, &selected_text).with(lib, &source);
        let r = interpret_file(Path::new(main), None, &files, &options).unwrap();
        let instances = r.doc.levels.iter().flat_map(|l| &l.instances).map(|i| i.name.as_str()).collect::<Vec<_>>();
        assert_eq!(instances, ["R1"], "{main} {comment}");
        assert_eq!(record_texts(&r).len(), 1);
        assert!(record_texts(&r)[0].contains("selected"));
        assert_eq!(record_line(&r.doc, 1, &source), Ok(5));
        let generated = "@RNL(type=\"generated\") { (7,8) @TEXT(\"written\") }";
        for out in [
            append_record(&r.doc, 1, &source, generated),
            replace_generated_with_header(&r.doc, 1, &source, generated, "0.3"),
        ] {
            assert!(out.find("written").unwrap() < out.find(closing).unwrap(), "{out}");
            assert!(out.ends_with(&source[source.find(closing).unwrap()..]), "{out}");
            let files = MemoryProvider::new().with(main, &selected_text).with(lib, &out);
            let reread = interpret_file(Path::new(main), None, &files, &options).unwrap();
            let records = record_texts(&reread);
            assert_eq!(records.len(), 2, "{out}: {records:?}");
            assert!(records.iter().any(|r| r.contains("written")));
            assert!(records.iter().all(|r| !r.contains("hidden") && !r.contains("outside")));
        }
    }
}

#[test]
fn whole_file_and_selected_library_readings_keep_their_own_write_boundaries() {
    for (main, prefix, lib, marker, whole, selected, open, close, if_, endif, subckt, ends, instance) in [
        (
            "t.sp",
            "* title\n",
            "lib.sp",
            "*",
            ".include lib.sp",
            ".lib lib.sp tt",
            ".lib",
            ".endl",
            ".if",
            ".endif",
            ".subckt",
            ".ends",
            "a b 1k",
        ),
        (
            "t.scs",
            "// title\nsimulator lang=spice\n",
            "lib.sp",
            "*",
            ".include lib.sp",
            ".lib lib.sp tt",
            ".lib",
            ".endl",
            ".if",
            ".endif",
            ".subckt",
            ".ends",
            "a b 1k",
        ),
        (
            "t.scs",
            "// title\n",
            "lib.scs",
            "//",
            "include \"lib.scs\"",
            "include \"lib.scs\" section=tt",
            "section",
            "endsection",
            "#if",
            "#endif",
            "subckt",
            "ends",
            "(a b) resistor r=1k",
        ),
    ] {
        let source = format!(
            "{marker} library\n{open} tt\n{marker} @RNL(type=\"header\") {{}}\n\
             {if_} 1\n{subckt} cell a b\nR1 {instance}\n{ends} cell\n{endif}\n{close} tt\n\
             {open} unused\n{marker} @RNL {{ (0,0) @TEXT(\"hidden\") }}\n"
        );
        for order in [[whole, selected], [selected, whole]] {
            let text = format!("{prefix}{}\n{}\n", order[0], order[1]);
            let files = MemoryProvider::new().with(main, &text).with(lib, &source);
            let options = Options { std: false, ..Options::default() };
            let r = interpret_file(Path::new(main), None, &files, &options).unwrap();
            let whole_level =
                r.doc.levels.iter().position(|l| l.file == lib && l.section.is_none() && l.name != "cell").unwrap();
            let selected_level =
                r.doc.levels.iter().position(|l| l.file == lib && l.section.as_deref() == Some("tt")).unwrap();
            let cell = r.doc.levels.iter().position(|l| l.name == "cell").unwrap();
            assert_eq!(r.doc.unwritable_in(whole_level, 6), Some((2, 9)), "{main} {order:?}");
            assert_eq!(r.doc.unwritable_in(selected_level, 6), Some((4, 8)), "real condition still excludes writes");
            assert_eq!(record_line(&r.doc, cell, &source), Err((4, 8)));
            assert_eq!(record_line(&r.doc, whole_level, &source), Ok(10));
            assert_eq!(record_line(&r.doc, selected_level, &source), Ok(9));
            let record = "@RNL(type=\"generated\") { (7,8) @TEXT(\"written\") }";
            for target in [whole_level, selected_level] {
                let out = replace_generated_with_header(&r.doc, target, &source, record, "0.3");
                let files = MemoryProvider::new().with(main, &text).with(lib, &out);
                let reread = interpret_file(Path::new(main), None, &files, &options).unwrap();
                let own = &reread.doc.levels[target].records;
                let other =
                    &reread.doc.levels[if target == whole_level { selected_level } else { whole_level }].records;
                assert!(own.iter().any(|r| r.text.contains("written")), "{main} {order:?}: {out}");
                assert!(!other.iter().any(|r| r.text.contains("written")), "{out}");
                assert!(
                    own.iter().any(|r| r.text.contains("VERSION(\"0.3\")")),
                    "the target's header can be updated: {out}"
                );
                assert!(!record_texts(&reread).iter().any(|r| r.contains("hidden")));
            }
        }
    }
}

#[test]
fn a_missing_library_section_is_diagnosed_and_has_no_write_place() {
    for (main, prefix, lib, marker, selected, open, close, instance) in [
        ("t.sp", "* title\n", "lib.sp", "*", ".lib lib.sp missing", ".lib", ".endl", "a 0 1k"),
        ("t.scs", "// title\nsimulator lang=spice\n", "lib.sp", "*", ".lib lib.sp missing", ".lib", ".endl", "a 0 1k"),
        (
            "t.scs",
            "// title\n",
            "lib.scs",
            "//",
            "include \"lib.scs\" section=missing",
            "section",
            "endsection",
            "(a 0) resistor r=1k",
        ),
    ] {
        let text = format!("{prefix}{selected}\n");
        let source = format!("{marker} library\n{open} tt\nR1 {instance}\n{close} tt\n");
        let files = MemoryProvider::new().with(main, &text).with(lib, &source);
        let r = interpret_file(Path::new(main), None, &files, &Options { std: false, ..Options::default() }).unwrap();
        assert!(r
            .diagnostics()
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains("section missing was not found")));
        let level = r.doc.levels.iter().position(|l| l.section.as_deref() == Some("missing")).unwrap();
        assert_eq!(record_line(&r.doc, level, &source), Err((1, 5)));
        let record = "@RNL(type=\"generated\") { (7,8) @TEXT(\"written\") }";
        assert_eq!(append_record(&r.doc, level, &source, record), source);
        assert_eq!(replace_generated_with_header(&r.doc, level, &source, record, "0.3"), source);
        assert_eq!(rnl_runtime::writeback::with_header(&r.doc, lib, &source, "0.3"), source);
    }
}

#[test]
fn a_subcircuit_entirely_in_a_conditional_has_no_unconditional_write_place() {
    let text = "* title\n.if 1\n.subckt cell a b\nR1 a b 1k\n.ends cell\n.endif\n";
    let r = read("t.sp", text);
    assert_eq!(r.doc.levels.len(), 2);
    assert_eq!(record_line(&r.doc, 1, text), Err((2, 6)));
    assert_eq!(append_record(&r.doc, 1, text, "@RNL {}"), text);
}

#[test]
fn ordinary_comments_that_mention_reliability_remain_carriers() {
    let text = "// title\n// reliability {\n* reliability {\n// @RNL { (1,2) @TEXT(\"visible\") }\n";
    let r = read("t.scs", text);
    assert_eq!(record_texts(&r).len(), 1);
    assert!(r.doc.excluded.is_empty());
}

#[test]
fn encrypted_envelopes_are_not_plaintext_protection_regions() {
    for prefix in ["", "// title\n"] {
        let text = format!(
            "{prefix}//pragma protect begin_protected\n// @RNL {{ hidden payload\n\
                            #if payload\n.include missing\n// pragma protect end_protected\n\
                            // @RNL {{ (1,2) @TEXT(\"visible\") }}\n"
        );
        let r = read("t.scs", &text);
        assert_eq!(record_texts(&r).len(), 1);
        assert!(r.doc.protected.is_empty());
        assert_eq!(r.doc.excluded.len(), 1);
        assert!(r.diagnostics().iter().any(|d| d.message.contains("encrypted text")));
        let out = rnl_runtime::writeback::with_header(&r.doc, "t.scs", &text, "0.3");
        let reread = read("t.scs", &out);
        assert_eq!(record_texts(&reread).len(), 2, "{out}");
        assert!(record_texts(&reread)[0].contains("header"));
        let payload = text[text.find("//pragma").unwrap()..].split("// pragma protect end_protected").next().unwrap();
        assert!(out.contains(payload), "{out}");
    }
}

#[test]
fn a_pending_newjob_title_or_cpp_continuation_is_not_a_write_position() {
    for (file, text, opening) in [
        ("t.sp", "* title\n.newjob", ".newjob"),
        ("t.scs", concat!("// title\n#define VALUE ", "\\", "\n// payload ", "\\"), "#define"),
        ("t.scs", "// title\n//pragma protect begin_protected\n// payload", "//pragma"),
    ] {
        let r = read(file, text);
        let out = replace_generated_with_header(
            &r.doc,
            0,
            text,
            "@RNL(type=\"generated\") { (1,2) @TEXT(\"written\") }",
            "0.3",
        );
        assert!(out.find("written").unwrap() < out.find(opening).unwrap(), "{out}");
        let reread = read(file, &out);
        assert_eq!(record_texts(&reread).len(), 2, "{out}");
    }
}

#[test]
fn reliability_as_an_instance_name_does_not_open_a_foreign_block() {
    let text = "// title\nreliability (a 0) resistor r=1k\nR2 (a 0) resistor r=2k\n\
                // @RNL { (1,2) @TEXT(\"visible\") }\n";
    let r = read("t.scs", text);
    assert_eq!(r.doc.main().instances.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["reliability", "R2"]);
    assert_eq!(record_texts(&r).len(), 1);
    assert!(r.doc.excluded.is_empty());
}

#[test]
fn selected_library_headers_move_before_subcircuits_with_real_reader_boundaries() {
    for (main, prefix, lib, marker, whole, include, open, close, subckt, ends, protect, unprotect, instance) in [
        (
            "t.sp",
            "* title\n",
            "lib.sp",
            "*",
            ".include lib.sp",
            ".lib lib.sp tt",
            ".lib",
            ".endl",
            ".subckt",
            ".ends",
            ".prot",
            ".unprot",
            "a b 1k",
        ),
        (
            "t.scs",
            "// title\nsimulator lang=spice\n",
            "lib.sp",
            "*",
            ".include lib.sp",
            ".lib lib.sp tt",
            ".lib",
            ".endl",
            ".subckt",
            ".ends",
            ".prot",
            ".unprot",
            "a b 1k",
        ),
        (
            "t.scs",
            "// title\n",
            "lib.scs",
            "//",
            "include \"lib.scs\"",
            "include \"lib.scs\" section=tt",
            "section",
            "endsection",
            "subckt",
            "ends",
            "protect",
            "unprotect",
            "(a b) resistor r=1k",
        ),
    ] {
        for includes in [include.to_string(), format!("{whole}\n{include}"), format!("{include}\n{whole}")] {
            for protected in [false, true] {
                let text = format!("{prefix}{includes}\n");
                let protect = if protected { format!("{protect}\n") } else { String::new() };
                let unprotect = if protected { format!("{unprotect}\n") } else { String::new() };
                let source = format!(
                    "{marker} models\n{open} tt\nR0 {instance}\n{protect}{subckt} cell a b\n\
                 {unprotect}R1 {instance}\n{ends} cell\n{marker} @RNL(type=\"header\") {{ }}\n{close} tt\n"
                );
                let options = Options { std: false, ..Options::default() };
                let files = MemoryProvider::new().with(main, &text).with(lib, &source);
                let r = interpret_file(Path::new(main), None, &files, &options).unwrap();
                let selected = r.doc.levels.iter().position(|l| l.section.as_deref() == Some("tt")).unwrap();
                let cell = r.doc.levels.iter().position(|l| l.name == "cell").unwrap();
                assert_eq!(r.doc.levels[selected].section_lines, Some((2, if protected { 10 } else { 8 })));
                let record = "@RNL(type=\"generated\") {}";
                let out = replace_generated_with_header(&r.doc, cell, &source, record, "0.3");
                let expected = format!(
                    "{marker} models\n{open} tt\nR0 {instance}\n\
                 {marker} @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n\
                 {protect}{subckt} cell a b\n{unprotect}R1 {instance}\n\
                 {marker} @RNL(type=\"generated\") {{}}\n{ends} cell\n{close} tt\n"
                );
                assert_eq!(out, expected, "{main}, {includes}, protected={protected}");
                assert_eq!(
                    rnl_runtime::writeback::move_header_for(&r.doc, cell, &source, "0.3"),
                    expected.replace(&format!("{marker} {record}\n"), ""),
                    "the solver's header prepass must stay in the same reading"
                );
                let files = MemoryProvider::new().with(main, &text).with(lib, &out);
                let reread = interpret_file(Path::new(main), None, &files, &options).unwrap();
                assert_eq!(reread.doc.levels[selected].records.len(), 1);
                assert!(reread.doc.levels[selected].records[0].text.contains("VERSION(\"0.3\")"));
                assert_eq!(reread.doc.levels[cell].records.len(), 1);
                assert!(
                    reread
                        .doc
                        .levels
                        .iter()
                        .filter(|l| l.file == lib && l.section.is_none() && l.name != "cell")
                        .all(|l| l.records.is_empty()),
                    "the whole-file reading must not acquire the section header"
                );
                assert_eq!(replace_generated_with_header(&reread.doc, cell, &out, record, "0.3"), out);
            }
        }
    }
}
