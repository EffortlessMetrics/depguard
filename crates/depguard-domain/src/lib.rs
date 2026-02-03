//! Pure policy evaluation (no IO).
//!
//! Input: a workspace model constructed elsewhere.
//! Output: findings + verdict + summary data.

#![forbid(unsafe_code)]

pub mod model;
pub mod policy;
pub mod report;

pub mod checks;
mod engine;

#[cfg(test)]
mod proptest;

pub use engine::evaluate;
pub use policy::{CheckPolicy, EffectiveConfig, FailOn, Scope};
