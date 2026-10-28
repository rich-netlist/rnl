//! The usages an object is a member of: the Document's reverse index (`Object::usages`), the
//! snapshot's (`ObjectView::usages`, `HostNet::usages`), and `rnl explain`.

use std::path::Path;
use std::process::Command;

use rnl_runtime::source::MemoryProvider;
use rnl_view::{BuildOptions, Document, MemberOf, Parts, RoleOf, RuleAt};

/// A netlist with a usage of each kind: written, nested two deep, of a set role, disabled, failed,
/// of a terminal, of a net, made by a rule, and one whose PLACE loses a conflict.
const NETLIST: &str = r#"* explain
.model nch nmos level=1
R1 a b 1k
R2 b c 1k
R3 c 0 1k
M1 d g 0 0 nch
* @RNL(type="header") { @VERSION("0.3")}
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $TWO = @DEFINE(roles=[["first", "usage"], "second"]) {};
*   $SET = @DEFINE(roles=["{members}"]) {};
*   $PIN = @DEFINE(roles=[["pin", "terminal"]]) {};
*   $NET = @DEFINE(roles=[["signal", "net"]]) {};
*   $LONE = @DEFINE(roles=["part"]) {};
*   %pair1 = #R1(left) #R2(right) @AS $PAIR;
*   %outer = %pair1(first) #R3(second) @AS $TWO;
*   %top = %outer(first) #R1(second) @AS $TWO;
*   %all = {#R1 #R3} @AS $SET;
*   %off = #R2(left) #R3(right) @AS(priority="disabled") $PAIR;
*   %bad = #R1(left) #R2(right) #R3(extra) @AS $PAIR;
*   %drain = #M1:d @AS $PIN;
*   %sig = &b @AS $NET;
*   # @SELECT-INSTANCE(name="R3") => @AS $LONE;
*   %gnd = $GROUND @ATTACH #R3:2;
*   %port = $IO_PORT @ATTACH &c => @PLACE(0, 0);
*   %port @PLACE(5, 5, priority="high")
* }
.end
"#;

fn document() -> Document {
    let files = MemoryProvider::new().with("/w/explain.sp", NETLIST);
    rnl_view::build("/w/explain.sp", &files, &BuildOptions { svg: false, ..BuildOptions::default() }).unwrap()
}

fn object<'d>(d: &'d Document, kind: &str, name: &str) -> &'d rnl_view::Object {
    d.objects.iter().find(|o| o.kind == kind && o.name == name).unwrap()
}

/// Each entry of an object: its usage's id, its roles and its state.
fn said(o: &rnl_view::Object) -> Vec<(&str, Vec<&str>, &str)> {
    o.usages.iter().map(|e| (e.id.as_str(), e.roles.iter().map(|r| r.role.as_str()).collect(), e.state)).collect()
}

#[test]
fn each_object_lists_the_usages_it_is_a_member_of() {
    let d = document();
    let r1 = object(&d, "instance", "R1");
    assert_eq!(
        said(r1),
        [
            ("%pair1", vec!["left"], "in-effect"),
            ("%top", vec!["second"], "in-effect"),
            ("%all", vec!["members"], "in-effect"),
            ("%bad", vec!["left"], "failed"),
        ]
    );
    let pair1 = &r1.usages[0];
    let usage = pair1.usage.unwrap();
    assert_eq!((d.objects[usage].kind, d.objects[usage].name.as_str()), ("usage", "%pair1"));
    assert_eq!(
        (pair1.definition.as_str(), pair1.file.as_deref(), pair1.line, pair1.source),
        ("$PAIR", Some("explain.sp"), 15, "written")
    );
    // a set role's member, and a failed application, which made no usage, with its reason
    assert!(r1.usages[2].roles[0].set);
    let bad = &r1.usages[3];
    assert_eq!((bad.usage, bad.line), (None, 20));
    assert_eq!(bad.reason.as_deref(), Some("the ordinary definition $PAIR has no role extra (9.4)"));
    // the chain upward: a usage's own object lists the usages it is in
    assert_eq!(said(&d.objects[usage]), [("%outer", vec!["first"], "in-effect")]);
    let outer = d.objects[usage].usages[0].usage.unwrap();
    assert_eq!(said(&d.objects[outer]), [("%top", vec!["first"], "in-effect")]);
}

#[test]
fn states_and_sources_of_the_usages() {
    let d = document();
    let r3 = object(&d, "instance", "R3");
    let by = |id: &str| r3.usages.iter().find(|e| e.id == id).unwrap();
    assert_eq!(by("%off").state, "disabled");
    assert_eq!((by("%bad").state, by("%bad").roles[0].role.as_str()), ("failed", "extra"));
    // a terminal of an instance, by its number and its name
    assert_eq!(
        by("%gnd").roles,
        [RoleOf { role: "p".into(), set: false, member: "#R3:2".into(), terminal: Some("n".into()) }]
    );
    // a usage a rule made, with where the rule is written
    let rule = r3.usages.iter().find(|e| e.source == "rule").unwrap();
    assert_eq!(rule.definition, "$LONE");
    assert_eq!(rule.rule, Some(RuleAt { package: String::new(), file: "explain.sp".into(), line: 23 }));
    // a rule of a package: the standard package's ground stubs on the MOSFET's tied terminals
    let m1 = object(&d, "instance", "M1");
    let stubs: Vec<&MemberOf> = m1.usages.iter().filter(|e| e.source == "rule").collect();
    assert!(!stubs.is_empty());
    assert!(stubs.iter().all(|e| e.rule.as_ref().is_some_and(|r| r.package == "rnl-std-fill")), "{stubs:?}");
    let drain = m1.usages.iter().find(|e| e.id == "%drain").unwrap();
    assert_eq!((drain.roles[0].member.as_str(), drain.roles[0].terminal.as_deref()), ("#M1:1", Some("d")));
    // a net, and a usage whose PLACE lost a conflict to one of higher priority
    assert_eq!(said(object(&d, "net", "b")), [("%sig", vec!["signal"], "in-effect")]);
    let port = &object(&d, "net", "c").usages[0];
    assert_eq!((port.id.as_str(), port.state), ("%port", "overridden"));
    assert!(port.reason.as_deref().is_some_and(|r| r.contains("discarded in the order of 8.5")), "{port:?}");
    // a usage that a solver wrote back, and one the body of a definition made
    let files = MemoryProvider::new().with(
        "/w/ota5t.sp",
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site/examples/ota5t.sp")).unwrap(),
    );
    let d = rnl_view::build("/w/ota5t.sp", &files, &BuildOptions { svg: false, ..BuildOptions::default() }).unwrap();
    let m5 = object(&d, "instance", "M5");
    assert!(m5.usages.iter().any(|e| e.id == "%gnd" && e.source == "solver"), "{:?}", m5.usages);
    let vdd = object(&d, "net", "vdd");
    assert!(vdd
        .usages
        .iter()
        .all(|e| e.source == "body" && e.made_in.as_deref().is_some_and(|p| p.starts_with("%vdd"))));
}

#[test]
fn the_snapshot_gives_each_object_its_usages() {
    let files = MemoryProvider::new().with("/w/explain.sp", NETLIST);
    let parts = Parts { drawing: false, host: true, model: true };
    let s = rnl_view::snapshot("/w/explain.sp", &files, &BuildOptions::default(), parts).unwrap();
    let s = serde_json::to_value(&s).unwrap();
    let level = &s["levels"][0];
    let objects = level["model"]["objects"].as_array().unwrap();
    let r1 = objects.iter().find(|o| o["id"] == "#R1").unwrap();
    let ids: Vec<&str> = r1["usages"].as_array().unwrap().iter().map(|u| u["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["%pair1", "%top", "%all", "%bad"]);
    assert_eq!(r1["usages"][0]["file"], 0);
    assert_eq!(r1["usages"][3]["state"], "failed");
    let pair1 = objects.iter().find(|o| o["id"] == "%pair1").unwrap();
    assert_eq!(pair1["usages"][0]["id"], "%outer");
    // an object in none has no list
    let top = objects.iter().find(|o| o["id"] == "%top").unwrap();
    assert!(top.get("usages").is_none());
    let net = level["host"]["nets"].as_array().unwrap().iter().find(|n| n["id"] == "&b").unwrap();
    assert_eq!(net["usages"][0]["roles"][0]["role"], "signal");
}

fn rnl(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace("\r\n", "\n");
    (out.status.code().unwrap_or(-1), text(&out.stdout), text(&out.stderr))
}

fn netlist() -> String {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-explain");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("explain.sp"), NETLIST).unwrap();
    rnl_cli::path_str(&dir.join("explain.sp"))
}

#[test]
fn explain_prints_a_line_for_each_usage_and_the_chain_upward() {
    let file = netlist();
    let (code, out, err) = rnl(&["explain", &file, "#R3"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "#R3 in explain.sp: 6 usages
  second of usage %outer (definition $TWO) · explain.sp:16 · written
    %outer is in turn first of usage %top (definition $TWO) · explain.sp:17 · written
  one of {members} of usage %all (definition $SET) · explain.sp:18 · written
  right of usage %off (definition $PAIR) · explain.sp:19 · written · DISABLED
  p (through terminal n) of usage %gnd (definition $GROUND) · explain.sp:24 · written
  part of usage @explain.sp:23:38 (definition $LONE) · explain.sp:23 · made by the rule at explain.sp:23
  extra of usage %bad (definition $PAIR) · explain.sp:20 · written · FAILED: the ordinary definition $PAIR has no role extra (9.4)
"
    );
    // a terminal, by its name or its number: only the usages of that terminal
    let (_, by_name, _) = rnl(&["explain", &file, "#M1:d"]);
    assert_eq!(
        by_name,
        "#M1:d in explain.sp: 1 usage\n  pin (through terminal d) of usage %drain (definition $PIN) · explain.sp:21 · written\n"
    );
    let (_, by_number, _) = rnl(&["explain", &file, "#M1:1"]);
    assert_eq!(by_number, by_name.replace("#M1:d in", "#M1:1 in"));
    let (_, out, _) = rnl(&["explain", &file, "%top"]);
    assert_eq!(out, "%top in explain.sp: in no usage\n");
}

#[test]
fn explain_gives_json_with_the_chain_in_outer() {
    let (code, out, _) = rnl(&["explain", &netlist(), "#R1", "--json"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let r1 = &v["objects"][0];
    assert_eq!((&r1["id"], &r1["kind"], &r1["level"]), (&"#R1".into(), &"instance".into(), &"explain.sp".into()));
    let pair1 = &r1["usages"][0];
    assert_eq!((&pair1["id"], &pair1["roles"][0]["role"]), (&"%pair1".into(), &"left".into()));
    assert_eq!(pair1["outer"][0]["id"], "%outer");
    assert_eq!(pair1["outer"][0]["outer"][0]["id"], "%top");
    assert_eq!(pair1["outer"][0]["outer"][0]["outer"], serde_json::json!([]));
}

#[test]
fn explain_refuses_an_unknown_reference_with_the_names_close_to_it() {
    let file = netlist();
    let (code, _, err) = rnl(&["explain", &file, "#R9"]);
    assert_eq!((code, err.trim()), (2, "rnl: no object is #R9; close names: #R1, #R2, #R3, #M1"));
    let (code, _, err) = rnl(&["explain", &file, "#R1", "--level", "amp"]);
    assert_eq!((code, err.trim()), (2, "rnl: no level is titled amp; the levels are explain.sp"));
    let (code, _, err) = rnl(&["explain", &file]);
    assert_eq!(code, 2);
    assert!(err.starts_with("rnl explain takes FILE and REF"), "{err}");
    let (code, _, err) = rnl(&["check", &file, "--json"]);
    assert_eq!((code, err.trim()), (2, "--json is an option of explain"));
}

#[test]
fn explain_on_the_five_transistor_ota() {
    let ota = rnl_cli::path_str(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site/examples/ota5t.sp"));
    let (code, out, _) = rnl(&["explain", &ota, "#M1"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "#M1 in ota5t: 2 usages
  in_p of usage %dp1 (definition $DIFFERENTIAL_PAIR) · ota5t.sp:12 · written
  a of usage %sym12 (definition $MIRRORED) · ota5t.sp:19 · written
"
    );
    // the level by its title
    let (code, same, _) = rnl(&["explain", &ota, "#M1", "--level", "ota5t"]);
    assert_eq!((code, same), (0, out));
}

/// Usages nested seven deep, with one, `%u3`, a member of two outer usages, `%u4` and `%v4`.
const DEEP: &str = r#"* deep
R1 a b 1k
R2 b c 1k
R3 c d 1k
* @RNL(type="header") { @VERSION("0.3")}
* @RNL {
*   $PAIR = @DEFINE(roles=["left", "right"]) {};
*   $TWO = @DEFINE(roles=[["first", "usage"], "second"]) {};
*   %u1 = #R1(left) #R2(right) @AS $PAIR;
*   %u2 = %u1(first) #R3(second) @AS $TWO;
*   %u3 = %u2(first) #R3(second) @AS $TWO;
*   %u4 = %u3(first) #R3(second) @AS $TWO;
*   %u5 = %u4(first) #R3(second) @AS $TWO;
*   %u6 = %u5(first) #R3(second) @AS $TWO;
*   %u7 = %u6(first) #R3(second) @AS $TWO;
*   %v4 = %u3(first) #R1(second) @AS $TWO;
*   %v5 = %v4(first) #R2(second) @AS $TWO;
* }
.end
"#;

#[test]
fn explain_indents_a_deep_chain_and_its_forks() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-explain");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("deep.sp"), DEEP).unwrap();
    let (code, out, err) = rnl(&["explain", &rnl_cli::path_str(&dir.join("deep.sp")), "#R2"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "#R2 in deep.sp: 2 usages
  right of usage %u1 (definition $PAIR) · deep.sp:9 · written
    %u1 is in turn first of usage %u2 (definition $TWO) · deep.sp:10 · written
      %u2 is in turn first of usage %u3 (definition $TWO) · deep.sp:11 · written
        %u3 is in turn first of usage %u4 (definition $TWO) · deep.sp:12 · written
          %u4 is in turn first of usage %u5 (definition $TWO) · deep.sp:13 · written
            %u5 is in turn first of usage %u6 (definition $TWO) · deep.sp:14 · written
              %u6 is in turn first of usage %u7 (definition $TWO) · deep.sp:15 · written
        %u3 is in turn first of usage %v4 (definition $TWO) · deep.sp:16 · written
          %v4 is in turn first of usage %v5 (definition $TWO) · deep.sp:17 · written
  second of usage %v5 (definition $TWO) · deep.sp:17 · written
"
    );
}

#[test]
fn explain_stops_a_chain_that_loops() {
    // The language cannot write a usage that contains itself, so the loop is made in the index.
    let files = MemoryProvider::new().with("/w/deep.sp", DEEP);
    let mut d = rnl_view::build("/w/deep.sp", &files, &BuildOptions { svg: false, ..BuildOptions::default() }).unwrap();
    let at = |d: &Document, name: &str| d.objects.iter().position(|o| o.kind == "usage" && o.name == name).unwrap();
    let (u1, u7) = (at(&d, "%u1"), at(&d, "%u7"));
    let into_u2 = d.objects[u1].usages[0].clone();
    d.objects[u7].usages.push(into_u2);
    let found = rnl_cli::explain::find(&d, "%u2", None).unwrap();
    assert_eq!(
        rnl_cli::explain::lines(&d, &found),
        [
            "%u2 in deep.sp: 1 usage",
            "  first of usage %u3 (definition $TWO) · deep.sp:11 · written",
            "    %u3 is in turn first of usage %u4 (definition $TWO) · deep.sp:12 · written",
            "      %u4 is in turn first of usage %u5 (definition $TWO) · deep.sp:13 · written",
            "        %u5 is in turn first of usage %u6 (definition $TWO) · deep.sp:14 · written",
            "          %u6 is in turn first of usage %u7 (definition $TWO) · deep.sp:15 · written",
            "            %u7 is in turn first of usage %u2 (definition $TWO) · deep.sp:10 · written",
            "              %u2 is in turn first of usage %u3 (definition $TWO) · deep.sp:11 · written · the chain loops back to %u3",
            "    %u3 is in turn first of usage %v4 (definition $TWO) · deep.sp:16 · written",
            "      %v4 is in turn first of usage %v5 (definition $TWO) · deep.sp:17 · written",
        ]
    );
}
