//! The solver of Rich Netlist (RNL) descriptions (standard 8.5, 11, 13.7, 14, chapter 17).
//!
//! [`solve`] takes the problem of one drawing level, the [`LevelProblem`] that the runtime builds
//! when it reads a document with `rnl_runtime::Options::solver()`, and gives a [`Solution`]: the
//! positions and orientations of the objects and the wires of the ROUTEs. The runtime writes it
//! back as the level's record of the solver:
//!
//! ```text
//! let solved = rnl_solver::solve(problem, &options);
//! let hash = rnl_runtime::problem::sig1(problem, &options.canonical());
//! let written = rnl_runtime::solution::write_record(problem, &solved.solution, rnl_solver::AUTHOR, &hash);
//! ```
//!
//! [`SolveReport`] says what became of each fact: 8.5 takes the facts in the order of
//! [`LevelProblem::facts`] and keeps each that fits with those kept before it; a fact that does not
//! is discarded, with the facts it conflicts with.
//!
//! The work is in stages (the development plan of solver v1):
//!
//! - resolution of 8.5 with exact rationals and an incremental simplex (`rational`, `simplex`,
//!   `model`, `orient`, `resolve`);
//! - structure-first search over the discrete decisions, configured by the layered objectives
//!   (`objectives`, `structure`, `search`);
//! - coordinates by a lexicographic LP, routing of the ROUTEs, layered scoring (`coords`, `route`,
//!   `score`, `driver`).
//!
//! **Status: stage S5.** [`resolve()`] resolves the facts of a level as 8.5 says, exactly, and gives
//! a witness, a placement that satisfies every fact kept. [`solve`] then analyses the structure of
//! the level, searches candidates structure first (rows by current depth, orders, orientations),
//! places each by a lexicographic LP rounded to the grid, keeping every fact kept, routes the
//! ROUTEs of the best of them by a rectilinear A* around the symbols (13.7), with branch points
//! and junctions (13.2), and takes the best by the layered objectives in force on their wires.
//! `rnl solve` (in `rnl-cli`) solves every level of a document and writes the records back.
//!
//! The crate does no file I/O and takes no time or randomness, so it builds for
//! `wasm32-unknown-unknown` and the same input and options give the same result on every platform.

mod coords;
mod driver;
mod model;
mod objectives;
mod options;
mod orient;
mod rational;
mod resolve;
mod route;
mod score;
mod search;
mod simplex;
mod structure;

pub use options::{Budget, Layer, Objectives, SolverOptions, DEFAULT_LAYERS, OBJECTIVES};
pub use resolve::Resolved;
pub use rnl_runtime::problem::LevelProblem;
pub use rnl_runtime::solution::Solution;

/// The `author` the solver writes in the head of its record (2.8, chapter 17).
pub const AUTHOR: &str = "rnl-solver 0.1";

/// What [`solve`] gives: the solution, and the report of how it was reached.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solved {
    pub solution: Solution,
    pub report: SolveReport,
}

/// What became of the facts, and how much work it took.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SolveReport {
    /// The facts kept, by their index in [`LevelProblem::facts`], in that order (8.5).
    pub kept: Vec<usize>,
    /// The facts discarded, in the order of [`LevelProblem::facts`] (8.5).
    pub discarded: Vec<Discarded>,
    /// The objectives that are on but have nothing to work with, such as the current direction
    /// without a supply or ground symbol, or that this version does not compute: one sentence
    /// each.
    pub inert_objectives: Vec<String>,
    /// The ROUTEs whose nets the router could not join into one tree (13.7), in the order of the
    /// ROUTEs. The wires it made for them are in the solution all the same; a viewer draws flight
    /// lines for what is left. Not an error.
    pub unrouted: Vec<Unrouted>,
    pub work: Work,
    /// Anything else worth saying, such as a budget that ran out.
    pub notes: Vec<String>,
}

/// A fact the resolution of 8.5 discarded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discarded {
    /// Its index in [`LevelProblem::facts`].
    pub fact: usize,
    /// The facts kept before it that it conflicts with, by index, sorted: a minimal set, so that
    /// without any one of them it would fit. It is found from the facts that explain the conflict
    /// by leaving out those of the latest rank first, so of several such sets it leans towards
    /// the facts that rank first. Empty when the fact cannot hold even alone.
    pub conflicts_with: Vec<usize>,
    /// The budget for deciding it ran out ([`Budget::resolve_nodes`]): it is discarded without
    /// a proof that it conflicts, and `conflicts_with` is what the search had found by then. The
    /// standard does not say yet what a solver does then.
    pub undecided: bool,
}

/// A ROUTE whose net the router left in parts (13.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unrouted {
    /// The ROUTE, by its index in [`LevelProblem::routes`]: the first ROUTE of its net, whose style
    /// the wires routed for it take.
    pub route: usize,
    pub net: String,
    /// The parts the net is left in, two or more: its pins, and the wires written on it, that no
    /// wire joins to the rest.
    pub parts: usize,
    /// The budget of the router for the net ran out ([`Budget::astar_expansions`]); otherwise no
    /// path around the symbols and the other wires joins a part to the rest.
    pub budget: bool,
}

/// The work done, counted as the [`Budget`] counts it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// The nodes of the searches of the resolution of 8.5.
    pub resolve_nodes: u64,
    /// The pivots of the simplex.
    pub pivots: u64,
    /// The candidates the search made.
    pub candidates: u64,
    /// The cells the router expanded, over every candidate routed.
    pub astar_expansions: u64,
}

/// Solves one drawing level: resolves its facts (8.5), places and orients its objects, and
/// routes its ROUTEs (13.7).
///
/// Places every object with a position of its own and orients every object that has an
/// orientation, as the best candidate of the search says, every fact kept holding exactly; a
/// position that the file gives on both axes by facts kept, where the object is, is left to the
/// file. Routes the ROUTEs, each wire from a pin or a branch point to a pin or a branch point
/// (13.2, 13.7). The objectives that are on but inert in the level, and the ROUTEs left in parts,
/// are in the report.
pub fn solve(problem: &LevelProblem, options: &SolverOptions) -> Solved {
    driver::solve(problem, options)
}

/// Resolves the facts of one drawing level (8.5): takes them in the order of
/// [`LevelProblem::facts`], keeps each that fits with those kept before it and discards the others,
/// each with a minimal set of facts it conflicts with, and gives a witness, a placement that
/// satisfies every fact kept ([`Resolved`]).
pub fn resolve(problem: &LevelProblem, options: &SolverOptions) -> Resolved {
    resolve::resolve(problem, options)
}
