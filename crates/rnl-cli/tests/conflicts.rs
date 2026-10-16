//! The hints of a conflict of geometric facts (the draft 0.3, 8.5), in a file that declares 0.3
//! read with `Options::draft`. A fact discarded in a conflict gives a hint that names every set of
//! facts taken in that has no solution together with it, each fact by the statement that wrote
//! it, and each statement in these sets gets a hint that names the facts discarded because of it.
//! The toolkit does not solve: of the facts of one object on one axis, or of its orientation, it
//! keeps the one first in the order of 8.5 and takes in those that agree with it, so the sets of a
//! discarded fact are the kept facts that contradict it, one fact each. A constraint discarded
//! against fixed coordinates names the smallest sets of them that leave it no solution. A file of
//! 0.2 gives the hints it always gave, with the switch or without.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::geom::Orient;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn draft() -> Options {
    Options { std: false, draft: true, ..Options::default() }
}

/// A file of `version` with `statements` in one record, one on each line, the statement `k` on
/// line `line(k)` at column 5, and after it R1, R2 and so on in a row, up to the last that the
/// statements place.
fn netlist(version: &str, statements: &[&str]) -> String {
    let mut text = format!("* conflicts\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n* @RNL {{\n");
    for (k, statement) in statements.iter().enumerate() {
        let end = if k + 1 < statements.len() { ";" } else { "" };
        text += &format!("*   {statement}{end}\n");
    }
    text += "* }\n";
    let placed = (1..=9).filter(|k| statements.iter().any(|s| s.contains(&format!("#R{k}"))));
    for k in 1..=placed.max().unwrap_or(0) {
        text += &format!("R{k} n{k} n{} 1k\n", k + 1);
    }
    text + ".end\n"
}

fn line(k: usize) -> u32 {
    4 + k as u32
}

fn read(text: &str, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, &MemoryProvider::new().with("t.sp", text), options).unwrap()
}

/// The diagnostics, each by its line, column and message, in the order given; none but hints.
fn hints(r: &Interpretation) -> Vec<(u32, u32, String)> {
    let shown = r.diagnostics();
    let other: Vec<&String> = shown.iter().filter(|d| d.severity != Severity::Hint).map(|d| &d.message).collect();
    assert!(other.is_empty(), "{other:?}");
    shown.iter().map(|d| (d.loc.as_ref().unwrap().line, d.loc.as_ref().unwrap().col, d.message.clone())).collect()
}

/// `what`, written by the statement `k` at column `col`.
fn at(what: &str, k: usize, col: u32) -> String {
    format!("{what} at t.sp:{}:{col}", line(k))
}

/// The hint of the fact `what`, discarded: each set by its facts.
fn discarded(what: &str, sets: &[&[String]]) -> String {
    let sets: Vec<String> = sets
        .iter()
        .map(|set| match set {
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last} together", rest.join(", ")),
            [] => panic!("an empty set"),
        })
        .collect();
    format!("{what} is discarded in the order of 8.5: it has no solution with {} (8.5)", sets.join(", nor with "))
}

/// The hint of a statement whose facts are in the sets of the facts `discarded`.
fn note(discarded: &[String]) -> String {
    format!("facts discarded in the order of 8.5 because of this statement: {} (8.5)", discarded.join("; "))
}

/// Where the model draws an instance, and how it is turned.
fn drawn(r: &Interpretation, id: &str) -> ((f64, f64), Orient) {
    let item = r.model.levels[0].items.iter().find(|i| i.id == id).unwrap();
    (item.at, item.orient)
}

#[test]
fn two_place_facts_on_one_axis() {
    let r = read(&netlist("0.3", &["#R1 @PLACE(0, 0)", "#R1 @PLACE(x=10)"]), &draft());
    let x0 = at("the x of #R1 = 0", 0, 5);
    let x10 = at("the x of #R1 = 10", 1, 5);
    assert_eq!(hints(&r), [(line(0), 5, discarded("the x of #R1 = 0", &[&[x10]])), (line(1), 5, note(&[x0]))]);
    assert_eq!(drawn(&r, "#R1").0, (10.0, 0.0));
}

#[test]
fn three_place_facts_on_one_axis() {
    // two facts discarded, each conflicting with the one kept, which is told of both
    let r = read(&netlist("0.3", &["#R1 @PLACE(0, 0)", "#R1 @PLACE(x=5)", "#R1 @PLACE(x=10)"]), &draft());
    let x10 = at("the x of #R1 = 10", 2, 5);
    assert_eq!(
        hints(&r),
        [
            (line(0), 5, discarded("the x of #R1 = 0", &[&[x10.clone()]])),
            (line(1), 5, discarded("the x of #R1 = 5", &[&[x10]])),
            (line(2), 5, note(&[at("the x of #R1 = 0", 0, 5), at("the x of #R1 = 5", 1, 5)])),
        ]
    );
    assert_eq!(drawn(&r, "#R1").0, (10.0, 0.0));

    // two facts kept that agree: the one discarded names both, the later first, and both
    // statements are told
    let r = read(&netlist("0.3", &["#R1 @PLACE(0, 0)", "#R1 @PLACE(x=10)", "#R1 @PLACE(x=10)"]), &draft());
    let x0 = at("the x of #R1 = 0", 0, 5);
    assert_eq!(
        hints(&r),
        [
            (
                line(0),
                5,
                discarded("the x of #R1 = 0", &[&[at("the x of #R1 = 10", 2, 5)], &[at("the x of #R1 = 10", 1, 5)]])
            ),
            (line(1), 5, note(&[x0.clone()])),
            (line(2), 5, note(&[x0])),
        ]
    );

    // a fact that agrees with the one kept is taken in with it, though a fact between them does
    // not: only that one is discarded, and it conflicts with both
    let r = read(&netlist("0.3", &["#R1 @PLACE(0, 0)", "#R1 @PLACE(y=5)", "#R1 @PLACE(y=0)"]), &draft());
    let y5 = at("the y of #R1 = 5", 1, 5);
    assert_eq!(
        hints(&r),
        [
            (
                line(1),
                5,
                discarded("the y of #R1 = 5", &[&[at("the y of #R1 = 0", 2, 5)], &[at("the y of #R1 = 0", 0, 5)]])
            ),
            (line(0), 5, note(&[y5.clone()])),
            (line(2), 5, note(&[y5])),
        ]
    );
    assert_eq!(drawn(&r, "#R1").0, (0.0, 0.0));
}

#[test]
fn an_orient_conflict() {
    // the ORIENT of a later segment of the pipe, at column 25
    let r = read(&netlist("0.3", &["#R1 @PLACE(0, 0) => @ORIENT(\"R90\")", "#R1 @ORIENT(\"MY\")"]), &draft());
    assert_eq!(
        hints(&r),
        [
            (line(0), 25, discarded("the orientation R90 of #R1", &[&[at("the orientation MY of #R1", 1, 5)]])),
            (line(1), 5, note(&[at("the orientation R90 of #R1", 0, 25)])),
        ]
    );
    assert_eq!(drawn(&r, "#R1").1, Orient::MY);
}

#[test]
fn priorities_decide_and_a_low_fact_gives_no_hint() {
    // the high PLACE is kept against a later one; the low x is discarded without a hint, and it is
    // not named to the statement kept
    let r = read(
        &netlist(
            "0.3",
            &["#R1 @PLACE(0, 0, priority=\"high\")", "#R1 @PLACE(5, 5)", "#R1 @PLACE(x=7, priority=\"low\")"],
        ),
        &draft(),
    );
    let (x5, y5) = (at("the x of #R1 = 5", 1, 5), at("the y of #R1 = 5", 1, 5));
    assert_eq!(
        hints(&r),
        [
            (line(1), 5, discarded("the x of #R1 = 5", &[&[at("the x of #R1 = 0", 0, 5)]])),
            (line(1), 5, discarded("the y of #R1 = 5", &[&[at("the y of #R1 = 0", 0, 5)]])),
            (line(0), 5, note(&[x5, y5])),
        ]
    );
    assert_eq!(drawn(&r, "#R1").0, (0.0, 0.0));
}

#[test]
fn a_fact_of_an_expansion_is_named_with_its_origin() {
    // the PLACE of the body, at column 34, takes the place of the AS in the order of 8.5 (9.4)
    let r = read(
        &netlist("0.3", &["$AT = @DEFINE(roles=[\"a\"]) { ?a @PLACE(x=0) }", "#R1 @AS $AT", "#R1 @PLACE(10, 0)"]),
        &draft(),
    );
    let applying = format!("applying $AT (t.sp:{}:5)", line(1));
    assert_eq!(
        hints(&r),
        [
            (line(0), 34, discarded("the x of #R1 = 0", &[&[at("the x of #R1 = 10", 2, 5)]])),
            (line(2), 5, note(&[format!("{} [{applying}]", at("the x of #R1 = 0", 0, 34))])),
        ]
    );
    assert_eq!(r.diagnostics()[0].origin, applying);
}

#[test]
fn a_constraint_names_the_smallest_sets_it_conflicts_with() {
    // the ALIGN comes after the coordinates in the order of 8.5; R1 and R2 agree, so it has no
    // solution with R3 and either of them
    let r = read(
        &netlist(
            "0.3",
            &["{#R1 #R2 #R3} @ALIGN(\"y\")", "#R1 @PLACE(0, 0)", "#R2 @PLACE(10, 0)", "#R3 @PLACE(20, 10)"],
        ),
        &draft(),
    );
    let (y1, y2, y3) = (at("the y of #R1 = 0", 1, 5), at("the y of #R2 = 0", 2, 5), at("the y of #R3 = 10", 3, 5));
    let align = at("ALIGN", 0, 5);
    assert_eq!(
        hints(&r),
        [
            (line(0), 5, discarded("ALIGN", &[&[y1, y3.clone()], &[y2, y3]])),
            (line(1), 5, note(&[align.clone()])),
            (line(2), 5, note(&[align.clone()])),
            (line(3), 5, note(&[align])),
        ]
    );
    assert_eq!(r.model.levels[0].constraints[0].status, "dropped");
}

/// The conflicts of the tests above in one file of 0.2, the ALIGN first.
const CONFLICTS: &[&str] = &[
    "{#R1 #R2 #R3} @ALIGN(\"y\")",
    "#R1 @PLACE(0, 20)",
    "#R1 @PLACE(x=10)",
    "#R2 @PLACE(0, 30)",
    "#R2 @PLACE(x=5)",
    "#R2 @PLACE(x=10)",
    "#R3 @PLACE(0, 40)",
    "#R3 @PLACE(x=10)",
    "#R3 @PLACE(x=10)",
    "#R4 @PLACE(20, 0) => @ORIENT(\"R90\")",
    "#R4 @ORIENT(\"MY\")",
    "#R5 @PLACE(30, 0)",
    "#R5 @PLACE(y=5)",
    "#R5 @PLACE(y=0)",
];

#[test]
fn a_file_of_0_2_gives_the_hints_it_always_gave() {
    let dropped = |what: &str, by: &str| format!("{what} conflicts with {by} and is dropped in the order of 8.5 (8.5)");
    let expected = [
        (line(1), 5, dropped("the x of #R1 = 0", "the x of #R1 = 10")),
        (line(3), 5, dropped("the x of #R2 = 0", "the x of #R2 = 5")),
        (line(4), 5, dropped("the x of #R2 = 5", "the x of #R2 = 10")),
        (line(6), 5, dropped("the x of #R3 = 0", "the x of #R3 = 10")),
        (line(11), 5, dropped("the y of #R5 = 0", "the y of #R5 = 5")),
        (line(12), 5, dropped("the y of #R5 = 5", "the y of #R5 = 0")),
        (line(9), 26, dropped("the orientation R90 of #R4", "the orientation MY of #R4")),
        (line(0), 5, "ALIGN conflicts with fixed coordinates and is dropped in the order of 8.5 (8.5)".to_string()),
    ];
    let text = netlist("0.2", CONFLICTS);
    for options in [Options { std: false, ..Options::default() }, draft()] {
        assert_eq!(hints(&read(&text, &options)), expected);
    }
    // the same file of the draft gives the hints of 0.3, one for each fact discarded and one for
    // each statement told
    let r = read(&netlist("0.3", CONFLICTS), &draft());
    let found = hints(&r);
    assert_eq!(found.iter().filter(|h| h.2.contains(" is discarded in the order of 8.5: ")).count(), 7);
    assert_eq!(found.iter().filter(|h| h.2.starts_with("facts discarded in the order of 8.5 ")).count(), 10);
}
