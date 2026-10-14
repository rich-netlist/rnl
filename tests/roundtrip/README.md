# roundtrip

Future preservation and writeback cases for RNL and host netlists.

Until they exist, [`crates/rnl-cli/tests/roundtrip.rs`](../../crates/rnl-cli/tests/roundtrip.rs) runs the writeback checks on the [conformance cases](../conformance/README.md): stripping the generated records of each case keeps every other line, and a solver's record written into one case takes effect when the file is read again.

See the [roadmap](../../docs/roadmap.md) for planned work.
