//! Focused regressions found while auditing chapters 9–15.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(path: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(path, text);
    let r = interpret_file(Path::new(path), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let errors: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect();
    assert!(errors.is_empty(), "{errors:?}");
    r
}

#[test]
fn a_shorter_bundle_member_contributes_only_its_existing_terminals() {
    // 11.8 keeps a heterogeneous bundle with a warning. Its first member gives the pins, and a
    // terminal of the bundle stands for the terminals of that name of the members: the resistor
    // has none of the MOS's names, so these pins retain only the MOS's nets.
    let r = read(
        "mixed.sp",
        r#"* mixed bundle
* @RNL(type="header") { @VERSION("0.4") }
M1 drain gate source bulk nch
R1 rtop rbottom 1k
.model nch nmos
* @RNL {
*   $S = @DEFINE-SYMBOL {
*     (0, 3) @PIN(terminal=1); (0, 1) @PIN(terminal=2);
*     (0, -1) @PIN(terminal=3); (0, -3) @PIN(terminal=4)
*   };
*   %b = {#M1 #R1} @BUNDLE => @SET-SYMBOL $S => @PLACE(0, 0);
*   %b:3::net @WIRE(mode="straight") {%b:3 (5, -1)};
*   %b @SELECT-TERMINAL(net="source") => @TEXT("third", key="mark", dx=2, dy=-1)
* }
.end
"#,
    );
    let warnings: Vec<String> =
        r.diagnostics().into_iter().filter(|d| d.severity == Severity::Warning).map(|d| r.sink.render(d)).collect();
    assert!(warnings.iter().any(|w| w.contains("not the same kind")), "{warnings:?}");
    let top = &r.model.levels[0];
    let bundle = top.items.iter().find(|i| i.kind == "bundle").unwrap();
    assert_eq!(bundle.pin_nets, [["drain"], ["gate"], ["source"], ["bulk"]]);
    assert_eq!(top.wires.len(), 1);
    assert_eq!(top.wires[0].nets, ["source"]);
    assert_eq!(top.texts.iter().filter(|t| t.markup == "third").count(), 1);
}

#[test]
fn terminal_selections_obey_the_hosts_case_rules() {
    // These are distinct Spectre ports. Only the uppercase terminal matches "P".
    let spectre = read(
        "case.scs",
        r#"simulator lang=spectre
// @RNL(type="header") { @VERSION("0.4") }
subckt cell (P p)
ends cell
I0 (upper lower) cell
// @RNL {
//   $S = @DEFINE-SYMBOL { (-1, 0) @PIN(terminal=1); (1, 0) @PIN(terminal=2) };
//   #I0 @SET-SYMBOL $S => @PLACE(0, 0);
//   #I0 @SELECT-TERMINAL(terminal="P") => @TEXT("selected", key="mark", dx=0, dy=2)
// }
"#,
    );
    let selected: Vec<_> = spectre.model.levels[0].texts.iter().filter(|t| t.markup == "selected").collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].attach.as_deref(), Some("#I0:1"));

    // SPICE terminal names remain case insensitive.
    let spice = read(
        "case.sp",
        r#"* terminal case
* @RNL(type="header") { @VERSION("0.4") }
R1 upper lower 1k
* @RNL {
*   $S = @DEFINE-SYMBOL { (-1, 0) @PIN(terminal="p"); (1, 0) @PIN(terminal="n") };
*   #R1 @SET-SYMBOL $S => @PLACE(0, 0);
*   #R1 @SELECT-TERMINAL(terminal="P") => @TEXT("selected", key="mark", dx=0, dy=2)
* }
.end
"#,
    );
    assert_eq!(spice.model.levels[0].texts.iter().filter(|t| t.markup == "selected").count(), 1);

    // In a Spectre file the instance's own mode decides, not the file's initial mode.
    let mixed = read(
        "mixed.scs",
        r#"simulator lang=spectre
// @RNL(type="header") { @VERSION("0.4") }
simulator lang=spice
R1 upper lower 1k
* @RNL {
*   $S = @DEFINE-SYMBOL { (-1, 0) @PIN(terminal="P"); (1, 0) @PIN(terminal="N") };
*   #R1 @SET-SYMBOL $S => @PLACE(0, 0);
*   #R1:P @TEXT("direct", key="direct", dx=0, dy=-2);
*   #R1 @SELECT-TERMINAL(terminal="P") => @TEXT("selected", key="mark", dx=0, dy=2)
* }
"#,
    );
    assert_eq!(mixed.model.levels[0].texts.iter().filter(|t| t.markup == "selected").count(), 1);
    assert_eq!(mixed.model.levels[0].texts.iter().filter(|t| t.markup == "direct").count(), 1);
}
