//! §3.1: native output preserves its input encoding unless conversion was explicitly chosen.
use std::path::{Path, PathBuf};
use std::process::Command;

use rnl_cli::codec::{decode, encode, OutputEncoding};
use rnl_cli::preview::{save_copy, save_copy_with_encoding, Copy};
use rnl_cli::solve::{write, write_with_encoding, SolveRun, SolvedFile, Target};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rnl-encoding").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn file(path: &Path, text: &str) -> SolvedFile {
    SolvedFile { path: rnl_cli::path_str(path), text: text.into() }
}

fn run(files: Vec<SolvedFile>) -> SolveRun {
    SolveRun {
        entry: files[0].path.clone(),
        read: files.iter().map(|f| f.path.clone()).collect(),
        files,
        levels: Vec::new(),
        not_solved: Vec::new(),
        errors: 0,
        raised: Vec::new(),
    }
}

#[test]
fn encoding_failure_leaves_every_output_untouched_then_explicit_utf8_succeeds() {
    let dir = scratch("batch");
    let first = dir.join("top.sp");
    let second = dir.join("lib.inc");
    std::fs::write(&first, b"* original\n").unwrap();
    std::fs::write(&second, b"* caf\xe9\r\n").unwrap();
    let run = run(vec![file(&first, "* changed\n"), file(&second, "* café Ω 中\r\n")]);
    let result = write(&run, &Target::InPlace);
    assert!(result.written.is_empty());
    assert!(result.error.unwrap().contains("Latin-1"));
    assert_eq!(std::fs::read(&first).unwrap(), b"* original\n");
    assert_eq!(std::fs::read(&second).unwrap(), b"* caf\xe9\r\n");
    let result = write_with_encoding(&run, &Target::InPlace, OutputEncoding::Utf8);
    assert_eq!(result.error, None);
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "* café Ω 中\r\n");
}

#[test]
fn single_and_folder_copies_refuse_conversion_by_default() {
    for many in [false, true] {
        let dir = scratch(if many { "copy-folder" } else { "copy-file" });
        let original = dir.join("top.sp");
        std::fs::write(&original, b"* caf\xe9\n").unwrap();
        let mut run = run(vec![file(&original, "* café Ω\n")]);
        let out = dir.join(if many { "out" } else { "out.sp" });
        if many {
            let include = dir.join("lib.inc");
            std::fs::write(&include, b"* \xb5 unchanged\n").unwrap();
            run.read.push(rnl_cli::path_str(&include));
        } else {
            std::fs::write(&out, b"destination stays intact").unwrap();
        }
        let copy = Copy::of(&run);
        assert!(save_copy(&copy, &out).unwrap_err().contains("Latin-1"));
        if many {
            assert!(!out.exists());
        } else {
            assert_eq!(std::fs::read(&out).unwrap(), b"destination stays intact");
        }
        save_copy_with_encoding(&copy, &out, OutputEncoding::Utf8).unwrap();
        let saved = if many { out.join("top.sp") } else { out.clone() };
        assert_eq!(std::fs::read_to_string(saved).unwrap(), "* café Ω\n");
        if many {
            assert_eq!(std::fs::read(out.join("lib.inc")).unwrap(), b"* \xb5 unchanged\n");
        }
        assert_eq!(std::fs::read(&original).unwrap(), b"* caf\xe9\n");
    }
}

#[test]
fn strip_generated_preserves_source_encoding_bom_and_line_endings() {
    let dir = scratch("strip");
    let input = dir.join("top.sp");
    let output = dir.join("stripped.sp");
    for original in [
        b"* caf\xe9\n".as_slice(),
        "* café\n".as_bytes(),
        b"\xef\xbb\xbf* caf\xc3\xa9\n",
        b"\xef\xbb\xbf\xef\xbb\xbf* caf\xc3\xa9\n",
    ] {
        for ending in ["\n", "\r\n"] {
            for final_newline in [true, false] {
                let title = decode(original).trim_end().to_owned();
                let retained = format!("{title}{ending}* @RNL(type=\"header\") {{ @VERSION(\"0.3\") }}{ending}");
                let tail = format!(".end{}", if final_newline { ending } else { "" });
                let text = format!("{retained}* @RNL(type=\"generated\", author=\"test\") {{ }}{ending}{tail}");
                let expected = format!("{retained}{tail}");
                std::fs::write(&input, encode(Some(original), &text, OutputEncoding::Preserve).unwrap()).unwrap();
                let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
                    .args(["strip-generated", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
                    .output()
                    .unwrap();
                assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
                let preserved = std::fs::read(&output).unwrap();
                assert_eq!(preserved, encode(Some(original), &expected, OutputEncoding::Preserve).unwrap());
                assert_eq!(decode(&preserved), expected);
                let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
                    .args([
                        "strip-generated",
                        input.to_str().unwrap(),
                        "-o",
                        output.to_str().unwrap(),
                        "--encoding",
                        "utf8",
                    ])
                    .output()
                    .unwrap();
                assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
                let converted = std::fs::read(&output).unwrap();
                assert_eq!(decode(&converted), expected);
                let expected_bytes = if expected.starts_with('\u{feff}') {
                    [b"\xef\xbb\xbf".as_slice(), expected.as_bytes()].concat()
                } else {
                    expected.as_bytes().to_vec()
                };
                assert_eq!(converted, expected_bytes);
            }
        }
    }
}

#[test]
fn solve_cli_accepts_explicit_utf8_and_preserves_latin1_by_default() {
    let dir = scratch("solve-cli");
    let input = dir.join("top.sp");
    std::fs::write(&input, b"* caf\xe9\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\nR1 a 0 1k\n.end\n").unwrap();
    for (name, args) in
        [("preserve", vec![]), ("utf8", vec!["--encoding", "utf8"]), ("equals", vec!["--encoding=utf8"])]
    {
        let out = dir.join(name);
        let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
            .args(["solve", input.to_str().unwrap(), "-o", out.to_str().unwrap()])
            .args(args)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let bytes = std::fs::read(out.join("top.sp")).unwrap();
        assert!(decode(&bytes).contains("café"));
        assert_eq!(std::str::from_utf8(&bytes).is_ok(), name != "preserve");
    }
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .args(["solve", input.to_str().unwrap(), "--encoding", "latin1"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown encoding"));
}

#[test]
fn missing_disk_sources_do_not_silently_become_utf8() {
    let dir = scratch("missing-source");
    let source = dir.join("missing.sp");
    let output = dir.join("out.sp");
    let run = run(vec![file(&source, "* café Ω\n")]);
    std::fs::write(&output, b"unchanged").unwrap();
    let error = save_copy(&Copy::of(&run), &output).unwrap_err();
    assert!(error.contains("preserve its encoding"), "{error}");
    assert_eq!(std::fs::read(&output).unwrap(), b"unchanged");
    let result = write(&run, &Target::InPlace);
    assert!(result.error.unwrap().contains("preserve its encoding"));
    assert!(!source.exists());
    save_copy_with_encoding(&Copy::of(&run), &output, OutputEncoding::Utf8).unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), "* café Ω\n".as_bytes());
}

#[test]
fn strip_generated_uses_actual_disk_or_bundled_origin_not_the_display_name() {
    use rnl_runtime::source::Provider;

    let dir = scratch("strip-bundled");
    let output = dir.join("stripped.rnl");
    let entry = "<bundled>/0.3/rnl-std.rnl";
    let expected = rnl_cli::bundled().read(entry).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .current_dir(&dir)
        .args(["strip-generated", entry, "-o", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(std::fs::read(&output).unwrap(), expected.as_bytes());

    // A real file with the same displayed basename is still a Latin-1 disk source.
    // This .rnl name intentionally holds host text to simulate a disk shadow. strip-generated
    // uses host carriers; a package itself cannot contain generated records. The ordinary
    // comment prefix must survive removal. Exercise this on every platform, not only Unix.
    let physical = dir.join("rnl-std.rnl");
    let disk = b"* caf\xe9\r\n* @RNL(type=\"header\") { @VERSION(\"0.3\") }\r\n* retained note\r\n";
    let with_generated = [disk.strip_suffix(b"\r\n").unwrap(), b" @RNL(type=\"generated\") { }\r\n"].concat();
    std::fs::write(&physical, &with_generated).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .args(["strip-generated", physical.to_str().unwrap(), "-o", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(std::fs::read(&output).unwrap(), disk);
    let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
        .args(["strip-generated", physical.to_str().unwrap(), "-o", output.to_str().unwrap(), "--encoding", "utf8"])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(std::fs::read(&output).unwrap(), decode(disk).as_bytes());
    assert_eq!(std::fs::read(&physical).unwrap(), with_generated);

    // On platforms where angle brackets are legal in paths, disk also wins at the exact
    // virtual-package name. Windows cannot create this filename.
    #[cfg(not(windows))]
    {
        let shadow = dir.join(entry);
        std::fs::create_dir_all(shadow.parent().unwrap()).unwrap();
        std::fs::write(&shadow, &with_generated).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
            .current_dir(&dir)
            .args(["strip-generated", entry, "-o", output.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(std::fs::read(&output).unwrap(), disk);
        let result = Command::new(env!("CARGO_BIN_EXE_rnl"))
            .current_dir(&dir)
            .args(["strip-generated", entry, "-o", output.to_str().unwrap(), "--encoding", "utf8"])
            .output()
            .unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
        assert_eq!(std::fs::read(&output).unwrap(), decode(disk).as_bytes());
        assert_eq!(std::fs::read(&shadow).unwrap(), with_generated);
    }
}
