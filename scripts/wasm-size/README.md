# Temporary WASM size investigation

This branch adds measurement tooling, not product changes. Builds and tool
installation run only on GitHub-hosted CI. The source checkout is pinned to
`9ee4fe83d7401239472b4b9d5a568b3bb2d2422e`; the harness checkout is the dispatched
branch revision. All prototypes are applied to the disposable source checkout,
recorded in the artifact, and restored between variants.

Before this workflow exists on the default branch, dispatch through the existing
Rust workflow (which calls `wasm-size.yml` from this branch):

```sh
gh workflow run rust.yml --repo rich-netlist/rnl --ref ext/wasm-size -f wasm_size=true
```

Once registered, `wasm-size.yml` also supports `workflow_dispatch` directly.
Neither route publishes, tags, deploys, nor commits product changes. Artifact
names are `wasm-size-{profiles,code,binaryen}-RUN_ID-ATTEMPT`, retained 30 days.
`results.json` records source/harness/compiler identity and every variant's
raw/gzip/Brotli bytes, actual-byte equivalence and test results. Per-variant
files include patches, commands, test logs and paired runtime samples.
The names-only build provides `breakdown.json` and twiggy reports. Binaries are
omitted from the uploaded report; the exact build instructions remain available.

The compressed-package prototype adds **miniz_oxide 0.8.9** only in the disposable
source tree. Its resolved lock and generated compressed data are retained.
It restores every original package byte before parsing. Ablations (`no-write`,
`no-snapshot`, `no-catalog`, `read-only`) remove capabilities and are measurements
of an upper bound, never compatible replacements.

The benchmark reuses the repository's 11 representative inputs and deterministic
chain-1024/diffmirror-128 generators. It measures the WASM ABI, with alternating
baseline/candidate order, 3 warmups and 9 samples. The Windows solver collector
does not benchmark this module: it has no solver entry point. Its offline harness
checks run separately, as do workspace tests and native/WASM conformance checks.
The result is not a browser/phone/network benchmark.

Local checks compile and install nothing:

```sh
python scripts/wasm-size/test_measurement.py
node --check scripts/wasm-size/verify.mjs
```

`wasm_bytes.py` attributes function bodies to surviving demangled symbols after
LTO, verifies the names-only companion has identical non-custom sections, and
accounts for every section byte. Compressed sizes are not additive: standalone
gzip sizes and the marginal change when code bytes are zeroed are labelled
diagnostics, not exact gzip ownership or safe removal savings.
