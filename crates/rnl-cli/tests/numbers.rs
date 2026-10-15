//! The numbers of the objects without a name (the draft 0.3, 5.9): their order in a level, the
//! solver's view that refers to them, and their check, which `Options::draft` gives.

use std::path::Path;
use std::process::Command;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::numbering::{check, level_check};
use rnl_runtime::problem::{LevelProblem, Reference};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

fn draft() -> Options {
    Options { draft: true, ..Options::default() }
}

fn read(text: &str, options: &Options) -> Interpretation {
    read_files(MemoryProvider::new().with("t.sp", text), options)
}

fn read_files(files: MemoryProvider, options: &Options) -> Interpretation {
    interpret_file(Path::new("t.sp"), None, &files, options).unwrap()
}

/// A file of the draft whose main level has `records`, R1, R2, C1 and M1 between a, b and 0.
fn netlist(version: &str, records: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nR1 a b 1k\nR2 b 0 1k\nC1 a 0 1p\nM1 b a 0 0 nch\n\
         .model nch nmos level=1\n{records}.end\n"
    )
}

/// One record on one carrier line.
fn record(head: &str, body: &str) -> String {
    format!("* @RNL{head} {{ {body} }}\n")
}

fn level_index(r: &Interpretation, title: &str) -> usize {
    r.doc.levels.iter().position(|l| l.title() == title).unwrap_or_else(|| panic!("no level {title}"))
}

/// The lines of the numbers of a level's authored records.
fn authored(r: &Interpretation, title: &str) -> Vec<String> {
    let lv = level_index(r, title);
    r.numbering.iter().find(|n| n.level == lv).unwrap().authored.clone()
}

fn diagnostics(r: &Interpretation, severity: Severity) -> Vec<(u32, String)> {
    let found = r.diagnostics().into_iter().filter(|d| d.severity == severity);
    found.map(|d| (d.loc.as_ref().map_or(0, |l| l.line), d.message.clone())).collect()
}

fn problem<'a>(r: &'a Interpretation, title: &str) -> &'a LevelProblem {
    r.problems.iter().find(|p| p.title == title).unwrap()
}

fn number(n: usize, path: &str) -> Reference {
    Reference::Number { number: n, path: if path.is_empty() { Vec::new() } else { vec![path.to_string()] } }
}

/// The labels of the objects of a problem by their numbers, 1 first.
fn numbered(p: &LevelProblem) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = p
        .objects
        .iter()
        .filter_map(|o| match &o.reference {
            Reference::Number { number, path } if path.is_empty() => Some((*number, o.label.clone())),
            _ => None,
        })
        .chain(p.wires.iter().filter_map(|w| match &w.reference {
            Reference::Number { number, path } if path.is_empty() => Some((*number, w.label.clone())),
            _ => None,
        }))
        .chain(p.usages.iter().filter(|u| u.object.is_none()).filter_map(|u| match &u.reference {
            Reference::Number { number, path } if path.is_empty() => Some((*number, u.label.clone())),
            _ => None,
        }))
        .collect();
    out.sort();
    out
}

/// The example of the draft, 2.5: a subcircuit whose authored record makes a ground, a wire with
/// a free point and two labels, none with a name.
const AMP: &str = "* the example of the draft 0.3, 2.5\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n\
.subckt amp in out vdd vss\nM1 out in vss vss nch\nR1 vdd out 10k\n\
* @RNL {\n\
*   #M1 #R1 @ALIGN(\"x\");\n\
*   $GND @ATTACH &vss;                       /* %#1: a usage without a name */\n\
*   &out @WIRE {#M1:1 (_, _) #R1:2};         /* %#2: the wire; %#3: the free point */\n\
*   {#M1 #R1} @TEXT(\"${name}\")               /* %#4: the label on M1; %#5: the label on R1 */\n\
* }\n";

const AMP_LINES: [&str; 5] = [
    "$GND @ATTACH &vss",
    "&out @WIRE { #M1:1 ( _ , _ ) #R1:2 }",
    "&out @WIRE { #M1:1 ( _ , _ ) #R1:2 }",
    "{ #M1 #R1 } @TEXT ( \"${name}\" )\t#M1",
    "{ #M1 #R1 } @TEXT ( \"${name}\" )\t#R1",
];

/// The example with a record of the solver after the authored one.
fn amp(solver: &str) -> String {
    format!("{AMP}{solver}.ends amp\n.end\n")
}

#[test]
fn the_example_of_the_draft_gives_its_lines() {
    // the standard package's $GND has an empty body: its usage makes no other object
    let r = read(&amp(""), &draft());
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert_eq!(authored(&r, "amp"), AMP_LINES);
    let lv = level_index(&r, "amp");
    assert_eq!(level_check(&r, lv), Some(check(&AMP_LINES.map(String::from)).as_str()));
    // the solver's view refers to them by these numbers
    let r = read(&amp(""), &Options { draft: true, ..Options::solver() });
    let p = problem(&r, "amp");
    assert_eq!(p.version, "0.3");
    assert_eq!(
        numbered(p),
        [(1, "a usage of $GND"), (2, "a wire"), (3, "(_, _)"), (4, "a text"), (5, "a text")]
            .map(|(n, l)| (n, l.to_string()))
    );
    // the ports that the standard package's fill makes are generated, not numbered (10.5)
    assert!(p
        .usages
        .iter()
        .filter(|u| u.definition == "$IOPORT")
        .all(|u| matches!(u.reference, Reference::Generated { .. })));
    // the text of the label on M1 says which instance it is attached to
    let label4 = p.objects.iter().find(|o| o.reference == number(4, "")).unwrap();
    assert_eq!(label4.text.as_ref().unwrap().lines.len(), 1);
}

#[test]
fn one_token_makes_an_object_for_each_member() {
    let text = netlist("0.3", &record("", "{#M1 #R1 #C1} @TEXT(\"${name}\"); &a @WIRE {#R1:1 (_, _) (3, _) #C1:1}"));
    let r = read(&text, &draft());
    assert_eq!(
        authored(&r, "t.sp"),
        [
            "{ #M1 #R1 #C1 } @TEXT ( \"${name}\" )\t#M1",
            "{ #M1 #R1 #C1 } @TEXT ( \"${name}\" )\t#R1",
            "{ #M1 #R1 #C1 } @TEXT ( \"${name}\" )\t#C1",
            "&a @WIRE { #R1:1 ( _ , _ ) ( 3 , _ ) #C1:1 }",
            "&a @WIRE { #R1:1 ( _ , _ ) ( 3 , _ ) #C1:1 }",
            "&a @WIRE { #R1:1 ( _ , _ ) ( 3 , _ ) #C1:1 }",
        ]
    );
    // a terminal by its number, an object without a name by its number; the keyword first
    let text = netlist("0.3", &record("", "{#R1:2 #M1:d} @TEXT(\"${net}\"); @TEXT(\"q\") {(1, 1) (2, 2)}"));
    let r = read(&text, &draft());
    assert_eq!(
        authored(&r, "t.sp"),
        [
            "{ #R1:2 #M1:d } @TEXT ( \"${net}\" )\t#R1:2",
            "{ #R1:2 #M1:d } @TEXT ( \"${net}\" )\t#M1:1",
            "@TEXT ( \"q\" ) { ( 1 , 1 ) ( 2 , 2 ) }\t%#5",
            "@TEXT ( \"q\" ) { ( 1 , 1 ) ( 2 , 2 ) }\t%#6",
            "@TEXT ( \"q\" ) { ( 1 , 1 ) ( 2 , 2 ) }",
            "@TEXT ( \"q\" ) { ( 1 , 1 ) ( 2 , 2 ) }",
        ]
    );
}

const DEFS: &str = "$IN = @DEFINE(roles=[\"a\"]) {(_, _) @TEXT(\"i\"); %m = (1, 2)}; \
                    $OUT = @DEFINE(roles=[\"a\"]) {?a @AS $IN; %in = ?a @AS $IN; (1, 1) @TEXT(\"o\")}";

#[test]
fn the_body_of_a_usage_comes_right_after_it() {
    // an unnamed usage, then its body; then the statement after it
    let text = netlist("0.3", &record("", &format!("{DEFS}; #M1 @AS $IN; (5, 5) @TEXT(\"after\")")));
    let r = read(&text, &draft());
    assert_eq!(
        authored(&r, "t.sp"),
        [
            "#M1 @AS $IN",
            "#M1 @AS $IN\t( _ , _ ) @TEXT ( \"i\" )",
            "#M1 @AS $IN\t( _ , _ ) @TEXT ( \"i\" )",
            "( 5 , 5 ) @TEXT ( \"after\" )",
            "( 5 , 5 ) @TEXT ( \"after\" )",
        ]
    );
    // a local name is reached through the number of its usage
    let r = read(&text, &Options { draft: true, ..Options::solver() });
    let p = problem(&r, "t.sp");
    assert!(p.objects.iter().any(|o| o.reference == number(1, "::m")));
    assert_eq!(numbered(p)[0], (1, "a usage of $IN".to_string()));

    // a named usage has no number, but the objects of its body do, at its place
    let text = netlist("0.3", &record("", &format!("{DEFS}; %u = #M1 @AS $IN; (5, 5) @TEXT(\"after\")")));
    let r = read(&text, &draft());
    assert_eq!(
        authored(&r, "t.sp"),
        [
            "%u = #M1 @AS $IN\t( _ , _ ) @TEXT ( \"i\" )",
            "%u = #M1 @AS $IN\t( _ , _ ) @TEXT ( \"i\" )",
            "( 5 , 5 ) @TEXT ( \"after\" )",
            "( 5 , 5 ) @TEXT ( \"after\" )",
        ]
    );
    let r = read(&text, &Options { draft: true, ..Options::solver() });
    let p = problem(&r, "t.sp");
    assert!(p.objects.iter().any(|o| o.reference == Reference::Name("%u::m".into())));

    // applied for each member: each usage, then its body
    let text = netlist("0.3", &record("", &format!("{DEFS}; {{#M1 #R1}} @AS $IN")));
    let r = read(&text, &draft());
    assert_eq!(
        authored(&r, "t.sp"),
        [
            "{ #M1 #R1 } @AS $IN\t#M1",
            "{ #M1 #R1 } @AS $IN\t#M1\t( _ , _ ) @TEXT ( \"i\" )",
            "{ #M1 #R1 } @AS $IN\t#M1\t( _ , _ ) @TEXT ( \"i\" )",
            "{ #M1 #R1 } @AS $IN\t#R1",
            "{ #M1 #R1 } @AS $IN\t#R1\t( _ , _ ) @TEXT ( \"i\" )",
            "{ #M1 #R1 } @AS $IN\t#R1\t( _ , _ ) @TEXT ( \"i\" )",
        ]
    );
}

#[test]
fn nested_usages_are_numbered_depth_first() {
    let text = netlist("0.3", &record("", &format!("{DEFS}; #M1 @AS $OUT")));
    let r = read(&text, &draft());
    let body = "#M1 @AS $OUT\t?a @AS $IN";
    let local = "#M1 @AS $OUT\t%in = ?a @AS $IN";
    let point = "\t( _ , _ ) @TEXT ( \"i\" )";
    assert_eq!(
        authored(&r, "t.sp"),
        [
            "#M1 @AS $OUT".to_string(),
            body.to_string(),
            format!("{body}{point}"),
            format!("{body}{point}"),
            format!("{local}{point}"),
            format!("{local}{point}"),
            "#M1 @AS $OUT\t( 1 , 1 ) @TEXT ( \"o\" )".to_string(),
            "#M1 @AS $OUT\t( 1 , 1 ) @TEXT ( \"o\" )".to_string(),
        ]
    );
    // the local usage is reached by name through the number of the outer one
    let r = read(&text, &Options { draft: true, ..Options::solver() });
    let p = problem(&r, "t.sp");
    assert!(p
        .objects
        .iter()
        .any(|o| o.reference == Reference::Number { number: 1, path: vec!["::in".into(), "::m".into()] }));
    assert!(p.usages.iter().any(|u| u.reference == number(1, "::in")));
}

#[test]
fn a_scope_record_comes_after_the_levels_own_text() {
    // written before the subcircuit in the text, it still comes after the subcircuit's own record
    let text = "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n\
                * @RNL(scope=\"ota\") { (1, 1) @TEXT(\"scope\") }\n\
                .subckt ota a b\nR1 a b 1k\n* @RNL { (2, 2) @TEXT(\"own\") }\n.ends ota\n\
                X1 n1 n2 ota\n.end\n";
    let r = read(text, &draft());
    assert!(diagnostics(&r, Severity::Error).is_empty(), "{:?}", diagnostics(&r, Severity::Error));
    assert_eq!(
        authored(&r, "ota"),
        [
            "( 2 , 2 ) @TEXT ( \"own\" )",
            "( 2 , 2 ) @TEXT ( \"own\" )",
            "( 1 , 1 ) @TEXT ( \"scope\" )",
            "( 1 , 1 ) @TEXT ( \"scope\" )",
        ]
    );
    // and makes nothing in the level it is written in
    assert!(authored(&r, "t.sp").is_empty());
}

#[test]
fn what_rules_make_and_what_names_reach_is_not_numbered() {
    // a TEXT on a selection makes its texts one by one (10.5); a named point, its text with a key
    let text = netlist(
        "0.3",
        &record("", "# @SELECT-INSTANCE(name=\"R*\") => @TEXT(\"${value}\"); %p = (1, 2); %w = &a @WIRE {#R1:1 %p}; %w @TEXT(\"w\", key=\"k\"); (9, 9) @TEXT(\"x\")"),
    );
    let r = read(&text, &draft());
    assert_eq!(authored(&r, "t.sp"), ["( 9 , 9 ) @TEXT ( \"x\" )", "( 9 , 9 ) @TEXT ( \"x\" )"]);
    let r = read(&text, &Options { draft: true, ..Options::solver() });
    let p = problem(&r, "t.sp");
    assert!(p.objects.iter().any(|o| o.reference == Reference::Name("%w::k".into())));
    assert_eq!(numbered(p).len(), 2);
    // the grounds that the standard package adds are generated, and their wires too: those are
    // reached through the usage written out, the wires not at all
    let text = netlist("0.3", &record("", "(9, 9) @TEXT(\"x\")"));
    let r = read(&text, &Options { draft: true, ..Options::solver() });
    let p = problem(&r, "t.sp");
    assert!(p.wires.iter().any(|w| w.generated));
    assert!(p.wires.iter().filter(|w| w.generated).all(|w| w.reference == Reference::Anonymous));
    assert!(p.usages.iter().filter(|u| u.rule_made).all(|u| matches!(u.reference, Reference::Generated { .. })));
    assert_eq!(numbered(p).len(), 2);
}

#[test]
fn rnl_problem_gives_the_numbers_in_a_file_of_the_draft() {
    let f = Path::new(env!("CARGO_TARGET_TMPDIR")).join("numbers-amp.sp");
    std::fs::write(&f, amp("")).unwrap();
    let f = f.to_string_lossy();
    let out =
        Command::new(env!("CARGO_BIN_EXE_rnl")).args(["problem", &f, "--draft", "--level", "amp"]).output().unwrap();
    assert!(out.status.success());
    let p: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let level = &p["levels"][0];
    let point = level["objects"].as_array().unwrap().iter().find(|o| o["label"] == "(_, _)").unwrap();
    assert_eq!(point["reference"], serde_json::json!({"number": 3, "path": ""}));
    assert_eq!(level["wires"][0]["reference"], serde_json::json!({"number": 2, "path": ""}));
    // in a file of 0.2 the same objects have no reference, as before
    std::fs::write(f.as_ref(), amp("").replace("@VERSION(\"0.3\")", "@VERSION(\"0.2\")")).unwrap();
    let out =
        Command::new(env!("CARGO_BIN_EXE_rnl")).args(["problem", &f, "--draft", "--level", "amp"]).output().unwrap();
    let p: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(p["levels"][0]["wires"][0]["reference"], serde_json::Value::Null);
}

#[test]
fn numbers_are_given_only_with_the_draft() {
    let r = read(&amp("").replace("0.3", "0.2"), &Options::default());
    assert!(r.numbering.is_empty());
    assert_eq!(level_check(&r, 1), None);
    // read with the draft, a file of 0.2 has its numbers and checks too
    let r = read(&amp("").replace("@VERSION(\"0.3\")", "@VERSION(\"0.2\")"), &draft());
    assert_eq!(authored(&r, "amp"), AMP_LINES);
}
