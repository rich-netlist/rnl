//! The pin of a virtual part whose role takes a net, such as a label or a supply stub, is on that
//! net in the problem (`ProblemNet::pins`), as the pins of instances are on theirs.
use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::problem::LevelProblem;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};
use std::path::Path;

fn read(name: &str, text: &str, std: bool) -> Interpretation {
    let r = interpret_file(
        Path::new(name),
        None,
        &MemoryProvider::new().with(name, text),
        &Options { std, ..Options::solver() },
    )
    .unwrap();
    assert!(!r.diagnostics().iter().any(|d| d.severity == Severity::Error), "{:?}", r.diagnostics());
    r
}

/// The objects whose pins the problem lists on the net `net`.
fn on_net(p: &LevelProblem, net: &str) -> Vec<String> {
    let net = p.nets.iter().find(|n| n.name == net).unwrap_or_else(|| panic!("&{net}"));
    net.pins.iter().filter_map(|q| q.pin_key()).map(|(o, _)| p.ids.objects[o].clone()).collect()
}

#[test]
fn the_pin_of_a_part_attached_to_a_net_is_on_that_net() {
    let text = "* label\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a n 1k\nR2 n b 1k\n* @RNL {\n*   $S = @DEFINE-SYMBOL { (0,0) @PIN(terminal=1); (2,0) @PIN(terminal=2) };\n*   # @SELECT-INSTANCE => @SET-SYMBOL $S;\n*   $L = @DEFINE(roles=[[\"p\", \"net\"]]) {};\n*   $LS = @DEFINE-SYMBOL { (0,0) @PIN(terminal=\"p\") };\n*   %l = $L @ATTACH &n;\n*   %l @SET-SYMBOL $LS;\n* }\n.end\n";
    let r = read("main.sp", text, false);
    let on = on_net(&r.problems[0], "n");
    assert!(on.iter().any(|o| o == "%l"), "{on:?}");
    assert!(!on_net(&r.problems[0], "a").iter().any(|o| o == "%l"));
}

#[test]
fn the_pins_of_supply_parts_of_the_standard_packages_are_on_their_nets() {
    let text = "* sampler\n* @RNL(type=\"header\") {\n*   @VERSION(\"0.4\");\n*   @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\");\n*   @DEVICE(name=\"M*\", model=\"n*\", terminals=[\"d\", \"g\", \"s\", \"b\"], type=\"nmos\");\n*   @DEVICE(name=\"M*\", model=\"p*\", terminals=[\"d\", \"g\", \"s\", \"b\"], type=\"pmos\");\n* }\n.model n1 nmos level=1\n.model p1 pmos level=1\n.subckt inv a y vdd gnd\nMC y a vdd vdd p1\nMD y a gnd gnd n1\n* @RNL {\n*   &vdd @ADD-TO $SUPPLY_NET_SET;\n*   &gnd @ADD-TO $GROUND_NET_SET;\n*   #MC(pmos) #MD(nmos) @AS $CMOS_INVERTER;\n*   %s = $SUPPLY @ATTACH &vdd;\n*   %t = $SUPPLY_STUB @ATTACH #MC::s;\n* }\n.ends inv\n";
    let r = read("inv.sp", text, true);
    let p = r.problems.iter().find(|p| p.nets.iter().any(|n| n.name == "vdd")).expect("the level of inv");
    let on = on_net(p, "vdd");
    assert!(on.iter().any(|o| o == "%s"), "{on:?}");
    // the $SUPPLY that the stub at #MC::s makes, a part of the package
    assert!(on.iter().any(|o| o.starts_with("@rnl-std-basic.rnl:")), "{on:?}");
}
