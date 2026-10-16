//! The terminal names that the draft 0.3 gives T, O, U and Z by default (6.12), in files that
//! declare 0.3 read with `Options::draft`: `p1 n1 p2 n2` for T and O, `a b ref` for U and `d g s`
//! for Z, with the adapter's defaults of the SPICE family. They are the names of the instances
//! written in such a file, in a SPICE netlist and in the SPICE mode of a Spectre one, and a
//! DEVICE of the file still overrides them. An instance written in a file of 0.2 has none, with
//! the switch or without, as before.

use std::path::Path;

use rnl_cli::{dump, interpret_file};
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn draft() -> Options {
    Options { std: false, draft: true, ..Options::default() }
}

fn read(main: &str, files: &MemoryProvider, options: &Options) -> Interpretation {
    interpret_file(Path::new(main), None, files, options).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

/// The terminal names of the instances of the main level, by instance.
fn names(r: &Interpretation) -> Vec<(String, Vec<Option<String>>)> {
    let lv = &r.model.levels[0];
    let insts = &r.doc.levels[lv.level].instances;
    insts.iter().map(|i| i.name.clone()).zip(lv.terminal_names.iter().cloned()).collect()
}

fn named(names: &[&str]) -> Vec<Option<String>> {
    names.iter().map(|n| Some(n.to_string())).collect()
}

const ELEMENTS: &str = "T1 a 0 b 0 z0=50 td=1n\nO1 a 0 b 0 lossy\nU1 a b 0 urc\nZ1 d g 0 mesf\n\
                        .model lossy ltra\n.model urc urc\n.model mesf nmf\n";

/// A SPICE netlist of `version` with one each of T, O, U and Z, and `records`.
fn netlist(version: &str, header: &str, records: &str) -> String {
    format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){header}}}\n{ELEMENTS}{records}.end\n")
}

#[test]
fn t_o_u_and_z_have_terminal_names_in_a_file_of_0_3() {
    let wires = "* @RNL { &a @WIRE {#T1:p1 #O1:p1 #U1:a}; &b @WIRE {#T1:p2 #O1:p2 #U1:b}; &0 @WIRE {#T1:n2 #O1:n1 #U1:ref #Z1:s} }\n";
    let files = MemoryProvider::new().with("t.sp", &netlist("0.3", "", wires));
    let r = read("t.sp", &files, &draft());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let tl = named(&["p1", "n1", "p2", "n2"]);
    assert_eq!(
        names(&r),
        [
            ("T1".into(), tl.clone()),
            ("O1".into(), tl),
            ("U1".into(), named(&["a", "b", "ref"])),
            ("Z1".into(), named(&["d", "g", "s"])),
        ]
    );
}

#[test]
fn a_device_of_the_file_still_overrides_them() {
    let header = "; @DEVICE(name=\"Z*\", terminals=[\"drain\", \"gate\", \"source\"])";
    let r = read("t.sp", &MemoryProvider::new().with("t.sp", &netlist("0.3", header, "")), &draft());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&r)[3], ("Z1".into(), named(&["drain", "gate", "source"])));
    assert_eq!(names(&r)[0].1, named(&["p1", "n1", "p2", "n2"]));
}

#[test]
fn a_file_of_0_2_has_no_names_for_them() {
    let files = MemoryProvider::new().with("t.sp", &netlist("0.2", "", ""));
    let r = read("t.sp", &files, &draft());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    for (inst, names) in names(&r) {
        assert!(names.iter().all(Option::is_none), "{inst}: {names:?}");
    }
    // and reads as it did, with the switch or without; a name of 0.3 is not a terminal there
    let wire = "* @RNL { &a @WIRE {#T1:p1 #U1:a} }\n";
    let files = MemoryProvider::new().with("t.sp", &netlist("0.2", "", wire));
    let without = read("t.sp", &files, &Options { std: false, ..Options::default() });
    assert_eq!(dump::interpretation(&read("t.sp", &files, &draft())), dump::interpretation(&without));
    assert!(!errors(&without).is_empty());
}

#[test]
fn the_names_follow_the_file_the_instance_is_written_in() {
    // the elements of an included file outside a subcircuit belong to the circuit that includes
    // it (6.1), and have the names of the file they are written in
    let part = |version: &str| {
        format!("* part\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nT2 c 0 d 0 z0=50 td=1n\n")
    };
    let main = |version: &str| {
        format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n.include part.inc\nT1 a 0 b 0 z0=50 td=1n\n.end\n")
    };
    let tl = named(&["p1", "n1", "p2", "n2"]);
    let none = vec![None; 4];
    for (outer, inner, want) in [("0.3", "0.2", [tl.clone(), none.clone()]), ("0.2", "0.3", [none, tl])] {
        let files = MemoryProvider::new().with("t.sp", &main(outer)).with("part.inc", &part(inner));
        let r = read("t.sp", &files, &draft());
        assert!(errors(&r).is_empty(), "{:?}", errors(&r));
        let got: Vec<Vec<Option<String>>> = names(&r).into_iter().map(|(_, n)| n).collect();
        // T2 is read first, at the include
        assert_eq!(got, [want[1].clone(), want[0].clone()], "{outer} including {inner}");
    }
}

#[test]
fn the_spice_mode_of_a_spectre_netlist_has_them_too() {
    let net = "// t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.3\")}\nR0 (a 0) resistor r=1k\n\
               simulator lang=spice\nT1 a 0 b 0 z0=50 td=1n\nsimulator lang=spectre\n";
    let r = read("n.scs", &MemoryProvider::new().with("n.scs", net), &draft());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&r)[1], ("T1".into(), named(&["p1", "n1", "p2", "n2"])));
    // a Spectre primitive keeps the names of its master
    assert_eq!(names(&r)[0], ("R0".into(), named(&["p", "n"])));
}
