# CLAUDE.md — depguard

## Purpose

Public Rust facade for depguard’s pure evaluation API.

## What belongs here

- Stable re-exports over the internal domain engine crates
- Feature forwarding for check-level gating
- Facade-focused tests that protect the public import surface

## What does not belong here

- Evaluation logic
- Check implementations
- Filesystem, process, or network I/O

## Public surface

```rust
pub use depguard_domain::evaluate;

pub mod model;
pub mod policy;
pub mod report;
pub mod checks;
```

Root-level re-exports should stay ergonomic for:

```rust
use depguard::{evaluate, EffectiveConfig, WorkspaceModel};
use depguard::checks::run_all;
use depguard::model::WorkspaceModel;
use depguard::policy::Scope;
```

## Dependency shape

```
depguard
  -> depguard-domain
     -> depguard-domain-core
     -> depguard-domain-checks
```

Keep this crate thin.
