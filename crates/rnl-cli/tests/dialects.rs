//! The rule package of the other dialect (the draft 0.3, 16.3), in files that declare 0.3. A file
//! in the SPICE dialect does not use `rnl-std-spectre`, and one in the Spectre dialect does not
//! use `rnl-std-spice`, as if the file's header record wrote an empty inline package of that name,
//! for its own levels only. The dialect of a file is the language
//! mode of its first line: in a Spectre netlist a `.scs` file or one that starts with
//! `simulator lang=spectre` is in the Spectre dialect and any other file in the SPICE dialect, a
//! `simulator lang=` further down changing nothing; every file of a SPICE netlist is in the SPICE
//! dialect. A USE-PACKAGE of either package that the file, or a file that includes it, writes
//! takes effect instead. Nothing is dropped without a standard package, with a standard series
//! that has no such packages (the toolkit's 0.2), or in a file of 0.2.
//!
//! The standard package here is a small series 0.3 held in memory: `rnl-std` references
//! `rnl-std-core`, which references `rnl-std-basic` with two symbols and the two rule packages,
//! `rnl-std-spice` by name and `rnl-std-spectre` by master. A file of 0.3 uses the series 0.3
//! (16.3), and of the two series 0.3 this one comes first: its package directory comes before the
//! bundled packages (16.2). A file of 0.2 uses the series 0.2 unless it pins 0.3.

use std::path::Path;

use rnl_cli::interpret_file;
use rnl_runtime::diag::Severity;
use rnl_runtime::source::MemoryProvider;
use rnl_runtime::{Interpretation, Options};

const STD: &str = r#"@RNL(type="header") {
  @VERSION("0.3");
  @PACKAGE(name="rnl-std", version="0.3");
  @USE-PACKAGE(name="rnl-std-core", version="0.3")
}
"#;

const CORE: &str = r#"@RNL(type="header") {
  @VERSION("0.3");
  @PACKAGE(name="rnl-std-core", version="0.3");
  @USE-PACKAGE(name="rnl-std-basic", version="0.3");
  @USE-PACKAGE(name="rnl-std-spice", version="0.3");
  @USE-PACKAGE(name="rnl-std-spectre", version="0.3")
}
"#;

const BASIC: &str = r#"@RNL(type="header") { @VERSION("0.3"); @PACKAGE(name="rnl-std-basic", version="0.3")}
@RNL {
  $RES = @DEFINE-SYMBOL(box=[-1, -3, 1, 3]) { (0, 3) @PIN(terminal=1); (0, -3) @PIN(terminal=2); @RECTANGLE {(-1, -2) (1, 2)} };
  $ISRC = @DEFINE-SYMBOL(box=[-2, -3, 2, 3]) { (0, 3) @PIN(terminal=1); (0, -3) @PIN(terminal=2); (0, 0) @CIRCLE(radius=2) }
}
"#;

const SPICE: &str = r#"@RNL(type="header") {
  @VERSION("0.3");
  @PACKAGE(name="rnl-std-spice", version="0.3");
  @USE-PACKAGE(name="rnl-std-basic", version="0.3")
}
@RNL {
  # @SELECT-INSTANCE(name="R*") => @SET-SYMBOL $RES;
  # @SELECT-INSTANCE(name="I*") => @SET-SYMBOL $ISRC
}
"#;

const SPECTRE: &str = r#"@RNL(type="header") {
  @VERSION("0.3");
  @PACKAGE(name="rnl-std-spectre", version="0.3");
  @USE-PACKAGE(name="rnl-std-basic", version="0.3")
}
@RNL {
  # @SELECT-INSTANCE(model="resistor") => @SET-SYMBOL $RES;
  # @SELECT-INSTANCE(model="isource") => @SET-SYMBOL $ISRC
}
"#;

/// The files, with the small standard series 0.3 in the package directory `pkgs`.
fn with_std(files: &[(&str, &str)]) -> MemoryProvider {
    let mut p = MemoryProvider::new()
        .with("pkgs/rnl-std.rnl", STD)
        .with("pkgs/rnl-std-core.rnl", CORE)
        .with("pkgs/rnl-std-basic.rnl", BASIC)
        .with("pkgs/rnl-std-spice.rnl", SPICE)
        .with("pkgs/rnl-std-spectre.rnl", SPECTRE);
    p.package_dirs.push("pkgs".into());
    for (path, text) in files {
        p.insert(path, text);
    }
    p
}

fn read(main: &str, files: &MemoryProvider, options: &Options) -> Interpretation {
    interpret_file(Path::new(main), None, files, options).unwrap()
}

fn errors(r: &Interpretation) -> Vec<String> {
    r.diagnostics().into_iter().filter(|d| d.severity == Severity::Error).map(|d| r.sink.render(d)).collect()
}

/// The rule packages in effect at the drawing level titled `title`.
fn rules(r: &Interpretation, title: &str) -> Vec<String> {
    let lv = r.model.levels.iter().find(|l| l.title == title).unwrap_or_else(|| panic!("no level {title}"));
    let names = lv.packages.iter().map(|p| p.name.clone());
    names.filter(|n| n == "rnl-std-spice" || n == "rnl-std-spectre").collect()
}

/// The symbol an instance of the level titled `title` is drawn with.
fn symbol(r: &Interpretation, title: &str, id: &str) -> String {
    let lv = r.model.levels.iter().find(|l| l.title == title).unwrap_or_else(|| panic!("no level {title}"));
    let item = lv.items.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("{id} is not drawn in {title}"));
    lv.symbols[item.symbol].name.clone()
}

const SPICE_ONLY: [&str; 1] = ["rnl-std-spice"];
const SPECTRE_ONLY: [&str; 1] = ["rnl-std-spectre"];
const BOTH: [&str; 2] = ["rnl-std-spice", "rnl-std-spectre"];

/// A SPICE netlist of `version` with a resistor R1, and X1, an instance of a subcircuit named
/// like the Spectre master `resistor`, which the rules of Spectre would draw as a resistor.
fn spice_netlist(version: &str, header: &str) -> String {
    format!(
        "* t\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\"){header}}}\n.subckt resistor p n\n.ends\n\
         R1 a 0 1k\nX1 a 0 resistor\n* @RNL {{ #R1 @PLACE(0, 0); #X1 @PLACE(10, 0) }}\n.end\n"
    )
}

/// A Spectre netlist of `version` that includes `sp.inc`, a file of SPICE, and `x.inc`, a file
/// that starts in the Spectre mode, and switches to SPICE in the middle for the subcircuit
/// `mid`. I0 is an instance of a subcircuit named as SPICE would name a current source.
fn spectre_netlist(version: &str, header: &str) -> MemoryProvider {
    let main = format!(
        "// t\nsimulator lang=spectre\n// @RNL(type=\"header\") {{ @VERSION(\"{version}\"){header}}}\n\
         subckt cell (p n)\nends cell\nI0 (a 0) cell\nR0 (a 0) resistor r=1k\n\
         include \"sp.inc\"\ninclude \"x.inc\"\n\
         simulator lang=spice\n.subckt mid a b\nI3 a b 1m\n* @RNL {{ #I3 @PLACE(0, 0) }}\n.ends\nsimulator lang=spectre\n\
         // @RNL {{ #I0 @PLACE(0, 0); #R0 @PLACE(10, 0) }}\n"
    );
    let sp = format!(
        "* sp.inc\n* @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\n.subckt spsub a b\nI1 a b 1m\nR1 a b 1k\n\
         * @RNL {{ #I1 @PLACE(0, 0); #R1 @PLACE(10, 0) }}\n.ends\n"
    );
    let x = format!(
        "simulator lang=spectre\n// @RNL(type=\"header\") {{ @VERSION(\"{version}\")}}\nsubckt xsub (p n)\n\
         I2 (p n) cell\nR2 (p n) resistor r=1k\n// @RNL {{ #I2 @PLACE(0, 0); #R2 @PLACE(10, 0) }}\nends xsub\n"
    );
    with_std(&[("n.scs", &main), ("sp.inc", &sp), ("x.inc", &x)])
}

#[test]
fn a_spice_file_drops_the_rules_of_spectre() {
    let files = with_std(&[("t.sp", &spice_netlist("0.3", ""))]);
    let r = read("t.sp", &files, &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(rules(&r, "t.sp"), SPICE_ONLY);
    assert_eq!(symbol(&r, "t.sp", "#R1"), "$RES");
    // the rules of Spectre would draw X1 by its master
    assert_ne!(symbol(&r, "t.sp", "#X1"), "$RES");
}

#[test]
fn each_file_of_a_spectre_netlist_drops_by_the_mode_of_its_first_line() {
    let r = read("n.scs", &spectre_netlist("0.3", ""), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    // the main file is a .scs file: the rules of SPICE are dropped, so I0, an instance of a
    // subcircuit, is not drawn as a current source, and R0 is drawn by its master
    assert_eq!(rules(&r, "n.scs"), SPECTRE_ONLY);
    assert_ne!(symbol(&r, "n.scs", "#I0"), "$ISRC");
    assert_eq!(symbol(&r, "n.scs", "#R0"), "$RES");
    // a file of SPICE that it includes drops the rules of Spectre, and draws its current source
    assert_eq!(rules(&r, "spsub"), SPICE_ONLY);
    assert_eq!((symbol(&r, "spsub", "#I1"), symbol(&r, "spsub", "#R1")), ("$ISRC".into(), "$RES".into()));
    // a file that starts with `simulator lang=spectre` is in the Spectre dialect
    assert_eq!(rules(&r, "xsub"), SPECTRE_ONLY);
    assert_ne!(symbol(&r, "xsub", "#I2"), "$ISRC");
    // a `simulator lang=spice` in the middle of the main file does not change its dialect: the
    // current source written there in SPICE is not drawn by the rules of SPICE
    assert_eq!(rules(&r, "mid"), SPECTRE_ONLY);
    assert_ne!(symbol(&r, "mid", "#I3"), "$ISRC");
}

#[test]
fn a_written_use_package_of_either_package_takes_effect() {
    // a Spectre file that also wants the rules of SPICE (16.3)
    let header = "; @USE-PACKAGE(name=\"rnl-std-spice\", version=\"0.3\")";
    let r = read("n.scs", &spectre_netlist("0.3", header), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(rules(&r, "n.scs"), BOTH);
    assert_eq!(symbol(&r, "n.scs", "#I0"), "$ISRC");
    // so do the files it includes, by the USE-PACKAGE of the file that includes them, and each
    // still drops the other package by its own dialect
    assert_eq!(rules(&r, "spsub"), SPICE_ONLY);
    assert_eq!(rules(&r, "xsub"), BOTH);
    assert_eq!(rules(&r, "mid"), BOTH);

    // a written package of the file's own dialect does not stop the drop of the other: an
    // empty inline package of it drops it as well (it is in effect, with nothing in it), and no
    // rules are left
    let r = read(
        "t.sp",
        &with_std(&[("t.sp", &spice_netlist("0.3", "; @USE-PACKAGE(name=\"rnl-std-spice\") {}"))]),
        &Options::default(),
    );
    assert_eq!(rules(&r, "t.sp"), SPICE_ONLY);
    assert_ne!(symbol(&r, "t.sp", "#R1"), "$RES");
    assert_ne!(symbol(&r, "t.sp", "#X1"), "$RES");

    // an inline package of the other dialect's name takes effect with its content
    let inline = "; @USE-PACKAGE(name=\"rnl-std-spectre\", version=\"0.3\") { $MINE = @DEFINE-SYMBOL { @RECTANGLE {(-1, -1) (1, 1)} }; # @SELECT-INSTANCE(model=\"resistor\") => @SET-SYMBOL $MINE }";
    let r = read("t.sp", &with_std(&[("t.sp", &spice_netlist("0.3", inline))]), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(rules(&r, "t.sp"), BOTH);
    assert_eq!(symbol(&r, "t.sp", "#X1"), "$MINE");
}

#[test]
fn an_included_file_follows_the_use_package_of_the_file_that_includes_it() {
    // a SPICE file writes the rules of Spectre, and the file of SPICE it includes keeps them;
    // the file of Spectre it includes drops the rules of SPICE, which nothing writes
    let main =
        "* t\n* @RNL(type=\"header\") { @VERSION(\"0.3\"); @USE-PACKAGE(name=\"rnl-std-spectre\", version=\"0.3\")}\n\
                .include sub.inc\n.end\n";
    let sub = "* sub\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.subckt s a b\nR1 a b 1k\n.ends\n";
    let r = read("t.sp", &with_std(&[("t.sp", main), ("sub.inc", sub)]), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert_eq!(rules(&r, "t.sp"), BOTH);
    assert_eq!(rules(&r, "s"), BOTH);
}

#[test]
fn nothing_is_dropped_without_rule_packages_to_drop() {
    // no standard package
    let files = spectre_netlist("0.3", "");
    let r = read("n.scs", &files, &Options { std: false, ..Options::default() });
    assert!(r.model.levels.iter().all(|l| l.packages.is_empty()));
    // a standard package dropped with an empty inline package of its name
    let r = read("n.scs", &spectre_netlist("0.3", "; @USE-PACKAGE(name=\"rnl-std\") {}"), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(rules(&r, "n.scs").is_empty() && rules(&r, "spsub").is_empty());
    // the toolkit's own series 0.2, which has no such packages, pinned by a file of 0.3 (16.3): its
    // rules by name still draw I0 as a current source
    let pin = "; @USE-PACKAGE(name=\"rnl-std\", version=\"0.2\")";
    let mut bundled = spectre_netlist("0.3", pin);
    bundled.package_dirs.clear();
    for p in ["rnl-std", "rnl-std-core", "rnl-std-basic", "rnl-std-spice", "rnl-std-spectre"] {
        bundled.files.remove(&format!("pkgs/{p}.rnl"));
    }
    let with = read("n.scs", &bundled, &Options::default());
    assert!(errors(&with).is_empty(), "{:?}", errors(&with));
    assert_eq!(symbol(&with, "n.scs", "#I0"), "$ISRC");
    // the same packages as in a SPICE file, each brought in by the USE-PACKAGE of its own file
    let without = |r: &Interpretation| {
        r.model.levels[0].packages.iter().map(|p| (p.name.clone(), p.version.clone(), p.at)).collect::<Vec<_>>()
    };
    let spice = read("t.sp", &MemoryProvider::new().with("t.sp", &spice_netlist("0.3", pin)), &Options::default());
    assert_eq!(without(&spice), without(&with));
    assert!(with.model.levels[0]
        .packages
        .iter()
        .all(|p| p.via.as_ref().is_some_and(|(n, l)| n == "rnl-std" && l.file == "n.scs")));
}

#[test]
fn a_file_of_0_2_uses_both_rule_packages() {
    // a file of 0.2 uses the series 0.2 (16.3), which has no rule packages
    let r = read("n.scs", &spectre_netlist("0.2", ""), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    assert!(rules(&r, "n.scs").is_empty());
    // one that pins the series 0.3 drops nothing, whatever its dialect (the series 0.3 is read with
    // the switch), and the files it includes take the version it pins
    let pin = "; @USE-PACKAGE(name=\"rnl-std\", version=\"0.3\")";
    let r = read("n.scs", &spectre_netlist("0.2", pin), &Options::default());
    assert!(errors(&r).is_empty(), "{:?}", errors(&r));
    for title in ["n.scs", "spsub", "xsub", "mid"] {
        assert_eq!(rules(&r, title), BOTH, "{title}");
    }
    assert_eq!(symbol(&r, "n.scs", "#I0"), "$ISRC");
    let r = read("t.sp", &with_std(&[("t.sp", &spice_netlist("0.2", pin))]), &Options::default());
    assert_eq!(rules(&r, "t.sp"), BOTH);
    assert_eq!(symbol(&r, "t.sp", "#X1"), "$RES");
    // it drops one when it writes an empty inline package of its name (16.2)
    let header = format!("{pin}; @USE-PACKAGE(name=\"rnl-std-spectre\") {{}}");
    let r = read("t.sp", &with_std(&[("t.sp", &spice_netlist("0.2", &header))]), &Options::default());
    assert_eq!((symbol(&r, "t.sp", "#R1"), symbol(&r, "t.sp", "#X1") == "$RES"), ("$RES".into(), false));
}
