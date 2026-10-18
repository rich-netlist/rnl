# Objectives v2 implementation contract

Status: Step1 production is frozen at R7 candidate02 for concentrated validation. Candidate08 remains the original performance/quality evidence, with actual default T1/T6 equivalence checked after R7; corrected low-budget paths require new execution. Original authorization was on2026-10-18. This document records the shared contract for implementation packages A, B, and C in `C:/Users/jielu/.codex/worktrees/cdac/rnl`, branch `codex/objectives-v2-step1`, based on `41d6b7d68544601f7a206af69c5b91286a2ed7c9` and merged upstream `476578a74010cf37406f15fd82bd978298ad8248`. All packages have handed integration back to A; validation owns only explicitly approved test migration/correction files while holding the sole build token. Production changes require a coordinated correction window and new source evidence.

## Ownership

| Package | Owner | Exclusive files |
| --- | --- | --- |
| A: configuration, objective model, search and integration | implementation_lead | `crates/rnl-solver/src/{config,options,rational,objectives,score,structure,search,coords,driver,lib,model,resolve}.rs`, solver tests other than routing tests, `config/rnl-solver.toml`, workspace and solver Cargo manifests/lock, this contract and decision log |
| B: routing and identity geometry | routing implementation agent | `crates/rnl-solver/src/route.rs`, `crates/rnl-solver/src/route/tests.rs`, new `crates/rnl-solver/src/routing_geometry.rs` and its tests |
| C: runtime and hosts | runtime_host_step1 | `crates/rnl-runtime/**`, `crates/rnl-cli/**`, `apps/desktop/**`, viewer assets and viewer tests, CLI/desktop manifests (request workspace dependency edits from A) |
| Wasm and CI | B completed; A now integrates | `bindings/runtime-wasm/**`, `.github/workflows/**`; original handoff retained separately |
| Validation | validation_prepare | generated material below `target/objectives-v2-*`; sole cargo/Node build/Playwright token |

Do not edit another active owner's files. Request narrow changes from its owner. No rebase of pushed branches, uncoordinated merge, release, tag, or parallel cargo. Main advances and later-step propagation use the specifically required chain of merges. Preserve pre-existing `notes/reviews/objectives-v2-phase0-2026-10-18.md`.

## Public solver configuration API (A provides)

The existing `SolverOptions` name remains the options accepted by `solve`. Configuration parsing is filesystem-free and works on wasm.

```rust
pub const DEFAULT_CONFIG: &str; // include_str of config/rnl-solver.toml
impl SolverOptions {
    pub fn from_toml(text: &str) -> Result<Self, ConfigError>;
    pub fn builtin() -> Self; // same parser and validator as from_toml
    pub fn with_overrides(&self, text: &str) -> Result<Self, String>;
    pub fn canonical(&self) -> String; // normalized complete effective configuration
    pub fn without_time_limit() -> Self;
    pub fn validate(&self) -> Result<(), String>;
}
pub struct ConfigError { pub line: usize, pub column: usize, pub message: String }
// Display contains the location. Coordinates are 1-based.
pub type Exact = num_rational::Ratio<num_bigint::BigInt>;
pub fn exact_text(value: &Exact) -> String; // reduced integer or n/d; never f64
```

`Default` equals `builtin()`. `with_overrides` accepts only scalar mechanism/rule settings, including the established `budget=name:count,...` spelling; `objectives=` is an error. All configuration quantities and objective values retain exact rationals. UI/JSON uses `exact_text` strings for non-integer numbers. Clock measurements may be f64; scoring/configuration never derives its rational from an f64.

`SolverOptions` keeps flattened rule names for direct internal use: `tolerance: Exact`, `grid: Option<Exact>`, `time_limit: Option<Exact>`, `beam: usize`, `route_beam: usize`, `budget: Budget`, `separate_symbols: bool`, `separate_gap: Exact`, `route_clearance: Exact`, `texts_at_anchor: bool`, `hints_for_low: bool`, and `objectives: Objectives`. `Budget` adds `soft_expansions` and `symmetry_steps` to `candidates` and `astar_expansions`. `hints_for_low` affects reporting only and is omitted from canonical options.

The objective model is parameterized terms grouped into levels. Public `Layer`, `Objectives` names remain, but callers must migrate from the former `(name, u32)` tuples. C should avoid objective introspection for the Step1 settings UI; that UI edits only the config path and scalar rules. A will notify C before Step3's complete term-editing API is frozen.

## Runtime CURRENT/sig3 contract (C provides)

Add `LevelProblem.currents: Vec<ProblemCurrent>`, in stable semantic-terminal order. Proposed types:

```rust
pub enum CurrentTerminal {
    Instance { instance: usize, terminal: String },
    Port { terminal: String },
}
pub enum CurrentDirection { In, Out }
pub struct ProblemCurrent {
    pub terminal: CurrentTerminal,
    pub net: String,
    pub direction: CurrentDirection,
    pub display: Option<PointRef>,
}
```

`instance` is the stable netlist-order index of the semantic host instance, even when bundled and absent from `LevelProblem.objects`. `display` can point to its bundle's physical pin; every semantic terminal remains an independent entry. Final CURRENT is resolved before this mapping. Unspecified entries are omitted. Local ports use the explicit `Port` variant, are exported, and are inactive for scoring in this version. CURRENT accepts no virtual terminal, so no Usage identity variant is added.

Add `PinView.direction_written: bool`, propagated through symbol inheritance/overrides. Runtime-generated default pins are false. A only scores entries whose `display` points to a drawn symbol pin with this flag and a cardinal direction. C provides `sig3`, `canonical_sig3`, freezes explicit `canonical_sig1` and `canonical_sig2`, and dispatches `same_input` by tag. C migrates CLI and problem output to sig3.

## Routing contract (B provides)

Keep the current entry point `route(problem, structure, geometry, origins, options, work) -> Routing`. Existing `Routing.wires`, `.branches`, `.paths`, `.drawn`, `.expansions` remain available during integration. Replace `.left` and `joins_left` with `.fallbacks: Vec<RoutingFallback>`. A owns the public report type in lib.rs; B may define its internal equivalent and A converts it.

```rust
pub struct RoutingFallback {
    pub route: usize,
    pub net: String,
    pub phase: String, // "soft" or "fallback"
    pub reason: String,
}
```

Add `Routing.geometry: RoutingGeometry` in the new B-owned module. It contains authored fixed/auto wires and generated route wires, with exact coordinates. It must retain an output-wire identity, ordered vertices, point identity at each vertex, optional physical-pin identity `(object, pin)`, network names, and source (authored fixed, authored auto, generated). Coordinate equality never supplies identity. Point objects and physical pins use their problem indices; branch identities are shared across wires; bends have per-wire identity. Semantic bus terminals are not collapsed in CURRENT, but routing physical-pin identity is shared.

B should send A its concrete geometry struct declarations before completing route conversion. A consumes the geometry in scoring and never reconstructs connections from coordinates. Step1 retains today's turn/crossing definitions while establishing identity representation; Step2 introduces A05's new measures. Fixed author wires remain unchanged. Auto wires keep today's straight appearance with no rewrite in Step1.

For soft local costs B calls `options.routing_weights() -> RoutingWeights` (A provides), whose exact fields are `wire_overlaps`, `wire_shorts`, `wire_crossings`, `wire_turns`, and `wire_length`. Disabled terms contribute zero; related level priorities use a fixed factor of 16, finite weights only. Length and old deterministic tie rules remain the final search tie mechanism even when all terms are off, as A04 states. Final candidate scoring always recomputes the full configuration's real scores.

Preserve `route_layout(layout, hard_budget)` as the legacy hard-only seam for the A02 test. Add a new configured path rather than silently changing that test seam's costs/order/geometry. Hard and soft budgets are separate. Hard success must remain byte-for-byte `NetOut` equivalent under identical layout/order/costs.

## Host/report contract

C owns one shared filesystem resolver in rnl-cli. Precedence: explicit path, nonempty `RNL_SOLVER_CONFIG`, executable/resource directory default candidate, builtin. Only NotFound of the default candidate falls back. Relative paths use cwd. C reuses it for problem, preview, solve, and desktop, with scalar overrides applied last.

A adds `SolveReport.routing_fallbacks` and `SolveReport.scoring`; removes `unrouted`. `scoring` contains canonical effective config, config digest, and levels with exact-string raw term values, weights, weighted totals, tolerance, normalized parameters, estimated/real status, inactive status and diagnostics. C serializes these and adds invocation provenance (solver revision and input manifest), keeping human output useful. A will provide exact report struct declarations with the first API checkpoint.

## Dependency order and change notification

1. A creates exact configuration/public option/report API. B can implement legacy hard seam, identity types, and fallback mechanics in parallel. C can implement CURRENT/sig3 and host filesystem resolver in parallel.
2. A integrates CURRENT structure and B geometry using the agreed types. C connects config/CLI/desktop after A's public API checkpoint.
3. Each package reports files changed, unresolved API assumptions, and validation requests. Only the validation owner executes commands that invoke cargo, including Node wasm builds and Playwright setup.
4. An API change requires a coordinator message to A, B, C before either consumer is changed; A updates this file and records rationale. No silent changes to identity or exact-number interpretation.
5. After three packages integrate: one serialized local-check queue, adversarial review, fixes and review again, full gate, comparisons, then draft PR. Step2 starts from Step1; Step3 starts from Step2. All remain draft and unmerged.
