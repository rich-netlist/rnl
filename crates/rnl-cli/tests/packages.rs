//! Packages that reference packages (standard 16.2), and the standard package as a package
//! collection of its sub-packages (16.3). The header record of a package file may reference other
//! packages with USE-PACKAGE: used, a package brings in what it references first, each package
//! once, and on a chain of levels one package of each name is in effect. Ported from the tests of
//! the language prototype (rnl-dev d668901).

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::{Diag, Severity};
use rnl_runtime::interp::obj::Lit;
use rnl_runtime::model::{LevelPackage, LevelView};
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

/// The standard package as the root brings it in: the sub-packages each package collection
/// references first, then the collection itself (16.3).
const STD_ORDER: [&str; 8] = [
    "rnl-std-style",
    "rnl-std-symbols",
    "rnl-std-virtual",
    "rnl-std-core",
    "rnl-std-fill",
    "rnl-std-semantics",
    "rnl-std-layout",
    "rnl-std",
];

const NETLIST: &str = "R1 a 0 1k\nR2 a b 1k\n";

/// A package file. A reference is (name, version, path); an empty path is left out, so that the
/// package is found by name in the package directories (16.2).
fn pk(name: &str, version: &str, refs: &[(&str, &str, &str)], body: &str) -> String {
    let mut head = vec!["@VERSION(\"0.2\")".to_string(), format!("@PACKAGE(name=\"{name}\", version=\"{version}\")")];
    for (n, v, path) in refs {
        let path = if path.is_empty() { String::new() } else { format!(", path=\"{path}\"") };
        head.push(format!("@USE-PACKAGE(name=\"{n}\", version=\"{v}\"{path})"));
    }
    let body = if body.is_empty() { String::new() } else { format!("@RNL {{ {body} }}\n") };
    format!("@RNL(type=\"header\") {{ {}}}\n{body}", head.join("; "))
}

/// A reference by path, to `NAME.rnl` beside the package.
fn by_path<'a>(name: &'a str, version: &'a str) -> (&'a str, &'a str, String) {
    (name, version, format!("{name}.rnl"))
}

fn refs<'a>(list: &'a [(&'a str, &'a str, String)]) -> Vec<(&'a str, &'a str, &'a str)> {
    list.iter().map(|(n, v, p)| (*n, *v, p.as_str())).collect()
}

/// A netlist with its header record right after `netlist` (line 4 after `NETLIST`) and one more
/// record after that, as given.
fn doc(header: &str, body: &str, netlist: &str) -> String {
    let header = if header.is_empty() { String::new() } else { format!("; {header}") };
    let body = if body.is_empty() { String::new() } else { format!("* @RNL {{ {body} }}\n") };
    // the header record is the first record of the file, above the subcircuits (2.8)
    format!("* t\n* @RNL(type=\"header\") {{ @VERSION(\"0.2\"){header}}}\n{netlist}{body}.end\n")
}

fn read(files: &MemoryProvider, entry: &str) -> Interpretation {
    interpret_file(Path::new(entry), None, files, &Options::default()).unwrap()
}

fn read_doc(text: &str) -> Interpretation {
    read(&MemoryProvider::new().with("t.sp", text), "t.sp")
}

fn shown(r: &Interpretation, severity: Severity) -> Vec<&Diag> {
    r.diagnostics().into_iter().filter(|d| d.severity == severity).collect()
}

fn messages(r: &Interpretation, severity: Severity) -> Vec<String> {
    shown(r, severity).iter().map(|d| d.message.clone()).collect()
}

fn files_of(r: &Interpretation, severity: Severity) -> Vec<(String, u32)> {
    shown(r, severity).iter().map(|d| d.loc.as_ref().map_or((String::new(), 0), |l| (l.file.clone(), l.line))).collect()
}

fn errors(r: &Interpretation) -> String {
    shown(r, Severity::Error).iter().map(|d| r.sink.render(d)).collect::<Vec<_>>().join("\n")
}

fn level<'a>(r: &'a Interpretation, title: &str) -> &'a LevelView {
    r.model.levels.iter().find(|l| l.title == title).unwrap()
}

/// The packages placed at the root of a level, in queue order (8.3, 16.3).
fn root_names(lv: &LevelView) -> Vec<&str> {
    lv.packages.iter().filter(|p| p.at.is_none()).map(|p| p.name.as_str()).collect()
}

fn package<'a>(lv: &'a LevelView, name: &str) -> &'a LevelPackage {
    lv.packages.iter().find(|p| p.name == name).unwrap()
}

/// The definition of the usage a level binds to `%name`.
fn definition<'a>(lv: &'a LevelView, name: &str) -> &'a str {
    let o = lv.objects.iter().find(|o| o.id == format!("%{name}")).unwrap();
    o.definition.as_deref().unwrap()
}

/// The role names of that usage's definition.
fn roles(lv: &LevelView, name: &str) -> Vec<String> {
    let o = lv.objects.iter().find(|o| o.id == format!("%{name}")).unwrap();
    o.attach.iter().map(|(role, _)| role.clone()).collect()
}

/// The virtual elements of the standard package, which its fill adds.
const VIRTUAL: [&str; 5] = ["$GND", "$SUPPLY", "$IOPORT", "$GND_STUB", "$SUPPLY_STUB"];

/// The definitions of the usages of a level but those of the virtual elements, sorted.
fn usages(lv: &LevelView) -> Vec<&str> {
    let mut out: Vec<&str> =
        lv.objects.iter().filter_map(|o| o.definition.as_deref()).filter(|d| !VIRTUAL.contains(d)).collect();
    out.sort();
    out
}

/// The usages of the virtual elements of the standard package in all levels: (level kind,
/// definition), sorted.
fn virtual_usages(r: &Interpretation) -> Vec<(&str, &str)> {
    let mut out: Vec<(&str, &str)> = r
        .model
        .levels
        .iter()
        .flat_map(|lv| lv.objects.iter().filter_map(move |o| Some((lv.kind, o.definition.as_deref()?))))
        .filter(|(_, d)| VIRTUAL.contains(d))
        .collect();
    out.sort();
    out
}

/// The line colour of a drawn item.
fn linecolor<'a>(lv: &'a LevelView, id: &str) -> &'a str {
    let item = lv.items.iter().find(|i| i.id == id).unwrap();
    match item.style.get("linecolor") {
        Some(Lit::Str(s)) => s.as_str(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_package_brings_in_what_it_references_first() {
    // the standard package is a package collection whose rules keep the order of the old single
    // file: the sub-packages each collection references come first, each once (16.3)
    let r = read_doc(&doc("", "#R1 @PLACE(0, 0)", NETLIST));
    assert_eq!(errors(&r), "");
    let top = level(&r, "t.sp");
    assert_eq!(root_names(top), STD_ORDER);
    assert!(top.packages.iter().all(|p| p.version == "0.2" && p.via.is_none()), "{:?}", top.packages);
    let names: Vec<&str> = r.model.packages.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, STD_ORDER);

    // a collection of one's own, at the root and in a subcircuit: the rule of the package
    // expanded last wins
    let style = |n: &str, color: &str| {
        format!(
            "$s{n} = @DEFINE-STYLE(linecolor=\"{color}\") {{}}; # @SELECT-INSTANCE(name=\"R*\") => @SET-STYLE $s{n}"
        )
    };
    let q = [by_path("q", "1.0")];
    let mut files = MemoryProvider::new()
        .with("q.rnl", &pk("q", "1.0", &[], &style("q", "#000001")))
        .with("a.rnl", &pk("a", "1.0", &refs(&q), &style("a", "#000002")))
        .with("c.rnl", &pk("c", "1.0", &refs(&q), &style("c", "#000003")));
    let net = "R1 a 0 1k\nX1 a 0 sub\n.subckt sub p n\nR9 p n 1k\n\
               * @RNL { @USE-PACKAGE(name=\"b2\", version=\"1.0\", path=\"b2.rnl\"); #R9 @PLACE(0, 0) }\n.ends\n";
    let main = doc("@USE-PACKAGE(name=\"b\", version=\"1.0\", path=\"b.rnl\")", "#R1 @PLACE(0, 0)", net);
    files.insert("t.sp", &main);
    for (order, names, color) in [(["a", "c"], ["q", "a", "c"], "#000003"), (["c", "a"], ["q", "c", "a"], "#000002")] {
        let list = order.map(|n| by_path(n, "1.0"));
        files.insert("b.rnl", &pk("b", "1.0", &refs(&list), ""));
        files.insert("b2.rnl", &pk("b2", "1.0", &refs(&list), ""));
        let r = read(&files, "t.sp");
        assert_eq!(errors(&r), "");
        let (top, sub) = (level(&r, "t.sp"), level(&r, "sub"));
        let mut want: Vec<&str> = STD_ORDER.to_vec();
        want.extend(names);
        want.push("b");
        assert_eq!(root_names(top), want);
        // what the USE-PACKAGE of b brings in is placed at it, and says so
        let via = &package(top, "q").via.as_ref().unwrap();
        assert_eq!((via.0.as_str(), via.1.file.as_str(), via.1.line), ("b", "t.sp", 2));
        assert_eq!(linecolor(top, "#R1"), color);
        assert_eq!(linecolor(sub, "#R9"), color);
        // q, a and c again, now at the subcircuit's USE-PACKAGE, in the order b2 brings them in
        let at_sub: Vec<&str> =
            sub.packages.iter().filter(|p| p.at == Some(sub.level)).map(|p| p.name.as_str()).collect();
        let mut want: Vec<&str> = names.to_vec();
        want.push("b2");
        assert_eq!(at_sub, want);
    }
}

#[test]
fn the_names_of_referenced_packages_are_passed_on() {
    // the `$` names of the packages a package references, directly or not, are seen in it and
    // passed on to the file that uses it; `%` names stay private (16.2)
    let pq = [by_path("pq", "1.0")];
    let pa = [by_path("pa", "1.0")];
    let files = MemoryProvider::new()
        .with("pq.rnl", &pk("pq", "1.0", &[], "$PQ = @DEFINE(roles=[\"a\"]) {}"))
        .with(
            "pa.rnl",
            &pk(
                "pa",
                "1.0",
                &refs(&pq),
                "$PA = @DEFINE(roles=[\"a\"]) {%q = ?a @AS $PQ}; %PRIV = @DEFINE-STYLE(linecolor=\"#ff0000\") {}",
            ),
        )
        .with(
            "bun.rnl",
            &pk(
                "bun",
                "1.0",
                &refs(&pa),
                "$BUN = @DEFINE(roles=[\"a\"]) {%x = ?a @AS $PA; %y = ?a @AS $PQ}; \
                 $BAD = @DEFINE-STYLE(linecolor=\"#00ff00\") {%PRIV}",
            ),
        )
        .with(
            "t.sp",
            &doc(
                "@USE-PACKAGE(name=\"bun\", version=\"1.0\", path=\"bun.rnl\")",
                "%u = #R1 @AS $BUN; %v = #R2 @AS $PQ; %w = #R2 @AS $PA",
                NETLIST,
            ),
        );
    let r = read(&files, "t.sp");
    let errors = messages(&r, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with("%PRIV cannot be found"), "{errors:?}");
    assert_eq!(files_of(&r, Severity::Error)[0].0, "bun.rnl");
    let top = level(&r, "t.sp");
    assert_eq!((definition(top, "u"), definition(top, "v"), definition(top, "w")), ("$BUN", "$PQ", "$PA"));
    // in the bodies: $PA and $PQ in bun's, $PQ in pa's, twice
    assert_eq!(usages(top), ["$BUN", "$PA", "$PA", "$PQ", "$PQ", "$PQ", "$PQ"]);
}

#[test]
fn a_later_reference_hides_an_earlier_one() {
    // several referenced packages define `$X`: the later reference hides the earlier one, and the
    // package's own definition hides them all, with a hint in the package (16.2); used by a file,
    // the hints of a package are not shown (7.6)
    let list = [by_path("pa", "1.0"), by_path("pc", "1.0")];
    let files = MemoryProvider::new()
        .with("pa.rnl", &pk("pa", "1.0", &[], "$X = @DEFINE(roles=[\"a\"]) {}; $Y = @DEFINE(roles=[\"a\"]) {}"))
        .with("pc.rnl", &pk("pc", "1.0", &[], "$X = @DEFINE(roles=[\"c\"]) {}"))
        .with("bun2.rnl", &pk("bun2", "1.0", &refs(&list), "$Y = @DEFINE(roles=[\"own\"]) {}"));
    let r = read(&files, "bun2.rnl");
    assert_eq!(
        messages(&r, Severity::Hint),
        [
            "$X in package pc hides the definition of the same name in package pa, referenced before it; package bun2 uses the one of pc (16.2)",
            "$Y hides the definition of the same name in the referenced package pa; package bun2 uses this one (16.2)",
        ]
    );
    assert_eq!(files_of(&r, Severity::Hint), [("bun2.rnl".to_string(), 1), ("bun2.rnl".to_string(), 2)]);

    let main = doc(
        "@USE-PACKAGE(name=\"bun2\", version=\"1.0\", path=\"bun2.rnl\")",
        "%x = #R1 @AS $X; %y = #R2 @AS $Y",
        NETLIST,
    );
    let r = read(&files.with("t.sp", &main), "t.sp");
    let other: Vec<String> = r
        .diagnostics()
        .iter()
        .filter(|d| d.severity != Severity::Hint || d.message.contains("hides"))
        .map(|d| r.sink.render(d))
        .collect();
    assert!(other.is_empty(), "{other:?}");
    let top = level(&r, "t.sp");
    assert_eq!((roles(top, "x"), roles(top, "y")), (vec!["c".to_string()], vec!["own".to_string()]));
}

#[test]
fn an_empty_inline_package_drops_the_fill() {
    // an empty inline package of the same name drops a package of the standard package: no
    // ground, supply or port tag is added any more; the stubs are in rnl-std-virtual and can
    // still be applied by hand (16.3)
    let net = "R1 a 0 1k\nR2 a vdd 1k\nX1 a 0 sub\n.subckt sub p n\nR9 p n 1k\n.ends\n";
    let r = read_doc(&doc("", "&vdd @ADD-TO $SUPPLY_NET_SET", net));
    assert_eq!(errors(&r), "");
    let before = virtual_usages(&r);
    for x in [("file", "$GND_STUB"), ("file", "$SUPPLY_STUB"), ("subckt", "$IOPORT")] {
        assert!(before.contains(&x), "{before:?}");
    }
    let drop = "@USE-PACKAGE(name=\"rnl-std-fill\") {}";
    let r = read_doc(&doc(drop, "%g = $GND_STUB @ATTACH #R1:2", net));
    assert_eq!(errors(&r), "");
    // only the one written
    assert_eq!(virtual_usages(&r), [("file", "$GND"), ("file", "$GND_STUB")]);
    // the empty package of that name takes its place
    let top = level(&r, "t.sp");
    assert_eq!(root_names(top), STD_ORDER);
    let fill = package(top, "rnl-std-fill");
    assert_eq!(
        (fill.version.as_str(), fill.via.as_ref().map(|v| (v.0.as_str(), v.1.line))),
        ("", Some(("rnl-std-fill", 2)))
    );
    let r = read_doc(&doc(drop, "&vdd @ADD-TO $SUPPLY_NET_SET", net));
    assert!(errors(&r).contains("$SUPPLY_NET_SET cannot be found"), "{}", errors(&r));
}

#[test]
fn the_smallest_collection_alone() {
    // drop rnl-std and use rnl-std-core: symbols and styles stay, no fill, no semantics (16.3)
    let header = "@USE-PACKAGE(name=\"rnl-std\") {}; @USE-PACKAGE(name=\"rnl-std-core\", version=\"0.2\")";
    let r = read_doc(&doc(header, "%d = #R1 #R2 @AS $DIFFPAIR; #R1 @PLACE(0, 0)", NETLIST));
    let top = level(&r, "t.sp");
    assert_eq!(root_names(top), ["rnl-std-style", "rnl-std-symbols", "rnl-std-virtual", "rnl-std-core", "rnl-std"]);
    let errors = messages(&r, Severity::Error);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with("$DIFFPAIR cannot be found"), "{errors:?}");
    let r1 = top.objects.iter().find(|o| o.id == "#R1").unwrap();
    assert_eq!(r1.symbol.as_ref().unwrap().name, "$RES");
    assert_eq!(linecolor(top, "#R1"), "#1d1d1f");
    assert!(virtual_usages(&r).is_empty());
}

#[test]
fn a_collection_of_ones_own_adds_by_reference() {
    // more: a collection that references a package of the standard package by name and one of
    // its own by path; a package that is already there is not added twice (16.3)
    let list = [("rnl-std-semantics", "0.2", ""), ("mine", "1.0", "mine.rnl")];
    let files = MemoryProvider::new()
        .with("mine.rnl", &pk("mine", "1.0", &[], "$MINE = @DEFINE(roles=[\"a\"]) {}"))
        .with("more.rnl", &pk("more", "1.0", &list, ""));
    let header = "@USE-PACKAGE(name=\"rnl-std\") {}; @USE-PACKAGE(name=\"rnl-std-core\", version=\"0.2\"); \
                  @USE-PACKAGE(name=\"more\", version=\"1.0\", path=\"more.rnl\")";
    let main = doc(header, "%d = #R1 #R2 @AS $DIFFPAIR; %m = #R1 @AS $MINE", NETLIST);
    let r = read(&files.with("t.sp", &main), "t.sp");
    assert_eq!(errors(&r), "");
    let top = level(&r, "t.sp");
    assert_eq!(
        root_names(top),
        [
            "rnl-std-style",
            "rnl-std-symbols",
            "rnl-std-virtual",
            "rnl-std-core",
            "rnl-std-semantics",
            "rnl-std",
            "mine",
            "more"
        ]
    );
    assert_eq!((definition(top, "d"), definition(top, "m")), ("$DIFFPAIR", "$MINE"));

    // the fill written again in the header: one copy of it, at its place in the standard package
    let stubs = |r: &Interpretation| virtual_usages(r).iter().filter(|u| u.1 == "$GND_STUB").count();
    let n = stubs(&read_doc(&doc("", "", NETLIST)));
    assert!(n > 0);
    let r = read_doc(&doc("@USE-PACKAGE(name=\"rnl-std-fill\", version=\"0.2\")", "", NETLIST));
    assert_eq!(errors(&r), "");
    assert_eq!(stubs(&r), n);
    assert_eq!(root_names(level(&r, "t.sp")), STD_ORDER);
}

/// q in three versions, and three packages that reference it.
fn versions() -> MemoryProvider {
    MemoryProvider::new()
        .with("q-1.0.rnl", &pk("q", "1.0", &[], "$Q_OLD = @DEFINE(roles=[\"a\"]) {}"))
        .with(
            "q-1.2.rnl",
            &pk("q", "1.2", &[], "$Q_OLD = @DEFINE(roles=[\"a\"]) {}; $Q_NEW = @DEFINE(roles=[\"a\"]) {}"),
        )
        .with("q-2.0.rnl", &pk("q", "2.0", &[], "$Q_TWO = @DEFINE-STYLE(linecolor=\"#ff0000\") {}"))
        .with(
            "pk-a.rnl",
            &pk("pk-a", "1.0", &[("q", "1.0", "q-1.0.rnl")], "$A_NEW = @DEFINE(roles=[\"a\"]) {%n = ?a @AS $Q_NEW}"),
        )
        .with("pk-b.rnl", &pk("pk-b", "1.0", &[("q", "1.2", "q-1.2.rnl")], ""))
        .with(
            "pk-c.rnl",
            &pk("pk-c", "1.0", &[("q", "2.0", "q-2.0.rnl")], "# @SELECT-INSTANCE(name=\"R1\") => @SET-STYLE $Q_TWO"),
        )
}

fn use_package(name: &str) -> String {
    format!("@USE-PACKAGE(name=\"{name}\", version=\"1.0\", path=\"{name}.rnl\")")
}

#[test]
fn a_use_package_without_a_version_takes_the_first_directory_of_the_newest() {
    // two package directories hold q 1.0, each its own: a USE-PACKAGE that names no version takes
    // the newest found, and of its copies the one of the first directory, as one that names the
    // version does (16.2)
    let q = |def: &str| pk("q", "1.0", &[], &format!("{def} = @DEFINE(roles=[\"a\"]) {{}}"));
    for header in ["@USE-PACKAGE(name=\"q\")", "@USE-PACKAGE(name=\"q\", version=\"1.0\")"] {
        let main = doc(header, "%u = #R1 @AS $Q_A", NETLIST);
        let mut files =
            MemoryProvider::new().with("a/q.rnl", &q("$Q_A")).with("b/q.rnl", &q("$Q_B")).with("t.sp", &main);
        files.package_dirs = vec!["a".into(), "b".into()];
        let r = read(&files, "t.sp");
        assert_eq!(errors(&r), "", "{header}");
        assert_eq!(usages(level(&r, "t.sp")), ["$Q_A"], "{header}");
    }
}

#[test]
fn references_take_the_newest_version_of_one_major_version() {
    // brought in only by references: the newest version within the major version of the
    // reference expanded first, with a hint at the USE-PACKAGE written in the file (16.2)
    let main = doc(&format!("{}; {}", use_package("pk-a"), use_package("pk-b")), "%u = #R1 @AS $A_NEW", NETLIST);
    let r = read(&versions().with("t.sp", &main), "t.sp");
    assert_eq!(errors(&r), "");
    let top = level(&r, "t.sp");
    assert_eq!(package(top, "q").version, "1.2");
    // pk-a sees q 1.2, which has the name it uses
    assert_eq!(usages(top), ["$A_NEW", "$Q_NEW"]);
    let hints: Vec<&Diag> =
        shown(&r, Severity::Hint).into_iter().filter(|d| d.message.contains("references")).collect();
    assert_eq!(hints.len(), 1, "{hints:?}");
    assert_eq!(
        hints[0].message,
        "package pk-a references package q 1.0; of the references to it, the newest within that major version is 1.2, which is used (16.2)"
    );
    let loc = hints[0].loc.as_ref().unwrap();
    assert_eq!((loc.file.as_str(), loc.line), ("t.sp", 2)); // at the USE-PACKAGE of pk-a
}

#[test]
fn a_reference_of_another_major_version_fails() {
    // the reference expanded later fails, with the statements that use its names; the error is
    // on the reference and names the USE-PACKAGE in the file that brought it in (16.2)
    let main = doc(&format!("{}; {}", use_package("pk-a"), use_package("pk-c")), "#R1 @PLACE(0, 0)", NETLIST);
    let r = read(&versions().with("t.sp", &main), "t.sp");
    let top = level(&r, "t.sp");
    assert_eq!(package(top, "q").version, "1.0");
    assert_eq!(
        messages(&r, Severity::Error),
        [
            "package pk-c references package q 2.0, of another major version than the reference expanded first (pk-a references 1.0): this reference fails, and the statements of pk-c that use its names fail with it (16.2)",
            "$Q_TWO cannot be found: it is in package q, and the q in effect here does not have it (dropped, replaced, or its reference failed); this statement fails (16.2)",
        ]
    );
    let errors = shown(&r, Severity::Error);
    assert_eq!(
        files_of(&r, Severity::Error).iter().map(|f| f.0.as_str()).collect::<Vec<_>>(),
        ["pk-c.rnl", "pk-c.rnl"]
    );
    assert!(errors[0].origin.starts_with("brought in by the USE-PACKAGE of pk-c (t.sp:2:"), "{}", errors[0].origin);
    assert_eq!(linecolor(top, "#R1"), "#1d1d1f");
}

#[test]
fn a_package_written_in_the_file_wins_over_references() {
    // written in the file: that one, also where packages reference it, with a warning when a
    // reference asks for another major version (16.2)
    let header = format!("@USE-PACKAGE(name=\"q\", version=\"2.0\", path=\"q-2.0.rnl\"); {}", use_package("pk-a"));
    let r = read(&versions().with("t.sp", &doc(&header, "%u = #R1 @AS $A_NEW", NETLIST)), "t.sp");
    assert_eq!(package(level(&r, "t.sp"), "q").version, "2.0");
    assert_eq!(
        messages(&r, Severity::Warning),
        ["package q is 2.0 here, and package pk-a references 1.0: another major version, so pk-a may use names that q 2.0 does not have (16.2)"]
    );
    assert_eq!(files_of(&r, Severity::Warning), [("t.sp".to_string(), 2)]);
    // the name pk-a uses is not in q 2.0
    let errors = errors(&r);
    assert!(errors.contains("pk-a.rnl:2:") && errors.contains("$Q_NEW cannot be found"), "{errors}");
}

#[test]
fn a_reference_that_closes_a_cycle_fails() {
    // the reference that closes the cycle is an error and fails; which one depends on where the
    // expansion starts (16.2)
    let files = MemoryProvider::new()
        .with("pk-a.rnl", &pk("pk-a", "1.0", &refs(&[by_path("pk-b", "1.0")]), "$A = @DEFINE(roles=[\"a\"]) {}"))
        .with("pk-b.rnl", &pk("pk-b", "1.0", &refs(&[by_path("pk-a", "1.0")]), "$B = @DEFINE(roles=[\"a\"]) {}"));
    let main = doc(&use_package("pk-a"), "%a = #R1 @AS $A; %b = #R2 @AS $B", NETLIST);
    let r = read(&files.clone().with("t.sp", &main), "t.sp");
    assert_eq!(
        messages(&r, Severity::Error),
        ["packages reference each other in a cycle, pk-a -> pk-b -> pk-a: this reference fails (16.2)"]
    );
    assert_eq!(files_of(&r, Severity::Error)[0].0, "pk-b.rnl");
    let top = level(&r, "t.sp");
    assert_eq!((definition(top, "a"), definition(top, "b")), ("$A", "$B"));
    // checked on its own, pk-b is where the expansion starts
    let r = read(&files, "pk-b.rnl");
    assert_eq!(
        messages(&r, Severity::Error),
        ["packages reference each other in a cycle, pk-b -> pk-a -> pk-b: this reference fails (16.2)"]
    );
}

#[test]
fn use_package_is_written_only_in_the_header_of_a_package_file() {
    // USE-PACKAGE in a package's other records, a reference with a body, and USE-PACKAGE in an
    // inline package are errors; the rest of each package stays (16.2)
    let pk_b = "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"pk-b\", version=\"1.0\"); \
                @USE-PACKAGE(name=\"pk-a\", version=\"1.0\", path=\"pk-a.rnl\") {} }\n\
                @RNL { $B = @DEFINE(roles=[\"a\"]) {} }\n";
    let files = MemoryProvider::new()
        .with(
            "pk-a.rnl",
            &pk("pk-a", "1.0", &[], "@USE-PACKAGE(name=\"rnl-std-fill\"); $A = @DEFINE(roles=[\"a\"]) {}"),
        )
        .with("pk-b.rnl", pk_b);
    let main = doc(
        &format!("{}; {}", use_package("pk-a"), use_package("pk-b")),
        "@USE-PACKAGE(name=\"emb\") { @USE-PACKAGE(name=\"rnl-std-fill\"); $E = @DEFINE(roles=[\"a\"]) {} }; \
         %e = #R1 @AS $E; %a = #R2 @AS $A; %b = #R2 @AS $B",
        NETLIST,
    );
    let r = read(&files.with("t.sp", &main), "t.sp");
    assert_eq!(
        messages(&r, Severity::Error),
        [
            "USE-PACKAGE is not written in the records of a package other than its header record: a package file references other packages in its header record (16.2)",
            "a reference to another package in the header record of a package file writes only name, version and path, without a body: this reference fails (16.2)",
            "an inline package has no header record and cannot reference other packages; to use the definitions of another package, write them into its body (16.2)",
        ]
    );
    let where_: Vec<String> = files_of(&r, Severity::Error).into_iter().map(|f| f.0).collect();
    assert_eq!(where_, ["pk-a.rnl", "pk-b.rnl", "t.sp"]);
    let top = level(&r, "t.sp");
    assert_eq!((definition(top, "e"), definition(top, "a"), definition(top, "b")), ("$E", "$A", "$B"));
}

#[test]
fn a_dropped_package_that_another_uses() {
    // the statements of the package that use its names fail, with errors that say why; the rest
    // of the standard package stays (16.2)
    let r = read_doc(&doc("@USE-PACKAGE(name=\"rnl-std-virtual\") {}", "#R1 @PLACE(0, 0)", NETLIST));
    let errors = shown(&r, Severity::Error);
    assert!(!errors.is_empty());
    assert!(errors.iter().all(|d| d.loc.as_ref().is_some_and(|l| l.file == "rnl-std-fill.rnl")), "{errors:?}");
    let text: Vec<&str> = errors.iter().map(|d| d.message.as_str()).collect();
    for name in ["$GND_STUB", "$SUPPLY_STUB", "$IOPORT", "$NC", "$GND", "$SUPPLY"] {
        let want = format!("{name} cannot be found: it is in package rnl-std-virtual, and the rnl-std-virtual in effect here does not have it");
        assert!(text.iter().any(|m| m.starts_with(&want)), "{name}: {text:?}");
    }
    let top = level(&r, "t.sp");
    assert!(virtual_usages(&r).is_empty());
    let r1 = top.objects.iter().find(|o| o.id == "#R1").unwrap();
    assert_eq!(r1.symbol.as_ref().unwrap().name, "$RES");
}

#[test]
fn a_reference_by_path_is_relative_to_the_package_file() {
    // path= of a reference is relative to the directory of the package file (16.2); an alias
    // package cannot be referenced (16.5)
    let files = MemoryProvider::new()
        .with("lib/inner.rnl", &pk("inner", "1.0", &[], "$INNER = @DEFINE(roles=[\"a\"]) {}"))
        .with("lib/outer.rnl", &pk("outer", "1.0", &[("inner", "1.0", "inner.rnl")], ""))
        .with(
            "t.sp",
            &doc(
                "@USE-PACKAGE(name=\"outer\", version=\"1.0\", path=\"lib/outer.rnl\")",
                "%i = #R1 @AS $INNER",
                NETLIST,
            ),
        );
    let r = read(&files, "t.sp");
    assert_eq!(errors(&r), "");
    assert_eq!(definition(level(&r, "t.sp"), "i"), "$INNER");

    let files = MemoryProvider::new()
        .with("zh.rnl", "@RNL(type=\"header\") { @VERSION(\"0.2\"); @PACKAGE(name=\"zh\", version=\"1.0\"); @ALIAS(\"@at\", \"@PLACE\")}\n")
        .with("p.rnl", &pk("p", "1.0", &[("zh", "1.0", "zh.rnl")], ""))
        .with("t.sp", &doc("@USE-PACKAGE(name=\"p\", version=\"1.0\", path=\"p.rnl\")", "", NETLIST));
    let r = read(&files, "t.sp");
    assert_eq!(
        messages(&r, Severity::Error),
        ["package zh is an alias package; load it with @USE-ALIASES(name=\"zh\") in the header record. This reference fails (16.5)"]
    );
    assert_eq!(files_of(&r, Severity::Error)[0].0, "p.rnl");
}

#[test]
fn a_referenced_package_gives_its_terminal_names_first() {
    // DEVICE of the packages in the order they are brought in: the package's own comes after
    // those of the packages it references, and wins (6.12)
    let device = |terms: &str| format!("@DEVICE(name=\"X*\", model=\"cell\", terminals=[{terms}])");
    let inner = format!(
        "@RNL(type=\"header\") {{ @VERSION(\"0.2\"); @PACKAGE(name=\"inner\", version=\"1.0\"); {} }}\n",
        device("\"p\", \"q\"")
    );
    let outer = format!(
        "@RNL(type=\"header\") {{ @VERSION(\"0.2\"); @PACKAGE(name=\"outer\", version=\"1.0\"); \
         @USE-PACKAGE(name=\"inner\", version=\"1.0\", path=\"inner.rnl\"); {} }}\n",
        device("\"in\", \"out\"")
    );
    let net = "X1 a b cell\n";
    let files = MemoryProvider::new()
        .with("inner.rnl", &inner)
        .with("outer.rnl", &outer)
        .with("t.sp", &doc("@USE-PACKAGE(name=\"outer\", version=\"1.0\", path=\"outer.rnl\")", "", net));
    let r = read(&files, "t.sp");
    assert_eq!(errors(&r), "");
    let names = &level(&r, "t.sp").terminal_names[0];
    assert_eq!(names, &[Some("in".to_string()), Some("out".to_string())]);
    // without outer's own, inner's
    let outer = outer.replace(&format!("; {} }}", device("\"in\", \"out\"")), " }");
    let r = read(&files.with("outer.rnl", &outer), "t.sp");
    assert_eq!(errors(&r), "");
    assert_eq!(&level(&r, "t.sp").terminal_names[0], &[Some("p".to_string()), Some("q".to_string())]);
}
