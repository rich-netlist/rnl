//! Report when the core has read its first source, without changing file lookup or parsing.
use std::cell::Cell;

use rnl_runtime::source::Provider;
use tauri::{Emitter, WebviewWindow};

pub struct LoadingProvider<'a> {
    inner: &'a dyn Provider,
    window: &'a WebviewWindow,
    request_id: Option<&'a str>,
    parsing: Cell<bool>,
}

impl<'a> LoadingProvider<'a> {
    pub fn cached(&self) {
        if let Some(id) = self.request_id {
            let _ = self.window.emit("load-stage", serde_json::json!({ "id": id, "stage": "cached" }));
        }
    }

    pub fn new(inner: &'a dyn Provider, window: &'a WebviewWindow, request_id: Option<&'a str>) -> Self {
        Self { inner, window, request_id, parsing: Cell::new(false) }
    }
}

impl Provider for LoadingProvider<'_> {
    fn auto_include(&self) -> Option<&dyn rnl_runtime::source::LibraryLookup> {
        self.inner.auto_include()
    }
    fn read(&self, path: &str) -> Option<String> {
        let text = self.inner.read(path);
        if text.is_none() || self.parsing.replace(true) {
            return text;
        }
        if let Some(id) = self.request_id {
            let _ = self.window.emit("load-stage", serde_json::json!({ "id": id, "stage": "parsing" }));
        }
        text
    }

    fn package_files(&self) -> Vec<String> {
        self.inner.package_files()
    }

    fn package_candidates(&self, name: &str) -> Vec<String> {
        self.inner.package_candidates(name)
    }

    fn include_candidates(&self, name: &str) -> Vec<String> {
        self.inner.include_candidates(name)
    }
}
