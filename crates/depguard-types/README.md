# depguard-types

Stable protocol types and identifier registries for depguard.

This crate defines the contract consumed by depguard adapters, renderers, and CI integrations. It provides the data types, identifiers, and explanation registry that form depguard's public API.

## Purpose

The types crate serves as the stable contract layer:
- Report envelope types for multiple schema versions
- Stable check IDs and finding codes
- Explanation registry for remediation guidance
- Canonical path types for repository references

## Key Features

### Report Types

Support for multiple report schema versions:

| Schema | Constant | Description |
|--------|----------|-------------|
| `depguard.report.v1` | `SCHEMA_REPORT_V1` | Legacy report format |
| `depguard.report.v2` | `SCHEMA_REPORT_V2` | Current report format |
| `sensor.report.v1` | `SCHEMA_SENSOR_REPORT_V1` | CI sensor format |

### Baseline Types

Suppression baseline format:

| Schema | Constant | Description |
|--------|----------|-------------|
| `depguard.baseline.v1` | `SCHEMA_BASELINE_V1` | Baseline suppressions |

### Buildfix Types

Automated fix plan format:

| Schema | Constant | Description |
|--------|----------|-------------|
| `buildfix.plan.v1` | `SCHEMA_BUILDFIX_PLAN_V1` | Fix action plan |

### Stable Identifiers

Check IDs and codes are **stable contract values** that must not be renamed:

```rust
pub mod ids {
    // Check IDs
    pub const CHECK_DEPS_NO_WILDCARDS: &str = "deps.no_wildcards";
    pub const CHECK_DEPS_PATH_REQUIRES_VERSION: &str = "deps.path_requires_version";
    pub const CHECK_DEPS_PATH_SAFETY: &str = "deps.path_safety";
    // ... more check IDs

    // Finding codes
    pub const CODE_WILDCARD_VERSION: &str = "wildcard_version";
    pub const CODE_PATH_WITHOUT_VERSION: &str = "path_without_version";
    pub const CODE_ABSOLUTE_PATH: &str = "absolute_path";
    // ... more codes
}
```

### Explanation Registry

Remediation guidance for each check and code:

```rust
pub fn lookup_explanation(query: &str) -> Option<Explanation>;

pub struct Explanation {
    pub id: String,
    pub summary: String,
    pub remediation: String,
    pub examples: Vec<ExamplePair>,
}
```

## Public API

```rust
// Report types
pub use receipt::{
    ReportEnvelope, ReportEnvelopeV2,
    DepguardReport, DepguardReportV1, DepguardReportV2,
    Finding, FindingV2,
    Verdict, VerdictV2, VerdictStatus, VerdictCounts,
    Severity, SeverityV2,
    Location, ToolMeta, ToolMetaV2, RunMeta,
    ArtifactPointer, ArtifactType,
    SCHEMA_REPORT_V1, SCHEMA_REPORT_V2, SCHEMA_SENSOR_REPORT_V1,
};

// Baseline types
pub use baseline::{
    DepguardBaselineV1, BaselineFinding,
    SCHEMA_BASELINE_V1,
};

// Buildfix types
pub use buildfix::{
    BuildfixPlanV1, BuildfixAction, BuildfixActionType,
    BuildfixFixAction, BuildfixLocation, BuildfixMetadata,
    SCHEMA_BUILDFIX_PLAN_V1,
};

// Explanation registry
pub use explain::{Explanation, ExamplePair, lookup_explanation};

// Path type
pub use path::RepoPath;

// Identifiers
pub mod ids;
```

## Usage Example

### Working with Reports

```rust
use depguard_types::{
    ReportEnvelope, Finding, Severity, VerdictStatus,
    SCHEMA_SENSOR_REPORT_V1,
};

let report = ReportEnvelope {
    schema: SCHEMA_SENSOR_REPORT_V1.to_string(),
    tool: ToolMeta { name: "depguard", version: env!("CARGO_PKG_VERSION") },
    run: RunMeta { /* ... */ },
    verdict: VerdictStatus::Pass,
    findings: vec![],
};
```

### Looking Up Explanations

```rust
use depguard_types::lookup_explanation;

// Look up by check ID
if let Some(explanation) = lookup_explanation("deps.no_wildcards") {
    println!("Summary: {}", explanation.summary);
    println!("Remediation: {}", explanation.remediation);
}

// Look up by code
if let Some(explanation) = lookup_explanation("wildcard_version") {
    println!("How to fix: {}", explanation.remediation);
}
```

### Using RepoPath

```rust
use depguard_types::RepoPath;

let path = RepoPath::new("crates/my-crate/Cargo.toml");
assert_eq!(path.as_str(), "crates/my-crate/Cargo.toml");
```

## Design Constraints

- **IDs and codes are stable**: Never rename; deprecate via aliases only
- **Serialization-friendly**: All types support `serde`
- **Schema-friendly**: Types derive `schemars::JsonSchema`
- **No I/O**: Pure data types only

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` | Serialization |
| `serde_json` | JSON support |
| `schemars` | JSON schema generation |
| `camino` | UTF-8 path types |
| `time` | Timestamp types |

## Related Crates

- [`depguard-app`](../depguard-app/) - Report production
- [`depguard-render`](../depguard-render/) - Report rendering
- [`depguard-check-catalog`](../depguard-check-catalog/) - Check metadata
- [`depguard-domain`](../depguard-domain/) - Domain types

## Related Documentation

- [Architecture](../../docs/architecture.md)
- [Microcrates](../../docs/microcrates.md)
