//! Rich Netlist (RNL) runtime: the language core shared by the CLI, viewers and solvers.
//!
//! It reads the RNL records that a host adapter cut out of a netlist, resolves names and
//! references, applies the effect rules of the language and reports diagnostics. It does not read
//! files itself: host adapters and the embedding application provide file contents.

pub mod diag;
pub mod geom;
pub mod host;
pub mod interp;
pub mod markup;
pub mod model;
pub mod numbering;
pub mod previous;
pub mod problem;
pub mod records;
pub mod solution;
pub mod source;
pub mod syntax;
pub mod words;

pub use interp::{check_package, interpret, Interpretation, Options};
pub mod writeback;
