# Step1 tiny-budget work audit (read-only source analysis)

Historical pre-R7 analysis. Independent review subsequently classified the per-node bootstrap allowance below as P1, rather than an authorized exception. That obsolete full tiny round was stopped with preserved evidence; R7 replaces the candidate admission policy as recorded in `objectives-v2-step1-budget-r7.md`. This audit's old work bounds and observed populations describe the old binary only. No pass or timeout conclusion follows from its partial execution. The audit itself changed no production and started no workload.

## Actual test population and independent units

`driver::tests::a_tiny_budget_still_gives_a_valid_solution` reads the seven T files directly under `tests/integration/solver` (not its 0.3 subdirectory), including ota5t/opamp sublevels: nine levels. `check_capped` solves each level twice to check complete deterministic equality, so a successful round performs eighteen solves.

The options are candidates=0, hard A* expansions=12, with independent soft_expansions=200000 unchanged. Internal candidate resolutions/LP use Caps{nodes:2,pivots:30}. These caps reset for each Resolver/LP call; they do not bound aggregate SolveReport.work. Initial 8.5 resolution uses the explicitly disabled time limit and StoppedClock, not these two internal caps; its exact written witness uses the normal CAPS. Finite fact/literal enumeration is not a small wall-time guarantee.

`phased_layout` allocates a fresh hard budget and one shared remaining soft budget for each net, for every candidate routed. The soft budget is not reset for each join of the same net. Every charged expansion decrements remaining once. For R routed candidates and J net jobs, the aggregate charged routing bound is R*J*(12+200000); check_capped repeats this work. T4 has J=7 and R<=4, hence 5,600,336 charged expansions per solve is a safe upper bound, and 11,200,672 across the deterministic pair. The minimal actual T4 diagnostic measured only 54,156 across all candidates and 10,112 on the chosen candidate. Those are different scopes.

Soft expansion cost is not constant across inputs: each transition computes exact rational lengths/costs, examines boxes, physical pins, and prior output-wire segments, and clones path-identity penalty sets. Heap stale-label pops, initialization of source/target attachments, component bookkeeping, collision/score recomputation and deterministic fallback do not all count as expansions. Their loops are finite but can add substantial CPU per charged step.

## Search work not represented by the public candidates number

`Search::made` increases only at `Search::estimate`. `orient -> ranked_values` evaluates all eight orientations of every local class without increasing that counter. Barycentre order generation can run eight rounds before the normal neighbourhood cutoff.

At zero candidates, node/child bootstrap code deliberately allows the first estimate through `out_of_budget && !tried.is_empty` (and the analogous next-population guard). Thus zero is currently not literal zero work or only P0. With N normalized tree nodes and C free classes, a conservative structural bound for the zero-budget bootstrap is about 2N-1 calls to Search::estimate, plus at most 72*C full configured orientation-array estimates (144*C also remains a safe looser bound allowing two bases). P0 and the optional EvenRows twins are counted separately. At defaults the zero-budget population can contain four placed/routed seed candidates; T4 in fact reported four and work.candidates=8.

This is an explicit documentation/review point: existing text saying the search makes "at most candidates" does not describe the mandatory seed exception precisely. The observed counter alone does not prove unbounded enumeration, but it also must not be presented as a strict total CPU-work cap. No change to this legacy bootstrap or test budget is made by this audit.

## Evidence and limits

The one-solve T4 diagnostic is `target/objectives-v2-validation/tiny-t4-owner-exemption-01.json`, SHA256 29afd729e3d8e11287a321c8cf2b807aaa55a4d4619073c542d72b415ca68afa. It took 167.93 s test wall time, work 859 resolution nodes /1205 pivots /8 candidates /54156 A* expansions. Its route4 is the accepted zero-length connection of distinct identities, unrelated to this performance audit. It did not profile stages, so it does not establish how much time each source consumes.

The full pending test's roughly 787 s wall/775 s CPU and stable ~24MB process working set show active CPU cost, not which case or phase it has reached. Existing output has no per-case start/end trace; no current phase is inferred from PID, CPU or RAM. Attaching an intrusive debugger or rerunning the whole test was not proposed. Future already-required full gates can include test-only per-case progress logging if separately authorized.

Reopen performance/correctness investigation if observed per-net phase work exceeds its true budget unit, if work/memory growth is not explained by these finite populations and phases, if an input newly fails or does not complete, or if full-corpus data exposes another unexplained tail. A finite-loop argument by itself is not acceptance of arbitrary latency. The completed T6 investigation remains limited to its own input/config/binary and must not be generalized into a stage profile for this tiny test.
