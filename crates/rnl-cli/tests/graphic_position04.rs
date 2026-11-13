//! PLACE on the `::position` of a RECTANGLE, CIRCLE or ELLIPSE written with a centre (11.1, 13.4):
//! in a file of 0.4 it moves the centre, as for any point; PLACE on the graphic itself fails. A file
//! of 0.3 reads it as it did.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn file(name: &str, version: &str, body: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("graphic-position04");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let text =
        format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\") }}\nR1 a b 1k\n* @RNL {{ {body} }}\n.end\n");
    std::fs::write(&path, text).unwrap();
    path
}

fn rnl(args: &[&str], path: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).arg("--no-std").arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn errors(path: &Path) -> Vec<String> {
    rnl(&["check"], path).lines().filter(|l| l.starts_with("error")).map(String::from).collect()
}

/// The points of the graphics drawn in the main level.
fn shapes(path: &Path) -> Vec<(String, Vec<[f64; 2]>)> {
    let dump: Value = serde_json::from_str(&rnl(&["dump"], path)).unwrap();
    dump["levels"][0]["shapes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            let points =
                s["points"].as_array().unwrap().iter().map(|p| [p[0].as_f64().unwrap(), p[1].as_f64().unwrap()]);
            (s["shape"].as_str().unwrap().to_string(), points.collect())
        })
        .collect()
}

#[test]
fn place_moves_the_centre_of_a_graphic_written_with_one() {
    for (name, graphic) in [
        ("rectangle", "@RECTANGLE(width=2, height=1)"),
        ("circle", "@CIRCLE(radius=1)"),
        ("ellipse", "@ELLIPSE(width=2, height=1)"),
    ] {
        for centre in ["(0, 0)", "(_, _)"] {
            let body = format!("%g = {graphic} {{{centre}}}; %g::position @PLACE(3, 3)");
            let path = file(&format!("{name}.sp"), "0.4", &body);
            assert_eq!(errors(&path), Vec::<String>::new(), "{body}");
            let drawn = shapes(&path);
            assert_eq!(drawn.len(), 1, "{body}: {drawn:?}");
            assert_eq!(drawn[0].1, vec![[3.0, 3.0]], "{body}");
        }
    }
}

#[test]
fn place_on_the_graphic_itself_fails() {
    let path = file("itself.sp", "0.4", "%c = @CIRCLE(radius=1) {(0, 0)}; %c @PLACE(4, 4)");
    let e = errors(&path);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("RNL4300 place-not-taken: PLACE does not take CIRCLE"), "{e:?}");
}

#[test]
fn a_file_of_0_3_reads_it_as_it_did() {
    // accepted, and the centre drawn where it is written
    let path = file("v03.sp", "0.3", "%c = @CIRCLE(radius=1) {(0, 0)}; %c::position @PLACE(3, 3)");
    assert_eq!(errors(&path), Vec::<String>::new());
    assert_eq!(shapes(&path), vec![("CIRCLE".to_string(), vec![[0.0, 0.0]])]);
}
