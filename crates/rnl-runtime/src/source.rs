//! Access to files without touching the file system (architecture 4.1): the embedding
//! application supplies a [`Provider`]; paths are `/`-separated strings.

use std::collections::BTreeMap;

/// Supplies file contents and the tool's package directories (16.2).
pub trait Provider {
    /// The text of a file, or None when it does not exist or cannot be read.
    fn read(&self, path: &str) -> Option<String>;

    /// The `.rnl` files in the tool's package directories, in directory order (16.2).
    fn package_files(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Files held in memory: for tests, WASM and bundled packages.
#[derive(Default, Clone, Debug)]
pub struct MemoryProvider {
    pub files: BTreeMap<String, String>,
    pub package_dirs: Vec<String>,
}

impl MemoryProvider {
    pub fn new() -> MemoryProvider {
        MemoryProvider::default()
    }

    pub fn with(mut self, path: &str, text: &str) -> MemoryProvider {
        self.files.insert(normalize(path), text.to_string());
        self
    }

    pub fn insert(&mut self, path: &str, text: &str) {
        self.files.insert(normalize(path), text.to_string());
    }
}

impl Provider for MemoryProvider {
    fn read(&self, path: &str) -> Option<String> {
        self.files.get(&normalize(path)).cloned()
    }

    fn package_files(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.package_dirs {
            let prefix = if d.is_empty() { String::new() } else { format!("{}/", normalize(d)) };
            for f in self.files.keys() {
                if f.starts_with(&prefix) && f.ends_with(".rnl") && !f[prefix.len()..].contains('/') {
                    out.push(f.clone());
                }
            }
        }
        out
    }
}

/// A provider that tries several in order: the first that has the file wins; package files are
/// the concatenation of all of them.
pub struct Layered<'a> {
    pub layers: Vec<&'a dyn Provider>,
}

impl Provider for Layered<'_> {
    fn read(&self, path: &str) -> Option<String> {
        self.layers.iter().find_map(|p| p.read(path))
    }

    fn package_files(&self) -> Vec<String> {
        self.layers.iter().flat_map(|p| p.package_files()).collect()
    }
}

/// The directory part of a path, without the trailing `/`; "" for a bare file name.
pub fn dirname(path: &str) -> String {
    let p = path.replace('\\', "/");
    match p.rfind('/') {
        Some(0) => "/".to_string(),
        Some(k) => p[..k].to_string(),
        None => String::new(),
    }
}

/// `path` relative to the directory `dir` when it is in it or below, such as `sub/x.inc` for
/// `/w/sub/x.inc` in `/w`, or `x.inc` for `/x.inc` in the root `/`; None when it is not, or `dir`
/// is "".
pub fn relative_to<'a>(path: &'a str, dir: &str) -> Option<&'a str> {
    if dir.is_empty() {
        return None;
    }
    path.strip_prefix(dir.trim_end_matches('/'))?.strip_prefix('/')
}

/// `rel` relative to the directory `base`; an absolute `rel` stands alone.
pub fn join(base: &str, rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') || (rel.len() > 1 && rel.as_bytes()[1] == b':') || base.is_empty() {
        return normalize(&rel);
    }
    normalize(&format!("{}/{}", base.trim_end_matches('/'), rel))
}

/// Remove `.` and `..` components and doubled separators.
pub fn normalize(path: &str) -> String {
    let p = path.replace('\\', "/");
    let absolute = p.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for c in p.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|l| *l != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            _ => parts.push(c),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(join("a/b", "../c.inc"), "a/c.inc");
        assert_eq!(join("", "x/y.sp"), "x/y.sp");
        assert_eq!(join("/p/q", "./r"), "/p/q/r");
        assert_eq!(join("D:/w", "lib/x.rnl"), "D:/w/lib/x.rnl");
        assert_eq!(dirname("a/b/c.sp"), "a/b");
        assert_eq!(dirname("c.sp"), "");
        assert_eq!(dirname("/c.sp"), "/");
        assert_eq!(relative_to("/w/sub/x.inc", "/w"), Some("sub/x.inc"));
        assert_eq!(relative_to("/x.inc", "/"), Some("x.inc"));
        assert_eq!(relative_to("/sub/x.inc", &dirname("/top.sp")), Some("sub/x.inc"));
        assert_eq!(relative_to("D:/w/x.inc", &dirname("D:/w/top.sp")), Some("x.inc"));
        assert_eq!(relative_to("/wx/a.inc", "/w"), None);
        assert_eq!(relative_to("/w", "/w"), None);
        assert_eq!(relative_to("x.inc", ""), None);
    }

    #[test]
    fn memory_packages() {
        let p = MemoryProvider { package_dirs: vec!["pk".into()], ..Default::default() }
            .with("pk/a.rnl", "x")
            .with("pk/sub/b.rnl", "y")
            .with("other.rnl", "z");
        assert_eq!(p.package_files(), vec!["pk/a.rnl".to_string()]);
    }
}
