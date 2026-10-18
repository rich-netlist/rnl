# Solver configuration

Copy builtin: `rnl solver-config --defaults -o my-solver.toml`. Edit it, then `rnl solve circuit.sp -o solved --solver-config my-solver.toml --solver-options "time_limit=none" --report`. Without --defaults solver-config prints selected TOML/source; -o saves editable original text, not canonical fractions.

Precedence: explicit file, nonempty RNL_SOLVER_CONFIG, executable/resource_dir rnl-solver.toml, then builtin only if automatic file absent. Chosen unreadable/invalid files fail. Missing keys inherit defaults; omit root level to inherit levels or set root level=[] before [rules] to disable all.

```toml
format = "rnl-solver-config/1"
[rules]
beam = 4
time_limit = "none"
[[level]]
tolerance = 0
[[level.term]]
kind = "wire_turns"
weight = 0.1
[[level.term]]
kind = "wire_crossings"
weight = 2
```

Levels follow author order, sum positive exact weighted values and pass only their minimum+tolerance window to lower levels. Noninteger kinds must stand alone; fractional weights do not change integer classification. Same normalized kind/parameters duplicates fail; variants coexist. Term enabled=false preserves source while disabling its score. Decimal TOML numbers stay exact; ratio=[16,9] contains positive integers. Canonical n/d JSON is readonly, never an editable decimal. Errors show line/column.

Rules are mechanisms: grid/none, tolerance, separation/gap, clearance, beams and work budgets. Missing route_beam follows explicit beam; missing soft_expansions follows explicit astar_expansions. texts_at_anchor remains mandatory. Candidate 0 still runs required P0/work 1; independent per-net/candidate hard/soft caps do not remove fallback. Time limit 5 s covers 8.5 only; --report requests expensive low-priority conflict hints. --solver-options overrides rule scalars/counts, not objectives. No 0.3 @SOLVER_CONFIG directive.

Reports preserve revision, original Provider-decoded dependency hashes, canonical/digest and exact per-term metadata. Different configured time limit changes digest even if results agree. CURRENT has no guessed default electrical type. See the phase's [solver measures](../crates/rnl-solver/README.md).

Step1 supports13 kinds; symmetry, pin_wire_alignment, orientation_r0 and origin_distance are unsupported/absent from its default. Settings chooses file/rule scalars only; edit objectives in TOML. Legacy objective preferences drop once with notice; explicit rules remain. OK/Cancel do not write config files.
