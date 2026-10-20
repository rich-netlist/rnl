# Optimized solver measurements

This is a bounded measurement collection, not a replacement for solver, golden or
conformance tests. The fixture manifest freezes 11 SHA-distinct representative
inputs, four geometric scale families (n = 2, 4, ..., 1024), and the same 25 package
files used in the previous profiling campaign. No solver defaults are overridden.

The existing **Rust** workflow provides the manual entry point on an unmerged
`ext/` branch; its default-branch workflow already supports `workflow_dispatch`:

```sh
gh workflow run rust.yml --repo rich-netlist/rnl --ref ext/solver-performance-bench \
  -f solver_performance=true \
  -f baseline_sha=0e9b63ecc84b87fdaa69b31251ba683b740fb9bf \
  -f candidate_sha=PUBLIC_CANDIDATE_40_CHARACTER_SHA
```

Leave `candidate_sha` empty for the initial baseline alone. Both refs must be
immutable commits in the public repository. This entry point accepts no source
patch, archive, local branch or alternate repository. Local-only objectives work
must never be uploaded to this workflow or its artifacts.

The reusable workflow builds the CLI with Cargo's default release profile, pinned
Rust 1.97.1, Windows GNU target and jobs=4 on **windows-2022**. All requested builds
finish before measurement. Two resource samples, at least five seconds apart,
must each have at least 8 GiB free and no compiler, solver or browser driver.
The job records OS, CPU, compiler, source and executable SHA, fixture SHA, options,
outputs, raw profiles, process memory, resource samples and owned-tree cleanup.
The optimized EXEs and build manifests are uploaded for later same-machine use.

## Fixed initial measurement policy

* Each representative gets one off command capped at **10 seconds**, followed by
  one on command. M04 and PFD on have a 120-second diagnostic cap; other on have
  ten seconds. There are no automatic retries or repeated medians.
* Each family starts at n=2 and doubles. At each n, off has ten seconds. If off
  completes, on has ten seconds and the family continues. At the first off
  timeout, that point gets its single on observation with up to 120 seconds for
  diagnosis, then the family stops. An input/error stops that family separately.
  Other off errors retain the off observation, mark on NOT_RUN and stop that
  family. Reaching n=1024 below ten seconds is an explicit queue guard, not evidence of
  reaching the performance boundary.
* The cap and the performance field cover CreateProcess through owned-tree exit,
  including controlled startup and CLI writeback. Resume-to-exit, CLI profile
  time and solver subtotal remain separate. Compile, resource wait and outer
  environment setup are excluded. A late return is a censored failure.
* Only a complete **off** optimized command at or below ten seconds passes the
  performance criterion. An on completion never rescues an off timeout. An off
  completion with on timeout has unknown complete equality and overhead; the
  partial is only the last observed work boundary. Inclusive nested spans must
  not be added to exclusive stages.

A green collection job means that the harness returned valid evidence. It does
**not** mean all examples met ten seconds or all outputs matched. The uploaded
summary and job summary explicitly retain those failures. Standard CI tests stay
separate, and the pull request must still have green ordinary checks. No workflow
merges a PR or enables auto-merge.

The summary records collection completion, every observed off and any unpaired
off if on collection fails. Missing/malformed canonical default options or bad
profile/timing relationships are explicit validation errors. Observed memory
pressure remains marked and excludes a point from clean speed comparisons.

For a candidate, `comparison.json` separates common passing points, newly completed
points and observed regressions; complete common off outputs are compared as
actual bytes and record/file hashes. On metrics exclude timing when comparing
work, and remain unknown when incomplete. Different adaptive terminal n are not
silently pooled. Each point has only one observation; results describe this
Windows GNU runner, not a universal Windows desktop or Web performance guarantee.
Candidate-only points beyond the baseline's adaptive stop are reported as
baseline NOT_RUN, not as measured timeout improvements. Build EXEs and metadata
are uploaded once before measurement; the final evidence artifact is always kept.

File-only checks (no cargo or solver):

```sh
python scripts/solver-performance/test_benchmark.py
python scripts/solver-performance/benchmark.py --fixtures tests/performance
```

The real owned-process natural/timeout/partial fixture is explicit and CI-only.
`ci_build.py` rejects local execution. Local release builds require their own
explicit user authorization and a separately assigned resource token.
