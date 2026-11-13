//! Rules and bundles (10.1, 11.8, 8.2): in a file of 0.4 the bundles of step 3 are known when the
//! symbol queue evaluates the selections of its rules, so a rule's selection reaches the bundle by
//! the conditions of its first member, and never a member: members are not drawn on their own. A
//! file of 0.3 resolves bundles after the queue's rules, as it did.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// A netlist of `version` with the two halves of the bus instance XI0, the subcircuit `cell`, and
/// `body` as its last record; `head` is added to its header record.
fn netlist(version: &str, head: &str, body: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){head} }}\nXI0<1> a b cell\nXI0<0> a b cell\nR1 a b 1k\n\
         .subckt cell p q\nR1 p q 1k\n.ends\n* @RNL {{\n{body}\n* }}\n.end\n"
    )
}

const SYMBOL: &str = "*   $BS = @DEFINE-SYMBOL { (0,1) @PIN(terminal=\"p\", direction=\"up\"); (0,-1) @PIN(terminal=\"q\", direction=\"down\"); (0,0) @CIRCLE(radius=1) };";

fn file(name: &str, text: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bundle-rules04");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn rnl(args: &[&str], path: &Path) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).arg(path).output().unwrap();
    (String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// The lines of `rnl check` that are errors.
fn errors(path: &Path, std: bool) -> Vec<String> {
    let args: &[&str] = if std { &["check"] } else { &["check", "--no-std"] };
    rnl(args, path).0.lines().filter(|l| l.starts_with("error")).map(String::from).collect()
}

/// The name of the symbol the item `id` of the main level is drawn with.
fn symbol_of(path: &Path, std: bool, id: &str) -> String {
    let args: &[&str] = if std { &["dump"] } else { &["dump", "--no-std"] };
    let dump: Value = serde_json::from_str(&rnl(args, path).0).unwrap();
    let level = &dump["levels"][0];
    let item = level["items"].as_array().unwrap().iter().find(|i| i["id"] == id).unwrap_or_else(|| panic!("{id}"));
    level["symbols"][item["symbol"].as_u64().unwrap() as usize]["name"].as_str().unwrap().to_string()
}

#[test]
fn a_rule_gives_the_bundle_its_symbol_in_0_4() {
    for (name, selection) in [
        ("by-name", "# @SELECT-INSTANCE(name=\"XI0*\") => @SET-SYMBOL $BS"),
        ("by-model", "# @SELECT-INSTANCE(model=\"cell\") => @SET-SYMBOL $BS"),
    ] {
        let body = format!("{SYMBOL}\n*   %b = #XI0<1:0> @BUNDLE;\n*   {selection};\n*   %b @PLACE(0, 0)");
        let path = file(&format!("{name}.sp"), &netlist("0.4", "", &body));
        assert_eq!(errors(&path, false), Vec::<String>::new(), "{name}");
        assert_eq!(symbol_of(&path, false, "%b"), "$BS", "{name}");
    }
}

#[test]
fn a_file_of_0_3_resolves_bundles_after_the_rules_as_it_did() {
    let body = format!(
        "{SYMBOL}\n*   %b = #XI0<1:0> @BUNDLE;\n*   # @SELECT-INSTANCE(name=\"XI0*\") => @SET-SYMBOL $BS;\n*   %b @PLACE(0, 0)"
    );
    let path = file("v03.sp", &netlist("0.3", "", &body));
    let e = errors(&path, false);
    assert_eq!(e.len(), 2, "{e:?}");
    assert!(e.iter().all(|l| l.contains("RNL4601 bundle-member-drawn-alone")), "{e:?}");
}

#[test]
fn the_standard_package_draws_a_bundle_of_0_4() {
    // the standard rules give the bundle the symbol of its first member, and not its members
    let head = "; @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\")";
    let text = format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"){head} }}\nRA<1> a b 1k\nRA<0> a b 1k\n\
         * @RNL {{\n*   %b = #RA<1:0> @BUNDLE;\n*   %b @PLACE(0, 0)\n* }}\n.end\n"
    );
    let path = file("std.sp", &text);
    assert_eq!(errors(&path, true), Vec::<String>::new());
    assert_eq!(symbol_of(&path, true, "%b"), "$RESISTOR");
}
