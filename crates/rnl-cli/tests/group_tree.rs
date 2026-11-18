//! `rnl groups`: the tree of groups (11.5) of each level, with the parts each group holds directly.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

/// A level of `version` with a group in a group, a group on the members of another, one that
/// crosses another, one turned off and one of a net alone, and a ground as a virtual part. The
/// groups are written so that a later one comes first in the tree (11.5): %again before %pair,
/// %block before %cross.
fn netlist(version: &str) -> String {
    format!(
        r#"* groups
* @RNL(type="header") {{ @VERSION("{version}"); @USE-PACKAGE(name="rnl-std-basic", version="{version}") }}
R1 a 0 1k
R2 a b 1k
R3 b 0 1k
R4 b c 1k
R5 c 0 1k
R6 c 0 1k
* @RNL {{
*   %g = $GROUND @ATTACH &0;
*   %cross = {{#R3 #R4}} @GROUP;
*   %block = {{#R1 #R2 #R3 %g}} @GROUP;
*   %pair = {{#R1 #R2}} @GROUP;
*   %again = {{#R1 #R2}} @GROUP;
*   %off = {{#R5 #R6}} @GROUP;
*   %off @PRIORITY("disabled");
*   %e = {{&a}} @GROUP
* }}
.end
"#
    )
}

/// `text` written as `name` in a directory of its own, so that tests running at once do not read a
/// file another is writing: the directory.
fn written(name: &str, text: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("rnl-group-tree-{n}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), text).unwrap();
    dir
}

/// `rnl groups` run in `dir` with `args`: its exit code, what it prints and what it says.
fn groups(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).current_dir(dir).arg("groups").args(args).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.code().unwrap_or(-1), text(&out.stdout), text(&out.stderr))
}

/// Each group of the tree stands under the smallest that holds it, with the devices it holds
/// directly and how many virtual parts, its number of devices at the end and where it is written;
/// the groups outside the tree follow, each with what it is: the members of another, one that
/// crosses another with what they share, one turned off, one with no member with a place of its
/// own.
#[test]
fn a_level_of_0_4_prints_its_tree_of_groups() {
    let dir = written("g.sp", &netlist("0.4"));
    let (code, out, err) = groups(&dir, &["g.sp"]);
    assert_eq!((code, err.as_str()), (0, ""));
    let want = "g.sp · language 0.4 · 2 groups, 4 outside the tree
  (level) · #R4 #R5 #R6 · 3 devices
    %block · #R3 + 1 virtual · 1 device (written at g.sp:12)
      %again · #R1 #R2 · 2 devices (written at g.sp:14)
  outside the tree:
    %cross · CROSSES %block: they share #R3 and neither holds the other · #R3 #R4 · 2 devices (written at g.sp:11)
    %pair · the members of %again, which stands for it · #R1 #R2 · 2 devices (written at g.sp:13)
    %off · turned off by PRIORITY · #R5 #R6 · 2 devices (written at g.sp:15)
    %e · no member with a place of its own · also &a · 0 devices (written at g.sp:17)
";
    assert_eq!(out, want);
}

/// `--json` gives the same tree, nested, with the groups outside it and every virtual part;
/// `--level` keeps one level, and a title no level has is an error that names the titles.
#[test]
fn json_and_level() {
    let dir = written("g.sp", &netlist("0.4"));
    let (code, out, _) = groups(&dir, &["g.sp", "--level", "g.sp", "--json"]);
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(&out).unwrap();
    let level = &v["levels"][0];
    assert_eq!(v["levels"].as_array().unwrap().len(), 1);
    assert_eq!((level["language"].as_str(), level["tree"].as_bool()), (Some("0.4"), Some(true)));
    assert_eq!(level["root"]["devices"], serde_json::json!(["#R4", "#R5", "#R6"]));
    let block = &level["root"]["groups"][0];
    assert_eq!((block["id"].as_str(), block["name"].as_str()), (Some("%block"), Some("%block")));
    assert_eq!(block["devices"], serde_json::json!(["#R3"]));
    assert_eq!(block["virtual"], serde_json::json!([{"id": "%g", "label": "%g"}]));
    assert_eq!(block["source"], serde_json::json!({"kind": "written", "at": "g.sp:12"}));
    assert_eq!(block["groups"][0]["id"], "%again");
    assert_eq!(block["groups"][0]["devices"], serde_json::json!(["#R1", "#R2"]));
    let outside: Vec<(&str, &str)> = level["outside"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| (g["id"].as_str().unwrap(), g["state"].as_str().unwrap()))
        .collect();
    assert_eq!(outside, [("%cross", "crosses"), ("%pair", "same"), ("%off", "disabled"), ("%e", "empty")]);
    assert_eq!(level["outside"][0]["with"], "%block");

    let (code, _, err) = groups(&dir, &["g.sp", "--level", "amp"]);
    assert_eq!(code, 2);
    assert_eq!(
        err,
        "error: RNL8005 cli-explain-level-not-found: no level is titled amp; the levels are g.sp
"
    );
}

/// A group a definition's body makes is named by the usage that applies the definition and the
/// definition, with `#1`, `#2` when that usage makes several, and ends with where its GROUP is.
#[test]
fn a_group_of_a_definition_is_named_by_its_usage() {
    let text = r#"* groups
* @RNL(type="header") { @VERSION("0.4") }
R1 a 0 1k
R2 a b 1k
R3 b 0 1k
R4 b 0 1k
* @RNL {
*   $BLOCK = @DEFINE(roles=["{parts}"]) { ?parts @GROUP };
*   $TWO = @DEFINE(roles=["a", "b"]) { ?a @GROUP; ?b @GROUP };
*   %b = {#R1 #R2} @AS $BLOCK;
*   %t = #R3(a) #R4(b) @AS $TWO
* }
.end
"#;
    let dir = written("d.sp", text);
    let (code, out, _) = groups(&dir, &["d.sp"]);
    assert_eq!(code, 0);
    assert!(out.contains("\n    %b · $BLOCK · #R1 #R2 · 2 devices (applied at d.sp:10)\n"), "{out}");
    assert!(out.contains("\n    %t · $TWO #1 · #R3 · 1 device ("), "{out}");
    assert!(out.contains("\n    %t · $TWO #2 · #R4 · 1 device ("), "{out}");
}

/// A level of 21 resistors in no group, and a group a definition makes of 21 of 22: a node of the
/// tree that holds more than 20 devices directly has the hint, on the level at its first line, on
/// the group at the statement of the usage whose definition makes it. Twenty do not.
#[test]
fn check_hints_at_a_node_of_more_than_20_devices() {
    let resistors = |n: usize| (1..=n).map(|k| format!("R{k} n{k} 0 1k\n")).collect::<String>();
    let head = "* crowd\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n";
    let check = |name: &str, text: &str| {
        let dir = written(name, text);
        let out = Command::new(env!("CARGO_BIN_EXE_rnl")).current_dir(&dir).args(["check", name]).output().unwrap();
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains("RNL9406"))
            .map(String::from)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        check("flat.sp", &format!("{head}{}.end\n", resistors(21))),
        ["hint flat.sp:1: RNL9406 crowded-node: flat.sp: 21 devices directly in this level, in no group under it; \
          consider grouping its functional blocks with GROUP (more than 20 devices in one node of the tree of groups, 11.5)"]
    );
    assert_eq!(check("twenty.sp", &format!("{head}{}.end\n", resistors(20))), Vec::<String>::new());
    let members = (1..=21).map(|k| format!("#R{k}")).collect::<Vec<_>>().join(" ");
    let grouped = format!(
        "{head}{}* @RNL {{\n*   $BLOCK = @DEFINE(roles=[\"{{parts}}\"]) {{ ?parts @GROUP }};\n*   %b = {{{members}}} @AS $BLOCK\n* }}\n.end\n",
        resistors(22)
    );
    let hints = check("grouped.sp", &grouped);
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert!(
        hints[0].starts_with(
            "hint grouped.sp:27:5: RNL9406 crowded-node: grouped.sp: 21 devices directly in the group %b · $BLOCK,"
        ),
        "{hints:?}"
    );
}
