//! The integration tests of the editing library, one test program: each file of this directory is a
//! module of it, so that the toolkit is linked once rather than once for each file. Run one file's
//! tests with its name as a filter, such as `cargo test -p rnl-edit --test edit upgrade::`.

mod diagnostics;
mod host_diagnostics04;
mod junction04;
mod upgrade;
mod upgrade_approximate;
mod upgrade_collections;
mod upgrade_directions;
mod upgrade_exact;
mod upgrade_host_terminals;
mod upgrade_legacy;
mod upgrade_local_symbols;
mod upgrade_manual;
mod upgrade_mechanical;
mod upgrade_references;
mod upgrade_selections;
mod upgrade_sets;
mod upgrade_terminals;
