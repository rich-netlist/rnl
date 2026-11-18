//! What a writeback may and may not change: a record read twice is removed once, an output never
//! takes the place of another input, a rewritten file is replaced whole, and a level whose result
//! does not read back is not written.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-writeback-safety").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn rnl(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rnl")).current_dir(dir).args(args).output().unwrap()
}

#[test]
fn a_record_read_through_a_section_selected_twice_is_removed_once() {
    let dir = scratch("section-twice");
    let source = "* a section selected twice\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\n.lib self.sp cell\n.lib self.sp cell\n.lib cell\n* @RNL(type=\"generated\") {}\n.model keep_me D\n.endl cell\n.end\n";
    std::fs::write(dir.join("self.sp"), source).unwrap();
    let output = rnl(&dir, &["strip-generated", "self.sp", "-o", "out.sp"]);
    assert!(output.status.success(), "{output:?}");
    let stripped = std::fs::read_to_string(dir.join("out.sp")).unwrap();
    assert_eq!(stripped, source.replace("* @RNL(type=\"generated\") {}\n", ""));
}

const TWO_RESISTORS: &str =
    "* two resistors\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a 0 1k\nR2 a 0 2k\n.end\n";

#[test]
fn an_output_directory_that_holds_another_input_is_refused_before_writing() {
    // the entry and an include of the same name in `lib/`: `-o lib` would write the entry over it
    let dir = scratch("output-over-input");
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    let library = "* a model library\n.model nch nmos level=1\n";
    std::fs::write(dir.join("lib/top.sp"), library).unwrap();
    std::fs::write(dir.join("top.sp"), TWO_RESISTORS.replace("R1 a 0 1k", ".include \"lib/top.sp\"\nR1 a 0 1k"))
        .unwrap();
    let output = rnl(&dir, &["solve", "top.sp", "--no-std", "-o", "lib", "--solver-config", "limit=\"work\",work=0"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("another file the document reads"), "{output:?}");
    assert_eq!(std::fs::read_to_string(dir.join("lib/top.sp")).unwrap(), library);
    assert!(!dir.join("lib/lib").exists());
}

#[test]
fn a_file_solved_in_place_is_replaced_whole_and_leaves_nothing_beside_it() {
    let dir = scratch("in-place");
    std::fs::write(dir.join("top.sp"), TWO_RESISTORS).unwrap();
    let output = rnl(&dir, &["solve", "top.sp", "--no-std", "--in-place", "--solver-config", "limit=\"work\",work=0"]);
    assert!(output.status.success(), "{output:?}");
    assert!(std::fs::read_to_string(dir.join("top.sp")).unwrap().contains("type=\"generated\""));
    let names: Vec<String> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["top.sp"]);
    // and into a directory of its own, over the copy it wrote before
    for _ in 0..2 {
        let output =
            rnl(&dir, &["solve", "top.sp", "--no-std", "-o", "out", "--solver-config", "limit=\"work\",work=0"]);
        assert!(output.status.success(), "{output:?}");
    }
    let names: Vec<String> = std::fs::read_dir(dir.join("out"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["top.sp"]);
}

#[test]
fn a_level_without_dots_whose_result_loses_its_texts_is_not_written() {
    // the notes go on the pins no wire reaches; the route the solve writes reaches them, so the
    // document read with the record no longer makes the notes
    let dir = scratch("lost-texts");
    let source = "* notes on pins\n* @RNL(type=\"header\") { @VERSION(\"0.4\") }\nR1 a 0 1k\nR2 a 0 2k\n* @RNL {\n* $S = @DEFINE-SYMBOL { (-1,0) @PIN(terminal=1); (1,0) @PIN(terminal=2); (-0.5,-0.5) (0.5,0.5) @RECTANGLE };\n* {#R1 #R2} @SET-SYMBOL $S => @ORIENT(\"R0\"); #R1 @PLACE(0,0); #R2 @PLACE(5,0);\n* $D = @DEFINE(roles=[[\"p\",\"pin\"]]) { ?p::net @ROUTE-NET };\n* # @SELECT-PIN(wired=false) => @AS $D;\n* #R1 @SELECT-PIN(wired=false) => @TEXT(\"NOTE\", key=\"k\")\n* }\n.end\n";
    std::fs::write(dir.join("notes.sp"), source).unwrap();
    let output =
        rnl(&dir, &["solve", "notes.sp", "--no-std", "-o", "out", "--solver-config", "limit=\"work\",work=50000"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not read back as it was solved"), "{output:?}");
    assert!(!dir.join("out/notes.sp").exists());
}
