//! The pieces of the `rnl` command: reading files and JSON output. What reads no files, such as
//! dialect detection, the bundled packages and the level status, comes from `rnl-view`. The
//! conformance tests use them too.

pub mod dump;
pub mod provider;

use std::path::Path;

use rnl_runtime::diag::Sink;
use rnl_runtime::host::HostDocument;
use rnl_runtime::source::Provider;
use rnl_runtime::{Interpretation, Options};

pub use provider::{path_str, FsProvider};
pub use rnl_view::{bundled, is_spectre, level_status, status_lines};

/// Read a host document with the adapter its dialect needs, with the bundled packages under
/// `provider`.
pub fn read_host(
    file: &Path,
    dialect: Option<&str>,
    provider: &dyn Provider,
    sink: &mut Sink,
) -> Result<HostDocument, String> {
    rnl_view::read_host(&file.to_string_lossy(), dialect, provider, sink)
}

/// Read and interpret a file: a netlist, or a package file (`.rnl`) checked on its own, with the
/// bundled packages under `provider`.
pub fn interpret_file(
    file: &Path,
    dialect: Option<&str>,
    provider: &dyn Provider,
    options: &Options,
) -> Result<Interpretation, String> {
    rnl_view::interpret(&file.to_string_lossy(), dialect, provider, options)
}
