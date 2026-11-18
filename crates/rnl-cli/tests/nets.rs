//! The nets of a level in the problem a solver reads (6.3): each net with the pins on it, the
//! wires that draw it, the ROUTE-NETs that ask for it and its place among the ports, in the order
//! the netlist first names them; and the nets that each virtual part and each text is on.

use std::path::Path;

use rnl_cli::{interpret_file, FsProvider};
use rnl_runtime::problem::{LevelProblem, PointRef};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const NET: &str = "* nets\n* @RNL(type=\"header\") { @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\") }\n.subckt amp in out vdd vss\n\
                   M1 out in vss vss nch\nR1 vdd out 10k\nC1 out vss 1p\n\
                   * @RNL {\n\
                   *   $GROUND @ATTACH &vss;\n\
                   *   &out @WIRE {#M1:1 #R1:2};\n\
                   *   %w = &vss @WIRE {#M1:3 #C1:2};\n\
                   *   &out @TEXT(\"vout\");\n\
                   *   #R1:1 @TEXT(\"v\");\n\
                   *   %w @TEXT(\"ground wire\");\n\
                   *   $IO_PORT @ATTACH &in;\n\
                   *   &in @ROUTE-NET\n\
                   * }\n.ends amp\nX1 a b c 0 amp\n.end\n";

fn read(text: &str) -> Interpretation {
    let files = MemoryProvider::new().with("t.sp", text);
    interpret_file(Path::new("t.sp"), None, &files, &Options::solver()).unwrap()
}

fn level<'a>(r: &'a Interpretation, title: &str) -> &'a LevelProblem {
    r.problems.iter().find(|p| p.title == title).unwrap_or_else(|| panic!("no level {title}"))
}

/// The nets of the pins on net `name`, as the labels of their objects, in order.
fn on(p: &LevelProblem, name: &str) -> Vec<String> {
    let net = p.nets.iter().find(|n| n.name == name).unwrap_or_else(|| panic!("no net {name}"));
    net.pins
        .iter()
        .map(|q| match q {
            PointRef::Pin { object, .. }
            | PointRef::SymbolPin { object, .. }
            | PointRef::PinPosition { object, .. } => p.objects[*object].label.clone(),
            other => panic!("not a pin: {other:?}"),
        })
        .collect()
}

#[test]
fn the_nets_of_a_level_with_their_pins_wires_routes_and_ports() {
    let r = read(NET);
    let p = level(&r, "amp");
    // in the order the netlist names them: the ports of the subcircuit first
    let names: Vec<&str> = p.nets.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["in", "out", "vdd", "vss"]);
    let ports: Vec<Option<usize>> = p.nets.iter().map(|n| n.port).collect();
    assert_eq!(ports, [Some(0), Some(1), Some(2), Some(3)]);
    assert!(p.nets.iter().all(|n| n.reference == rnl_runtime::problem::Reference::Name(format!("&{}", n.name))));
    // every pin on a net, in the order of the objects: the instances, then the virtual parts:
    // the ground on vss, the label %l that the standard package's fill puts on each unwired pin
    // and the port it draws on each port of the subcircuit
    assert_eq!(on(p, "out"), ["#M1", "#R1", "#C1", "%l", "a usage of $IO_PORT"]);
    assert_eq!(on(p, "vdd"), ["#R1", "%l", "a usage of $IO_PORT"]);
    let vss = on(p, "vss");
    assert!(vss.contains(&"#M1".to_string()) && vss.contains(&"#C1".to_string()), "{vss:?}");
    assert!(vss.iter().any(|l| l == "a usage of $GROUND"), "{vss:?}");
    let mut sorted = p.nets.iter().find(|n| n.name == "vss").unwrap().pins.clone();
    sorted.sort_by_key(|q| match q {
        PointRef::Pin { object, pin, .. }
        | PointRef::SymbolPin { object, pin, .. }
        | PointRef::PinPosition { object, pin, .. } => (*object, *pin),
        _ => (usize::MAX, 0),
    });
    assert_eq!(sorted, p.nets.iter().find(|n| n.name == "vss").unwrap().pins);
    // the wires that draw each net, and the ROUTE-NET that asks for one; the port on in is written
    // so that the fill gives the pin of M1 on in no label of its own, which would unroute it
    let wires = |name: &str| p.nets.iter().find(|n| n.name == name).unwrap().wires.clone();
    assert_eq!((wires("out").len(), wires("vss").len(), wires("in").len()), (1, 1, 0));
    assert_eq!(p.wires[wires("vss")[0]].nets, ["vss"]);
    let routes = |name: &str| p.nets.iter().find(|n| n.name == name).unwrap().routes.clone();
    assert_eq!((routes("in"), routes("out")), (vec![0], vec![]));
    assert_eq!(p.routes[0].net, "in");
    // the file level: its nets, none a port
    let top = level(&r, "t.sp");
    let names: Vec<&str> = top.nets.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c", "0"]);
    assert!(top.nets.iter().all(|n| n.port.is_none()));
    // the fill of rnl-std-all reaches the file level too: the unwired pin of X1 on a gets its label
    assert_eq!(on(top, "a"), ["#X1", "%l"]);
}

#[test]
fn the_nets_of_a_virtual_part_and_of_a_text() {
    let r = read(NET);
    let p = level(&r, "amp");
    let index = |name: &str| p.nets.iter().position(|n| n.name == name).unwrap();
    let nets = |label: &str| {
        let o = p.objects.iter().find(|o| o.label == label).unwrap_or_else(|| panic!("no {label}"));
        o.nets.clone()
    };
    // the ground on vss, by its role
    assert_eq!(nets("a usage of $GROUND"), [index("vss")]);
    // the instances list none: their pins say where they are
    assert!(nets("#M1").is_empty() && nets("#R1").is_empty());
    // a text on a net, on a terminal, on a wire
    let texts: Vec<(Vec<String>, Vec<usize>)> = p
        .objects
        .iter()
        .filter(|o| o.text.is_some())
        .map(|o| (o.text.as_ref().unwrap().lines.iter().map(|l| format!("{l:?}")).collect(), o.nets.clone()))
        .collect();
    let of =
        |content: &str| texts.iter().find(|(lines, _)| lines.iter().any(|l| l.contains(content))).unwrap().1.clone();
    assert_eq!(of("vout"), [index("out")]);
    assert_eq!(of("\"v\""), [index("vdd")]);
    assert_eq!(of("ground wire"), [index("vss")]);
}

#[test]
fn a_bus_pin_of_a_bundle_is_on_each_net_of_its_members() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/cases-0.4/from-0.2/09-buses-and-bundles.sp");
    let r = interpret_file(&corpus, None, &FsProvider::new(&[]), &Options::solver()).unwrap();
    let mut seen = 0;
    for p in &r.problems {
        for (k, o) in p.objects.iter().enumerate() {
            if o.kind != rnl_runtime::problem::ObjectKind::Bundle {
                continue;
            }
            // a pin of the bundle on more than one net: the bus
            let mut by_pin: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
            for n in &p.nets {
                for q in &n.pins {
                    if let PointRef::Pin { object, pin, .. } = q {
                        if *object == k {
                            *by_pin.entry(*pin).or_default() += 1;
                        }
                    }
                }
            }
            assert!(by_pin.values().any(|&count| count > 1), "{}: {by_pin:?}", o.label);
            seen += 1;
        }
    }
    assert!(seen > 0, "the case has a bundle");
}

#[test]
fn the_nets_go_into_the_input_digest() {
    // connecting a terminal elsewhere changes the problem's nets and so its sig2 digest
    let r = read(NET);
    let moved = read(&NET.replace("R1 vdd out 10k", "R1 vdd in 10k"));
    let (p, q) = (level(&r, "amp"), level(&moved, "amp"));
    assert_ne!(p.nets, q.nets);
    assert_ne!(rnl_runtime::problem::input_digest(p), rnl_runtime::problem::input_digest(q));
    // read twice, the same
    assert_eq!(read(NET).problems, r.problems);
}
