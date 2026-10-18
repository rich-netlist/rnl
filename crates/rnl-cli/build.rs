use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
}

fn track(name: &str) {
    if let Some(path) = git(&["rev-parse", "--git-path", name]) {
        println!("cargo:rerun-if-changed={path}");
    }
}

fn main() {
    let revision = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=RNL_SOLVER_REVISION={revision}");
    for name in ["HEAD", "index", "packed-refs"] {
        track(name);
    }
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        track(&branch);
    }
}
