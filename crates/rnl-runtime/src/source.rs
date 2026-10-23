//! Access to files without touching the file system (architecture 4.1): the embedding
//! application supplies a [`Provider`]; paths are `/`-separated strings.

use std::collections::BTreeMap;

/// An opt-in viewer library lookup. Ordinary providers do not enable automatic includes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryDefinition {
    pub name: String,
    pub path: String,
    pub file: String,
    /// User-facing source path, independent of a browser's internal pool namespace.
    pub source: String,
    pub line: u32,
}

pub trait LibraryLookup {
    fn candidates(&self, name: &str, case_sensitive: bool) -> Vec<LibraryDefinition>;
    fn definitions(&self, path: &str) -> Vec<LibraryDefinition>;
}

/// Supplies file contents and the tool's package directories (16.2).
pub trait Provider {
    /// The text of a file, or None when it does not exist or cannot be read.
    fn read(&self, path: &str) -> Option<String>;

    /// The `.rnl` files in the tool's package directories, in directory order (16.2).
    fn package_files(&self) -> Vec<String> {
        Vec::new()
    }

    /// Candidates for a named package, in search order. Implementations can prefer its filename
    /// within each directory; the runtime still checks PACKAGE and selects the version (16.2).
    fn package_candidates(&self, _name: &str) -> Vec<String> {
        self.package_files()
    }

    /// Fallback paths for a bare include filename, after the including file's directory.
    fn include_candidates(&self, _name: &str) -> Vec<String> {
        Vec::new()
    }

    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        None
    }

    /// The key a path is compared by when a frontend sets something for a file by its path, such
    /// as its format: the path itself, or as a file system that ignores case compares it, such as
    /// in lower case. Both the path a frontend gives and the one the reading resolves, from an
    /// include written in another case, go through it. A provider that wraps another gives its key.
    fn path_key(&self, path: &str) -> String {
        path.to_string()
    }
}

/// Explicit files in a viewer's temporary file pool. Their original paths stay intact, so
/// relative references in a pool file continue to resolve beside that file.
pub struct FilePool<'a> {
    pub files: &'a dyn Provider,
    pub paths: &'a [String],
}

impl Provider for FilePool<'_> {
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.files.auto_include()
    }
    fn path_key(&self, path: &str) -> String {
        self.files.path_key(path)
    }
    fn read(&self, path: &str) -> Option<String> {
        self.files.read(path)
    }
    fn package_files(&self) -> Vec<String> {
        let mut out: Vec<_> = self.paths.iter().filter(|p| p.to_ascii_lowercase().ends_with(".rnl")).cloned().collect();
        out.extend(self.files.package_files());
        out
    }
    fn package_candidates(&self, name: &str) -> Vec<String> {
        let mut out: Vec<_> = self.paths.iter().filter(|p| p.to_ascii_lowercase().ends_with(".rnl")).cloned().collect();
        prefer_filename(&mut out, name);
        out.extend(self.files.package_candidates(name));
        out
    }
    fn include_candidates(&self, name: &str) -> Vec<String> {
        let mut out: Vec<_> = self.paths.iter().filter(|p| p.rsplit('/').next() == Some(name)).cloned().collect();
        out.extend(self.files.include_candidates(name));
        out
    }
}

/// Whether a directory name follows the tool's numeric version convention.
pub fn version_directory(name: &str) -> bool {
    !name.is_empty() && name.split('.').all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

fn package_in_directory(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".rnl")
        && match path.split_once('/') {
            None => true,
            Some((version, file)) => version_directory(version) && !file.contains('/'),
        }
}

/// Prefer the conventional filename without dropping packages named differently.
pub fn prefer_filename(paths: &mut [String], name: &str) {
    let file = format!("{name}.rnl");
    paths.sort_by_key(|p| p.rsplit('/').next() != Some(file.as_str()));
}

/// Resolve a host include, keeping relative paths authoritative. A fallback is a tool setting,
/// not a change to the meaning of a path containing directory components.
pub fn resolve_include(
    provider: &dyn Provider,
    from: &str,
    target: &str,
    sink: &mut crate::diag::Sink,
    loc: crate::diag::Loc,
) -> String {
    let relative = join(&dirname(from), target);
    if target.contains(['/', '\\', ':']) || provider.read(&relative).is_some() {
        return relative;
    }
    for candidate in provider.include_candidates(target) {
        if provider.read(&candidate).is_some() {
            sink.hint(format!("include {target} resolved through the file pool or PATH to {candidate}"), Some(loc));
            return candidate;
        }
    }
    relative
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
                if f.starts_with(&prefix) && package_in_directory(&f[prefix.len()..]) {
                    out.push(f.clone());
                }
            }
        }
        out
    }

    fn package_candidates(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.package_dirs {
            let prefix = if d.is_empty() { String::new() } else { format!("{}/", normalize(d)) };
            let mut group: Vec<_> = self
                .files
                .keys()
                .filter(|f| f.starts_with(&prefix) && package_in_directory(&f[prefix.len()..]))
                .cloned()
                .collect();
            prefer_filename(&mut group, name);
            // A direct match precedes matches in version subdirectories.
            let direct = join(d, &format!("{name}.rnl"));
            group.sort_by_key(|p| p != &direct);
            out.extend(group);
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
    fn auto_include(&self) -> Option<&dyn LibraryLookup> {
        self.layers.iter().find_map(|p| p.auto_include())
    }
    /// The key of the last layer, the workspace under the others: the layers above it hold texts
    /// in memory over its files, such as edits, and the workspace's file system says how its paths
    /// compare.
    fn path_key(&self, path: &str) -> String {
        self.layers.last().map_or_else(|| path.to_string(), |p| p.path_key(path))
    }
    fn read(&self, path: &str) -> Option<String> {
        self.layers.iter().find_map(|p| p.read(path))
    }

    fn package_files(&self) -> Vec<String> {
        self.layers.iter().flat_map(|p| p.package_files()).collect()
    }

    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.layers.iter().flat_map(|p| p.package_candidates(name)).collect()
    }

    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.layers.iter().flat_map(|p| p.include_candidates(name)).collect()
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
/// `/w/sub/x.inc` in `/w`, or `x.inc` for `/x.inc` in the root `/`; None when it is not, as
/// `/../x.inc` is not in `/`, or when `dir` is "". Both are normalized paths ([`normalize`]).
pub fn relative_to<'a>(path: &'a str, dir: &str) -> Option<&'a str> {
    if dir.is_empty() {
        return None;
    }
    let rest = path.strip_prefix(dir.trim_end_matches('/'))?.strip_prefix('/')?;
    (rest != ".." && !rest.starts_with("../")).then_some(rest)
}

/// `rel` relative to the directory `base`; an absolute `rel` stands alone.
pub fn join(base: &str, rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') || (rel.len() > 1 && rel.as_bytes()[1] == b':') || base.is_empty() {
        return normalize(&rel);
    }
    normalize(&format!("{}/{}", base.trim_end_matches('/'), rel))
}

/// Remove `.` and `..` components and doubled separators. A `..` that climbs above the start of the
/// path stays, as in `../x.inc`, and in a rooted path too, as in `/../x.inc`: what is above the root
/// is for the provider to say. A file system reads `/../x.inc` as `/x.inc`; a workspace of files in
/// memory has nothing there.
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
                } else {
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
        // a `..` above the start stays, of a rooted path too
        assert_eq!(join("a", "../../c.inc"), "../c.inc");
        assert_eq!(join("/", "../c.inc"), "/../c.inc");
        assert_eq!(join("/w", "../../c.inc"), "/../c.inc");
        assert_eq!(join("/w", "../w/./c.inc"), "/w/c.inc");
        assert_eq!(normalize("/../../c.inc"), "/../../c.inc");
        assert_eq!(dirname("a/b/c.sp"), "a/b");
        assert_eq!(dirname("c.sp"), "");
        assert_eq!(dirname("/c.sp"), "/");
        assert_eq!(relative_to("/w/sub/x.inc", "/w"), Some("sub/x.inc"));
        assert_eq!(relative_to("/x.inc", "/"), Some("x.inc"));
        assert_eq!(relative_to("/sub/x.inc", &dirname("/top.sp")), Some("sub/x.inc"));
        assert_eq!(relative_to("D:/w/x.inc", &dirname("D:/w/top.sp")), Some("x.inc"));
        assert_eq!(relative_to("/wx/a.inc", "/w"), None);
        assert_eq!(relative_to("/w", "/w"), None);
        assert_eq!(relative_to("/../x.inc", "/"), None);
        assert_eq!(relative_to("/..", "/"), None);
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
