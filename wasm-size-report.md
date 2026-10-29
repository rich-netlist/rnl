# RNL core WASM size investigation

The 2.63 MB WASM is predominantly executable interpreter, document, rendering and editing code: 2.306 MB of code and 0.321 MB of data. It already excludes the solver and its arbitrary-precision rational dependencies, and already strips debug information. Standard packages account for 138 KB of source text, but compress well; compressing them inside WASM makes the download larger. The best measured option with approximately unchanged warm execution time is the existing `s` profile followed by Binaryen 133 `-Oz` with its three inlining thresholds set to zero: **2,258,024 raw / 796,796 gzip / 596,717 Brotli bytes**, saving **3.68% gzip**. The smallest compatible download tested is `z` plus that post-processing: **1,876,037 raw / 655,436 gzip / 502,613 Brotli bytes**, saving **20.77% gzip**, but warm ABI execution takes about twice as long. There is no measured, behavior-preserving code/data deletion that beats that tradeoff. This branch changes only measurement tooling and reports.

## Baseline and evidence

Measurements ran on GitHub-hosted Ubuntu 24.04 on **2026-10-29 UTC**. Source is frozen at main as initially fetched, [`9ee4fe83d7401239472b4b9d5a568b3bb2d2422e`](https://github.com/rich-netlist/rnl/commit/9ee4fe83d7401239472b4b9d5a568b3bb2d2422e), version 0.5.3. Main advanced during the investigation; these numbers must not silently be applied to a later revision or the separate Viewer optimization PR.

The baseline uses the site's actual `node bindings/runtime-wasm/build.mjs` entry point, target `wasm32-unknown-unknown`, and `[profile.wasm]`: `opt-level="s"`, fat LTO, one codegen unit, `strip=true`. Rust **1.99.1**, LLVM **23.1.1**, matches the compiler in the baseline commit's [actual Pages build](https://github.com/rich-netlist/rnl/actions/runs/40889165832). That build logged the same **2,633,886 raw bytes**, and **831,781 gzip bytes using Node's compressor**. Every comparison below consistently uses Python gzip level 9 with `mtime=0`, and Brotli 1.1.0 quality 11. Different compressor implementations explain why the table's baseline is **827,210** gzip bytes; it is not a new deployment's wire-size measurement. Node for execution was **22.23.4**, Python 3.12. Tools and dependencies are installed only on CI. No local Rust build or tool installation was performed.

The durable [machine-readable results](wasm-size-results.json) retain source/harness SHA, WASM SHA-256, settings, outcomes, breakdown and all paired timing samples. Full patches, commands, corpus response hashes, logs and symbol lists are in these artifacts (30-day retention):

| Run | Artifact | Purpose |
| --- | --- | --- |
| [40903427505](https://github.com/rich-netlist/rnl/actions/runs/40903427505) | `wasm-size-profiles-40903427505-1` | Production compiler profile matrix; native tests and conformance |
| same run | `wasm-size-code-40903427505-1` | Code/data prototypes and capability ablations |
| same run | `wasm-size-binaryen-40903427505-1` | Binaryen matrix and native/WASM checks for both recommended candidates |
| [40908397124](https://github.com/rich-netlist/rnl/actions/runs/40908397124) | `wasm-size-analysis-40908397124-1` | Corrected attribution and bounded `twiggy top`, `dominators`, `monos` |

The profile job's original attribution assertion failed because changing `strip` changed executable bytes. Its variant sizes, equality and timings had already been recorded; the assertion was replaced with explicit companion attribution and rerun separately. This is an analysis failure, not a claimed successful job or a test failure. Corrected attribution recognizes Rust v0 crate disambiguators and leaves ambiguous primitive owners unattributed. Earlier Rust 1.97.1 runs, an initial invalid mutable-request recorder, and an unsuitable Binaryen setup are superseded and excluded from the main table. The original full-workspace test log in `wasm-size-profiles-40898308911-1` records 919 passing tests on Rust 1.97.1; it is supplemental evidence, not a claim that 919 tests ran on 1.99.1.

## What occupies the binary

### Exact release section accounting

| Section | Raw bytes | Raw share | Standalone gzip |
| --- | --- | --- | --- |
| Header | 8 | 0.0003% | - |
| Type | 649 | 0.025% | 253 |
| Function declarations | 5,940 | 0.226% | 2,109 |
| Table | 9 | 0.000% | 29 |
| Memory | 5 | 0.000% | 25 |
| Globals | 11 | 0.000% | 31 |
| Exports | 114 | 0.004% | 113 |
| Elements | 871 | 0.033% | 782 |
| Code | 2,305,619 | 87.537% | 724,505 |
| Data | 320,660 | 12.174% | 99,451 |
| Total | 2,633,886 | 100% | 827,210 (whole module) |

There are no custom/debug/name sections in the shipped baseline. Section gzip values are independent compression diagnostics: shared history means they do not add up to the whole-module gzip. The module has a zero-import C ABI rather than a wasm-bindgen runtime dependency.

### Crate and function attribution

An unstripped companion provides symbol names. Its executable/non-custom content is **2,634,642 bytes**, **756 bytes (0.029%) larger** than the real release, with gzip **827,577 bytes**. It is not byte-identical to the release; names were never transferred onto the release's different function indices. The following body sizes refer to that companion's **5,939 functions**, including body length prefixes. Raw shares use its non-custom total. LTO moves/inlines code across boundaries; attribution follows the leading qualified symbol owner, not every crate in generic arguments. Primitive/anonymous owners without a clear crate stay unattributed.

There is no unique additive attribution of gzip to crates or functions. `gzip delta` below is the change in the companion's whole-module gzip when the selected function bodies are zeroed; its percentage uses **827,577** as denominator. These diagnostics are **not safe removals, additive ownership, or predicted deployable savings**. The JSON also includes standalone gzip for each group.

| Symbol owner | Functions | Raw body bytes | Raw share | gzip delta (diagnostic) | Delta / whole gzip |
| --- | --- | --- | --- | --- | --- |
| `rnl_runtime` | 610 | 714,869 | 27.13% | 268,761 | 32.48% |
| `core` | 1,343 | 454,560 | 17.25% | 127,233 | 15.37% |
| `alloc` | 1,824 | 332,710 | 12.63% | 84,895 | 10.26% |
| `hashbrown` | 807 | 257,751 | 9.78% | 30,250 | 3.66% |
| `rnl_view` | 161 | 116,944 | 4.44% | 41,081 | 4.96% |
| `serde_json` | 367 | 102,097 | 3.88% | 22,073 | 2.67% |
| `<unattributed>` | 312 | 70,250 | 2.67% | 21,227 | 2.56% |
| `rnl_edit` | 75 | 46,285 | 1.76% | 13,776 | 1.66% |
| `rnl_host_spice` | 37 | 41,220 | 1.56% | 14,685 | 1.77% |
| `std` | 133 | 38,286 | 1.45% | 5,843 | 0.71% |
| `rnl_render` | 36 | 35,146 | 1.33% | 13,512 | 1.63% |
| `rnl_host_spectre` | 24 | 34,441 | 1.31% | 12,200 | 1.47% |
| `serde` | 66 | 22,069 | 0.84% | 5,116 | 0.62% |
| `rnl_runtime_wasm` | 44 | 12,456 | 0.47% | 3,764 | 0.45% |
| `dlmalloc` | 18 | 6,694 | 0.25% | 2,862 | 0.35% |
| `libm` | 10 | 5,807 | 0.22% | 2,576 | 0.31% |
| `unicode_segmentation` | 8 | 4,639 | 0.18% | 1,935 | 0.23% |
| `indexmap` | 23 | 3,605 | 0.14% | 1,274 | 0.15% |
| `compiler_builtins` | 3 | 2,437 | 0.09% | 1,213 | 0.15% |
| `serde_core` | 18 | 2,344 | 0.09% | 672 | 0.08% |
| `unicode_width` | 2 | 781 | 0.03% | 427 | 0.05% |
| `itoa` | 6 | 588 | 0.02% | 157 | 0.02% |
| `__rustc` | 10 | 242 | 0.01% | 73 | 0.01% |
| `zmij` | 2 | 143 | 0.01% | 39 | 0.00% |

Largest functions (shortened display names):

| Function | Raw body bytes | Raw share | gzip delta (diagnostic) | Delta / whole gzip |
| --- | --- | --- | --- | --- |
| `Interp::keyword` | 45,341 | 1.72% | 14,007 | 1.69% |
| `Interp::level_problem` | 42,391 | 1.61% | 13,825 | 1.67% |
| `Interp::level_view` | 30,408 | 1.15% | 10,438 | 1.26% |
| `Interp::resolve_geometry` | 20,000 | 0.76% | 6,194 | 0.75% |
| `Interp::read_unit` | 18,422 | 0.70% | 6,151 | 0.74% |
| `Interp::check_constraints` | 18,168 | 0.69% | 6,097 | 0.74% |
| `SpectreReader::read_file` | 17,880 | 0.68% | 6,252 | 0.76% |
| `rnl_view::document::document` | 17,702 | 0.67% | 6,748 | 0.82% |
| `Parser::set_expr` | 16,447 | 0.62% | 6,504 | 0.79% |
| `Interp::eff` | 16,324 | 0.62% | 5,543 | 0.67% |
| `Interp::eval_obj` | 16,296 | 0.62% | 5,611 | 0.68% |
| `Interp::run` | 16,005 | 0.61% | 5,617 | 0.68% |
| `Interp::select` | 13,649 | 0.52% | 4,345 | 0.53% |
| `rnl_edit::ops::parse` | 12,138 | 0.46% | 3,360 | 0.41% |

The dependency tree and symbols include `rnl-runtime`, `rnl-view`, `rnl-edit`, `rnl-render`, SPICE/Spectre hosts, serde/JSON, collections, `libm`, and Unicode segmentation/width. **`rnl-solver`, `num-bigint`, `num-rational`, and regex are absent.** A build “without the solver” has zero additional opportunity. `syn`, `quote` and `proc-macro2` in Cargo's tree serve the host-side derive compiler; they are not linked into this WASM.

### Embedded data and supporting machinery

| Embedded content | Source bytes | Standalone gzip-9 | Evidence |
| --- | ---: | ---: | --- |
| All 25 standard packages, 0.2 and 0.3 | 138,042 | 23,974 as one concatenation | Every entire file occurs once |
| `words-0.2.json` | 16,104 | 2,436 | Entire file occurs once |
| Noto Sans SC metrics | 8,536 | 3,107 | Entire file occurs once |
| Whole SPICE/Spectre test inputs | 0 found | — | Conformance/Viewer inputs longer than 32 bytes |

The three known payload groups total **162,682 bytes**, about half the data section. The remainder includes literals, diagnostics, Unicode/static lookup tables, numeric tables and metadata. It was not reliably partitioned further and must not all be labelled “panic strings.” The fixture scan detects complete payloads, not shared substrings. Inspected fixture `include_str!` uses are under `cfg(test)`.

| Overlapping name-based slice | Functions | Raw body bytes | Companion raw share |
| --- | --- | --- | --- |
| serde / JSON (including derives) | 842 | 209,472 | 7.95% |
| formatting | 138 | 39,189 | 1.49% |
| Unicode functions | 26 | 13,992 | 0.53% |
| panic / errors | 194 | 40,406 | 1.53% |

These semantic slices overlap and are name-based indicators after inlining. Derived serializers often appear under RNL types or collections, so `serde_json` alone understates serialization-related code. Unicode tables occupy data beyond named functions. Formatting and diagnostic strings are observable outputs. Unicode width/segmentation and font metrics participate in deterministic rendering (decision 0009); replacing them with ASCII assumptions changes output.

`twiggy monos` identifies repeated sorting/collection instantiations: stable partition has **43,201 bytes** across 68 instances (estimated 41,915 “bloat” bytes), and stable quicksort has **29,461 bytes** (estimated 28,249). Those estimates assume hypothetical sharing, not a semantics-preserving optimization. Type-erased comparators or centralized representations can add indirect calls, allocation or marshalling. No refactor is justified by these estimates alone; overlapping families cannot be summed into promised savings.

`twiggy dominators` shows exported `rnl_build` retaining about **1,328,836 bytes**, primarily through `respond_json` and the multi-request dispatcher. Shared code/data also have other roots. Twiggy percentages include the unstripped file's custom sections and are not production percentages. Its “unreachable” custom/debug items are not 1.99 MB of removable production code. Full bounded reports are in the analysis artifact.

## Every measured variant

All sizes are decimal bytes. Each row changes only the stated setting/prototype relative to baseline, except the combinations defined below. Runtime is the median of the **13 per-input candidate/baseline median ratios**; less than 1 is faster. Repeated baselines show runner/process noise. `0/574` means raw UTF-8 response bytes matched, including ordering, whitespace and diagnostics. This is a bounded corpus, not a proof over every possible request. Incompatible variants are not benchmarked or recommended.

| Group / variant | Raw | gzip-9 | Brotli-11 | ABI differences | Other test results | Warm ratio |
| --- | --- | --- | --- | --- | --- | --- |
| profiles / baseline | 2,633,886 | 827,210 | 604,842 | 0/574 | JS + 3 equality modes | 0.998x |
| profiles / opt-z | 2,219,406 | 686,568 | 509,687 | 0/574 | JS + 3 equality modes | 1.768x |
| profiles / panic-abort | 2,633,886 | 825,871 | 604,906 | 0/574 | JS | 0.993x |
| profiles / lto-thin | 2,634,642 | 827,565 | 605,404 | 0/574 | JS | 0.983x |
| profiles / lto-off | 2,636,305 | 828,055 | 605,334 | 0/574 | JS | 0.995x |
| profiles / cgu-16 | 2,740,981 | 833,402 | 614,769 | 0/574 | JS | 1.004x |
| profiles / symbols | 5,484,019 | 1,321,859 | 978,767 | 0/574 | JS; attribution rerun | 0.990x |
| profiles / debug-stripped | 2,633,795 | 827,056 | 604,849 | 0/574 | JS | 1.008x |
| profiles / debug-unstripped | 44,583,346 | 9,104,948 | 6,396,523 | 0/574 | JS | 0.989x |
| code / baseline | 2,633,886 | 827,210 | 604,842 | 0/574 | JS | 1.001x |
| code / compact-words | 2,632,070 | 827,130 | 605,002 | 0/574 | JS + native | 1.004x |
| code / compressed-packages | 2,539,792 | 837,374 | 618,442 | 0/574 | JS + native | 1.012x |
| code / json-value-output | 2,668,799 | 830,238 | 606,723 | 0/574 | JS + native | 1.023x |
| code / direct-json-input | 2,637,316 | 830,459 | 607,500 | 10/574 | rejected | not run |
| code / no-write | 2,424,833 | 763,354 | 561,317 | 3/574 | rejected | not run |
| code / no-snapshot | 2,590,181 | 813,518 | 594,466 | 93/574 | rejected | not run |
| code / no-catalog | 2,593,837 | 816,546 | 596,989 | 2/574 | rejected | not run |
| code / read-only | 2,339,606 | 737,543 | 543,406 | 98/574 | rejected | not run |
| code / combined | 2,122,964 | 696,732 | 523,572 | 0/574 | JS + native | 2.028x |
| code / combined-value | 2,147,004 | 698,102 | 525,192 | 0/574 | JS + native | 2.087x |
| binaryen / baseline | 2,633,886 | 827,210 | 604,842 | 0/574 | JS | 0.997x |
| binaryen / opt-z | 2,219,406 | 686,568 | 509,687 | 0/574 | JS | 1.982x |
| binaryen / baseline-Os | 2,289,861 | 878,638 | 654,496 | 0/574 | JS | 0.955x |
| binaryen / baseline-Oz | 2,267,247 | 878,250 | 655,855 | 0/574 | JS | 0.989x |
| binaryen / baseline-Oz-no-inline | 2,258,024 | 796,796 | 596,717 | 0/574 | JS + 3 equality modes | 1.011x |
| binaryen / opt-z-Os | 1,875,678 | 726,247 | 548,829 | 0/574 | JS | 1.802x |
| binaryen / opt-z-Oz | 1,854,696 | 725,282 | 549,663 | 0/574 | JS | 1.945x |
| binaryen / opt-z-Oz-no-inline | 1,876,037 | 655,436 | 502,613 | 0/574 | JS + 3 equality modes | 1.998x |
| analysis / baseline | 2,633,886 | 827,210 | 604,842 | 0/574 | ABI only; analysis passes | not run |
| analysis / symbols | 5,484,019 | 1,321,859 | 978,767 | 0/574 | ABI only; analysis passes | not run |

Test results for every row: all compatible matrix rows passed the JS core suite (**17 passed**, 2 deliberate `test-panic`-only skips). The later analysis-only repeat does not rerun JS/timing. `compact-words`, `compressed-packages`, `json-value-output`, `combined` and `combined-value` additionally passed affected native libraries and CLI conformance. Baseline, `opt-z`, and both `*-Oz-no-inline` variants passed all three native/WASM equality modes. The `symbols` row's original attribution error is explained above. Every row with differences is rejected for compatibility; the four capability ablations intentionally remove behavior.

### Build and post-processing

`z` alone saves **140,642 gzip bytes (17.00%)**, but median warm execution is **1.768x** on the profile runner and **1.982x** on the Binaryen runner. This is measured variation, not grounds for choosing only the faster figure. Fat/thin/off LTO have little size separation here; 16 codegen units are worse. Explicit `panic=abort` leaves raw size unchanged and reduces gzip by 1,339 bytes while slightly increasing Brotli. The target already uses abort, so there is no hidden unwind engine. Stripping already works: removing strip adds about 2.85 MB of custom sections; full debug without strip creates a 44.6 MB artifact.

Binaryen 133 `-Os`/`-Oz` reduces raw bytes but increases gzip: baseline `-Oz` is **13.92% smaller raw but 6.17% larger gzip**. Zeroing its three inlining thresholds reverses the gzip result: **796,796** instead of **878,250** bytes, with similar median warm performance. This supports the explanation that minimizing uncompressed instructions can replace repeated, compressible patterns with less compressible bodies/layouts; inlining policy is a significant contributor here. It does not prove that every byte of regression is due to inlining. Binaryen does not optimize the final HTTP gzip objective.

Exact additional flags after `-Oz`:

```sh
--always-inline-max-function-size 0 \
--one-caller-inline-max-function-size 0 \
--flexible-inline-max-function-size 0
```

Only the input's relevant features are enabled (bulk memory, sign extension, non-trapping float-to-int, multivalue, reference types and mutable globals), not `--all-features`. Binaryen's official archive is version- and SHA-256-pinned. Applying this after `z` saves another **31,132 gzip bytes (4.53% of z)**, for **171,774 bytes (20.77%)** versus original `s`.

### Code/data prototypes and costs

| Prototype | Change and churn | gzip saving vs baseline | Cost / compatibility |
| --- | --- | ---: | --- |
| Compact word list | Generated minified internal JSON; one `rnl-runtime` call site | 80 bytes | Original public constant retained; compatible, negligible; Brotli grows |
| Compress standard packages inside | Generated compressed concatenation/ranges; `rnl-view` loader/manifest | **−10,164 bytes** (growth) | Adds **miniz_oxide 0.8.9**, inflation and a retained heap buffer; restores source byte-for-byte; compatible |
| JSON Value output | Typed response through `serde_json::Value`; one binding expression | **−3,028 bytes** | Compatible on corpus; extra representation/allocation/code; ~1.023x warm |
| Direct typed JSON input | `from_value` to `from_slice` in dispatch | **−3,249 bytes** | Still parses dispatch Value; no saving; 10 error differences, rejected |
| `combined` | `z` + compact words + compressed packages | 130,478 bytes | Compatible; 10,164 gzip bytes larger than `z` alone; 2.028x warm |
| `combined-value` | `combined` + JSON Value output | 129,108 bytes | Compatible; larger and slower (2.087x) than `z` alone |

For `{}`, direct typed input adds `at line 1 column 2` to the missing-`entry` error. Error wording is part of compatibility; successful documents alone are insufficient. A different boundary parser must preserve dispatch, defaults, unknown-field handling, field order and diagnostics; it was not implemented here.

An external package asset was not implemented: the measured alternative was compression inside. The complete text bundle is only **23,974 gzip bytes** separately, so externalization does not eliminate that first-render transfer. It needs a stable content/version cache key, initialization/failure behavior and preservation of `<bundled>` origins and revisions. Independent caching could help, but net first-load saving and startup speedup were not measured. Another fetch and lost cross-resource compression may offset a smaller WASM. Do not subtract 138 KB from gzip download size.

### Capability gates and module splitting

| Deliberately removed capability | Raw saved | gzip saved | Responses changed |
| --- | ---: | ---: | ---: |
| Write/edit dispatch | 209,053 | 63,856 (7.72%) | 3 |
| Snapshot dispatch | 43,705 | 13,692 (1.66%) | 93 |
| Catalog dispatch | 40,049 | 10,664 (1.29%) | 2 |
| All three (`read-only`) | 294,280 | 89,667 (10.84%) | 98 |

These disposable edits expose capability-removal budgets through dead-code elimination. They are not implemented Cargo features or interchangeable runtime packages. The shared npm runtime exposes these operations; consumers cannot receive the ablated binary. Savings overlap and are not additive.

A first-load Viewer module plus lazy editing/catalog module is a possible follow-up, but no multi-module system or net total was measured here. Modules may duplicate interpreter/JSON code, libraries, allocators and compiled code, and require a protocol for library indexes, lazy drawings and state. The **63,856-byte** write budget is a starting point, not promised net savings. A real split needs core/Viewer/npm coordination and more churn than build-only changes. No solver feature gate is needed.

## Runtime and correctness

Timing uses the repository performance harness's 11 frozen representative inputs plus its deterministic chain-1024 and diffmirror-128 (640 MOS) generators. It exercises document/primitives construction through the actual WASM ABI, including JSON marshalling and response copying. A process alternates baseline/candidate order, with 3 warmups and 9 samples per input. The existing solver harness's **20 offline tests pass**. Its Windows solver timing collector is inapplicable to this module; **these are not solve timings**.

Selected paired medians from the Binaryen job, milliseconds. Each pair retains its nearby baseline:

| Input | s -> z | s -> s + Oz/no-inline | s -> z + Oz/no-inline |
| --- | --- | --- | --- |
| T1-rc-lowpass.sp | 17.41 -> 36.35 | 17.14 -> 17.58 | 17.07 -> 37.31 |
| 04-fd-ota-cmfb.sp | 38.72 -> 68.31 | 40.69 -> 40.24 | 38.84 -> 69.11 |
| chain-1024 | 146.46 -> 248.86 | 148.60 -> 144.49 | 146.73 -> 247.38 |
| diffmirror-128 | 307.10 -> 495.50 | 306.78 -> 300.29 | 305.60 -> 479.92 |

Across 13 cases, `s + Oz/no-inline` has median **1.011x**, maximum **1.090x**; `z + Oz/no-inline` has median **1.998x**, maximum **2.186x**. These are CI Node warm measurements. Stored first-call timings do not use fresh instances for every input and cannot establish cold browser instantiation, mobile performance or first-paint latency. Browser end-to-end checks remain necessary before deployment. A 20.77% smaller download does not imply a 20.77% faster Viewer.

Correctness evidence on Rust 1.99.1:

- Affected native workspace libraries: **286 tests pass**; CLI conformance target: **7 pass**, including model/snapshot fixtures and standard packages. These pass for each compatible code/data prototype too.
- Compatible matrix variants: **574/574 raw response checks**, plus JS core tests. Coverage includes 0.2/0.3 conformance, SPICE/Spectre equivalence, standard packages, documents, primitives, snapshots, library indexes, lazy drawings, catalog, formats, writes and malformed requests. Baseline self-replay gates trust in candidate comparisons.
- Baseline, `z`, `s + Oz/no-inline`, `z + Oz/no-inline`: all three existing native/WASM modes pass on 93 files plus catalog, with **99 default + 99 primitives + 192 snapshot WASM builds** per candidate. Existing native snapshot expectations pass independently of cross-build comparison.
- Benchmark requests also require raw output equality. Four lightweight measurement-tool tests cover section accounting, unsigned LEB128, symbol grouping and lossless prototype generation; they compile/install nothing locally.

No exact-arithmetic algorithm, language rule, product API or production dependency was changed. The new runtime dependency exists only in the disclosed, rejected CI prototype.

## Ranked recommendations

1. **Evaluate `s + Binaryen 133 Oz/no-inline` first.** Save **375,862 raw / 30,414 gzip / 8,125 Brotli bytes** (14.27% / 3.68% / 1.34%), with median warm performance effectively unchanged in this corpus. Cost: one pinned build tool/step shared by site/npm; no core refactor. Risk: engine/platform variation and optimizer changes. Retain byte/conformance gates and check browser loading/interactions before shipping. This is the strongest measured size/performance balance, not a deployment made by this PR.
2. **Use `z + Oz/no-inline` only if download benefit outweighs CPU cost.** It reaches **655,436 gzip bytes**, saving 20.77%, at about 2x warm ABI latency. Correctness gates pass, but it is not a free win. Coordinate the separate Viewer `z` work against this frozen baseline and its own browser evidence.
3. **If substantially more is needed, scope an optional write module/feature separately.** The ablation exposes at most a 63,856-byte gzip budget before duplication/glue. Cost and compatibility risk are high. Preserve the full npm API and measure both initial and eventual total downloads. Net savings are unmeasured.
4. **Reject the measured data/JSON tricks for size.** An 80-byte word-list gain is immaterial; internal package compression and JSON-boundary prototypes grow gzip, and direct input changes errors. Keep stripping/fat-LTO/one-codegen-unit as the baseline. Removing an already-absent solver cannot help. Generic sharing merits targeted work only with another justification; twiggy estimates are not savings commitments.

## Reproduction

The temporary [workflow](.github/workflows/wasm-size.yml) supports `workflow_dispatch` and `workflow_call`. While absent from the default branch, use the branch's existing Rust dispatcher:

```sh
gh workflow run rust.yml --repo rich-netlist/rnl --ref ext/wasm-size -f wasm_size=true
```

It checks out frozen source separately from the harness, installs tools on the hosted runner, applies/restores prototypes and uploads text/JSON even on failure. It does not publish/deploy. The [README](scripts/wasm-size/README.md) documents selected variants; for attribution only:

```sh
gh workflow run rust.yml --repo rich-netlist/rnl --ref ext/wasm-size \
  -f wasm_size=true -f 'wasm_size_groups=["analysis"]'
```

Download the four artifacts and regenerate the durable JSON without compilation or WASM execution:

```sh
python scripts/wasm-size/summarize.py PROFILES_DIR CODE_DIR BINARYEN_DIR ANALYSIS_DIR --out wasm-size-results.json
```

The main matrix used harness `903b838d89c3f3ecd3b748cf2f1a1b92f12a553f`; the analysis receipt records its later revision. Current tooling additionally fails a run if an invoked validation suite exits nonzero and records Python/zlib versions for future runs. These changes do not alter frozen product source or previously measured variants. Only one draft PR was opened on `ext/wasm-size`; no main push, merge, tag or publication is part of this investigation.
