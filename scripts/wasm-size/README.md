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
The compiler is Rust 1.99.1, the version recorded by the baseline commit's actual
Pages build (run 40889165832); compression uses Python Brotli 1.1.0 and gzip level 9.
An earlier exploratory campaign used the solver harness's Rust 1.97.1 pin;
its records retain that identity and must not be mixed with the production-toolchain table.
Binaryen 133 is downloaded from its official release and SHA-256 checked. Only
the input's supported WASM features are enabled, so optimization cannot opt into
GC or other unrelated proposals. Its no-inlining experiment sets all three
inlining size thresholds to zero using Binaryen's documented CLI options.

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
The per-variant native checks cover all workspace libraries except the untouched
solver and CLI's unrelated integration suites, plus the CLI conformance target.
The full baseline workspace test log is retained with the investigation evidence.

Local checks compile and install nothing:

```sh
python scripts/wasm-size/test_measurement.py
node --check scripts/wasm-size/verify.mjs
```

For selected reruns, pass `wasm_size_groups` as a JSON array and
`wasm_size_variants` as comma-separated names to `rust.yml`. Baseline is always
built and replayed first. Do not combine this queue with the solver queues.
After downloading the three artifacts, a compact, durable result file can be
created with `python scripts/wasm-size/summarize.py DIR1 DIR2 DIR3 --out RESULTS.json`.

`wasm_bytes.py` attributes function bodies to surviving demangled symbols after
LTO and accounts for every release section byte. It checks whether the names-only
companion has identical non-custom sections. If Rust's strip option changes code
layout, it attributes the companion's own code and records its size delta; it
never transfers names to different release function indices. A selected
`analysis` group can rerun just these builds and the bounded twiggy commands.
Compressed sizes are not additive: standalone
gzip sizes and the marginal change when code bytes are zeroed are labelled
diagnostics, not exact gzip ownership or safe removal savings.
