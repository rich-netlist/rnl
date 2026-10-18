# rnl-solver

The solver consumes a runtime LevelProblem, resolves its geometric facts, places finite candidates and completes requested identity-bearing WIRE/ROUTE output. The host owns configuration lookup, clocks and record insertion.

```rust
let solved = rnl_solver::solve(problem, &options);
let hash = rnl_runtime::problem::sig3(problem, &options.canonical());
let written = rnl_runtime::solution::write_record(problem, &solved.solution, rnl_solver::AUTHOR, &hash);
```

See [configuration](../../docs/solver-configuration.md), [CLI](../rnl-cli/README.md#solve) and [runtime input](../rnl-runtime/README.md#for-a-solver).

## Interface, configuration and reports

solve/resolve and their _with_clock variants retain 8.5 fact resolution: facts inconsistent alone fail; others are retained/discarded in authored priority/order with minimal conflict groups, completeness and statement hints. Only these checks use the 5-second default time limit. Convenience functions use StoppedClock; hosts provide a real clock. Same effective input/options with no reached clock limit are deterministic.

The crate does no filesystem lookup. Hosts select explicit --solver-config, nonempty RNL_SOLVER_CONFIG, rnl-solver.toml in executable/resource_dir, then builtin only when that automatic file is absent. Chosen unreadable/invalid files are errors. DEFAULT_CONFIG includes config/rnl-solver.toml rather than a second list. Missing keys inherit defaults; omitted root level inherits standard levels, root level=[] disables all. Root keys precede tables. Exact positive weights and nonnegative level tolerances never pass through f64. Noninteger measures occupy a level alone; fractional weights do not change integer classification. Duplicate normalized(kind,parameters) is rejected while parameter variants coexist.

Each level is the exact weighted sum; successive minimum+tolerance windows decide priority, with canonical candidate code for final ties. No fixed overlap/crowding/misleading/unrouted score precedes configured levels. Rules control mechanisms: grid, separation/gap, clearance, beams and caps; texts_at_anchor=true remains mandatory. --solver-options overrides rule scalars/counts only; objectives live in TOML. Canonical n/d summaries are readonly, not editable decimal literals.

SolveReport includes fact decisions/groups/hints, errors/warnings/notes, routing_fallbacks, exact scoring and Work. Score entries include kind/normalized parameters/raw/weight/weighted/level/tolerance/estimated/inactive/diagnostics plus canonical config/digest. Different time_limit legitimately changes config/digest even if every other report field agrees. Sig3 includes new semantic/config fields; historical sig1/sig2 projections remain fixed. CLI adds revision and original Provider-decoded UTF8 dependency hashes, separate from raw bytes. `rnl solve` exits with 1 on input, configuration, solve or file-I/O errors; malformed CLI arguments exit with 2. Other commands retain their own exit-code rules.

Candidate N>=1 caps actual attempts including P0; zero permits only necessary P0 and reports 1. No per-node bootstrap exemption; exhausted admission stops future preparation. Hard/soft expansions are independent per net and repeated per routed candidate. WIRE/ROUTE always produces output: each net completes hard->soft->fallback before later nets, with truthful routing_fallbacks phase/reason. Local soft costs are declared approximations; final scores are measured from output. Zero-coordinate identity edges keep distinct endpoints; coincidence alone never connects. Rational arithmetic retains the existing runtime f64 geometry/writeback boundary.

## Explicit semantic inputs

CURRENT only counts marked in/out terminals with symbol PIN-written directions, transformed by actual orientation and reversed for incoming current. Unmarked/unspecified/absent directions and generated default-box directions do not count. No electrical-type or polarity name guesses remain. symbol_direction uses literal configured names; missing names are inactive. Ambiguous single-pin or multi-pin directions require reference/references, distinct from CURRENT's explicit-direction rule.

## Step1 capability and temporary mechanisms

This branch supports13 kinds: symbol_overlaps, wire_shorts, wire_overlaps, label_overlaps, current_direction, symbol_direction, symbol_alignment, symbol_distribution, wire_turns, wire_crossings, area, aspect, wire_length. Symmetry, pin_wire_alignment, orientation_r0 and origin_distance are configuration errors and absent from its default.

Step1 retains positive-area symbol overlap, current partial drawn-net pin shorts, positive-length box-interior wire incidents, texts_over approximation, legacy polyline/exit turns, different-net crossings and component-box area. Aspect uses |w-r*h|/(w+r*h), zero denominator 0 and integer-pair ratio. Alignment counts distinct origin coordinates; distribution sums max(0,gap kinds-1) per axis; group/global scope is configurable. Length is Manhattan with pre-route MST estimate.

The four-stage legacy coordinate LP (including resolved-witness distance), top-two ranked local orientations and straight unrevised auto WIRE are declared temporary mechanisms. Settings selects config/file and scalar rules only. Hard fixed-cost routing remains only an A02 compatibility seam; production hard/soft derive finite exact configured costs. Legacy row-gap/own-box lead/text-anchor approximations are documented mechanisms, not hidden priorities. Soft/fallback completion is not a collision-free or globally optimal guarantee.

## Verification and limitations

Exact config/CURRENT/sig3-history/name-free literals/admission/identity-routing have source regressions; actual complete gates/corpus comparisons belong to the acceptance record. Finite search and legacy LP may miss a better legal layout. Significant runtime/quality changes require input/config/bin-bound evidence, never a lower-priority-area excuse. No runtime/global optimum guarantee is made. True obstacle routing of author auto WIRE remains outside this revision.
