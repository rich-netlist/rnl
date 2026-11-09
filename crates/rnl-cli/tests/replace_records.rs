//! `rnl replace-records`: the records of one level replaced in the file that holds it, in a netlist
//! that has records already (standard 2.2, 2.8, 17). Every other byte of every file stays.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rnl_cli::FsProvider;
use rnl_runtime::diag::Sink;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-replace-records").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rnl(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rnl")).args(args).output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

const MAIN: &str = "* top netlist\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.include \"amp.inc\"\n\
                    X1 a b amp\nX2 a b amp2\n* @RNL {\n*   #X1 @PLACE(0, 0);\n*   #X2 @PLACE(10, 0)\n* }\n.end\n";

/// An included file of two subcircuits, in CRLF line endings: the first has its authored record and
/// the solver's, the second has both too.
const AMP: &str = "* amp library\r\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\r\n.subckt amp in out\r\n\
    R1 in mid 1k\r\nR2 mid out 2k\r\n* @RNL {\r\n*   #R1 @PLACE(0, 0);\r\n*   #R2 @PLACE(4, 0)\r\n* }\r\n\
    * @RNL(type=\"generated\", author=\"rnl-solver 0.1\") {\r\n*   #R1 @PLACE(0, 0)\r\n* }\r\n.ends amp\r\n\
    .subckt amp2 in out\r\nR3 in out 1k\r\n* @RNL(type=\"generated\") { #R3 @PLACE(2, 2) }\r\n\
    * @RNL { #R3 @PLACE(2, 2) }\r\n.ends amp2\r\n";

/// A netlist that includes a file of the draft 0.4.
const MAIN04: &str = "* top netlist of 0.4\n.include \"v04.inc\"\nX1 a b amp04\n.end\n";

/// A file of the draft 0.4, with a subcircuit of its own.
const V04: &str = "* a file of 0.4\n* @RNL(type=\"header\") { @VERSION(\"0.4\")}\n.subckt amp04 in out\n\
                   R4 in out 1k\n* @RNL { #R4 @PLACE(0, 0) }\n.ends amp04\n";

const NEW: &str = "@RNL(author=\"analog-canvas\") {\n  #R1 @PLACE(0, 0);\n  #R2 @PLACE(6, 0)\n}\n";

/// The files of a netlist written in `dir`.
fn netlist(dir: &Path, files: &[(&str, &str)]) -> PathBuf {
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    dir.join(files[0].0)
}

fn standard(dir: &Path) -> PathBuf {
    netlist(dir, &[("main.sp", MAIN), ("amp.inc", AMP)])
}

/// The records of the level titled `title` in the netlist `entry`, as written, with the space after
/// the comment marker taken off the lines after the first.
fn records(entry: &Path, title: &str) -> Vec<String> {
    let provider = FsProvider::new(&[]);
    let doc = rnl_cli::read_host(entry, None, &provider, &mut Sink::new()).unwrap();
    let level = doc.levels.iter().find(|l| l.title() == title).unwrap();
    let plain = |t: &str| {
        t.split('\n').enumerate().map(|(k, l)| if k == 0 { l } else { &l[1..] }).collect::<Vec<_>>().join("\n")
    };
    level.records.iter().map(|r| plain(&r.text)).collect()
}

#[test]
fn the_records_of_a_subcircuit_in_an_included_file_are_replaced_and_nothing_else() {
    let dir = scratch("included");
    let entry = standard(&dir);
    let given = dir.join("amp.rnl");
    std::fs::write(&given, NEW).unwrap();
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "amp",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    let (stdout, stderr) = (text(&out.stdout), text(&out.stderr));
    assert!(out.status.success(), "{stdout}{stderr}");
    // one line, naming the level and the file
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    assert!(stdout.starts_with("replaced the records of amp in "), "{stdout}");
    assert!(stdout.trim_end().ends_with("amp.inc: 2 removed, 1 added"), "{stdout}");
    // the main file is byte for byte what it was
    assert_eq!(std::fs::read(&entry).unwrap(), MAIN.as_bytes());
    // every other line of the included file too, in its CRLF line endings; the new record is carried
    // by `*` and ends the subcircuit
    let mark = "* @RNL {\r\n*   #R1 @PLACE(0, 0);\r\n*   #R2 @PLACE(4, 0)\r\n* }\r\n\
                * @RNL(type=\"generated\", author=\"rnl-solver 0.1\") {\r\n*   #R1 @PLACE(0, 0)\r\n* }\r\n";
    let new = "* @RNL(author=\"analog-canvas\") {\r\n*   #R1 @PLACE(0, 0);\r\n*   #R2 @PLACE(6, 0)\r\n* }\r\n";
    assert!(AMP.contains(mark));
    assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.replace(mark, new).as_bytes());
    // read back: the level has the records given, the second subcircuit its own, the netlist no error
    assert_eq!(records(&entry, "amp"), [NEW.trim_end()]);
    assert_eq!(records(&entry, "amp2").len(), 2);
    let check = rnl(&["check", &entry.to_string_lossy()]);
    assert!(text(&check.stdout).contains("\n0 errors,"), "{}", text(&check.stdout));
    // the same records again: nothing changes
    let again = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "amp",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    assert!(again.status.success());
    assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.replace(mark, new).as_bytes());
    assert!(text(&again.stdout).trim_end().ends_with("amp.inc: 1 removed, 1 added"), "{}", text(&again.stdout));
}

#[test]
fn the_records_of_the_other_levels_stay_with_their_solver_records() {
    let dir = scratch("others");
    let entry = standard(&dir);
    let replace = |level: &str, records: &str| {
        let given = dir.join(format!("{level}.rnl"));
        std::fs::write(&given, records).unwrap();
        let out = rnl(&[
            "replace-records",
            &entry.to_string_lossy(),
            "--level",
            level,
            "--records",
            &given.to_string_lossy(),
            "--in-place",
        ]);
        assert!(out.status.success(), "{level}: {}", text(&out.stderr));
    };
    // the file level: its own record, not the header and not the subcircuits'
    replace("main.sp", "@RNL {\n  #X1 @PLACE(1, 1);\n  #X2 @PLACE(11, 1)\n}\n");
    let want = MAIN.replace("#X1 @PLACE(0, 0);\n*   #X2 @PLACE(10, 0)", "#X1 @PLACE(1, 1);\n*   #X2 @PLACE(11, 1)");
    assert_eq!(text(&std::fs::read(&entry).unwrap()), want);
    assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.as_bytes());
    // the second subcircuit loses its authored record and its solver's, and the first keeps both
    replace("amp2", "@RNL { #R3 @PLACE(3, 3) }\n");
    let amp = text(&std::fs::read(dir.join("amp.inc")).unwrap());
    let old = "* @RNL(type=\"generated\") { #R3 @PLACE(2, 2) }\n* @RNL { #R3 @PLACE(2, 2) }\n";
    assert_eq!(amp, text(AMP.as_bytes()).replace(old, "* @RNL { #R3 @PLACE(3, 3) }\n"));
    assert_eq!(records(&entry, "amp").len(), 2);
    assert_eq!(records(&entry, "amp2"), ["@RNL { #R3 @PLACE(3, 3) }"]);
}

#[test]
fn a_file_of_the_draft_0_4_is_replaced_in_as_well() {
    let dir = scratch("draft-0-4");
    let entry = netlist(&dir, &[("main04.sp", MAIN04), ("v04.inc", V04)]);
    let given = dir.join("new.rnl");
    std::fs::write(&given, "@RNL { #R4 @PLACE(5, 5) }\n").unwrap();
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "amp04",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(text(&std::fs::read(dir.join("v04.inc")).unwrap()), V04.replace("#R4 @PLACE(0, 0)", "#R4 @PLACE(5, 5)"));
    assert_eq!(std::fs::read(&entry).unwrap(), MAIN04.as_bytes());
    assert_eq!(records(&entry, "amp04"), ["@RNL { #R4 @PLACE(5, 5) }"]);
}

#[test]
fn a_subcircuit_in_a_region_written_to_be_encrypted_is_refused_and_nothing_is_written() {
    let dir = scratch("encrypted");
    let secret =
        "* secret library\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.prot\n.subckt hidden a b\nR1 a b 1k\n\
                  * @RNL { #R1 @PLACE(0, 0) }\n.ends hidden\n.unprot\n";
    let main = "* top\n* @RNL(type=\"header\") { @VERSION(\"0.3\")}\n.include \"secret.inc\"\nX1 a b hidden\n.end\n";
    let entry = netlist(&dir, &[("main.sp", main), ("secret.inc", secret)]);
    let given = dir.join("new.rnl");
    std::fs::write(&given, "@RNL { #R1 @PLACE(3, 3) }\n").unwrap();
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "hidden",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err = text(&out.stderr);
    assert!(
        err.starts_with(
            "error: RNL8405 writeback-replace-region-encrypted: the records are not replaced: hidden lies in a region \
             written to be encrypted, lines 3 to 8 of secret.inc"
        ),
        "{err}"
    );
    assert_eq!(std::fs::read(dir.join("secret.inc")).unwrap(), secret.as_bytes());
    assert_eq!(std::fs::read(&entry).unwrap(), main.as_bytes());
}

#[test]
fn records_that_do_not_read_are_refused_and_the_file_is_unchanged() {
    let dir = scratch("unread");
    let entry = standard(&dir);
    let given = dir.join("bad.rnl");
    let run = |new: &str| {
        std::fs::write(&given, new).unwrap();
        rnl(&[
            "replace-records",
            &entry.to_string_lossy(),
            "--level",
            "amp",
            "--records",
            &given.to_string_lossy(),
            "--in-place",
        ])
    };
    for (new, kind) in [
        // a syntax error
        ("@RNL {\n  #R1 @PLACE(0, 0;\n}\n", "RNL8410 writeback-replace-records-unread"),
        // an object the level does not have
        ("@RNL { #R9 @PLACE(0, 0) }\n", "RNL8410 writeback-replace-records-unread"),
        // no record at all
        ("amp drawn by hand\n", "RNL8410 writeback-replace-records-unread"),
    ] {
        let out = run(new);
        let err = text(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{new}: {err}");
        assert!(out.stdout.is_empty(), "{new}");
        assert!(err.starts_with(&format!("error: {kind}: ")), "{new}: {err}");
        assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.as_bytes(), "{new}");
    }
    // the reason names the line of the records given
    let err = text(&run("@RNL {\n  #R1 @PLACE(0, 0);\n  #R9 @PLACE(0, 0)\n}\n").stderr);
    assert!(err.contains("nothing is written: line 3: "), "{err}");
}

#[test]
fn a_level_that_is_not_there_is_refused_with_the_levels_there_are() {
    let dir = scratch("unknown");
    let entry = standard(&dir);
    let given = dir.join("new.rnl");
    std::fs::write(&given, NEW).unwrap();
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "nothing",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        text(&out.stderr).trim_end(),
        "error: RNL8403 writeback-replace-level-not-found: no level is titled nothing; the levels are main.sp, \
         amp.inc, amp, amp2"
    );
    assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.as_bytes());
}

#[test]
fn a_file_of_a_version_that_is_not_read_is_kept() {
    let dir = scratch("version");
    let future =
        "* a file of 9.9\n* @RNL(type=\"header\") { @VERSION(\"9.9\")}\n.subckt later a b\nR1 a b 1k\n.ends later\n";
    let main = "* top\n.include \"future.inc\"\nX1 a b later\n.end\n";
    let entry = netlist(&dir, &[("main.sp", main), ("future.inc", future)]);
    let given = dir.join("new.rnl");
    std::fs::write(&given, "@RNL { #R1 @PLACE(0, 0) }\n").unwrap();
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "later",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).starts_with("error: RNL8409 writeback-replace-version-unsupported: "),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(std::fs::read(dir.join("future.inc")).unwrap(), future.as_bytes());
}

#[test]
fn a_spectre_netlist_is_carried_by_slashes_and_a_latin_1_file_stays_latin_1() {
    let dir = scratch("spectre");
    let scs =
        "// top \u{e9}\n// @RNL(type=\"header\") { @VERSION(\"0.3\")}\nsimulator lang=spectre\nsubckt amp (in out)\n\
               r1 (in out) resistor r=1k\n// @RNL {\n//   #r1 @PLACE(0, 0)\n// }\nends amp\n";
    // Latin-1: the e acute is one byte
    let latin1: Vec<u8> = scs.chars().map(|c| u8::try_from(u32::from(c)).unwrap()).collect();
    let entry = dir.join("top.scs");
    std::fs::write(&entry, &latin1).unwrap();
    let given = dir.join("new.rnl");
    std::fs::write(&given, "@RNL(author=\"ac\") { #r1 @PLACE(2, 2) }\n").unwrap();
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "amp",
        "--records",
        &given.to_string_lossy(),
        "--in-place",
    ]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let want = scs.replace("// @RNL {\n//   #r1 @PLACE(0, 0)\n// }\n", "// @RNL(author=\"ac\") { #r1 @PLACE(2, 2) }\n");
    let want: Vec<u8> = want.chars().map(|c| u8::try_from(u32::from(c)).unwrap()).collect();
    assert_eq!(std::fs::read(&entry).unwrap(), want);
}

#[test]
fn the_file_can_be_written_under_a_directory_instead() {
    let dir = scratch("directory");
    let entry = standard(&dir);
    let given = dir.join("new.rnl");
    std::fs::write(&given, NEW).unwrap();
    let out_dir = dir.join("out");
    let out = rnl(&[
        "replace-records",
        &entry.to_string_lossy(),
        "--level",
        "amp",
        "--records",
        &given.to_string_lossy(),
        "-o",
        &out_dir.to_string_lossy(),
    ]);
    let stdout = text(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", text(&out.stderr));
    // the files read stay as they were; under the directory, the file with the level replaced and
    // the other files as they are
    assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.as_bytes());
    let written = text(&std::fs::read(out_dir.join("amp.inc")).unwrap());
    assert!(written.contains("author=\"analog-canvas\"") && !written.contains("rnl-solver 0.1"), "{written}");
    assert_eq!(std::fs::read(out_dir.join("main.sp")).unwrap(), MAIN.as_bytes());
    assert!(stdout.lines().next().unwrap().contains("; wrote "), "{stdout}");
}

#[test]
fn the_arguments_are_checked() {
    let dir = scratch("arguments");
    let entry = standard(&dir);
    let given = dir.join("new.rnl");
    std::fs::write(&given, NEW).unwrap();
    let (entry, given) = (entry.to_string_lossy().into_owned(), given.to_string_lossy().into_owned());
    let message = |args: &[&str]| {
        let out = rnl(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        text(&out.stderr).lines().next().unwrap().to_string()
    };
    fn with<'a>(entry: &'a str, more: &[&'a str]) -> Vec<&'a str> {
        let mut args = vec!["replace-records", entry];
        args.extend_from_slice(more);
        args
    }
    assert!(message(&with(&entry, &["--records", &given, "--in-place"]))
        .starts_with("error: RNL8036 cli-command-needs-option: replace-records needs --level TITLE"));
    assert!(message(&with(&entry, &["--level", "amp", "--in-place"]))
        .starts_with("error: RNL8036 cli-command-needs-option: replace-records needs --records FILE"));
    assert!(message(&with(&entry, &["--level", "amp", "--records", &given]))
        .starts_with("error: RNL8038 cli-replace-target-missing: replace-records needs -o DIR"));
    assert!(message(&with(&entry, &["--level", "amp", "--records", &given, "--in-place", "-o", "out"]))
        .starts_with("error: RNL8037 cli-replace-target-both: "));
    assert!(message(&with(&entry, &["--level", "amp", "--records", "no-such-file.rnl", "--in-place"]))
        .starts_with("error: RNL8000 cli-cannot-read: cannot read no-such-file.rnl"));
    assert_eq!(
        message(&["check", &entry, "--records", &given]),
        "error: RNL8011 cli-option-wrong-command: --records is an option of replace-records"
    );
    assert_eq!(message(&with(&entry, &["--records"])), "error: RNL8010 cli-option-needs-value: --records needs a file");
    // nothing was written
    assert_eq!(std::fs::read(dir.join("amp.inc")).unwrap(), AMP.as_bytes());
    let help = rnl(&["--help"]);
    assert!(text(&help.stdout).contains("rnl replace-records FILE --level TITLE --records RECORDS"));
}
