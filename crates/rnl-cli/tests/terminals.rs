//! The terminal names that the adapter gives T, O, U and Z by default (6.12): `p1 n1 p2 n2` for T
//! and O, `a b ref` for U and `d g s` for Z, with the other defaults of the SPICE family. They are
//! the names of the instances in a SPICE netlist and in the SPICE mode of a Spectre one, also in
//! included files and library sections, and a DEVICE of the file still overrides them.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn options() -> Options {
    Options { std: false, ..Options::default() }
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

/// A SPICE netlist with one each of T, O, U and Z, and `records`.
fn netlist(header: &str, records: &str) -> String {
    format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"){header} }}\n{ELEMENTS}{records}.end\n")
}

#[test]
fn t_o_u_and_z_have_terminal_names() {
    let wires = "* @RNL { &a @WIRE {#T1:p1 #O1:p1 #U1:a}; &b @WIRE {#T1:p2 #O1:p2 #U1:b}; &0 @WIRE {#T1:n2 #O1:n1 #U1:ref #Z1:s} }\n";
    let files = MemoryProvider::new().with("t.sp", &netlist("", wires));
    let r = read("t.sp", &files, &options());
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
    let r = read("t.sp", &MemoryProvider::new().with("t.sp", &netlist(header, "")), &options());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&r)[3], ("Z1".into(), named(&["drain", "gate", "source"])));
    assert_eq!(names(&r)[0].1, named(&["p1", "n1", "p2", "n2"]));
}

#[test]
fn the_elements_of_an_included_file_have_them_too() {
    // the elements of an included file outside a subcircuit belong to the circuit that includes
    // it (6.1); one included file declares its version, the other follows the file including it
    let tl = named(&["p1", "n1", "p2", "n2"]);
    for header in ["* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n", ""] {
        let part = format!("* part\n{header}T2 c 0 d 0 z0=50 td=1n\n");
        let main =
            "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include part.inc\nT1 a 0 b 0 z0=50 td=1n\n.end\n";
        let files = MemoryProvider::new().with("t.sp", main).with("part.inc", &part);
        let r = read("t.sp", &files, &options());
        assert!(errors(&r).is_empty(), "{:?}", errors(&r));
        let got: Vec<Vec<Option<String>>> = names(&r).into_iter().map(|(_, n)| n).collect();
        // T2 is read first, at the include
        assert_eq!(got, [tl.clone(), tl.clone()], "{header:?}");
    }
}

#[test]
fn the_instances_of_library_sections_have_them_too() {
    // a section read is a file (2.8): its instances have the names, in a subcircuit and outside
    // one, whichever section is read first
    let lib = "* lib\n.lib old\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt legacy a b c d\n\
               T1 a b c d z0=50 td=1n\n.ends legacy\nT3 e 0 f 0 z0=50 td=1n\n.endl old\n\
               .lib new\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt modern a b c d\n\
               T2 a b c d z0=50 td=1n\n* @RNL { #T2:p1 @TEXT(\"first port\") }\n.ends modern\n\
               T4 g 0 h 0 z0=50 td=1n\n.endl new\n";
    let tl = named(&["p1", "n1", "p2", "n2"]);
    for order in [["old", "new"], ["new", "old"]] {
        let main = format!(
            "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.lib v.lib {}\n.lib v.lib {}\n\
             X1 a b c d legacy\nX2 a b c d modern\n.end\n",
            order[0], order[1]
        );
        let files = MemoryProvider::new().with("t.sp", &main).with("v.lib", lib);
        let r = read("t.sp", &files, &options());
        assert!(errors(&r).is_empty(), "{order:?}: {:?}", errors(&r));
        let of = |title: &str, name: &str| {
            let lv = r.model.levels.iter().find(|l| l.title == title).unwrap();
            let insts = &r.doc.levels[lv.level].instances;
            lv.terminal_names[insts.iter().position(|i| i.name == name).unwrap()].clone()
        };
        assert_eq!((of("legacy", "T1"), of("modern", "T2")), (tl.clone(), tl.clone()), "{order:?}");
        // outside a subcircuit, the elements of a section belong to the circuit that reads it (6.1)
        assert_eq!((of("t.sp", "T3"), of("t.sp", "T4")), (tl.clone(), tl.clone()), "{order:?}");
    }
}

#[test]
fn a_section_without_a_header_read_through_two_includes_has_them_in_both() {
    // one library section without a header record, read by a subcircuit of each of two files:
    // read twice, it is two file levels (2.8, 16.1), and its instances have the names in both; in
    // a SPICE netlist, and in a Spectre one whose section is in SPICE mode
    let tl = named(&["p1", "n1", "p2", "n2"]);
    let shared = "* shared\n.lib shared\nT1 a b c d z0=50 td=1n\n.endl shared\n";
    let spice = |name: &str, records: &str| {
        format!(
            "* {name}\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.subckt {name} a b c d\n\
             .lib \"shared.lib\" shared\n{records}.ends {name}\n"
        )
    };
    let spectre = |name: &str, records: &str| {
        format!(
            "// {name}\n// @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\nsubckt {name} a b c d\n\
             include \"shared.lib\" section=shared\n{records}ends {name}\n"
        )
    };
    let first_port = "* @RNL { #T1:p1 @TEXT(\"first port\") }\n";
    let sp = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.include \"old.inc\"\n.include \"new.inc\"\n\
              X1 a b c d old\nX2 a b c d new\n.end\n";
    let scs = "// t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.4\") }\ninclude \"old.scs\"\n\
               include \"new.scs\"\nX1 (a b c d) old\nX2 (a b c d) new\n";
    let spice_files = MemoryProvider::new()
        .with("t.sp", sp)
        .with("old.inc", &spice("old", ""))
        .with("new.inc", &spice("new", first_port))
        .with("shared.lib", shared);
    let spectre_files = MemoryProvider::new()
        .with("t.scs", scs)
        .with("old.scs", &spectre("old", ""))
        .with("new.scs", &spectre("new", &first_port.replacen('*', "//", 1)))
        .with("shared.lib", shared);
    for (main, files) in [("t.sp", spice_files), ("t.scs", spectre_files)] {
        let r = read(main, &files, &options());
        assert!(errors(&r).is_empty(), "{main}: {:?}", errors(&r));
        let of = |title: &str| {
            let lv = r.model.levels.iter().find(|l| l.title == title).unwrap();
            lv.terminal_names[0].clone()
        };
        assert_eq!((of("old"), of("new")), (tl.clone(), tl.clone()), "{main}");
    }
}

#[test]
fn the_spice_mode_of_a_spectre_netlist_has_them_too() {
    let net = "// t\nsimulator lang=spectre\n// @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR0 (a 0) resistor r=1k\n\
               simulator lang=spice\nT1 a 0 b 0 z0=50 td=1n\nsimulator lang=spectre\n";
    let r = read("n.scs", &MemoryProvider::new().with("n.scs", net), &options());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(names(&r)[1], ("T1".into(), named(&["p1", "n1", "p2", "n2"])));
    // a Spectre primitive keeps the names of its master
    assert_eq!(names(&r)[0], ("R0".into(), named(&["p", "n"])));
}
