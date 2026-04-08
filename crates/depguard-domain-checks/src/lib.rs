//! Check implementations for depguard domain-level rules.
//!
//! This crate is intentionally limited to deterministic check execution and
//! related helpers. It depends on `depguard-domain-core` for model/policy
//! types and `depguard-check-catalog` for availability gating.

#![forbid(unsafe_code)]

pub mod checks;
pub mod fingerprint;
pub mod model;
pub mod policy;
#[cfg(test)]
mod test_support;

pub use checks::run_all;
