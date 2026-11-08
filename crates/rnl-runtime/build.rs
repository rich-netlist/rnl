//! The kinds of diagnostics (`diag::codes`), one constant for each entry of
//! `data/diagnostics.json`, named by its short name: `param-wrong-type` is `PARAM_WRONG_TYPE`. A
//! kind is added to the list, and so to the code, by adding its entry.

use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let list = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("data/diagnostics.json");
    println!("cargo:rerun-if-changed={}", list.display());
    let text = std::fs::read_to_string(&list).expect("data/diagnostics.json is read");
    let json: serde_json::Value = serde_json::from_str(&text).expect("data/diagnostics.json is JSON");
    let mut out = String::new();
    let mut all = Vec::new();
    for kind in json["kinds"].as_array().expect("a list of kinds") {
        let name = kind["name"].as_str().expect("a kind has a name");
        let severity = kind["severity"].as_str().expect("a kind has a severity");
        let sections: Vec<&str> =
            kind["sections"].as_array().map_or(Vec::new(), |s| s.iter().filter_map(|v| v.as_str()).collect());
        let constant = name.to_uppercase().replace('-', "_");
        let about = if sections.is_empty() { String::new() } else { format!(" ({})", sections.join(", ")) };
        match kind["number"].as_str() {
            Some(number) => {
                let n: u16 =
                    number.strip_prefix("RNL").and_then(|n| n.parse().ok()).expect("a number is RNL and four digits");
                writeln!(out, "/// {number}, {severity}{about}.").unwrap();
                writeln!(out, "pub const {constant}: Code = Code::new({n}, {name:?});").unwrap();
            }
            None => {
                writeln!(out, "/// Without a number: a kind of the versions 0.2 and 0.3 alone; {severity}{about}.")
                    .unwrap();
                writeln!(out, "pub const {constant}: Code = Code::unnumbered({name:?});").unwrap();
            }
        }
        all.push(constant);
    }
    writeln!(out, "/// Every kind, in the order of the list.").unwrap();
    writeln!(out, "pub const ALL: &[Code] = &[{}];", all.join(", ")).unwrap();
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("codes.rs"), out).unwrap();
}
