//! Names bound in records written into a subcircuit with scope= (standard 2.6, 6.6, 6.7, as the
//! draft 0.3 makes clear in its 7.4): they are published to the subcircuit, in one name space with
//! its own, so that its own records, and the solver's record written in its own text, use them. A
//! name bound again is an error, and the earlier binding is kept: the subcircuit's own text comes
//! first, then the records written with scope=, the outer ones later. Before, such a name could
//! not be found at all, not even in its own record, and the solver's view had no name for it.

use std::path::Path;

use rnl_cli::{interpret_file, read_host};
use rnl_runtime::diag::{Severity, Sink};
use rnl_runtime::problem::{sig1, LevelProblem, PointRef, Position, Reference, Slot};
use rnl_runtime::solution::{write_record, Placement, Solution};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::writeback::replace_generated;
use rnl_runtime::{Interpretation, Options};

/// A main file whose subcircuit ota has `own` as its own record, and that writes `scope` into it.
fn netlist(own: &str, scope: &str) -> String {
    let own = if own.is_empty() { String::new() } else { format!("* @RNL {{ {own} }}\n") };
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\")}}\n.subckt ota a b\nR1 a b 1k\n{own}.ends ota\n\
         X1 n1 n2 ota\n* @RNL(scope=\"ota\") {{ {scope} }}\n.end\n"
    )
}

fn interpret(files: &MemoryProvider, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, files, options).unwrap()
}

fn errors(r: &Interpretation) -> Vec<(u32, String)> {
    let errors = r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error);
    errors.map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone())).collect()
}

fn ota(r: &Interpretation) -> &LevelProblem {
    r.problems.iter().find(|p| p.title == "ota").unwrap()
}

/// The object a level's text refers to by `name`, with where it is fixed.
fn named<'a>(p: &'a LevelProblem, name: &str) -> (usize, &'a Position) {
    let k = p.objects.iter().position(|o| o.reference == Reference::Name(name.into())).unwrap();
    (k, &p.objects[k].position)
}

fn fixed_at(position: &Position) -> Option<(f64, f64)> {
    match position {
        Position::Own { x: Slot::Fixed { value: x, .. }, y: Slot::Fixed { value: y, .. } } => Some((*x, *y)),
        _ => None,
    }
}

#[test]
fn a_name_bound_in_a_scope_record_is_the_subcircuits() {
    let files = MemoryProvider::new().with("t.sp", &netlist("", "%q = (_, _); &a @WIRE {#R1:1 %q}; %q @PLACE(1, 2)"));
    let r = interpret(&files, &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    // the solver's view: %q is placed at (1, 2), wired, and written back by its name
    let r = interpret(&files, &Options::solver());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let p = ota(&r);
    assert_eq!(p.names, ["q"]);
    let (q, position) = named(p, "%q");
    assert_eq!(fixed_at(position), Some((1.0, 2.0)));
    assert_eq!(p.wires[0].anchors[1], PointRef::Origin(q));
    // as `rnl problem` prints it
    let level = rnl_cli::problem::level(p, "tolerance=0.000001");
    assert_eq!(level["objects"][q]["reference"], "%q");

    // drawn: with R1 placed, the wire ends at %q
    let files = MemoryProvider::new()
        .with("t.sp", &netlist("", "%q = (_, _); &a @WIRE {#R1:1 %q}; %q @PLACE(1, 2); #R1 @PLACE(-4, 0)"));
    let r = interpret(&files, &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let lv = r.model.levels.iter().find(|l| l.title == "ota").unwrap();
    assert_eq!(lv.wires[0].points.last(), Some(&(1.0, 2.0)));
}

#[test]
fn a_name_bound_in_a_scope_record_is_used_from_the_subcircuits_own_record() {
    let files = MemoryProvider::new().with("t.sp", &netlist("%q @PLACE(1, 2)", "%q = (_, _); &a @WIRE {#R1:1 %q}"));
    let r = interpret(&files, &Options::solver());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    let p = ota(&r);
    let (q, position) = named(p, "%q");
    assert_eq!(fixed_at(position), Some((1.0, 2.0)));
    assert_eq!(p.wires[0].anchors[1], PointRef::Origin(q));
}

#[test]
fn a_name_bound_again_in_a_scope_record_is_an_error() {
    // the subcircuit's own text comes first: its %q is kept, and the wire goes to it
    let files = MemoryProvider::new().with("t.sp", &netlist("%q = (5, 6)", "%q = (1, 2); &a @WIRE {#R1:1 %q}"));
    let r = interpret(&files, &Options::solver());
    let e = errors(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].0, 8, "{e:?}");
    assert!(e[0].1.contains("%q is already bound to another object in this level; the first binding is kept (6.6)"));
    let p = ota(&r);
    let (q, position) = named(p, "%q");
    assert_eq!(fixed_at(position), Some((5.0, 6.0)));
    assert_eq!(p.wires[0].anchors[1], PointRef::Origin(q));

    // of two levels that write into ota, the outer one comes later: the main file's binding fails
    let lib = "* the library\n.subckt ota a b\nR1 a b 1k\n.ends ota\n* @RNL(scope=\"ota\") { %q = (5, 6) }\n";
    let main = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.2\")}\n.include \"lib.inc\"\nX1 n1 n2 ota\n\
                * @RNL(scope=\"ota\") { %q = (1, 2); &a @WIRE {#R1:1 %q} }\n.end\n";
    let files = MemoryProvider::new().with("t.sp", main).with("lib.inc", lib);
    let r = interpret(&files, &Options::solver());
    let e = errors(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert_eq!(e[0].0, 5, "{e:?}");
    let p = ota(&r);
    assert_eq!(fixed_at(named(p, "%q").1), Some((5.0, 6.0)));
}

#[test]
fn the_solver_writes_such_a_point_back_by_its_name() {
    // the solver's record is written in the subcircuit's own text (2.6, 14.8), where the name holds
    let text = netlist("", "%q = (_, _); &a @WIRE {#R1:1 %q}");
    let files = MemoryProvider::new().with("t.sp", &text);
    let r = interpret(&files, &Options::solver());
    let p = ota(&r);
    let (q, _) = named(p, "%q");
    let solution = Solution {
        placements: vec![Placement { object: q, at: Some((3.0, 4.0)), orient: None }],
        ..Solution::default()
    };
    let written = write_record(p, &solution, "rnl-solver test", &sig1(p, "tolerance=0.000001"));
    assert!(written.unsolved.is_empty(), "{:?}", written.unsolved);
    assert!(written.record.contains("%q @PLACE(3, 4)"), "{}", written.record);
    let doc = read_host(Path::new("t.sp"), None, &files, &mut Sink::new()).unwrap();
    let solved = MemoryProvider::new().with("t.sp", &replace_generated(&doc, p.level, &text, &written.record));
    let r = interpret(&solved, &Options { problems: true, ..Options::default() });
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(fixed_at(named(ota(&r), "%q").1), Some((3.0, 4.0)));
}
