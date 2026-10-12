//! The pieces of the `rnl` command: reading files, the bundled packages, and JSON output.
//! The conformance tests use them too.

pub mod dump;
pub mod provider;

use std::path::Path;

use rnl_runtime::diag::Sink;
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::Provider;
use rnl_runtime::{Interpretation, Options};

pub use provider::{path_str, FsProvider};

/// Is the main file Spectre? By `dialect`, by the `.scs` extension, or a `simulator lang=spectre`
/// near the top.
pub fn is_spectre(path: &Path, text: &str, dialect: Option<&str>) -> bool {
    if let Some(d) = dialect {
        return d == "spectre";
    }
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("scs")) {
        return true;
    }
    text.lines().take(50).any(|l| {
        let l = l.trim().to_lowercase();
        l.starts_with("simulator") && l.contains("lang") && l.contains("spectre")
    })
}

/// Read a host document with the adapter its dialect needs.
pub fn read_host(file: &Path, dialect: Option<&str>, provider: &dyn Provider, sink: &mut Sink) -> Result<HostDocument, String> {
    let path = path_str(file);
    let text = provider.read(&path).ok_or_else(|| format!("cannot read {}", file.display()))?;
    let doc = if is_spectre(file, &text, dialect) {
        rnl_host_spectre::read(&path, provider, sink)
    } else {
        rnl_host_spice::read(&path, provider, sink)
    };
    doc.ok_or_else(|| format!("cannot read {}", file.display()))
}

/// Read and interpret a file: a netlist, or a package file (`.rnl`) checked on its own.
pub fn interpret_file(file: &Path, dialect: Option<&str>, provider: &dyn Provider, options: &Options) -> Result<Interpretation, String> {
    if file.extension().is_some_and(|e| e.eq_ignore_ascii_case("rnl")) {
        return Ok(rnl_runtime::check_package(&path_str(file), provider, options));
    }
    let mut sink = Sink::new();
    let doc = read_host(file, dialect, provider, &mut sink)?;
    Ok(rnl_runtime::interpret(doc, provider, options, sink))
}
