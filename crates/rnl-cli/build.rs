use std::path::PathBuf;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let commit = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=RNL_BUILD_COMMIT={commit}");
    // Worktrees keep HEAD and branch refs outside this checkout. Rebuild the metadata after a commit.
    let mut paths = vec!["HEAD".to_owned(), "packed-refs".to_owned()];
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        paths.push(branch);
    }
    for path in paths {
        if let Some(path) = git(&["rev-parse", "--git-path", &path]) {
            let path = PathBuf::from(path);
            let path = if path.is_absolute() { path } else { std::env::current_dir().unwrap().join(path) };
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
