# 0012 — Public packages use an immutable catalog

Status: accepted (2026-10-20). Implementation: `ext/packages-split`, pending pull-request review.

The standard series, including `rnl-zh`, remains in `rnl/libraries/standard/` and built into the core. Optional packages belong to the separate `rich-netlist/rnl-packages` repository. The initial split moves analog-canvas-extensions, eda-symbol, eda-style and pdk-sky130 with their history and unchanged package bytes.

That repository generates a versioned catalog from the package files and publishes it, with every version's files, through its GitHub Pages. Each record identifies a package and its `major.minor` version, kind, bilingual summary, immutable URL and SHA-256; version assets are hashed too. CI rejects changes to published records or files. Changed content needs a new version, and older URLs remain available.

The web viewer and the website follow the latest catalog and verify every downloaded package. Standard packages remain available if the catalog cannot be read. Legacy public-package URLs resolve through the catalog. Embedded viewers use the same verified package source.

Tests and skill generation use one `packages.lock.json` and a verified checkout-local cache. Missing data skips only the affected local tests and prints the download command; it fails in CI. Browser tests use a local catalog, including an empty local catalog when optional data has not been downloaded.

The next runtime npm version after preview.14 removes the optional Canvas data file and export. Published versions are not changed. Applications supply the verified package text in `files` and explicitly include its path in `pool`.

The website rebuilds main daily and retains manual dispatch. A separate daily compatibility check runs main against the latest public package versions and reports failures in Actions without becoming a merge gate for unrelated core changes. Package publication validates all versions and every symbol with a fixed RNL source revision.

This decision changes data distribution. It does not change package syntax, version resolution, the contents of the standard series, or the ownership of drawing behavior by the shared Rust core (decision 0008).
