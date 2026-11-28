//! A level drawn wherever its absolute coordinates put it: with one absolute coordinate, the record
//! written for any value of it is the same drawing, moved, and so it is when every absolute
//! coordinate moves by steps of the grid.
use std::collections::BTreeMap;
use std::path::Path;

use rnl_cli::solve::{solve, SolveOptions};
use rnl_runtime::source::MemoryProvider;
use rnl_solver::SolverOptions;

/// A level of two resistors and a capacitor in a row and a title that no fact places, `extra` at
/// the end of its record.
fn level(extra: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"); @VERSION(\"0.4\") }}\n\
         R1 a b 1k\nR2 b c 1k\nC1 c 0 1p\n\
         * @RNL {{\n*   {{#R1 #R2 #C1}} @ORDER(\"left-to-right\");\n*   (_, _) @TEXT(\"a title\") => %title{extra}\n* }}\n.end\n"
    )
}

/// The places of the record of the solver written for the level, by what it places.
fn places(extra: &str) -> BTreeMap<String, (f64, f64)> {
    let files = MemoryProvider::new().with("t.sp", &level(extra));
    let solver = SolverOptions { safety_seconds: None, ..SolverOptions::without_time_limit() };
    let run = solve(Path::new("t.sp"), &files, &SolveOptions { solver, ..SolveOptions::default() }).unwrap();
    assert_eq!(run.errors, 0);
    let text = &run.files[0].text;
    let record = &text[text.rfind("type=\"generated\"").expect("a record of the solver")..];
    let mut out = BTreeMap::new();
    for line in record.lines() {
        let Some((what, rest)) = line.trim_start_matches(['*', ' ']).split_once(" @PLACE(") else { continue };
        let Some((xy, _)) = rest.split_once(')') else { continue };
        let Some((x, y)) = xy.split_once(", ") else { continue };
        out.insert(what.to_string(), (x.parse().unwrap(), y.parse().unwrap()));
    }
    assert!(out.contains_key("#R1"), "{record}");
    out
}

/// The places moved so that `#R1` is at the origin.
fn relative(places: &BTreeMap<String, (f64, f64)>) -> BTreeMap<String, (f64, f64)> {
    let (x0, y0) = places["#R1"];
    places.iter().map(|(k, (x, y))| (k.clone(), (x - x0, y - y0))).collect()
}

#[test]
fn the_record_for_any_value_of_the_one_absolute_coordinate_is_the_same_drawing_moved() {
    let free = places("");
    for (x, y) in [(0.0, 0.0), (24.0, 0.0), (-11.0, 3.0)] {
        let placed = places(&format!(";\n*   #R1 @PLACE({x}, {y})"));
        assert_eq!(placed["#R1"], (x, y));
        assert_eq!(relative(&placed), relative(&free), "R1 at ({x}, {y})");
    }
}

#[test]
fn the_record_when_every_absolute_coordinate_moves_by_steps_of_the_grid_is_the_same_drawing_moved() {
    let first = places(";\n*   #R1 @PLACE(3, 1);\n*   #C1 @PLACE(17, 2)");
    let other = places(";\n*   #R1 @PLACE(10, -2);\n*   #C1 @PLACE(24, -1)");
    let moved: BTreeMap<String, (f64, f64)> = first.iter().map(|(k, (x, y))| (k.clone(), (x + 7.0, y - 3.0))).collect();
    assert_eq!(other, moved);
}
