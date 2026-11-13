# Line layout

RNL 0.4 files can request `@SOLVER-CONFIG(layout="lines")`, or callers can select it through the solver options. The default remains rows. The linked library, `rnl solve`, and the independent solver use the same engine.

| Group-tree layout | Line layout |
| --- | --- |
| ![Before](images/line-layout-before.png) | ![After](images/line-layout-after.png) |

This synthetic example has four two-pin symbols in two groups, with three routed nets. Both runs use the seed pass and the same drawing objectives. The line layout moves the branch farther from a pin, increasing clearance and the drawing's width. This illustrates a routing tradeoff, not a time benchmark.

The engine first builds group tables, then settles groups from the inside out, routes on ordered lines, and compacts the drawing while retaining the resolved facts. It compares the completed result with the tree layout under the line-layout order, which includes pin clearance as well as crossings and bends. A preferred result can therefore have a larger area. Failed compaction or exhausted search falls back to a completed candidate. Group spacing uses the main runtime's nesting and explicit-zero rules. Final port cleanup checks the original indexed facts, including complete orientation lists.

An intermediate line arrangement may establish the required order before it satisfies an equality spanning several groups. Strict compaction retains such fixed bounds as mandatory constraints and solves them before accepting the candidate. Disjunctive facts still require a satisfied branch, which compaction retains. A failed or interrupted repair is rejected; completed coordinates are checked against every selected retained fact before ranking.

Time and work limits cover the same sequence of positive-cost checkpoints. Standalone solver 0.4.6 gives initial tree refinement a fixed operation-cost estimate of 5,000,000,000 units. A reported coordinate pivot costs `4,754 + 157 × n` units and an A* expansion costs `114`, where `n` counts every object in the level problem; arithmetic saturates at 64 bits. Semantic objects without a drawing still contribute to the coordinate problem and are included. Adding drawing metadata to an existing object does not count it twice. Initialization contributes to the estimate but is completed under the shared budget before optional refinement can stop. The estimate is checked before trying another tree arrangement. The shared time/work budget still interrupts the interior of an arrangement, and line search uses what remains.

These are fixed scheduling weights, not a wall-clock deadline or a promise of five elapsed seconds. They do not depend on the requested seconds or total work. This replaces the preceding refinement allowance based on raw tableau-sized work, which could allocate different shares merely because a problem encoded more constraints. Tree and line stages keep their own completed results; exhausting the tree estimate does not exhaust the whole solve. The estimate and its coefficients are part of the replay policy, so replay requires the same solver build.

Standalone solver 0.4.2 omits zero-distance auxiliary rows for isolated origin coordinates already at their target. It retains their exact optimum with direct bounds for the objective layer; the normal layer rollback before grid rounding removes these bounds as before. Coordinates coupled to existing rows, relative targets, or repeated origin targets keep the full objective. This reduces constraint-processing work without changing the lexicographic objective; replay still requires the same build.

The first tree candidate is initialized under the shared budget. If initialization already used the estimate, refinement is skipped. This prevents a small stage estimate from turning an otherwise feasible initial route into forced wires. The global time or work limit still interrupts initialization.

Seed and unlimited modes retain their own stage behavior and canonical settings. Time and work results persist the admitted work count as their canonical limit. A safety timeout remains explicitly non-reproducible.

Orientation enumeration checks the shared budget before measuring each choice. Combination counts and route-window areas use fixed-width arithmetic on native and WebAssembly builds. In line-search tie breaking, a single-item list has the same canonical identity as its item: adding empty layers of row or column wrapping cannot count as an improvement or keep the seed search running indefinitely.

Routing-window preparation also consumes the shared budget while filling obstacle and clearance masks. An interrupted window is discarded before routing uses it, preserving the last completed candidate.

Crossing measurements consume the same budget during rerouting and final route scoring. A cutoff during those scans rejects the unfinished route rather than treating its partial crossing count as a completed score.

Auto wires with effective 0.4 source semantics call the runtime's shared default path (13.1), including whole waypoint chains and generated content. Unobstructed, unmerged two-anchor auto routes use that same path: bends first, then length, with the runtime's tie order. Existing wires and obstacles prevent that substitution. Compaction and port cleanup recheck the default at the final coordinates before reporting its score and skeleton. This new core rule can change pictures and scores compared with the experimental solver.

Writeback materializes the chosen path and junctions even after search stops; this is deterministic output conversion, not further layout search. Unevaluable written geometry is fatal and cannot replace an existing record. Independent routing trees remain independent despite matching net names or coordinates. A shared real anchor combines directions only at that identity, and a pin-position reference never acquires an electrical pin lead.

`SolveReport.skeleton` describes the final group tables and line identities. The independent solver sends it in protocol version 5. After port cleanup changes geometry, the description is refreshed from the final coordinates. Visible interface details use the main runtime's symbol, column-text, baseline, and position models.

Materialized annotations remain obstacles during routing and compaction. Their bounds are recomputed after coordinate changes, including text that depends on its anchor. Completed candidates, spacing and port cleanup retain the previous drawing if annotation clearance worsens. Unevaluable annotations retain the runtime's fatal reporting and cannot be written back. Existing junctions are reused by point identity, including physical pin aliases; coordinate coincidence alone does not reuse a junction.

Orientation lists retain only the group's class and open-way values. Search deduplication uses two domain-separated 64-bit fingerprints of exact geometry, with a fixed-width encoding shared by native and WebAssembly. Fingerprints affect candidate membership, never ordering or hard-fact validation; a collision can skip a candidate.

Each enumerated list holds at most 200,000 combinations. All live enumerated lists together hold at most 800,000, including lists still retained by a search core after cache eviction. Conservative, architecture-independent storage accounting further limits one list to 64 MiB and live lists to 256 MiB. These are limits for combination storage, not a process-wide memory guarantee. Reaching a storage limit marks the list incomplete and preserves its admitted prefix.

Every orientation trial pays its size-dependent listing cost before allocating geometry, through the same checkpoint in time and work modes. Interrupted listing therefore counts as work, and replay stops before the same trial. Output materialization runs after the search limit; profiling reports auto-default refresh, branch attachment, and materialization separately from search.

Standalone solver 0.4.3 propagates coordinate equalities between retained DISTRIBUTEs sharing the same endpoints. Members at the same fractional position align, including reversed spans and different subdivision counts. These consequences guide an inner group even when the span endpoints belong to its parent; they do not add or weaken authored facts. Coordinate aliases and singleton sets use their modeled identity, and inference storage grows linearly with input members. Positive inequalities inside an already merged coordinate class are reported as conflicts immediately instead of consuming the relaxation limit. Both changes use the shared work budget, so work counts require the same solver build for replay.

Standalone solver 0.4.4 preserves structural line-merge hints from semantic usages that have a coordinate object but no symbol. Nested roles naming these objects expand to their members; a virtual part drawn with a symbol remains a single object. This restores the same role-based hints without consulting definition names or adding constraints. The traversal uses the shared work budget, and legacy usage handling stays unchanged.

Standalone solver 0.4.5 uses the same electrical pin identity for terminal and physical symbol-pin references when estimating bends to order orientation candidates. A coordinate-only reference to a pin remains a position and supplies no electrical endpoint. The estimate and its tie order are otherwise unchanged; work replay requires the same build.

## Cost calibration

Local orientation filters inspect the actual candidate after applying authored `ORIENT` and `ORIENT-RELATION` alternatives. This includes choices that have no free orientation class. Same-symbol `ALIGN`, directed `ORDER`, and directed `DISTRIBUTE` constraints therefore filter invalid directions before routing and compaction. These checks use the shared work budget and are part of standalone solver 0.4.2's replay policy.

Standalone solver 0.4.0 introduced the following fixed line-search coefficients, retained in 0.4.1. They supplement the shared model, pivot, structure and routing checkpoints; work units are a replay protocol, not elapsed nanoseconds.

| Operation | Fitted cost coefficient (ns) |
| --- | ---: |
| Choice | 12,434 |
| Relaxation | 102 |
| Route | 7,907 × objects |
| A* expansion | 217 |
| Merge | 378,800 |
| Merge pivot row | 180 |
| Merge relaxation | 102 |
| Listing trial | 2,908 × objects |

Calibration used the optimized Windows x64 build from [this source](https://github.com/rich-netlist/rnl/commit/edff9a1cadba40f6c7693eacba38b19063ab6378), on an Intel Core i7-12700KF. The process ran at normal priority on one logical CPU. Four- and eight-symbol shared-net groups, a two-symbol link, a four-symbol fan and eight fixed symbols were each run twice in seed mode. Groups of sixteen and thirty-two symbols were each run twice with a ten-second limit. All fourteen unprofiled repeats or work replays reproduced the saved files byte for byte.

The fit used 7,582 completed spans: 6,574 choices, 497 routes, 497 merges and fourteen settling spans. Child route time was removed from choices; direct choice, merge and weighing time was removed from settling. Nonnegative least squares with relative residuals and a soft-L1 loss (scale 0.2) estimated the coefficients; relaxation and merge relaxation share one coefficient. The route intercept fitted to zero. Coefficients were rounded to integer nanoseconds. Median relative residual was 5.8% overall, 12.3% for routing, 11.0% for merging and 36.2% for settling. Profiling overhead and variation between input shapes remain part of these approximate estimates.

Six control runs of the preceding optimized build bracketed the measurements. Their median line time differed by -4.5% from the earlier control, within the preselected 15% drift limit; all controls were retained. The timed synthetic solves took 10.006–10.032 seconds inside the solver and at most 10.266 seconds as a process; their unprofiled work replays took 7.53–9.25 seconds as a process. These observations validate those inputs, not a universal ten-second bound.

The line tally converts each coefficient to thousandths of a work unit with `floor(ns × 1000 / 1050)` and carries fractional totals between charges. Listing is admitted before each trial: for `n = max(1, objects)`, its checkpoint costs `4 × n + ceil(2769 × n / 1000)` units, including the existing structure cost. Tests pin the coefficients and the atomic admission boundary. Recalibrating this policy requires work counts from the new solver build.
