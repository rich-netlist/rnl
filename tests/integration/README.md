# integration

Cross-module and cross-dialect integration cases.

| Path | Content |
| --- | --- |
| [`solver/`](solver/) | Seven small netlists written for the brainstorm on the solver's algorithms: an RC low-pass, a common-source stage (and the same after a solve and a drag in an editor), a five-transistor OTA in a testbench, a two-stage Miller OTA with facts that conflict, an inverting amplifier, an R-2R ladder. [`crates/rnl-cli/tests/solver.rs`](../../crates/rnl-cli/tests/solver.rs) checks the problem the runtime gives a solver for each. |

See the [roadmap](../../docs/roadmap.md) for planned work.
