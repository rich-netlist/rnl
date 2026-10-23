//! Named pins are matched in the owning object's dialect, independently of the defining file.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::model::{ItemView, LevelView};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn read(path: &str, text: &str) -> Interpretation {
    let files = MemoryProvider::new().with(path, text);
    let r = interpret_file(Path::new(path), None, &files, &Options { std: false, ..Options::solver() }).unwrap();
    let errors: Vec<_> = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).collect();
    assert!(errors.is_empty(), "{:?}", errors.iter().map(|d| r.sink.render(d)).collect::<Vec<_>>());
    r
}

fn item<'a>(lv: &'a LevelView, name: &str) -> &'a ItemView {
    lv.items.iter().find(|i| i.id == name).unwrap()
}

#[test]
fn spectre_case_distinct_pins_keep_separate_nets_and_wire_endpoints() {
    let r = read(
        "pins.scs",
        r#"simulator lang=spectre
// @RNL(type="header") { @VERSION("0.3") }
subckt cell (P p)
ends cell
I0 (upper lower) cell
// @RNL {
// $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal="P"); (2,0) @PIN(terminal="p") };
// #I0 @SET-SYMBOL $S => @PLACE(0,0);
// &upper @WIRE(mode="straight") {#I0:P (-5,0)};
// &lower @WIRE(mode="straight") {#I0:p (5,0)};
// #I0 @SELECT-TERMINAL(terminal="P") => @TEXT("upper", key="mark", dx=0, dy=1)
// }
"#,
    );
    let lv = &r.model.levels[0];
    let i = item(lv, "#I0");
    assert_eq!(i.pin_nets, [vec!["upper"], vec!["lower"]]);
    assert_eq!(lv.symbols[i.symbol].pins.iter().map(|p| p.pos).collect::<Vec<_>>(), [(-2.0, 0.0), (2.0, 0.0)]);
    assert_eq!(lv.wires.iter().map(|w| w.points[0]).collect::<Vec<_>>(), [(-2.0, 0.0), (2.0, 0.0)]);
    assert_eq!(lv.texts.iter().filter(|t| t.markup == "upper").count(), 1);
}

#[test]
fn a_spice_defined_symbol_is_reused_by_both_host_modes_without_losing_bus_members() {
    let r = read(
        "mixed.scs",
        r#"simulator lang=spectre
// @RNL(type="header") { @VERSION("0.3") }
simulator lang=spice
R1 positive negative 1k
* @RNL {
* $BASE = @DEFINE-SYMBOL { (-3,0) @PIN(terminal=["P","N"]) };
* $S = @DEFINE-SYMBOL { $BASE; (3,0) @PIN(terminal="p") };
* #R1 @SET-SYMBOL $S => @PLACE(10,0)
* }
simulator lang=spectre
subckt cell (P p N)
ends cell
I0 (upper lower other) cell
// @RNL { #I0 @SET-SYMBOL $S => @PLACE(0,0) }
"#,
    );
    let lv = &r.model.levels[0];
    let spice = item(lv, "#R1");
    let spectre = item(lv, "#I0");
    assert_eq!(spice.pin_nets, [vec!["negative"], vec!["positive"]]);
    assert_eq!(spectre.pin_nets, [vec!["upper", "other"], vec!["lower"]]);
    let folded = &lv.symbols[spice.symbol];
    let exact = &lv.symbols[spectre.symbol];
    assert_eq!(folded.pins[0].terminals, ["N"]);
    assert_eq!(exact.pins[0].terminals, ["P", "N"]);
    assert_eq!(folded.name, exact.name);
    assert_eq!(folded.origin, exact.origin);
    // The solver sees the same per-owner symbol and pin mapping as the renderer.
    assert_eq!(r.problems[0].symbols.len(), 2);
    assert!(r.problems[0].symbols.iter().any(|s| s.pins[0].terminals == ["N"]));
    assert!(r.problems[0].symbols.iter().any(|s| s.pins[0].terminals == ["P", "N"]));
}

#[test]
fn spice_overrides_are_insensitive_but_usage_role_names_remain_sensitive() {
    let r = read(
        "roles.sp",
        r#"* role names
* @RNL(type="header") { @VERSION("0.3") }
R1 positive negative 1k
* @RNL {
* $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal="P"); (2,0) @PIN(terminal="p") };
* $PAIR = @DEFINE(roles=["P","p"]) {};
* %pair = &positive(P) &negative(p) @AS $PAIR => @SET-SYMBOL $S => @PLACE(0,0);
* #R1 @SET-SYMBOL $S => @PLACE(10,0)
* }
.end
"#,
    );
    let lv = &r.model.levels[0];
    let spice = item(lv, "#R1");
    let usage = item(lv, "%pair");
    assert_eq!(lv.symbols[spice.symbol].pins.len(), 1);
    assert_eq!(lv.symbols[spice.symbol].pins[0].pos, (2.0, 0.0));
    assert_eq!(spice.pin_nets, [vec!["positive"]]);
    assert_eq!(lv.symbols[usage.symbol].pins.len(), 2);
    assert_eq!(usage.pin_nets, [vec!["positive"], vec!["negative"]]);
}

#[test]
fn block_symbol_metadata_matches_its_instances_in_each_host_dialect() {
    for (path, text, instance, count) in [
        (
            "block.sp",
            r#"* block pins
* @RNL(type="header") { @VERSION("0.3") }
* @RNL { $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal="P"); (2,0) @PIN(terminal="p"); (0,-2) @PIN(terminal="n") } }
.subckt cell p n
R0 p n 1k
* @RNL { # @SET-SYMBOL $S }
.ends cell
X1 a b cell
* @RNL { #X1 @PLACE(0,0) }
.end
"#,
            "#X1",
            2,
        ),
        (
            "block.scs",
            r#"simulator lang=spectre
// @RNL(type="header") { @VERSION("0.3") }
// @RNL { $S = @DEFINE-SYMBOL { (-2,0) @PIN(terminal="P"); (2,0) @PIN(terminal="p"); (0,-2) @PIN(terminal="n") } }
subckt cell (P p n)
R0 (p n) resistor r=1k
// @RNL { # @SET-SYMBOL $S }
ends cell
I0 (upper lower other) cell
// @RNL { #I0 @PLACE(0,0) }
"#,
            "#I0",
            3,
        ),
    ] {
        let r = read(path, text);
        let top = &r.model.levels[0];
        let instance = item(top, instance);
        let definition = r.model.levels.iter().find(|lv| lv.title == "cell").unwrap().block_symbol.as_ref().unwrap();
        assert_eq!(definition.view.pins.len(), count, "{path}");
        assert_eq!(definition.view.pins, top.symbols[instance.symbol].pins, "{path}");
        assert_eq!(definition.info.name, "$S");
    }
}
