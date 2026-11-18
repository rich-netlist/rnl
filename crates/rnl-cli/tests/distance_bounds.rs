//! DISTANCE bounds in files of 0.4 (14.5), including the version of scoped records and bodies.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{codes, Code, Severity};
use rnl_runtime::problem::FactKind;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn netlist(version: &str, body: &str) -> String {
    format!("* distance\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\nR2 b 0 1k\n* @RNL {{ {body} }}\n.end\n")
}

fn interpret(files: MemoryProvider) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, &files, &Options { std: false, ..Options::solver() }).unwrap()
}

fn read(version: &str, body: &str) -> Interpretation {
    interpret(MemoryProvider::new().with("t.sp", &netlist(version, body)))
}

fn errors(r: &Interpretation) -> Vec<Code> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.code).collect()
}

fn bounds(r: &Interpretation) -> Vec<(Option<f64>, Option<f64>)> {
    r.problems
        .iter()
        .flat_map(|p| &p.facts)
        .filter_map(|f| match &f.kind {
            FactKind::Distance { min, max, .. } => Some((*min, *max)),
            _ => None,
        })
        .collect()
}

#[test]
fn negative_bounds_fail_in_every_direction() {
    for direction in ["left-to-right", "right-to-left", "bottom-to-top", "top-to-bottom"] {
        for bound in ["min", "max", "exact"] {
            let body = format!("#R1 #R2 @DISTANCE(\"{direction}\", {bound}=-1)");
            let r = read("0.4", &body);
            assert_eq!(errors(&r), [codes::DISTANCE_NEGATIVE_BOUND_04], "{body}");
            assert!(bounds(&r).is_empty(), "{body}");
            assert!(r.diagnostics().iter().any(|d| r.sink.render(d).contains(&format!("{bound}=-1"))));
        }
    }
}

#[test]
fn omitted_min_is_zero_and_zero_is_a_valid_bound() {
    for (written, expected) in [
        ("max=10", (Some(0.0), Some(10.0))),
        ("min=0", (Some(0.0), None)),
        ("max=0", (Some(0.0), Some(0.0))),
        ("exact=0", (Some(0.0), Some(0.0))),
        ("min=2,max=4", (Some(2.0), Some(4.0))),
    ] {
        let r = read("0.4", &format!("#R1 #R2 @DISTANCE(\"left-to-right\", {written})"));
        assert_eq!(errors(&r), [], "{written}");
        assert_eq!(bounds(&r), [expected], "{written}");
    }
}

#[test]
fn invalid_combinations_still_fail() {
    for (written, code) in [
        ("", codes::PARAM_DISTANCE_NO_BOUND),
        (",exact=2,min=0", codes::PARAM_DISTANCE_EXACT_WITH_BOUNDS),
        (",exact=2,max=4", codes::PARAM_DISTANCE_EXACT_WITH_BOUNDS),
        (",min=3,max=2", codes::PARAM_DISTANCE_MIN_ABOVE_MAX),
        (",min=0,max=-1", codes::DISTANCE_NEGATIVE_BOUND_04),
    ] {
        let r = read("0.4", &format!("#R1 #R2 @DISTANCE(\"left-to-right\"{written})"));
        assert_eq!(errors(&r), [code], "{written}");
        assert!(bounds(&r).is_empty());
    }
}

/// A file of 0.4 that includes `inc.sp`, whose subcircuit `child` has R1 and R2, and writes
/// `statement` in a record scoped to it.
fn scoped(statement: &str) -> Interpretation {
    interpret(
        MemoryProvider::new()
            .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ {statement} }}\n.end\n"))
            .with("inc.sp", "* inc\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.subckt child a b\nR1 a b 1k\nR2 b 0 1k\n.ends\n"),
    )
}

#[test]
fn scoped_constraints_have_the_bounds_of_0_4() {
    let r = scoped("#R1 #R2 @DISTANCE(\"left-to-right\", max=10)");
    assert_eq!(errors(&r), []);
    assert_eq!(bounds(&r), [(Some(0.0), Some(10.0))]);
    let r = scoped("#R1 #R2 @DISTANCE(\"left-to-right\", min=-1)");
    assert_eq!(errors(&r), [codes::DISTANCE_NEGATIVE_BOUND_04]);
}

#[test]
fn definition_bodies_of_a_package_have_the_bounds_of_0_4() {
    let files = MemoryProvider::new()
        .with("p.rnl", "@RNL(type=\"header\") { @VERSION(\"0.4\"); @PACKAGE(name=\"p\",version=\"1.0\") }\n@RNL { $D = @DEFINE(roles=[\"a\",\"b\"]) { ?a ?b @DISTANCE(\"left-to-right\", max=10) } }")
        .with("t.sp", &netlist("0.4", "#R1 #R2 @AS $D").replace(" }\nR1 a b", "; @USE-PACKAGE(name=\"p\",version=\"1.0\",path=\"p.rnl\") }\nR1 a b"));
    let r = interpret(files);
    assert_eq!(errors(&r), []);
    assert_eq!(bounds(&r), [(Some(0.0), Some(10.0))]);
}

#[test]
fn a_scoped_distribute_takes_a_direction() {
    let r = scoped("{#R1 #R2} @DISTRIBUTE(\"left-to-right\")");
    assert_eq!(errors(&r), []);
}
