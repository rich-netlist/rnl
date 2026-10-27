//! The formats a netlist's files can be read with, and the choice of each file's.
//!
//! A frontend lists the formats ([`formats`]) and may set one for a file by its path
//! ([`crate::BuildOptions::formats`]). A file whose format is not set is read by [`detect_format`],
//! the main file by the dialect a frontend gives before it.

use std::collections::BTreeMap;

use rnl_runtime::host::format::{Format, Formats};
use rnl_runtime::source::{normalize, Provider};
use serde::Serialize;

/// The formats, in the order a frontend lists them: SPICE, the format of a file no other claims,
/// first, and text, that of a file that is not a netlist, last.
pub(crate) static FORMATS: [&dyn Format; 3] =
    [&rnl_host_spice::Spice, &rnl_host_spectre::Spectre, &rnl_runtime::host::text::Text];

/// A format a file can be read with, as `formats` lists them.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(ts_rs::TS))]
pub struct FormatInfo {
    /// Its name, as settings and requests give it, such as "spice".
    pub name: String,
    /// Its name as shown, such as "SPICE".
    pub title: String,
    /// The suffixes of the files it reads when none is set, lower case and without the dot.
    pub extensions: Vec<String>,
    /// The format of a file that no other format claims: exactly one has it.
    pub fallback: bool,
}

/// The formats a file can be read with.
pub fn formats() -> Vec<FormatInfo> {
    FORMATS
        .iter()
        .map(|f| FormatInfo {
            name: f.name().to_string(),
            title: f.title().to_string(),
            extensions: f.extensions().iter().map(|e| e.to_string()).collect(),
            fallback: f.name() == "spice",
        })
        .collect()
}

/// The index of the format named `name`.
fn index(name: &str) -> Option<usize> {
    FORMATS.iter().position(|f| f.name() == name)
}

/// The format a file is read with when none is set for it, by its path and its text: the first
/// that claims it, else SPICE. A `.scs` file, or one with a `simulator lang=spectre` among its first
/// 50 lines, as the PDKs write their model libraries, is Spectre; a file whose suffix marks a
/// hardware description language, such as `.va`, is text, which is not read.
pub fn detect_format(path: &str, text: &str) -> &'static str {
    FORMATS[detected(path, text)].name()
}

fn detected(path: &str, text: &str) -> usize {
    FORMATS.iter().position(|f| f.claims(path, text)).unwrap_or(0)
}

/// The formats set for files, by the key of each path ([`Provider::path_key`]), checked: an
/// unknown format name is an error, which names the formats there are.
pub(crate) fn set_formats(
    set: &BTreeMap<String, String>,
    files: &dyn Provider,
) -> Result<Vec<(String, usize)>, String> {
    set.iter()
        .map(|(path, name)| match index(name) {
            Some(k) => Ok((files.path_key(&normalize(&path.replace('\\', "/"))), k)),
            None => {
                let names: Vec<&str> = FORMATS.iter().map(|f| f.name()).collect();
                Err(format!("unknown format \"{name}\" for {path}; the formats are {}", names.join(", ")))
            }
        })
        .collect()
}

/// [`set_formats`], leaving out a format name that is not known.
pub(crate) fn known_formats(set: &BTreeMap<String, String>, files: &dyn Provider) -> Vec<(String, usize)> {
    set.iter()
        .filter_map(|(path, name)| Some((files.path_key(&normalize(&path.replace('\\', "/"))), index(name)?)))
        .collect()
}

/// The format of the main file: the one set for it, else the one `dialect` names, else
/// [`detect_format`]'s; and the one it has when none is set for it. A `dialect` that names no
/// format is SPICE, as it has been.
pub(crate) fn main_format(
    path: &str,
    text: &str,
    dialect: Option<&str>,
    set: &[(String, usize)],
    files: &dyn Provider,
) -> (usize, usize) {
    let default = match dialect {
        Some(d) => index(d).unwrap_or(0),
        None => detected(path, text),
    };
    (chosen(path, set, files).unwrap_or(default), default)
}

/// The format set for the file at `path`, if any.
fn chosen(path: &str, set: &[(String, usize)], files: &dyn Provider) -> Option<usize> {
    if set.is_empty() {
        return None;
    }
    let key = files.path_key(&normalize(path));
    set.iter().find(|(k, _)| *k == key).map(|(_, f)| *f)
}

/// Read with the formats of `FORMATS`: the main file with `main`, whose default is `main_default`
/// ([`main_format`]), each included file whose format is set with that one, and every other
/// included file by its own path and text ([`detect_format`]), its default.
pub(crate) fn with<T>(
    (main, main_default): (usize, usize),
    set: &[(String, usize)],
    files: &dyn Provider,
    read: impl FnOnce(&Formats) -> T,
) -> T {
    let choose = |path: &str, text: &str, is_main: bool| {
        if is_main {
            (main, main_default)
        } else {
            let default = detected(path, text);
            (chosen(path, set, files).unwrap_or(default), default)
        }
    };
    let list: &[&dyn Format] = &FORMATS;
    read(&Formats { list, choose: &choose })
}
