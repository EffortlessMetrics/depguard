# Architecture

> **Navigation**: [Quick Start](quickstart.md) | [Configuration](config.md) | [Checks](checks.md) | [CI Integration](ci-integration.md) | Architecture | [Design](design.md) | [Testing](testing.md)

Depguard uses **hexagonal (ports & adapters)** architecture with a **pure evaluation core** and a set of **adapters** that translate real repositories into an in-memory model. Think "load-bearing wall" vs "drywall": the domain crate is the wall; everything else can move.

## Architecture Overview Diagram

```mermaid
graph TB
    subgraph "External World"
        FS[Filesystem]
        GIT[Git]
        CI[CI/CD Systems]
    end

    subgraph "CLI Layer"
        CLI[depguard-cli]
        CARGO[cargo-depguard]
    end

    subgraph "Application Layer"
        APP[depguard-app]
        subgraph "Use Cases"
            CHECK[Check]
            BASELINE[Baseline]
            EXPLAIN[Explain]
            RENDER[Render]
        end
    end

    subgraph "Adapters"
        REPO[depguard-repo]
        SETTINGS[depguard-settings]
        RENDERER[depguard-render]
    end

    subgraph "Domain Core"
        DOMAIN[depguard-domain]
        DCORE[depguard-domain-core]
        DCHECKS[depguard-domain-checks]
        CATALOG[depguard-check-catalog]
    end

    subgraph "Foundation"
        TYPES[depguard-types]
        YANKED[depguard-yanked]
        PARSER[depguard-repo-parser]
        INLINE[depguard-inline-suppressions]
    end

    FS --> REPO
    GIT --> CLI
    CI --> CLI
    
    CLI --> APP
    CARGO --> CLI
    
    APP --> CHECK
    APP --> BASELINE
    APP --> EXPLAIN
    APP --> RENDER
    
    APP --> REPO
    APP --> SETTINGS
    APP --> RENDERER
    
    REPO --> DOMAIN
    SETTINGS --> DOMAIN
    
    DOMAIN --> DCORE
    DOMAIN --> DCHECKS
    DCHECKS --> CATALOG
    
    DCORE --> TYPES
    DCHECKS --> TYPES
    CATALOG --> TYPES
    REPO --> PARSER
    PARSER --> INLINE
    INLINE --> TYPES
    RENDERER --> TYPES
    DCORE --> YANKED

    style CLI fill:#e1f5fe
    style APP fill:#fff3e0
    style DOMAIN fill:#e8f5e9
    style TYPES fill:#fce4ec
```

## Crate overview

| Crate | Purpose |
|-------|---------|
| `depguard-types` | DTOs, config, report, findings; schema IDs; stable codes |
| `depguard-domain` | Facade: re-exports model/policy, delegates checks (pure, no I/O) |
| `depguard-domain-core` | Core model and policy types shared across domain crates |
| `depguard-domain-checks` | Pure check implementations (one module per check) |
| `depguard-check-catalog` | Check metadata, feature gates, and profile defaults |
| `depguard-settings` | Config parsing; profile presets; override resolution |
| `depguard-repo` | Workspace discovery; manifest loading; diff-scope |
| `depguard-repo-parser` | Pure TOML manifest parsing (no filesystem) |
| `depguard-render` | Markdown and GitHub annotations renderers |
| `depguard-app` | Use cases: check, md, annotations, explain; error handling |
| `depguard-cli` | clap wiring; filesystem paths; exit code mapping |
| `depguard-inline-suppressions` | Parse `// depguard:disable` comments in manifests |
| `depguard-yanked` | Offline yanked-index parsing and exact version lookup |
| `depguard-test-util` | Shared test utilities for fixtures and normalization |
| `xtask` | Schema emission; fixture generation; release tasks |

## Crate Dependency Graph

This diagram shows how the 14 crates depend on each other. Arrows point from dependent to dependency (A → B means "A depends on B").

```mermaid
graph BT
    subgraph "Foundation Layer"
        TYPES[depguard-types]
        YANKED[depguard-yanked]
        TESTUTIL[depguard-test-util]
    end

    subgraph "Parsing Layer"
        INLINE[depguard-inline-suppressions]
        PARSER[depguard-repo-parser]
    end

    subgraph "Domain Layer"
        DCORE[depguard-domain-core]
        CATALOG[depguard-check-catalog]
        DCHECKS[depguard-domain-checks]
        DOMAIN[depguard-domain]
    end

    subgraph "Adapter Layer"
        REPO[depguard-repo]
        SETTINGS[depguard-settings]
        RENDER[depguard-render]
    end

    subgraph "Application Layer"
        APP[depguard-app]
    end

    subgraph "CLI Layer"
        CLI[depguard-cli]
    end

    subgraph "Dev Tools"
        XTASK[xtask]
    end

    %% Foundation dependencies
    INLINE --> TYPES
    YANKED -.->|"no depguard deps"| YANKED
    TESTUTIL -.->|"no depguard deps"| TESTUTIL

    %% Parsing layer
    PARSER --> DCORE
    PARSER --> INLINE
    PARSER --> TYPES

    %% Domain layer
    DCORE --> TYPES
    DCORE --> YANKED
    CATALOG --> TYPES
    DCHECKS --> DCORE
    DCHECKS --> CATALOG
    DCHECKS --> TYPES
    DOMAIN --> DCORE
    DOMAIN --> DCHECKS
    DOMAIN --> TYPES

    %% Adapter layer
    REPO --> DCORE
    REPO --> PARSER
    REPO --> TYPES
    SETTINGS --> DCORE
    SETTINGS --> CATALOG
    SETTINGS --> TYPES
    RENDER --> TYPES

    %% Application layer
    APP --> DOMAIN
    APP --> REPO
    APP --> SETTINGS
    APP --> RENDER
    APP --> YANKED
    APP --> TYPES

    %% CLI layer
    CLI --> APP
    CLI --> DOMAIN
    CLI --> REPO
    CLI --> RENDER
    CLI --> SETTINGS
    CLI --> YANKED
    CLI --> TYPES

    %% Dev tools
    XTASK --> TYPES
    XTASK --> SETTINGS

    %% Styling
    style TYPES fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    style DOMAIN fill:#e8f5e9,stroke:#388e3c,stroke-width:2px
    style APP fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    style CLI fill:#e1f5fe,stroke:#0288d1,stroke-width:2px
```

### Dependency Rules

- **`depguard-types`** is the foundation: no depguard dependencies
- **`depguard-domain-core`** depends only on `depguard-types` and `depguard-yanked`
- **`depguard-domain`** is a facade: re-exports from `depguard-domain-core` and delegates to `depguard-domain-checks`
- **`depguard-repo`** depends on `depguard-domain` for the domain model and `depguard-repo-parser` for TOML parsing
- **`depguard-settings`** depends on `depguard-domain-core` for policy types and `depguard-check-catalog` for check metadata
- **`depguard-app`** orchestrates use cases but delegates I/O to callers
- **`depguard-cli`** is the only place allowed to:
  - Call `std::process::Command` (for `git diff`)
  - Read/write files to disk
  - Decide exit codes

## Data flow

```mermaid
flowchart TD
    subgraph Input["📥 Input"]
        DISK[("Repository on Disk")]
        CONFIGFILE[("Config File<br/>depguard.toml")]
        GITREFS["Git Refs<br/>--base / --head"]
    end

    subgraph CLI["CLI Layer"]
        CLIPARSE["Argument Parsing<br/>(clap)"]
        GITDIFF["Git Diff Call"]
    end

    subgraph App["Application Layer"]
        USECASE["Check Use Case"]
    end

    subgraph Adapters["Adapter Layer"]
        CONFIGPARSE["Config Parser<br/>(depguard-settings)"]
        DISCOVER["Workspace Discovery<br/>(depguard-repo)"]
        PARSE["Manifest Parsing<br/>(depguard-repo-parser)"]
    end

    subgraph Domain["Domain Layer"]
        MODEL["WorkspaceModel<br/>ManifestModel<br/>DependencyDecl"]
        POLICY["EffectiveConfig<br/>Profile + Overrides"]
        EVALUATE["Policy Evaluation<br/>(depguard-domain)"]
        CHECKS["Check Runners<br/>(depguard-domain-checks)"]
    end

    subgraph Output["📤 Output"]
        REPORT["DomainReport<br/>(Findings + Verdict)"]
        ENVELOPE["Receipt Envelope<br/>(depguard-types)"]
        MD["Markdown Report"]
        ANNOT["GitHub Annotations"]
        SARIF["SARIF Report"]
        EXIT["Exit Code<br/>(0=pass, 2=fail, 1=error)"]
    end

    DISK --> DISCOVER
    CONFIGFILE --> CONFIGPARSE
    GITREFS --> GITDIFF
    
    CLIPARSE --> USECASE
    GITDIFF --> DISCOVER
    
    USECASE --> CONFIGPARSE
    USECASE --> DISCOVER
    
    DISCOVER --> PARSE
    PARSE --> MODEL
    
    CONFIGPARSE --> POLICY
    
    MODEL --> EVALUATE
    POLICY --> EVALUATE
    
    EVALUATE --> CHECKS
    CHECKS --> EVALUATE
    
    EVALUATE --> REPORT
    REPORT --> ENVELOPE
    
    ENVELOPE --> MD
    ENVELOPE --> ANNOT
    ENVELOPE --> SARIF
    
    ENVELOPE --> EXIT

    style DISK fill:#f5f5f5
    style MODEL fill:#e8f5e9
    style EVALUATE fill:#e8f5e9
    style ENVELOPE fill:#fce4ec
    style EXIT fill:#ffebee
```

The key seam is between `depguard-repo` and `depguard-domain`: once the input model is built, evaluation is deterministic and testable without touching the filesystem.

### Data Flow Steps

1. **CLI Parsing**: `depguard-cli` parses command-line arguments using clap
2. **Config Loading**: `depguard-settings` reads and resolves configuration with profile presets
3. **Workspace Discovery**: `depguard-repo` discovers all `Cargo.toml` files in scope
4. **Manifest Parsing**: `depguard-repo-parser` parses TOML into domain models (pure, no I/O)
5. **Policy Evaluation**: `depguard-domain` evaluates all enabled checks against the model
6. **Report Generation**: Findings are collected, sorted, and wrapped in a receipt envelope
7. **Rendering**: Optional renderers produce Markdown, annotations, SARIF, etc.
8. **Exit**: CLI maps the verdict to an exit code

## Hexagonal Architecture

Depguard follows the **hexagonal (ports & adapters)** pattern, where the domain core is isolated from external concerns through well-defined interfaces.

```mermaid
graph TB
    subgraph "External World"
        FS[("📁 Filesystem<br/>Cargo.toml files")]
        GIT[("🔀 Git<br/>diff operations")]
        STDOUT[("📺 stdout<br/>reports")]
        FILESOUT[("📄 Output Files<br/>receipts")]
    end

    subgraph "Adapters (Inbound)"
        CLI["depguard-cli<br/> clap wiring<br/> exit codes"]
        CARGO["cargo-depguard<br/> subcommand wrapper"]
    end

    subgraph "Adapters (Infrastructure)"
        REPO["depguard-repo<br/> workspace discovery<br/> manifest loading"]
        SETTINGS["depguard-settings<br/> config parsing<br/> profile resolution"]
        RENDER["depguard-render<br/> markdown/annotations<br/> SARIF/JUnit"]
    end

    subgraph "Ports (Interfaces)"
        PORT1["Workspace Provider<br/> discover_manifests()"]
        PORT2["Config Provider<br/> load_config()"]
        PORT3["Output Renderer<br/> render(report)"]
    end

    subgraph "Domain Core (Hexagon)"
        subgraph "Pure Domain"
            DOMAIN["depguard-domain<br/> facade"]
            DCORE["depguard-domain-core<br/> model types"]
            DCHECKS["depguard-domain-checks<br/> check implementations"]
            CATALOG["depguard-check-catalog<br/> check metadata"]
        end
    end

    %% External to Adapters
    FS --> REPO
    GIT --> CLI
    STDOUT --> CLI
    FILESOUT --> CLI

    %% Adapters to Ports
    CLI --> PORT1
    CLI --> PORT2
    CLI --> PORT3
    CARGO --> CLI

    %% Ports to Adapters (implementation)
    PORT1 -.->|"implements"| REPO
    PORT2 -.->|"implements"| SETTINGS
    PORT3 -.->|"implements"| RENDER

    %% Adapters to Domain
    REPO --> DOMAIN
    SETTINGS --> DOMAIN
    RENDER --> DOMAIN

    %% Domain internal
    DOMAIN --> DCORE
    DOMAIN --> DCHECKS
    DCHECKS --> CATALOG

    style DOMAIN fill:#e8f5e9,stroke:#2e7d32,stroke-width:3px
    style DCORE fill:#c8e6c9,stroke:#2e7d32,stroke-width:2px
    style DCHECKS fill:#c8e6c9,stroke:#2e7d32,stroke-width:2px
    style CATALOG fill:#c8e6c9,stroke:#2e7d32,stroke-width:2px
```

### Hexagonal Principles in Depguard

| Principle | How Depguard Implements It |
|-----------|---------------------------|
| **Pure Domain** | `depguard-domain*` crates have no filesystem, network, or stdout dependencies |
| **Single Port** | "Provide a `WorkspaceModel` and `EffectiveConfig`" — one main interface |
| **Multiple Adapters** | Real filesystem, in-memory fixtures, synthetic fuzz inputs can all produce models |
| **Dependency Inversion** | Domain defines the model; adapters conform to it |
| **Testability** | Domain can be tested with pure in-memory inputs |

### The "Ports"

Rather than defining dozens of traits, the domain expects an in-memory model:

```rust
// The "port" is implicit: provide these types
struct WorkspaceModel {
    root: RepoPath,
    workspace_dependencies: BTreeMap<String, DepSpec>,
    manifests: Vec<ManifestModel>,
}

struct EffectiveConfig {
    profile: Profile,
    scope: Scope,
    fail_on: FailOn,
    checks: BTreeMap<CheckId, CheckConfig>,
}
```

### The "Adapters"

| Adapter | Responsibility |
|---------|---------------|
| `depguard-repo` | Filesystem + glob expansion + manifest discovery |
| `depguard-repo-parser` | Pure TOML parsing (no I/O) |
| `depguard-settings` | Config file parsing + profile resolution |
| `depguard-cli` | Git diff scoping, exit code mapping |
| `depguard-render` | Output format adapters (Markdown, SARIF, etc.) |

## Component Interaction During Check

This diagram shows the sequence of component interactions during a `depguard check` operation.

```mermaid
sequenceDiagram
    participant User
    participant CLI as depguard-cli
    participant App as depguard-app
    participant Settings as depguard-settings
    participant Repo as depguard-repo
    participant Parser as depguard-repo-parser
    participant Domain as depguard-domain
    participant Checks as depguard-domain-checks
    participant Types as depguard-types
    participant Render as depguard-render

    User->>CLI: depguard check --config depguard.toml
    
    rect rgb(227, 245, 254)
        Note over CLI: Parse CLI arguments (clap)
        CLI->>CLI: Resolve file paths
        CLI->>CLI: Call git diff (if --base/--head)
    end
    
    CLI->>App: invoke CheckUseCase
    
    rect rgb(255, 243, 224)
        Note over App,Settings: Load Configuration
        App->>Settings: parse_config(path)
        Settings->>Settings: Read TOML file
        Settings->>Settings: Resolve profile presets
        Settings->>Settings: Merge overrides
        Settings-->>App: EffectiveConfig
    end
    
    rect rgb(255, 243, 224)
        Note over App,Parser: Discover & Parse Manifests
        App->>Repo: discover_workspace(root, scope)
        Repo->>Repo: Walk directory tree
        Repo->>Repo: Filter by scope (repo/diff)
        Repo->>Parser: parse_manifest(path, content)
        Parser->>Parser: Parse TOML with locations
        Parser->>Parser: Extract dependencies
        Parser-->>Repo: ManifestModel
        Repo-->>App: WorkspaceModel
    end
    
    rect rgb(232, 245, 233)
        Note over App,Checks: Evaluate Policy (Pure)
        App->>Domain: evaluate(model, config)
        Domain->>Checks: run_check(check_id, model)
        
        loop For each enabled check
            Checks->>Checks: Evaluate rule
            Checks->>Types: Create Finding
            Checks-->>Domain: Vec<Finding>
        end
        
        Domain->>Domain: Sort findings deterministically
        Domain->>Domain: Apply max_findings cap
        Domain->>Types: Wrap in Receipt
        Domain-->>App: DomainReport
    end
    
    rect rgb(255, 243, 224)
        Note over App,Render: Render Output (Optional)
        App->>Render: render_markdown(report)
        Render-->>App: Markdown string
        App->>Render: render_annotations(report)
        Render-->>App: GitHub annotations
    end
    
    App-->>CLI: CheckResult { report, verdict }
    
    rect rgb(227, 245, 254)
        Note over CLI: Write outputs & exit
        CLI->>CLI: Write receipt.json
        CLI->>CLI: Write comment.md (if --emit-markdown)
        CLI->>CLI: Map verdict to exit code
    end
    
    CLI-->>User: Exit code (0=pass, 2=fail, 1=error)
```

### Key Interaction Points

1. **CLI → App**: The CLI layer calls use case functions with resolved paths and config
2. **App → Settings**: Configuration is loaded and resolved before any other work
3. **App → Repo**: Workspace discovery is I/O-bound; the repo adapter handles all filesystem access
4. **Repo → Parser**: TOML parsing is pure; the parser has no I/O dependencies
5. **App → Domain**: Policy evaluation is entirely pure; given the same inputs, it always produces the same outputs
6. **Domain → Checks**: Each check is a pure function that examines the model and produces findings
7. **App → Render**: Rendering is optional and happens after evaluation is complete
8. **CLI Exit**: The CLI is the only layer that decides exit codes

## Core abstractions

Depguard is opinionated about what "policy enforcement" means:

- **Input is manifests, not cargo metadata** (no build graph evaluation).
- **Policy is explicit and versioned** (config + profile).
- **Output is a receipt** (envelope + findings + data summary).
- **CI ergonomics are first-class** (Markdown + annotations + stable ordering).

The core model (owned by `depguard-domain`) is intentionally small:

| Type | Purpose |
|------|---------|
| `WorkspaceModel` | Repo root + workspace dependencies + manifests |
| `ManifestModel` | Path + package metadata + dependency declarations |
| `DependencyDecl` | Kind (normal/dev/build) + name + spec + location |
| `DepSpec` | Version string + path + workspace flag |
| `EffectiveConfig` | Resolved config with profile, scope, fail_on, per-check policies |

## Scopes

Depguard supports two scopes (selected by CLI/config):

| Scope | Behavior |
|-------|----------|
| `repo` | Scan all manifests reachable from the workspace root |
| `diff` | Scan only manifests affected by git refs (`--base`/`--head`) or a precomputed diff file (`--diff-file`), plus root for workspace deps |

Scope selection is an **adapter concern** (repo/git). The domain only sees the final manifest set.

## Findings model

A finding is a structured event:

| Field | Purpose |
|-------|---------|
| `check_id` | Stable identifier for the check (`deps.no_wildcards`, etc.) |
| `code` | Stable sub-code for the specific condition (`wildcard_version`, etc.) |
| `severity` | `info` / `warning` / `error` |
| `location` | Best-effort file + line/col |
| `message` | Human summary |
| `help` / `url` | Remediation guidance |
| `fingerprint` | Stable hash for dedup/trending |
| `data` | Check-specific structured payload (JSON) |

The emitted report is deterministic:
- Canonical path normalization (`RepoPath`)
- Stable ordering: `severity → path → line → check_id → code → message`
- Optional caps (`max_findings`) with explicit truncation reason

## See also

- [Design Notes](design.md) — Design decisions and rationale
- [Microcrates](microcrates.md) — Crate-by-crate contracts
- [Testing](testing.md) — Test strategy and organization
- [Implementation Plan](implementation-plan.md) — Development roadmap
