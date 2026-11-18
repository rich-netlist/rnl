# The reviewed cases of 0.4

Twelve netlists of the language 0.4, each with the RNL that says what the circuit is and how it is
drawn, and no record of a solver: the cases whose drawings were reviewed before the release of 0.4.
The tests of `rnl solve` solve them within a fixed work (`limit="work"`), so that a solve gives the
same text on every run and every machine:

- four of them, in `solved/`, are what `rnl solve` writes, byte for byte, in every `cargo test`
  (`solve::four_reviewed_cases_solved_are_their_snapshots`); `RNL_BLESS=1` writes them again after a
  change that is meant;
- all twelve are solved once in a release build, and each level is checked whole: no error, every
  object placed, every net routed, the lines layout taken, and the file read back
  (`solve::every_reviewed_case_is_laid_out_whole`, ignored in a debug run; CI runs it with
  `cargo test --release -p rnl-cli --test cli -- --ignored solve::every_reviewed_case`).

The files are under CC0-1.0, as the rest of the tests.
