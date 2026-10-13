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
    // a terminal reference also reaches the net at the terminal: `#R1:1` is on `in`
    let t = at(8, "#R1:1");
    assert!(t.iter().any(|x| x.len == 5 && x.target == RefTarget::Net("in".into())), "{t:?}");
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

/// The text of a failed span on one line.
fn failed_text(lines: &[String], f: &rnl_runtime::model::FailedSpan) -> String {
    assert_eq!(f.start.line, f.end.line);
    lines[f.start.line as usize - 1]
        .chars()
        .skip(f.start.col as usize - 1)
        .take((f.end.col - f.start.col + 1) as usize)
        .collect()
}

#[test]
fn statements_that_do_not_take_effect_are_listed_with_why() {
    // the marks in the case itself; the standard package has its own
    let own = |r: &Interpretation, name: &str, lines: &[String]| -> Vec<(bool, String)> {
        r.failed.iter().filter(|f| f.start.file.ends_with(name)).map(|f| (f.conflict, failed_text(lines, f))).collect()
    };
    let (r, lines) = interpret("12-diagnostics.sp");
    let failed = own(&r, "12-diagnostics.sp", &lines);
    // a wrong parameter fails the statement; an unknown symbol fails its segment
    assert!(failed.contains(&(false, "#C1 @PLACE(x=1, 2)".into())), "{failed:?}");
    assert!(failed.contains(&(false, "#R1 @SET-SYMBOL $NOPE".into())), "{failed:?}");
    // a placement that loses to another in the order of 8.5 is dropped, not failed
    assert!(failed.contains(&(true, "#R2 @PLACE(9, 9)".into())), "{failed:?}");
    // a record whose scope finds no subcircuit fails whole; so does a definition body's host reference
    assert!(failed.iter().any(|(c, t)| !c && t.starts_with("@RNL(scope=\"nosuch\")")), "{failed:?}");
    assert!(failed.contains(&(false, "#R1 @PLACE(0, 0)".into())), "{failed:?}");
    // a document whose statements all work lists none
    assert!(interpret("01-rc-lowpass.sp").0.failed.iter().all(|f| !f.start.file.ends_with("01-rc-lowpass.sp")));

    let (r, lines) = interpret("10-constraints-and-priority.sp");
    let failed = own(&r, "10-constraints-and-priority.sp", &lines);
    assert!(failed.iter().all(|(c, _)| *c), "{failed:?}");
    assert!(failed.contains(&(true, "{#R3 #R4} @ALIGN(\"y\")".into())), "{failed:?}");
}

#[test]
fn a_symbol_overridden_for_every_object_is_listed_as_overridden() {
    use rnl_runtime::source::MemoryProvider;
    let text = "* t
R1 a 0 1k
R2 a b 1k
* @RNL(type=\"header\") { @VERSION(\"0.2\")}
* @RNL {
*   #R1 @SET-SYMBOL $CAP;
*   #R1 @SET-SYMBOL $IND;
*   {#R1 #R2} @SET-SYMBOL $DIODE;
*   #R2 @SET-SYMBOL $VSRC
* }
.end
";
    let mem = MemoryProvider::new().with("doc.sp", text);
    let fs = FsProvider::new(&[]);
    let both = rnl_runtime::source::Layered { layers: vec![&mem, &fs] };
    let mut sink = rnl_runtime::diag::Sink::new();
    let doc = rnl_cli::read_host(Path::new("doc.sp"), None, &both, &mut sink).unwrap();
    let r = rnl_runtime::interpret(doc, &both, &Options::default(), sink);
    let lines: Vec<String> = text.lines().map(String::from).collect();
    let mut over: Vec<String> =
        r.failed.iter().filter(|f| f.conflict && f.start.file == "doc.sp").map(|f| failed_text(&lines, f)).collect();
    over.sort();
    // the later statement wins each object: the group wins R1 over both earlier ones, and loses
    // only R2, so it takes effect
    assert_eq!(over, ["#R1 @SET-SYMBOL $CAP", "#R1 @SET-SYMBOL $IND"]);
}
