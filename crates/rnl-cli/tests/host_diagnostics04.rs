//! Host diagnostics follow each file or library-section reading's language version.
use rnl_cli::interpret_file;
use rnl_runtime::{
    diag::{codes, Code},
    source::MemoryProvider,
    Interpretation, Options,
};
use std::path::Path;

fn header(version: &str, spectre: bool) -> String {
    format!("{} @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\n", if spectre { "//" } else { "*" })
}
fn read(path: &str, provider: MemoryProvider) -> Interpretation {
    interpret_file(Path::new(path), None, &provider, &Options { std: false, ..Default::default() }).unwrap()
}
fn one(version: &str, body: &str, spectre: bool) -> Interpretation {
    let path = if spectre { "main.scs" } else { "main.sp" };
    read(
        path,
        MemoryProvider::new()
            .with(path, &format!("{} host\n{}{body}", if spectre { "//" } else { "*" }, header(version, spectre))),
    )
}
fn diagnostic(r: &Interpretation, code: Code) -> &rnl_runtime::diag::Diag {
    let all: Vec<_> = r.diagnostics().into_iter().filter(|d| d.code == code).collect();
    assert_eq!(all.len(), 1, "{:?}", r.diagnostics());
    all[0]
}

#[test]
fn duplicate_instances_name_the_first_definition_in_both_dialects() {
    for spectre in [false, true] {
        let r = one(
            "0.4",
            if spectre { "R1 (a b) resistor r=1k\nR1 (a b) resistor r=2k\n" } else { "R1 a b 1k\nR1 a b 2k\n" },
            spectre,
        );
        let d = diagnostic(&r, codes::HOST_ELEMENT_DEFINED_TWICE);
        let path = if spectre { "main.scs" } else { "main.sp" };
        assert_eq!(d.message, format!("instance R1 is defined twice; first defined at {path}:3 (2.6)"));
        assert_eq!(d.loc.as_ref().unwrap().line, 4);
    }
}

#[test]
fn an_unclosed_subcircuit_is_named_at_its_declaration() {
    for spectre in [false, true] {
        let r = one(
            "0.4",
            if spectre {
                "subckt open_cell (a b)\nR1 (a b) resistor r=1k\n"
            } else {
                ".subckt open_cell a b\nR1 a b 1k\n"
            },
            spectre,
        );
        let d = diagnostic(&r, codes::HOST_SUBCKT_NO_ENDS);
        assert!(d.message.starts_with("subcircuit open_cell (main."));
        assert!(d.message.ends_with("has no closing ends declaration (2.6)"));
        assert_eq!(d.loc.as_ref().unwrap().line, 3);
    }
}

#[test]
fn nested_unclosed_subcircuits_are_all_identified() {
    for spectre in [false, true] {
        let r = one(
            "0.4",
            if spectre { "subckt outer (a)\nsubckt inner (b)\n" } else { ".subckt outer a\n.subckt inner b\n" },
            spectre,
        );
        let d = diagnostic(&r, codes::HOST_SUBCKT_NO_ENDS);
        assert!(d.message.contains("outer (main.") && d.message.contains("inner (main."), "{}", d.message);
        assert!(d.message.contains("have no closing ends declaration"));
        assert_eq!(d.loc.as_ref().unwrap().line, 3);
    }
}

#[test]
fn first_line_warnings_have_the_same_wording() {
    for spectre in [false, true] {
        let path = if spectre { "main.scs" } else { "main.sp" };
        let prefix = if spectre { "//" } else { "*" };
        let r =
            read(path, MemoryProvider::new().with(path, &format!("{prefix} @RNL {{}}\n{}", header("0.4", spectre))));
        assert_eq!(
            diagnostic(&r, codes::HOST_RNL_ON_FIRST_LINE).message,
            "the first line of a file is not read as RNL (2.2)"
        );
    }
}

#[test]
fn a_missing_section_points_to_the_directive_that_requested_it() {
    for spectre in [false, true] {
        let path = if spectre { "main.scs" } else { "main.sp" };
        let library = if spectre { "lib.scs" } else { "lib.sp" };
        let include = if spectre { "include \"lib.scs\" section=missing\n" } else { ".lib \"lib.sp\" missing\n" };
        let main = format!("{} main\n{}{include}", if spectre { "//" } else { "*" }, header("0.4", spectre));
        let r = read(
            path,
            MemoryProvider::new().with(path, &main).with(library, if spectre { "// library\n" } else { "* library\n" }),
        );
        let d = diagnostic(&r, codes::HOST_SECTION_NOT_FOUND);
        assert!(d.message.contains(&format!("section missing was not found in {library}")));
        let loc = d.loc.as_ref().unwrap();
        assert_eq!(loc.file, path);
        assert_eq!(loc.line, 3);
    }
}

#[test]
fn mixed_includes_use_the_version_of_each_reading() {
    for root in ["0.3", "0.4"] {
        let child = if root == "0.3" { "0.4" } else { "0.3" };
        let main = format!("* main\n{}.include child.sp\n", header(root, false));
        let child_text = format!("* child\n{}.subckt open_cell a b\nR1 a b 1k\n", header(child, false));
        let r = read("main.sp", MemoryProvider::new().with("main.sp", &main).with("child.sp", &child_text));
        let d = diagnostic(&r, codes::HOST_SUBCKT_NO_ENDS);
        assert_eq!(d.loc.as_ref().unwrap().line, if child == "0.4" { 3 } else { 4 });
        assert_eq!(d.message.contains("open_cell"), child == "0.4");
    }
}

#[test]
fn shared_raw_diagnostics_keep_both_modern_and_legacy_readings() {
    let main = format!("* main\n{}.include modern.sp\n.include legacy.sp\n", header("0.4", false));
    let modern = format!("* modern\n{}.include shared.sp\n", header("0.4", false));
    let legacy = format!("* legacy\n{}.include shared.sp\n", header("0.3", false));
    let r = read(
        "main.sp",
        MemoryProvider::new()
            .with("main.sp", &main)
            .with("modern.sp", &modern)
            .with("legacy.sp", &legacy)
            .with("shared.sp", "* @RNL {}\n"),
    );
    let messages: Vec<_> = r
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == codes::HOST_RNL_ON_FIRST_LINE)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(messages.len(), 2, "{:?}", r.diagnostics());
    assert!(messages.contains(&"the first line of a file is not read as RNL (2.2)"));
    assert!(messages.contains(&"the first line of a file is a title and is not read as RNL (2.2)"));
}

#[test]
fn released_versions_keep_the_original_host_wording_and_locations() {
    for version in ["0.2", "0.3"] {
        for spectre in [false, true] {
            let r = one(
                version,
                if spectre {
                    "subckt open_cell (a b)\nR1 (a b) resistor r=1k\n"
                } else {
                    ".subckt open_cell a b\nR1 a b 1k\n"
                },
                spectre,
            );
            let d = diagnostic(&r, codes::HOST_SUBCKT_NO_ENDS);
            assert_eq!(d.message, if spectre { "a subcircuit has no `ends`" } else { "a subcircuit has no `.ends`" });
            assert_eq!(d.loc.as_ref().unwrap().line, 4);
        }
    }
}

#[test]
fn ignored_library_records_use_the_selected_readings_version() {
    for spectre in [false, true] {
        for selected in [false, true] {
            let path = if spectre { "main.scs" } else { "main.sp" };
            let library = if spectre { "lib.scs" } else { "lib.sp" };
            let comment = if spectre { "//" } else { "*" };
            let include = match (spectre, selected) {
                (true, true) => "include \"lib.scs\" section=tt",
                (true, false) => "include \"lib.scs\"",
                (false, true) => ".lib \"lib.sp\" tt",
                (false, false) => ".include \"lib.sp\"",
            };
            let main = format!("{comment} main\n{}{include}\n", header("0.3", spectre));
            let (open, close) = if spectre { ("section tt", "endsection tt") } else { (".lib tt", ".endl tt") };
            let text = if selected {
                format!("{comment} library\n{comment} @RNL {{}}\n{open}\n{}{close}\n", header("0.4", spectre))
            } else {
                format!("{comment} library\n{}{open}\n{comment} @RNL {{}}\n{close}\n", header("0.4", spectre))
            };
            let r = read(path, MemoryProvider::new().with(path, &main).with(library, &text));
            let (code, message) = if selected {
                (codes::HOST_RNL_OUTSIDE_SELECTED_SECTION, "RNL outside the selected library section is not read (2.2)")
            } else {
                (
                    codes::HOST_RNL_IN_UNSELECTED_SECTION,
                    "RNL in a library section that is not selected is not read (2.2)",
                )
            };
            assert_eq!(diagnostic(&r, code).message, message);
        }
    }
}

#[test]
fn a_document_can_be_reinterpreted_with_an_empty_host_sink() {
    let source = "* @RNL {}\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n";
    let provider = MemoryProvider::new().with("main.sp", source);
    let original = read("main.sp", provider.clone());
    assert_eq!(original.doc.diagnostic_readings.len(), 1);
    let again = rnl_runtime::interp::interpret(
        original.doc,
        &provider,
        &Options { std: false, ..Default::default() },
        rnl_runtime::diag::Sink::new(),
    );
    assert!(again.diagnostics().is_empty(), "{:?}", again.diagnostics());
}

#[test]
fn an_old_host_index_never_rewrites_a_different_diagnostic() {
    use rnl_runtime::diag::{Loc, Sink};
    let source = "* @RNL {}\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n";
    let provider = MemoryProvider::new().with("main.sp", source);
    let original = read("main.sp", provider.clone());
    let mut sink = Sink::new();
    sink.warning(codes::HOST_RNL_ON_FIRST_LINE, "caller's distinct diagnostic", Some(Loc::line("elsewhere.sp", 1)));
    let again =
        rnl_runtime::interp::interpret(original.doc, &provider, &Options { std: false, ..Default::default() }, sink);
    let d = diagnostic(&again, codes::HOST_RNL_ON_FIRST_LINE);
    assert_eq!(d.message, "caller's distinct diagnostic");
    assert_eq!(d.loc.as_ref().unwrap().file, "elsewhere.sp");
}
