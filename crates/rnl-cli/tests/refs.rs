//! What the interpretation tells a tool about where host objects are named and drawn: the
//! references in the records, and the instances and nets behind each drawn item.

use std::path::Path;

use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::model::RefTarget;
use rnl_runtime::{Interpretation, Options};

fn interpret(name: &str) -> (Interpretation, Vec<String>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases").join(name);
    let text = std::fs::read_to_string(&path).unwrap();
    let r = interpret_file(&path, None, &FsProvider::new(&[]), &Options::default()).unwrap();
    (r, text.lines().map(String::from).collect())
}

/// The 1-based column where `needle` starts on a 1-based line.
fn col(lines: &[String], line: u32, needle: &str) -> u32 {
    lines[line as usize - 1].find(needle).unwrap() as u32 + 1
}

#[test]
fn references_name_their_instances_and_nets() {
    let (r, lines) = interpret("01-rc-lowpass.sp");
    let main = &r.doc.levels[0];
    let v1 = main.instances.iter().position(|i| i.name == "V1").unwrap();
    let r1 = main.instances.iter().position(|i| i.name == "R1").unwrap();
    let at = |line: u32, needle: &str| {
        let c = col(&lines, line, needle);
        r.refs.iter().filter(|x| x.loc.line == line && x.loc.col == c).collect::<Vec<_>>()
    };
    // `&in @WIRE(…) {#V1:1 #R1:1}`: the net, and the instances without their terminals
    let net = at(8, "&in");
    assert_eq!(net.len(), 1);
    assert_eq!((net[0].len, &net[0].target), (3, &RefTarget::Net("in".into())));
    let v = at(8, "#V1:1");
    assert_eq!((v[0].len, &v[0].target, v[0].level), (3, &RefTarget::Instance(v1), 0));
    assert_eq!(at(8, "#R1:1")[0].target, RefTarget::Instance(r1));
    // the solver's record names them too
    assert_eq!(at(19, "#R1")[0].target, RefTarget::Instance(r1));
    // each place once, however often it was evaluated
    let mut seen = std::collections::HashSet::new();
    assert!(r.refs.iter().all(|x| seen.insert(x.clone())));
}

#[test]
fn drawn_items_know_their_instances_and_pin_nets() {
    let (r, _) = interpret("01-rc-lowpass.sp");
    let lv = &r.model.levels[0];
    assert_eq!(lv.level, 0);
    let r1 = r.doc.levels[0].instances.iter().position(|i| i.name == "R1").unwrap();
    let item = lv.items.iter().find(|i| i.host == vec![r1]).unwrap();
    let mut nets: Vec<&str> = item.pin_nets.iter().flatten().map(String::as_str).collect();
    nets.sort();
    assert_eq!(nets, ["in", "out"]);
    // a usage, such as a ground stub, has no host instance
    assert!(lv.items.iter().any(|i| i.kind == "usage" && i.host.is_empty()));
}
