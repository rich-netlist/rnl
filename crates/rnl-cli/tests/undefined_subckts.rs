//! A subcircuit that no file read defines (6.4). It is normal for a PDK device, which a rule or a
//! package gives a symbol; one that nothing gives a symbol is drawn as the renderer's box of its
//! terminals (9.2.6), and that is most often an include that is missing. Each such subcircuit of a
//! level gets one hint, naming the instances drawn so. Before, nothing said so, and the viewers
//! showed a box with no word of why.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn interpret(net: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", net);
    interpret_file(Path::new("t.sp"), None, &files, &Options::default()).unwrap()
}

/// The hints that name an undefined subcircuit, with their lines.
fn hints(r: &Interpretation) -> Vec<(u32, String)> {
    r.diagnostics()
        .into_iter()
        .filter(|d| d.severity == Severity::Hint && d.message.contains("no file read defines"))
        .map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone()))
        .collect()
}

#[test]
fn a_subcircuit_no_file_defines_is_told_once_with_its_instances() {
    let r = interpret(
        "* t\n.subckt known a b\nR1 a b 1k\n.ends\nX1 a b L3_CDAC_V1\nX2 c d L3_CDAC_V1\nX3 a 0 known\nX4 a 0 other\n.end\n",
    );
    let said = hints(&r);
    assert_eq!(
        said,
        [
            (5, "no file read defines the subcircuit L3_CDAC_V1, so X1, X2 are drawn as a box of its terminals; an include may be missing (6.4, 9.2.6)".to_string()),
            (8, "no file read defines the subcircuit other, so X4 is drawn as a box of its terminals; an include may be missing (6.4, 9.2.6)".to_string()),
        ]
    );
    // a hint: nothing fails, and `rnl check` still passes
    assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
}

#[test]
fn a_subcircuit_a_rule_gives_a_symbol_is_not_told() {
    // as a PDK device is drawn by the symbol its package or a rule gives it
    let r = interpret(
        "* t\nX1 a b pdk_res\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n* @RNL { #X1 @SET-SYMBOL $RESISTOR }\n.end\n",
    );
    assert!(hints(&r).is_empty(), "{:?}", r.diagnostics());
}

#[test]
fn each_level_tells_its_own() {
    let r = interpret("* t\n.subckt cell a b\nXa a b leaf\n.ends\nX1 a b cell\nX2 a b leaf\n.end\n");
    // the main file's level first, which instantiates the subcircuit, then the subcircuit's
    let lines: Vec<u32> = hints(&r).into_iter().map(|(line, _)| line).collect();
    assert_eq!(lines, [6, 3]);
}
