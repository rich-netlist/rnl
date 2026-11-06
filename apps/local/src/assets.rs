//! The web viewer's files, embedded by build.rs, or read from a directory with `--site DIR`, with the types a browser
//! needs: a module script as `text/javascript`, WebAssembly as `application/wasm`, which it refuses otherwise.

use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/site.rs"));

/// Where the files of the site are from.
pub enum Site {
    /// The ones in the program.
    Embedded,
    /// A directory, so that a change to the page is seen without building the program again.
    Directory(PathBuf),
}

/// A file of the site: its bytes and its type.
pub struct Asset {
    pub bytes: std::borrow::Cow<'static, [u8]>,
    pub content_type: &'static str,
}

impl Site {
    /// Whether there is a site at all: a program built with none says so, and a directory may be empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Site::Embedded => SITE.is_empty(),
            Site::Directory(dir) => !dir.join("index.html").is_file(),
        }
    }

    /// The file at a path below the site's root, `/`-separated and without a leading `/`; an empty path is the page.
    pub fn get(&self, path: &str) -> Option<Asset> {
        let path = if path.is_empty() { "index.html" } else { path };
        if path.split('/').any(|part| part.is_empty() || part == "." || part == "..") || path.contains('\\') {
            return None;
        }
        match self {
            Site::Embedded => SITE
                .binary_search_by(|(name, _)| (*name).cmp(path))
                .ok()
                .map(|at| Asset { bytes: std::borrow::Cow::Borrowed(SITE[at].1), content_type: content_type(path) }),
            Site::Directory(dir) => {
                let file = dir.join(path);
                // (below the directory: no `..` is left, and a link may not leave it)
                let real = std::fs::canonicalize(&file).ok()?;
                let root = std::fs::canonicalize(dir).ok()?;
                if !real.starts_with(&root) || !real.is_file() {
                    return None;
                }
                std::fs::read(real).ok().map(|bytes| Asset { bytes: bytes.into(), content_type: content_type(path) })
            }
        }
    }
}

/// The type a browser needs for a file of the site, by its extension; the netlists and what they include are bytes.
pub fn content_type(path: &str) -> &'static str {
    let extension = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_types_a_browser_needs() {
        assert_eq!(content_type("core/rnl_runtime_wasm.wasm"), "application/wasm");
        assert_eq!(content_type("shells/web.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("fonts/noto-sans-sc-400-latin.woff2"), "font/woff2");
        assert_eq!(content_type("samples/cases/04-hierarchy.sp"), "application/octet-stream");
        assert_eq!(content_type("style.CSS"), "text/css; charset=utf-8");
    }

    #[test]
    fn a_path_that_leaves_the_root_is_no_file() {
        let site = Site::Directory(std::env::temp_dir());
        for path in ["../x", "a/../../x", "a//b", "a\\b", "./a"] {
            assert!(site.get(path).is_none(), "{path}");
        }
    }

    #[test]
    fn the_embedded_table_is_sorted_so_that_a_search_finds_each_file() {
        assert!(SITE.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
