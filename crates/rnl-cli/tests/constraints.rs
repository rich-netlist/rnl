//! The constraints of a level after expansion as the Document gives them (`BuildOptions::constraints`):
//! the facts a solver takes in, each object tied to the Document built with them, and what a report
//! of a solve says of each.

use std::path::Path;
use std::process::Command;

use rnl_cli::path_str;
use rnl_cli::preview::preview;
use rnl_cli::solve::SolveOptions;
use rnl_runtime::source::MemoryProvider;
use rnl_solver::{Clock, SolverOptions};
use rnl_view::{BuildOptions, ConstraintPoint, Document, LevelConstraints};
use serde_json::Value;

/// A pair, a structure over it, a ground that a rule of the standard package makes, a semantic
/// usage without a symbol, a handle, and one constraint turned off.
const NETLIST: &str = "* constraints
* @RNL(type=\"header\") { @VERSION(\"0.3\")}
M1 d g 0 0 nch
M2 d g 0 0 nch
R1 d g 1k
R2 d g 1k
* @RNL {
*   %c1 = {#M1 #M2} @ALIGN(\"x\");
*   {#R1 #R2} @ORDER(\"y\", priority=\"disabled\");
*   $WIRE_NET_SET @ROUTE;
* }
.model nch nmos level=1
.end
";

fn build(text: &str, name: &str) -> Document {
    let files = MemoryProvider::new().with(&format!("/w/{name}"), text);
    let options = BuildOptions { svg: false, constraints: true, ..BuildOptions::default() };
    rnl_view::build(&format!("/w/{name}"), &files, &options).unwrap()
}

fn constraints(d: &Document) -> &LevelConstraints {
    d.levels[0].constraints.as_ref().unwrap()
}

fn origins(points: &[ConstraintPoint]) -> Vec<usize> {
    points
        .iter()
        .map(|p| match p {
            ConstraintPoint::Origin { origin } => *origin,
            ConstraintPoint::Pin { object, .. } => *object,
            ConstraintPoint::Centre { .. } => panic!("a centre"),
        })
        .collect()
}

#[test]
fn only_when_asked_for() {
    let files = MemoryProvider::new().with("/w/a.sp", NETLIST);
    let d = rnl_view::build("/w/a.sp", &files, &BuildOptions::default()).unwrap();
    assert!(d.levels[0].constraints.is_none());
    assert!(serde_json::to_value(&d).unwrap()["levels"][0].get("constraints").is_none());
    // asked for, nothing else in the Document changes
    let with =
        rnl_view::build("/w/a.sp", &files, &BuildOptions { constraints: true, ..BuildOptions::default() }).unwrap();
    let mut with = serde_json::to_value(&with).unwrap();
    for level in with["levels"].as_array_mut().unwrap() {
        level.as_object_mut().unwrap().remove("constraints").unwrap();
    }
    assert_eq!(with, serde_json::to_value(&d).unwrap());
}

#[test]
fn a_fact_says_what_it_relates_and_its_objects_are_in_the_document() {
    let d = build(NETLIST, "a.sp");
    assert_eq!(d.diagnostics.iter().filter(|x| x.severity == "error").count(), 0);
    let c = constraints(&d);
    assert_eq!(c.input.len(), 16);
    let align = c.facts.iter().find(|f| f.label == "%c1").unwrap();
    assert_eq!(align.what, "ALIGN x {#M1 #M2}");
    assert_eq!(align.priority, "normal");
    assert_eq!(align.source.as_ref().map(|s| s.line), Some(8));
    let rnl_view::ConstraintKind::Align { axis, members } = &align.fact else { panic!("{:?}", align.fact) };
    assert_eq!(*axis, "x");
    let names: Vec<(&str, Option<&str>, Vec<&str>)> = origins(members)
        .into_iter()
        .map(|k| {
            let o = &c.objects[k];
            (o.kind, o.id.as_deref(), o.objects.iter().map(|&k| d.objects[k].name.as_str()).collect())
        })
        .collect();
    assert_eq!(names, [("instance", Some("#M1"), vec!["M1"]), ("instance", Some("#M2"), vec!["M2"])]);
    // what a hit stands for is what a member is, so that the drawing can be found from it
    assert!(d.objects[c.objects[origins(members)[0]].objects[0]].kind == "instance");
    // the ROUTE, with its pins, and nothing of the constraint turned off
    let route = c.routes.iter().find(|r| r.net == "d").unwrap();
    assert!(!route.pins.is_empty());
    assert!(c.facts.iter().all(|f| !f.what.starts_with("ORDER")));
}

#[test]
fn objects_a_rule_makes_are_in_the_document_too() {
    let d = build(NETLIST, "a.sp");
    let c = constraints(&d);
    // the standard package's rules make usages here; each is its row of the level's usages, whether
    // or not it has a symbol, and an object drawn as one is that row
    let made: Vec<_> = c.usages.iter().filter(|u| u.generated).collect();
    assert!(!made.is_empty());
    for u in &made {
        let object = u.object.unwrap_or_else(|| panic!("{} has no object", u.label));
        assert!(d.levels[0].usages.iter().any(|row| row.object == object), "{}", u.label);
    }
    for o in c.objects.iter().filter(|o| o.generated && o.kind == "usage") {
        assert_eq!(o.objects.len(), 1, "{}", o.label);
        assert!(d.levels[0].usages.iter().any(|row| row.object == o.objects[0]), "{}", o.label);
    }
    // the chain of a fact refers to these
    for f in &c.facts {
        assert!(f.chain.iter().all(|&u| u < c.usages.len()));
    }
}

fn rnl(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.code().unwrap_or(-1), text(&out.stdout), text(&out.stderr))
}

#[test]
fn rnl_problem_prints_the_same_facts_without_what() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-constraints");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.sp"), NETLIST).unwrap();
    let file = path_str(&dir.join("a.sp"));
    let (code, out, err) = rnl(&["view", &file, "--no-svg", "--constraints"]);
    assert_eq!((code, err.as_str()), (0, ""));
    let view: Value = serde_json::from_str(&out).unwrap();
    let (code, out, _) = rnl(&["problem", &file]);
    assert_eq!(code, 0);
    let problem: Value = serde_json::from_str(&out).unwrap();
    let mut facts = view["levels"][0]["constraints"]["facts"].as_array().unwrap().clone();
    assert!(!facts.is_empty());
    for f in &mut facts {
        assert!(f.as_object_mut().unwrap().remove("what").unwrap().is_string());
    }
    let level = problem.as_array().map_or(&problem["levels"][0], |levels| &levels[0]);
    assert_eq!(Value::Array(facts), level["facts"]);
    let (code, _, err) = rnl(&["check", &file, "--constraints"]);
    assert_eq!((code, err.trim()), (2, "--constraints is an option of view; rnl problem prints what a solver reads"));
}

struct NoTime;

impl Clock for NoTime {
    fn elapsed(&self) -> f64 {
        0.0
    }
}

#[test]
fn the_report_of_a_solve_tells_of_each_fact_and_the_document_with_its_record_keeps_the_objects() {
    let files = MemoryProvider::new().with("/w/a.sp", NETLIST);
    let options = SolveOptions {
        dialect: None,
        formats: Default::default(),
        std: true,
        level: None,
        solver: SolverOptions::default(),
    };
    let shown = preview("/w/a.sp", &files, &options, &NoTime, &|_| true).unwrap();
    let report = &shown.report.levels[0];
    let level = shown.document.levels.iter().find(|l| l.host == report.host).unwrap();
    let c = level.constraints.as_ref().unwrap();
    // the same input: rank by rank, the report's facts are these
    assert_eq!(report.input, c.input);
    assert_eq!(report.facts.len(), c.facts.len());
    for (line, fact) in report.facts.iter().zip(&c.facts) {
        assert!(line.text.contains(&fact.what), "{} is not {}", line.text, fact.what);
    }
    // the document is drawn with the record of the solver, and read without it for the facts: what a
    // rule makes, written out in the record, is still found
    assert!(shown.document.files[0].text.contains("type=\"generated\""));
    for o in c.objects.iter().filter(|o| matches!(o.kind, "instance" | "usage")) {
        assert!(o.id.is_some() && !o.objects.is_empty(), "{} is not found", o.label);
    }
    for u in c.usages.iter() {
        assert!(u.object.is_some(), "{} is not found", u.label);
    }
}
