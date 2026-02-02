//! Pure policy evaluation (no IO).
//!
//! Input: a workspace model constructed elsewhere.
//! Output: findings + verdict + summary data.

#![forbid(unsafe_code)]

pub mod model;
pub mod policy;
pub mod report;

mod engine;
pub mod checks;

pub use engine::evaluate;
