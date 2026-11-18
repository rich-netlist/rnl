//! Search order shared by the CLI and both viewer providers.
use rnl_cli::{path_str, FsProvider};
use rnl_runtime::source::{FilePool, MemoryProvider, Provider};
use rnl_view::{build, BuildOptions};
use std::path::{Path, PathBuf};

struct Dir(PathBuf);
impl Dir {
    fn new(name: &str) -> Self {
        let dir = Self(Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("search-{}-{name}", std::process::id())));
        std::fs::create_dir_all(&dir.0).unwrap();
        dir
    }
    fn put(&self, name: &str, text: &str) -> String {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path_str(&path)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn package(name: &str, version: &str, marker: &str) -> String {
    format!("@RNL(type=\"header\") {{ @VERSION(\"0.4\"); @PACKAGE(name=\"{name}\", version=\"{version}\") }}\n@RNL {{ ${marker} = @DEFINE(roles=[\"a\"]) {{}} }}\n")
}
fn netlist(version: &str) -> String {
    format!("* top\n* @RNL(type=\"header\") {{ @VERSION(\"0.4\"); @USE-PACKAGE(name=\"rnl-std-all\", version=\"0.4\"); @USE-PACKAGE(name=\"acme\", version=\"{version}\") }}\nR1 a 0 1k\n* @RNL {{ #R1 @AS $WANTED }}\n.end\n")
}
fn ok(entry: &str, provider: &dyn Provider) -> rnl_view::Document {
    let d = build(entry, provider, &BuildOptions::default()).unwrap();
    assert!(!d.diagnostics.iter().any(|d| d.severity == "error"), "{:?}", d.diagnostics);
    d
}

#[test]
fn filename_then_version_subdirectory_then_scan_with_directory_precedence() {
    let d = Dir::new("packages");
    let entry = d.put("top.sp", &netlist("1.0"));
    let first = d.0.join("first");
    let second = d.0.join("second");
    d.put("first/aaa.rnl", &package("acme", "1.0", "WRONG"));
    let direct = d.put("first/acme.rnl", &package("acme", "1.0", "WANTED"));
    d.put("first/1.0/acme.rnl", &package("acme", "1.0", "WANTED"));
    d.put("second/acme.rnl", &package("acme", "1.0", "WRONG"));
    let fs = FsProvider::new(&[first.clone(), second.clone()]);
    assert_eq!(fs.package_candidates("acme")[0], direct);
    let catalog =
        rnl_view::catalog(&fs, &rnl_view::CatalogOptions { packages: Some(vec!["acme".into()]), ..Default::default() });
    assert_eq!(catalog.packages[0].file, "acme.rnl");
    ok(&entry, &fs);
    std::fs::remove_file(&direct).unwrap();
    ok(&entry, &fs);
    // A renamed file in the first directory still beats a conventional name in the second.
    std::fs::remove_file(first.join("1.0/acme.rnl")).unwrap();
    d.put("first/aaa.rnl", &package("acme", "1.0", "WANTED"));
    ok(&entry, &fs);
    // A matching filename is not proof of its PACKAGE; warn and continue.
    d.put("first/acme.rnl", &package("other", "1.0", "WRONG"));
    let doc = ok(&entry, &fs);
    assert!(doc.diagnostics.iter().any(|d| d.severity == "warning" && d.message.contains("header does not match")));
    // Version selection remains exact even when another filename is preferred.
    d.put("first/acme.rnl", &package("acme", "2.0", "WRONG"));
    ok(&entry, &fs);
}

#[test]
fn includes_use_relative_then_pool_then_path_and_keep_nested_paths() {
    for (kind, top, part) in [
        ("sp", "* top\n.include \"part.inc\"\n.end\n", "R1 a 0 1k\n"),
        ("scs", "simulator lang=spectre\ninclude \"part.inc\"\n", "R1 (a 0) resistor r=1k\n"),
    ] {
        let d = Dir::new(kind);
        let entry = d.put(&format!("work/top.{kind}"), top);
        let local = d.put("work/part.inc", part);
        let pool_path = d.put("pool/part.inc", &part.replace("R1", "R2"));
        d.put("path/part.inc", &part.replace("R1", "R3"));
        d.put("later/part.inc", &part.replace("R1", "R4"));
        let dirs = [d.0.join("path"), d.0.join("later")];
        let fs = FsProvider::new(&dirs).with_search_paths(&dirs);
        let paths = vec![pool_path.clone()];
        let pool = FilePool { files: &fs, paths: &paths };
        let doc = ok(&entry, &pool);
        assert!(doc.files.iter().any(|f| f.text.contains("R1")));
        assert!(!doc.diagnostics.iter().any(|d| d.message.contains("resolved through")));
        std::fs::remove_file(local).unwrap();
        let doc = ok(&entry, &pool);
        assert!(doc.files.iter().any(|f| f.text.contains("R2")));
        assert!(doc.diagnostics.iter().any(|d| d.message.contains(&pool_path)));
        let doc = ok(&entry, &fs);
        assert!(doc.files.iter().any(|f| f.text.contains("R3")));
        d.put(&format!("work/top.{kind}"), &top.replace("part.inc", "sub/part.inc"));
        let doc = build(&entry, &pool, &BuildOptions::default()).unwrap();
        assert!(doc.diagnostics.iter().any(|d| d.message.contains("cannot be found")));
    }
}

#[test]
fn memory_pool_resolves_packages_includes_and_overrides_bundled_standard() {
    let files = MemoryProvider::new()
        .with("/work/top.sp", &netlist("1.0"))
        .with("/pool/different.rnl", &package("acme", "1.0", "WANTED"))
        .with("/pool/rnl-std-all.rnl", &package("rnl-std-all", "0.4", "MY_STD"));
    let paths = vec!["/pool/different.rnl".into(), "/pool/rnl-std-all.rnl".into()];
    let pool = FilePool { files: &files, paths: &paths };
    let r = rnl_cli::interpret_file(Path::new("/work/top.sp"), None, &pool, &Default::default()).unwrap();
    let names: Vec<_> = r.model.levels[0].packages.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"acme") && names.contains(&"rnl-std-all"));
    assert!(!names.contains(&"rnl-std-basic"), "the user standard replaces the bundled collection: {names:?}");
    let excluded = build("/work/top.sp", &files, &BuildOptions::default()).unwrap();
    assert!(excluded.diagnostics.iter().any(|d| d.message.contains("acme 1.0 cannot be found")));
}

#[test]
fn cli_path_searches_includes_while_package_dir_keeps_its_previous_meaning() {
    let d = Dir::new("cli");
    let entry = d.put("work/top.sp", "* top\n.include \"part.inc\"\n.end\n");
    d.put("path/part.inc", "R1 a 0 1k\n");
    let path = path_str(&d.0.join("path"));
    for (flag, expected) in [("--path", true), ("--package-dir", false)] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_rnl"))
            .args(["check", &entry, flag, &path])
            .output()
            .unwrap();
        assert_eq!(out.status.success(), expected, "{}", String::from_utf8_lossy(&out.stdout));
    }
}
