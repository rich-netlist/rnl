# rnl-solver

The solver of RNL descriptions. It takes the problem of one drawing level, the `LevelProblem` that the runtime builds when it reads a document with `Options::solver()` (see [For a solver](../rnl-runtime/README.md#for-a-solver)), resolves its geometric facts as 8.5 says, places and orients its objects and routes its ROUTEs (13.7), and gives a `Solution`, which the runtime writes back as the level's record of the solver:

```rust
let solved = rnl_solver::solve(problem, &options);
let hash = rnl_runtime::problem::sig1(problem, &options.canonical());
let written = rnl_runtime::solution::write_record(problem, &solved.solution, rnl_solver::AUTHOR, &hash);
```

**Status: stage S2 of solver v1.** The crate, its options, its exact arithmetic and the resolution of 8.5 are in place: `resolve` keeps or discards each fact, exactly, with minimal conflict sets, and gives a witness, a placement that satisfies every fact kept. `solve` places every object where the witness puts it: nothing is searched, rounded to the grid or routed yet, and the report's notes say so. The stages that follow are structure, search and coordinates (S3), routing, scoring and the driver (S4), the `rnl solve` command (S5) and the end-to-end demonstration (S6).

## Interface

- `resolve(&LevelProblem, &SolverOptions) -> Resolved`: the resolution of 8.5 alone (see [Resolution](#resolution)): `kept`, `discarded` as in the report below, the `work` done, `notes`, and the witness, read with `position(object)`, `orientation(object)` and `point(problem, &PointRef)`.
- `solve(&LevelProblem, &SolverOptions) -> Solved`: the `Solution`, and a `SolveReport` of what became of each fact. 8.5 takes the facts in the order of `LevelProblem::facts`, which is their rank, and keeps each that fits with those kept before it. `kept` lists the facts kept; `discarded` each fact discarded, with `conflicts_with`, a minimal set of facts kept before it that it conflicts with, the facts that rank first preferred, and `undecided` when the budget for deciding it ran out. `inert_objectives` lists the objectives that are on but have nothing to work with; `work` counts what was done, as the budget counts it.
- `SolverOptions`: the tolerance (11.7, `geom::TOL` by default), the grid free coordinates are rounded to (1), the layered objectives, the beam (4) and the budget.
- `SolverOptions::canonical()`: every setting that changes the result, in a fixed order, numbers by `geom::fmt_num`. It is what the options digest of `sig1` hashes (chapter 17), so the same options give the same hash on every platform. `SolverOptions::parse` reads it back, and `with_overrides` reads some of its settings over other options:

  ```text
  tolerance=0.000001;grid=1;objectives=current_direction,turns,ground_direction,component_alignment,distribution,symmetry,terminal_wire_alignment,area,crossings,wire_length,aspect;beam=4;budget=resolve_nodes:10000,pivots:1000000,candidates:256,astar_expansions:200000
  ```

### Objectives

The objectives are configured in layers. Within a layer the objectives add up with their weights; layers compare lexicographically, the first first; two values of a layer within its tolerance count as equal and the next layer decides. An objective left out is off: it is not computed, does not order candidates and breaks no ties. In the canonical text the layers are separated by `,`, the objectives of a layer by `+`, a weight other than 1 follows `*`, and a tolerance other than 0 ends the layer after `~`, as in `signal_flow,area+wire_length*2~0.5`; `none` is no objective at all.

The objectives are `current_direction`, `ground_direction`, `signal_flow`, `component_alignment`, `distribution`, `symmetry`, `turns`, `crossings`, `wire_length`, `area`, `aspect`, `terminal_wire_alignment`, `labels` and `bend_alignment`. The default is one layer each, weight 1, in the prototype's order: `current_direction`, `turns`, `ground_direction`, `component_alignment`, `distribution`, `symmetry`, `terminal_wire_alignment`, `area`, `crossings`, `wire_length`, `aspect`.

### Budget

Work is counted in steps, not time, so the result does not depend on the machine: `resolve_nodes`, the nodes of the search that decides one fact; `pivots`, the pivots of the simplex over the level; `candidates`, the candidates of the search over the level; `astar_expansions`, the cells the router expands for one net. When a count runs out the solver keeps the best it has, and says so in the report.

## Exact arithmetic

Every decision is taken with exact rationals (`num-rational` over `num-bigint`), never by comparing floating point, so native and WebAssembly builds decide alike.

- A number of the problem is read as its shortest decimal representation, the digits `fmt_num` writes, so `0.1` is 1/10. A result goes back as the nearest `f64`, ties to even, so a number that came in goes back unchanged.
- The tolerance (11.7) widens only the constant bounds of the facts: `x = v` becomes `v − tol ≤ x ≤ v + tol`, the `min` of DISTANCE is lowered and its `max` raised by it, and so is the one unit of ORDER; equalities between variables, such as ALIGN and DISTRIBUTE, stay exact.
- The simplex is incremental and bounded, after Dutertre and de Moura: each linear form is one row, made once; a fact is bounds on rows, each with the tag of the fact. A check moves the variable whose pivot costs least, one with no bound in its way first, then the one in the fewest rows, so a row added late, such as `t ≥ |xᵢ − xⱼ|` for the wire length, is fixed by one pivot on its own `t`; once variables have left the basis again 1000 times within one check it takes Bland's rule, which terminates. The same input gives the same pivots. When the bounds have no solution, the tags of the bounds that hold the failing row are the explanation, a set of facts that conflict. Bounds are undone back to a checkpoint, and the same tableau minimises linear forms for the layers of the coordinate LP, with Bland's rule, each objective a row whose reduced costs the pivots keep up to date.

The tests compare the simplex with complete deciders on systems made by a fixed pseudo-random sequence: Bellman–Ford on difference constraints, Gaussian elimination on equalities of three variables, Fourier–Motzkin elimination on small systems of any linear constraints and on the optimum of linear objectives. Each explanation is checked to have no solution by itself, whether a check pivots by cost or by Bland's rule, and a tableau whose bounds were asserted, undone and asserted again answers as one built afresh. One axis of a level of 150 objects, its facts taken in one by one and its wire-length rows made after them, checks in about one pivot per wire, as it does with those rows made first.

## Resolution

8.5 takes the facts in their order, `LevelProblem::facts`, and keeps each that fits with those kept before it. `resolve` does exactly that, with exact rationals:

- **The model** (`model`): on each axis, a variable for the origin of each object with a position of its own, for each pin and text placed by its symbol that a fact, a wire or a ROUTE reaches, for each centre of a set (6.5, 14.1), and for the separators and extremes of sets. Each fact is one unit: bounds on linear forms, and choices where the fact is a disjunction. PLACE, a coordinate of a point and the `dx` or `dy` of a text give `x = v`; ALIGN equal members; DISTRIBUTE equal steps; ORDER neighbours at least one unit apart (14.3); DISTANCE bounds the gap between the extents of its two sides (14.5), a lower bound for every pair through a separator, an upper bound for one pair. The constant bounds are widened by the tolerance (11.7), never the equalities between variables.
- **Orientations** (`orient`): ORIENT and ORIENT-RELATION make classes in a union-find whose edges carry an orientation, `ori_to = ori_from.then(relation)` (14.9), each edge one fact, so a conflict is explained by the path that closes the cycle.
- **Literals**, the choices of the search: the direction of an ORDER given only an axis, the side of a DISTANCE given only an axis, the pair that keeps a gap of a DISTANCE between sets within its upper bound, the lowest and highest point of a set of three or more whose centre a fact constrains, and the orientation of a class once facts reach two points of one of its objects on one axis. Only then do the object's pins follow its orientation in the simplex; with one point reached, the origin takes up any orientation.
- **Taking a fact in** (`resolve`): the simplex holds the bounds of the facts kept and of every literal at the value the witness gives it. When the witness satisfies the new fact already, it is kept without a check. Otherwise one check with the literals as they are keeps it, or discards it when the conflict names no literal. When it names one, or the fact changes the orientation classes, a depth-first search over the literals, the witness's values first, checks the simplex at each node; a conflict whose explanation names no open literal fails the node, otherwise the search decides one of the literals it names (conflict-directed backjumping), and the explanations with literals become nogoods that prune later searches. Orientations that give the links the same constants are tried once. The search is complete, so a fact is discarded only when no values of the literals fit.
- **Conflict sets**: the facts of the explanation, made minimal by a deletion filter, the facts of the latest rank left out first; each test is a small resolution of its own, and a test that still conflicts shrinks the set to its own explanation. A conflict that needs a long chain of facts costs a test per fact, each taking the chain in again.
- **Budget**: a search stops after `resolve_nodes` nodes; its fact is then discarded and reported `undecided`, with a note. Deciding the facts is NP-hard in general (the side of `"x"` DISTANCEs alone can encode graph colouring), so a level can need it; the standard does not say yet what a solver does then.
- **The witness** satisfies every fact kept. It is taken from the facts kept resolved once more without the tolerance when they hold so, so that a PLACE at 4 puts its object at 4.

## Modules

| Module | Content | Stage |
| --- | --- | --- |
| `options` | `SolverOptions`, `Objectives`, `Budget`, the canonical text | S1 |
| `rational` | `Q`, conversion from and to `f64` | S1 |
| `simplex` | The incremental bounded simplex: rows, tagged bounds, explanations, undo, minimisation | S1 |
| `model` | Variables per axis, points as linear forms, each fact as rows and bounds (14.1–14.5) | S2 |
| `orient` | Orientation classes: a union-find labelled with the eight orientations (11.2, 14.9) | S2 |
| `resolve` | The resolution of 8.5: literals, search, minimal conflict sets, a witness | S2 |
| `objectives` | The table of objectives: scope, inputs, structural side, estimate, score | S3 |
| `structure` | The tree of groups (11.5), what is left to decide, the connection graph, the sources of candidates | S3 |
| `search` | Candidates and the beam search | S3 |
| `coords` | The lexicographic LP of a candidate's coordinates, rounding to the grid (11.7) | S3 |
| `route` | Rectilinear routing of the ROUTEs, branch points and junctions (13.2, 13.7) | S4 |
| `score` | The layered score | S4 |
| `driver` | The flow of a solve, the budget, the report | S1 stub, S2 placeholder, S4 |

The public interface is `solve` and `resolve` and what they take and give: the options (`SolverOptions`, `Objectives`, `Layer`, `Budget`), the names of the objectives and the default order of the layers (`OBJECTIVES`, `DEFAULT_LAYERS`), the report types (`Solved`, `SolveReport`, `Discarded`, `Work`), the resolution (`Resolved`), the `AUTHOR` the solver writes in its record, and, re-exported from the runtime, `LevelProblem` and `Solution`. The modules are private.

The tests of `resolve` include the examples of the development plan, an orientation that matters only once two points of an object are reached, a randomized check that the explanations of the orientation classes are minimal, and, in `tests/resolve.rs`, every level of the cases written for the solver (`tests/integration/solver`) and of the conformance corpus, read from the files: every fact kept holds at the witness as a viewer checks it, every conflict set conflicts and is minimal, and T4 resolves as its reference says.

The crate does no file I/O and takes no time or randomness, so it builds for `wasm32-unknown-unknown` and the same input and options give the same output on every platform. Its dependencies are pure Rust: the runtime and the `num` crates.

See the [roadmap](../../docs/roadmap.md) for planned work.
