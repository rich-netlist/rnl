# RNL WebAssembly size investigation

Measurement in progress. The frozen baseline is `main` commit
`9ee4fe83d7401239472b4b9d5a568b3bb2d2422e` (2026-10-29 checkout), with
`opt-level = "s"`, fat LTO, one codegen unit and stripped symbols. The source
dependency graph already excludes `rnl-solver` and its exact-rational crates;
removing the solver is therefore not an expected saving. CI will quantify the
remaining code and data and test compatible variants before recommendations.

No product source or release configuration is changed by this branch. See
[`scripts/wasm-size/README.md`](scripts/wasm-size/README.md) for the frozen build,
prototype scope, byte-equivalence method, runtime checks and artifact names.
Results and a ranked recommendation will replace this progress note after the
CI artifacts have been inspected.
