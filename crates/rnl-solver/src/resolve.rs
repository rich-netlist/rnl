//! The resolution of 8.5: the facts in the order of [`rnl_runtime::problem::LevelProblem::facts`],
//! each kept when it fits with those kept before it, its literals searched depth first, a fact
//! that does not fit discarded with a minimal conflict set, and a witness that satisfies every
//! fact kept (plan 5.3).
//!
//! Stage S2.
