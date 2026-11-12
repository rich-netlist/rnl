//! Lossless, versioned messages between a drawing tool and an independent RNL solver.
//! JSON carries model indices and typed geometry; the runtime remains responsible for
//! writing the result back against the original problem.

mod bindings;
mod data;
mod fact;
mod input;
mod json;
mod model;
mod point;
mod problem;
mod report;
mod result;
mod solution;
mod validate;

pub use input::{decode_input, encode_input, InputDocument, LevelInput};
pub use json::{parse_json, Error};
pub use problem::{decode_problem, encode_problem};
pub use report::encode_budget;
pub use result::{decode_result, encode_result, LevelResult, ResultDocument, SolverIdentity};
pub use solution::{decode_solution, encode_solution};

/// Protocol version, independent of a solver executable's own version.
pub const VERSION: u64 = 3;
