# Optimized solver measurement tools

These tools collect bounded Windows GNU CLI evidence with unchanged solver
defaults. Ordinary pull-request CI runs the file/mock checks and an owned-process
smoke test; it does **not** start a performance collection. Existing Rust,
WebAssembly, golden and conformance checks remain enabled on pull requests.

## Frozen inputs and historical comparisons

`manifest.json` retains the entire selection and every SHA-256 from PR #152:
11 SHA-distinct representative inputs and four scale families, each at
n = 2, 4, 8, 16, 32, 64, 128, 256, 512 and 1024.

| Family | Meaning of n | Host devices |
| --- | --- | ---: |
| chain | Resistors in a chain with a FLOW fact | n |
| diffmirror | Grouped five-transistor differential-pair/mirror cells | 5n |
| star | Resistors sharing a hub, with ALIGN and ORDER facts | n |
| disjunctions | Independent bounded, two-way x-distance facts | 2n |

Only the 11 representatives are stored here. `prepare_inputs.py` generates all
40 synthetic inputs with fixed parameters, names, order, encoding, LF endings
and final newline. It reads the 25 standard packages with
`git show <library_source.commit>:libraries/standard/...`; no second standard
package tree or generated scale netlists are committed.

The package source is fixed to `0e9b63ecc84b87fdaa69b31251ba683b740fb9bf`,
independent of the revision being measured. The historical Windows package
copies used **CRLF**, while their source Git blobs used LF. The manifest therefore
explicitly fixes `library_source.line_endings` to `crlf`; extraction restores CRLF
before checking every original SHA-256 and byte count. Representatives keep their
original bytes; generation always writes binary bytes, independent of host OS.

`test_inputs.py` compares **all bytes** of every generated input directly against
the 40 original Git blobs at PR #152 commit
`3899379423891d7288583364e3fa2acf5701b09d`, including n=1024. It also compares
the prepared 11 representatives and 25 packages against that commit's original
copies. The original manifest SHA-256 is
`ce445cca7c7ae826a19d4f7dca3276fca24f2e7767b97a07d55bb858e6c9d384`.
The new manifest records that identity and the extraction provenance; its own
file hash changes, but all input hashes and collection parameters stay unchanged.
Thus old raw evidence remains comparable by input, package, source, binary,
toolchain and canonical-option identity. Timings from different machines or
incomplete profiles must still be identified as such.

## Manual entry point

Use **Actions → Rust → Run workflow**. All three special-mode switches default
to false. Enable exactly one of the following modes and provide full, immutable
40-character public `rich-netlist/rnl` commit SHAs:

| Input | Other inputs | Action |
| --- | --- | --- |
| `solver_performance=true` | `baseline_sha`, optional `candidate_sha` | Build, then collect the representative/adaptive matrix |
| `solver_profile_work=true` | `work_candidate_sha` | Build, then collect M04 once with `--profile-work` |
| `solver_build_only=true` | `work_candidate_sha` | Build one downloadable optimized CLI; no solver invocation |

Before merge, the existing Rust workflow can dispatch this branch explicitly:

```sh
gh workflow run rust.yml --repo rich-netlist/rnl --ref ext/solver-measurement-tools \
  -f solver_performance=true \
  -f baseline_sha=0e9b63ecc84b87fdaa69b31251ba683b740fb9bf \
  -f candidate_sha=PUBLIC_CANDIDATE_40_CHARACTER_SHA
```

Omit `candidate_sha` for baseline only. For a build without measurement:

```sh
gh workflow run rust.yml --repo rich-netlist/rnl --ref ext/solver-measurement-tools \
  -f solver_build_only=true -f work_candidate_sha=PUBLIC_CANDIDATE_40_CHARACTER_SHA
```

After merge, select `main` or pass `--ref main`. A work diagnostic uses the same
command with `solver_profile_work=true` instead of `solver_build_only=true`;
its selected product revision must support the work-diagnostics schema.
No dispatch accepts a patch, source archive or alternate repository. These tools
provide future collection capability; introducing them does not request a new
measurement campaign or upload local-only objectives work.

The reusable jobs build sequentially using the default release profile, pinned
Rust 1.97.1, target `x86_64-pc-windows-gnu`, jobs=4 and `windows-2022`. All matrix
builds finish before measurement. Two resource samples at least five seconds
apart require 8 GiB free and no compiler, solver or browser driver. Every solver
command runs sequentially in an owned Windows Job, with cleanup receipts, an
empty working directory and no `RNL_SOLVER_CONFIG` override.

## Collection scope and interpretation

* Each representative gets one **off** command capped at ten seconds, followed
  by one profiled **on** command. M04 and PFD on have a 120-second diagnostic cap;
  other representative on commands have ten seconds. There are no retries.
* Each scale family doubles from n=2. Off has ten seconds. A complete off gets
  one ten-second on, then continues; the first off timeout gets one diagnostic
  on capped at 120 seconds, then stops the family. Other off errors mark on
  NOT_RUN and stop the family. n=1024 is a queue guard, not a measured boundary.
* The cap covers CreateProcess through owned-tree exit, including CLI writeback.
  Compile, resource waits, CLI profile time and solver subtotals remain separate.
  Late returns are censored failures. Partial profiles preserve the last observed
  boundary; inclusive nested spans must not be added to exclusive stages.
* The M04 work mode runs exactly one profiled command capped at 120 seconds, with
  work counters and spans. It reuses existing baseline/off evidence and gives no
  off ten-second verdict. Complete output equivalence requires comparison with
  preserved prior outputs after download.

A green collection means valid collection evidence, **not** performance
acceptance. Only a complete optimized off command within ten seconds passes the
performance field. On completion cannot rescue off timeout. Missing defaults,
malformed profiles, errors, incomplete equality and observed memory pressure stay
explicit. Each point is one observation, not a repeat distribution.
`comparison.json` distinguishes common completed points, measured timeout-to-
complete transitions, regressions, errors and adaptive NOT_RUN points. Complete
outputs are compared as actual bytes and hashes; diagnostic work excludes timing.
This describes one Windows GNU runner, not a universal desktop or Web guarantee.

## Downloading results

Open the completed Rust run in **Actions**, then its **Artifacts** section.
Artifacts include the run ID and attempt and are retained for **14 days**:

* `solver-performance-binaries-RUN_ID-ATTEMPT`: optimized EXEs and build receipts,
  uploaded before the first measured command.
* `solver-performance-RUN_ID-ATTEMPT`: final matrix evidence, including EXEs,
  frozen manifest, source/compiler/binary identities, stdout/stderr, exact solved
  files, raw complete or partial profiles, timing/memory/resource observations,
  cleanup receipts, per-point measurements, summaries and `comparison.json/md`.
* `solver-build-RUN_ID-ATTEMPT` or `solver-m04-work-RUN_ID-ATTEMPT`: build-only or
  M04 work evidence; the latter retains completed or partial diagnostic records.

The final upload runs even on failure. Preserve these downloads before retention
expires. For example:

```sh
gh run download RUN_ID --repo rich-netlist/rnl \
  --name solver-performance-RUN_ID-ATTEMPT --dir OUTPUT_DIRECTORY
```

## Lightweight offline checks

Fetch the two `historical_source.commit` / `library_source.commit` pins once:

```sh
git fetch --no-tags origin 3899379423891d7288583364e3fa2acf5701b09d
git fetch --no-tags origin 0e9b63ecc84b87fdaa69b31251ba683b740fb9bf
python scripts/solver-performance/test_inputs.py
python scripts/solver-performance/test_benchmark.py
python scripts/solver-performance/test_work.py
python scripts/solver-performance/prepare_inputs.py --output NEW_DIRECTORY_OUTSIDE_REPOSITORY
python scripts/solver-performance/benchmark.py --fixtures NEW_DIRECTORY_OUTSIDE_REPOSITORY
```

These checks compile nothing and call no solver. The real owned-process smoke
uses only Python fixture processes and is explicitly Windows CI-only.
`ci_build.py` rejects local execution. The files here change measurement tooling
only; solver code, objectives, defaults and specification text are unchanged.
