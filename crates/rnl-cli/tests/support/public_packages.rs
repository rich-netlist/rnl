use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_lock() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root().join("packages.lock.json")).unwrap()).unwrap()
}

fn cache_dir(name: &str, version: &str) -> PathBuf {
    root().join(".rnl-packages").join(name).join(version)
}

/// The locked directory of a package: its first entry, where the lock holds several versions.
pub fn package_dir(name: &str) -> PathBuf {
    let lock = read_lock();
    let entry = lock["packages"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap();
    cache_dir(name, entry["version"].as_str().unwrap())
}

/// The directory of one locked version of a package.
pub fn version_dir(name: &str, version: &str) -> PathBuf {
    let lock = read_lock();
    assert!(
        lock["packages"].as_array().unwrap().iter().any(|p| p["name"] == name && p["version"] == version),
        "{name} {version} is not locked"
    );
    cache_dir(name, version)
}

pub fn available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let lock = read_lock();
        let missing = lock["packages"].as_array().unwrap().iter().any(|p| {
            let name = p["name"].as_str().unwrap();
            !cache_dir(name, p["version"].as_str().unwrap()).join(format!("{name}.rnl")).is_file()
        });
        let hint = "Public packages are missing; run `python scripts/packages.py download` from the checkout root.";
        if missing {
            assert!(std::env::var_os("CI").is_none(), "{hint}");
            let _ = writeln!(std::io::stderr(), "SKIP public package tests: {hint}");
            return false;
        }
        let status = std::process::Command::new("python")
            .arg(root().join("scripts/packages.py"))
            .arg("check")
            .status()
            .expect("Python is required to verify the downloaded public package hashes");
        assert!(status.success(), "Public package hash verification failed");
        true
    })
}
