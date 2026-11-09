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

#[test]
fn released_files_keep_negative_directed_bounds_and_an_omitted_min() {
    for version in ["0.2", "0.3"] {
        for (written, expected) in [
            ("min=-2", (Some(-2.0), None)),
            ("max=-1", (None, Some(-1.0))),
            ("exact=-1", (Some(-1.0), Some(-1.0))),
            ("max=10", (None, Some(10.0))),
        ] {
            let r = read(version, &format!("#R1 #R2 @DISTANCE(\"left-to-right\", {written})"));
            assert_eq!(errors(&r), [], "{version}: {written}");
            assert_eq!(bounds(&r), [expected], "{version}: {written}");
        }
        let r = read(version, "#R1 #R2 @DISTANCE(\"x\", min=-1)");
        assert_eq!(errors(&r), [codes::PARAM_DISTANCE_NEGATIVE_WITHOUT_SENSE]);
    }
}

#[test]
fn scoped_constraints_use_the_source_file_version() {
    for (outer, inner) in [("0.4", "0.3"), ("0.3", "0.4")] {
        for written in ["max=10", "min=-1"] {
            let files = MemoryProvider::new()
                .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ #R1 #R2 @DISTANCE(\"left-to-right\", {written}) }}\n.end\n"))
                .with("inc.sp", &format!("* inc\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\nR2 b 0 1k\n.ends\n"));
            let r = interpret(files);
            if written == "min=-1" && outer == "0.4" {
                assert_eq!(errors(&r), [codes::DISTANCE_NEGATIVE_BOUND_04]);
            } else {
                assert_eq!(errors(&r), []);
                let expected = if written == "min=-1" {
                    (Some(-1.0), None)
                } else {
                    ((outer == "0.4").then_some(0.0), Some(10.0))
                };
                assert_eq!(bounds(&r), [expected], "{outer} into {inner}: {written}");
            }
        }
    }
}

#[test]
fn definition_bodies_use_the_package_version() {
    for (host, package) in [("0.4", "0.3"), ("0.3", "0.4")] {
        let files = MemoryProvider::new()
            .with("p.rnl", &format!("@RNL(type=\"header\") {{ @VERSION(\"{package}\"); @PACKAGE(name=\"p\",version=\"1\") }}\n@RNL {{ $D = @DEFINE(roles=[\"a\",\"b\"]) {{ ?a ?b @DISTANCE(\"left-to-right\", max=10) }} }}"))
            .with("t.sp", &netlist(host, "#R1 #R2 @AS $D").replace(" }\nR1 a b", "; @USE-PACKAGE(name=\"p\",version=\"1\",path=\"p.rnl\") }\nR1 a b"));
        let r = interpret(files);
        assert_eq!(errors(&r), []);
        assert_eq!(bounds(&r), [((package == "0.4").then_some(0.0), Some(10.0))]);
    }
}

#[test]
fn scoped_direction_signatures_follow_the_source_file() {
    for (outer, inner, direction) in [("0.4", "0.3", "left-to-right"), ("0.3", "0.4", "x")] {
        let files = MemoryProvider::new()
            .with("t.sp", &format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{outer}\") }}\n.include \"inc.sp\"\n* @RNL(scope=\"child\") {{ {{#R1 #R2}} @DISTRIBUTE(\"{direction}\") }}\n.end\n"))
            .with("inc.sp", &format!("* inc\n* @RNL(type=\"header\") {{ @VERSION(\"{inner}\") }}\n.subckt child a b\nR1 a b 1k\nR2 b 0 1k\n.ends\n"));
        let r = interpret(files);
        assert_eq!(errors(&r), [], "{outer} into {inner}");
    }
}
