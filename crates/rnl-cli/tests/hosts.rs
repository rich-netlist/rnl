//! What the records rely on that the host adapters read: the ports of a subcircuit written on the
//! `+` lines that continue its `.subckt`, which name the terminals of its instances (6.4, 6.12),
//! in a SPICE netlist and in the SPICE mode of a Spectre one, and none from an end-of-line comment.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(main: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(main, text);
    interpret_file(Path::new(main), None, &files, &Options { std: false, ..Options::default() }).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

#[test]
fn a_port_on_a_continuation_line_names_a_terminal_of_the_instances() {
    let body = ".subckt cell a\n+ b\nR1 a b 1k\n.ends cell\nX1 in out cell\n* @RNL { #X1:b @TEXT(\"second port\") }\n";
    for (main, text) in [
        ("t.sp", format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n{body}.end\n")),
        ("t.scs", format!("// t\nsimulator lang=spice\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n{body}.end\n")),
    ] {
        let r = read(main, &text);
        assert!(errors(&r).is_empty(), "{main}: {:?}", errors(&r));
        let cell = r.doc.levels.iter().find(|l| l.name == "cell").unwrap();
        assert_eq!(cell.ports, ["a", "b"], "{main}");
    }
}

#[test]
fn an_end_of_line_comment_of_a_subckt_holds_no_port() {
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\n.subckt cell a b ; two ports\nR1 a b 1k\n\
                .ends cell\nX1 in out cell\n.end\n";
    let r = read("t.sp", text);
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let cell = r.doc.levels.iter().find(|l| l.name == "cell").unwrap();
    assert_eq!(cell.ports, ["a", "b"]);
}

#[test]
fn a_comment_line_within_a_statement_is_said_in_both_adapters() {
    // a comment line between a `.subckt` and the `+` line that continues it, or between an element
    // and its own: read, with a warning, in a SPICE netlist and in the SPICE mode of a Spectre one
    // (2.2); one after the last `+` line is a comment like any other
    let body =
        ".subckt cell a\n* between\n+ b\nR1 a b\n* again\n+ 1k\n* after\nR2 a b 1k\n.ends cell\nX1 in out cell\n";
    for (main, text, first) in [
        ("t.sp", format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n{body}.end\n"), 4),
        (
            "t.scs",
            format!("// t\nsimulator lang=spice\n* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}\n{body}.end\n"),
            5,
        ),
    ] {
        let r = read(main, &text);
        assert!(errors(&r).is_empty(), "{main}: {:?}", errors(&r));
        let within = r
            .diagnostics()
            .into_iter()
            .filter(|d| {
                d.severity == Severity::Warning && d.message.contains("between a statement and its continuation")
            })
            .filter_map(|d| d.loc.as_ref().map(|l| l.line))
            .collect::<Vec<_>>();
        assert_eq!(within, [first, first + 3], "{main}");
        assert_eq!(r.doc.levels.iter().find(|l| l.name == "cell").unwrap().ports, ["a", "b"], "{main}");
    }
}
