//! Reading a file into what the page shows: one SVG per drawing level, the diagnostics, and what
//! each level still lacks without a solver. The same runtime and renderer as `rnl render`.

use std::path::{Path, PathBuf};

use rnl_cli::{interpret_file, path_str, FsProvider};
use rnl_runtime::diag::{Diag, Sink};
use rnl_runtime::Options;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Document {
    /// The file as opened, for reloading.
    pub path: String,
    /// Its file name, for the title bar.
    pub name: String,
    pub levels: Vec<Level>,
    pub diagnostics: Vec<Diagnostic>,
    /// Why the document could not be interpreted at all, such as an unsupported language version.
    pub failed: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Level {
    pub title: String,
    /// "file" or "subckt".
    pub kind: &'static str,
    pub svg: String,
    /// Nothing on the level can be drawn: no object has a position yet.
    pub empty: bool,
    /// What the level lacks without a solver; see [`rnl_cli::level_status`].
    pub status: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Diagnostic {
    /// "error", "warning" or "hint".
    pub severity: &'static str,
    /// The file as shown: relative to the opened file's directory when it is inside it.
    pub file: String,
    /// The file as the runtime knows it.
    pub full_path: String,
    pub line: u32,
    /// 0 when the diagnostic is about the whole line.
    pub col: u32,
    pub message: String,
    /// Expansion, rule or package history, such as "applying $GND_STUB (a.sp:12:3)".
    pub origin: String,
    /// The upstream cause, as "file:line:col: message".
    pub cause: Option<String>,
}

/// Read and interpret `path` with the bundled standard packages, and draw every level.
pub fn load(path: &Path) -> Result<Document, String> {
    let path: PathBuf = std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let provider = FsProvider::new(&[]);
    let r = interpret_file(&path, None, &provider, &Options::default())?;
    let dir = path_str(path.parent().unwrap_or(Path::new("")));
    let shown_file = |file: &str| match file.strip_prefix(&dir).and_then(|f| f.strip_prefix('/')) {
        Some(rel) if !dir.is_empty() => rel.to_string(),
        _ => file.to_string(),
    };
    let diagnostic = |d: &Diag, sink: &Sink| {
        let (file, line, col) = match &d.loc {
            Some(l) => (l.file.clone(), l.line, l.col),
            None => (String::new(), 0, 0),
        };
        let cause = d.cause.map(|c| {
            let c = sink.get(c);
            match &c.loc {
                Some(l) => format!("{}:{}{}: {}", shown_file(&l.file), l.line, col_suffix(l.col), c.message),
                None => c.message.clone(),
            }
        });
        Diagnostic {
            severity: d.severity.as_str(),
            file: shown_file(&file),
            full_path: file,
            line,
            col,
            message: d.message.clone(),
            origin: d.origin.clone(),
            cause,
        }
    };
    let diagnostics = r.diagnostics().into_iter().map(|d| diagnostic(d, &r.sink)).collect();
    let levels = r
        .model
        .levels
        .iter()
        .map(|lv| {
            let prims = rnl_render::primitives(lv);
            Level {
                title: lv.title.clone(),
                kind: lv.kind,
                svg: rnl_render::svg_document(&prims, &rnl_render::RenderOptions::default()),
                empty: prims.is_empty(),
                status: rnl_cli::level_status(lv),
            }
        })
        .collect();
    Ok(Document {
        path: path.to_string_lossy().into_owned(),
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        levels,
        diagnostics,
        failed: r.model.failed.clone(),
    })
}

fn col_suffix(col: u32) -> String {
    if col > 0 {
        format!(":{col}")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/cases").join(name)
    }

    #[test]
    fn draws_every_level_with_its_status() {
        let d = load(&case("04-hierarchy.sp")).unwrap();
        assert_eq!(d.name, "04-hierarchy.sp");
        assert!(d.levels.len() >= 3, "{:?}", d.levels.iter().map(|l| &l.title).collect::<Vec<_>>());
        assert!(d.levels.iter().all(|l| l.svg.starts_with("<svg") && !l.empty));
        let d = load(&case("10-constraints-and-priority.sp")).unwrap();
        assert!(d.levels[0].status.as_deref().is_some_and(|s| s.contains("needs-solver")));
    }

    #[test]
    fn diagnostics_are_relative_to_the_opened_file() {
        let d = load(&case("12-diagnostics.sp")).unwrap();
        assert!(d.diagnostics.iter().any(|x| x.severity == "error"));
        // its subcircuit has nothing with a position
        assert!(d
            .levels
            .iter()
            .any(|l| l.empty && l.status.as_deref().is_some_and(|s| s.contains("without a position"))));
        let own: Vec<_> = d.diagnostics.iter().filter(|x| x.full_path.ends_with("12-diagnostics.sp")).collect();
        assert!(!own.is_empty());
        assert!(own.iter().all(|x| x.file == "12-diagnostics.sp" && x.line > 0));
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(load(&case("no-such-case.sp")).is_err());
    }
}
