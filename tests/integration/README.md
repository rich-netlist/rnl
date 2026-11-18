# integration

Cross-module and cross-dialect integration cases.

| Path | Content |
| --- | --- |
| [`solver/`](solver/) | Two small netlists of the brainstorm on the solver's algorithms, in 0.4: a common-source stage after a solve and a drag in an editor (T2b), and a two-stage Miller OTA with facts that conflict (T4), each with its record as `rnl solve` writes it in `solved/`. [`crates/rnl-cli/tests/solver.rs`](../../crates/rnl-cli/tests/solver.rs) checks the problem the runtime gives a solver for each; the tests of [`rnl-solver`](../../crates/rnl-solver/README.md) resolve and solve them. The other cases of the brainstorm are replaced by those of `solver-0.4/`. |
| [`solver/solved/`](solver/solved/) | The same netlists as `rnl solve --solver-options time_limit=none` writes them, the solver's default options without the time limit, so that each is the resolution of 8.5 and the same on every run: each drawing level with its record of the solver (`type="generated"`) at the end of its text, the hash of its input and of the options in the head. [`crates/rnl-cli/tests/solve.rs`](../../crates/rnl-cli/tests/solve.rs) solves each netlist again and requires the same text, byte for byte, and reads it back without errors, with every object placed, every decision of the solver in the record, and every constraint `ok` but those that 8.5 discards, which are `dropped`; fact by fact, the solution holds the facts kept and none of those discarded. After a change to the solver that is meant, `RNL_BLESS=1 cargo test -p rnl-cli --test cli solve::` writes them again. They are snapshots, not cases: the tests that read every netlist of `solver/` do not read this directory. |

See the [roadmap](../../docs/roadmap.md) for planned work.
