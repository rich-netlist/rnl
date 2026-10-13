//! What every frontend of decision 0006 shares that reads no files: dialect detection, reading
//! and interpreting a document through the frontend's own file access with the standard packages
//! bundled here, and the status of a drawing level.
//!
//! The crate does no file I/O, so it builds for `wasm32-unknown-unknown`.

use rnl_runtime::diag::Sink;
use rnl_runtime::host::{file_name, HostDocument};
use rnl_runtime::model::LevelView;
use rnl_runtime::source::{MemoryProvider, Provider};
use rnl_runtime::{Interpretation, Options};

/// The standard packages bundled with the core (16.2, 16.3).
const BUNDLED: [(&str, &str); 2] = [
    ("rnl-std.rnl", include_str!("../../../libraries/standard/rnl-std.rnl")),
    ("rnl-zh.rnl", include_str!("../../../libraries/standard/rnl-zh.rnl")),
];

/// The directory of the bundled packages, as the runtime names it.
pub const BUNDLED_DIR: &str = "<bundled>";

/// The bundled standard packages, as a package directory of their own (16.2).
pub fn bundled() -> MemoryProvider {
    let mut p = MemoryProvider { package_dirs: vec![BUNDLED_DIR.into()], ..MemoryProvider::default() };
    for (name, text) in BUNDLED {
        p.insert(&format!("{BUNDLED_DIR}/{name}"), text);
    }
    p
}

/// Is the entry Spectre? By `dialect`, by the `.scs` extension, or a `simulator lang=spectre`
/// near the top.
pub fn is_spectre(entry: &str, text: &str, dialect: Option<&str>) -> bool {
    if let Some(d) = dialect {
        return d == "spectre";
    }
    if extension(entry).is_some_and(|e| e.eq_ignore_ascii_case("scs")) {
        return true;
    }
    text.lines().take(50).any(|l| {
        let l = l.trim().to_lowercase();
        l.starts_with("simulator") && l.contains("lang") && l.contains("spectre")
    })
}

/// Read a host document with the adapter its dialect needs.
pub fn read_host(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    sink: &mut Sink,
) -> Result<HostDocument, String> {
    let path = slashed(entry);
    let text = files.read(&path).ok_or_else(|| format!("cannot read {entry}"))?;
    let doc = if is_spectre(&path, &text, dialect) {
        rnl_host_spectre::read(&path, files, sink)
    } else {
        rnl_host_spice::read(&path, files, sink)
    };
    doc.ok_or_else(|| format!("cannot read {entry}"))
}

/// Interpret an entry: a netlist, or a package file (`.rnl`) checked on its own. The bundled
/// packages are layered under `files`.
pub fn interpret(
    entry: &str,
    dialect: Option<&str>,
    files: &dyn Provider,
    options: &Options,
) -> Result<Interpretation, String> {
    interpret_in(entry, dialect, &Workspace::new(files), options)
}

/// [`interpret`] in a workspace, which has the bundled packages already.
fn interpret_in(
    entry: &str,
    dialect: Option<&str>,
    ws: &Workspace,
    options: &Options,
) -> Result<Interpretation, String> {
    if extension(entry).is_some_and(|e| e.eq_ignore_ascii_case("rnl")) {
        return Ok(rnl_runtime::check_package(&slashed(entry), ws, options));
    }
    let mut sink = Sink::new();
    let doc = read_host(entry, dialect, ws, &mut sink)?;
    Ok(rnl_runtime::interpret(doc, ws, options, sink))
}

/// What a drawing level still lacks without a solver: objects without a position, and the
/// constraints by the status their check against the written coordinates gave. None when
/// everything is placed and every constraint holds.
pub fn level_status(lv: &LevelView) -> Option<String> {
    let mut parts = Vec::new();
    if !lv.unplaced.is_empty() {
        parts.push(format!("{} without a position", lv.unplaced.len()));
    }
    let mut by_status: Vec<(&str, usize)> = Vec::new();
    for c in &lv.constraints {
        match by_status.iter_mut().find(|(s, _)| *s == c.status) {
            Some((_, n)) => *n += 1,
            None => by_status.push((&c.status, 1)),
        }
    }
    if by_status.iter().any(|(s, _)| *s != "ok") {
        let list: Vec<String> = by_status.iter().map(|(s, n)| format!("{n} {s}")).collect();
        parts.push(format!("constraints: {}", list.join(", ")));
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

/// The status of every level that lacks something, one line each, as `rnl check` prints them.
pub fn status_lines(r: &Interpretation) -> Vec<String> {
    r.model.levels.iter().filter_map(|lv| level_status(lv).map(|s| format!("status {}: {s}", lv.title))).collect()
}

/// The frontend's files with the bundled packages under them.
struct Workspace<'a> {
    files: &'a dyn Provider,
    bundled: MemoryProvider,
}

impl Workspace<'_> {
    fn new(files: &dyn Provider) -> Workspace<'_> {
        Workspace { files, bundled: bundled() }
    }
}

impl Provider for Workspace<'_> {
    fn read(&self, path: &str) -> Option<String> {
        if path.starts_with(BUNDLED_DIR) {
            return self.bundled.read(path);
        }
        self.files.read(path)
    }

    /// The frontend's package directories, then the bundled packages (16.2).
    fn package_files(&self) -> Vec<String> {
        let mut out = self.files.package_files();
        out.extend(self.bundled.package_files());
        out
    }
}

/// A path with `/` for every `\`, as the runtime reads paths.
fn slashed(path: &str) -> String {
    path.replace('\\', "/")
}

/// The extension of a path's file name, as `std::path::Path::extension` gives it.
fn extension(path: &str) -> Option<&str> {
    let name = file_name(path);
    match name.rfind('.') {
        Some(k) if k > 0 => Some(&name[k + 1..]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::diag::Severity;

    const NETLIST: &str = "* untitled
R1 a 0 1k
C1 a 0 1p
* @RNL(type=\"header\") { @VERSION(\"0.2\")}
* @RNL {
*   #R1 @PLACE(0, 0);
*   #C1 @PLACE(4, 0)
* }
.end
";

    #[test]
    fn extensions_are_those_of_the_file_name() {
        assert_eq!(extension("a/b.SCS"), Some("SCS"));
        assert_eq!(extension("a.d/b"), None);
        assert_eq!(extension("a/.rnl"), None);
        assert_eq!(extension(r"x\y.rnl"), Some("rnl"));
    }

    #[test]
    fn spectre_by_option_extension_or_simulator_statement() {
        assert!(is_spectre("a.SCS", "", None));
        assert!(!is_spectre("a.scs", "", Some("spice")));
        assert!(is_spectre("a.sp", "", Some("spectre")));
        assert!(is_spectre("a.net", "// t\nsimulator lang=spectre\n", None));
        assert!(!is_spectre("dir.scs/a.sp", "* t\n", None));
    }

    #[test]
    fn an_untitled_text_is_interpreted_with_the_bundled_packages() {
        let files = MemoryProvider::new().with("untitled-1", NETLIST);
        let r = interpret("untitled-1", None, &files, &Options::default()).unwrap();
        assert!(r.diagnostics().iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics());
        assert_eq!(r.model.levels[0].items.iter().filter(|i| !i.host.is_empty()).count(), 2);
        let path = format!("{BUNDLED_DIR}/rnl-std.rnl");
        let package = interpret(&path, None, &MemoryProvider::new(), &Options::default()).unwrap();
        assert!(package.diagnostics().is_empty());
    }
}
