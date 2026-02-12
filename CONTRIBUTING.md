# Contributing to depguard

Thank you for your interest in contributing to depguard! This guide will help you get started.

## Development setup

### Prerequisites

- Rust 1.75+ (stable)
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/your-org/depguard.git
cd depguard

# Build all crates
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features
```

### Project structure

```
crates/
  depguard-types     # DTOs, stable IDs, schema definitions
  depguard-settings  # Config parsing, profiles
  depguard-domain    # Pure policy evaluation (no I/O)
  depguard-repo      # Workspace discovery, TOML parsing
  depguard-render    # Output formatters
  depguard-app       # Use case orchestration
  depguard-cli       # CLI binary
xtask/               # Developer tooling
schemas/             # JSON schemas
tests/
  fixtures/          # Golden test fixtures
  features/          # BDD feature files
docs/                # Documentation
```

See [docs/architecture.md](docs/architecture.md) for the full architecture overview.

## Making changes

### Workflow

1. **Fork** the repository
2. **Create a branch** from `main` for your changes
3. **Make your changes** following the code style guidelines
4. **Add tests** for new functionality
5. **Run the test suite** to ensure nothing is broken
6. **Submit a pull request** with a clear description

### Code style

- Run `cargo fmt` before committing
- Run `cargo clippy --all-targets --all-features` and fix warnings
- Keep the domain crate pure (no I/O, no filesystem, no network)
- Follow existing patterns in the codebase

### Commit messages

Use clear, descriptive commit messages:

```
feat: add new check for git dependencies
fix: handle empty workspace members list
docs: update configuration examples
test: add property tests for ordering invariants
```

## Testing

### Running tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Single crate
cargo test -p depguard-domain

# With output
cargo test -- --nocapture
```

### Test types

| Type | Location | Purpose |
|------|----------|---------|
| Unit tests | `src/*.rs` | Test individual functions |
| Property tests | `depguard-domain` | Verify invariants with random inputs |
| Golden fixtures | `tests/fixtures/` | Catch output drift |
| BDD scenarios | `tests/features/` | Human-readable acceptance tests |
| Integration tests | `crates/*/tests/` | End-to-end behavior |

### Updating golden fixtures

If you intentionally change output format:

```bash
cargo xtask fixtures
```

Review the diff carefully before committing.

## Adding a new check

1. **Add IDs** in `crates/depguard-types/src/ids.rs`:

   ```rust
   pub const DEPS_MY_CHECK: &str = "deps.my_check";
   pub const MY_CODE: &str = "my_code";
   ```

2. **Add explanation** in `crates/depguard-types/src/explain.rs`:

   ```rust
   (DEPS_MY_CHECK, Explanation {
       title: "My Check",
       description: "What this check detects",
       help: "How to fix it",
       url: Some("https://docs.example.com/my-check"),
   })
   ```

3. **Implement the check** in `crates/depguard-domain/src/checks/my_check.rs`:

   ```rust
   pub fn run(
       manifest: &ManifestModel,
       workspace_deps: &HashMap<String, DepSpec>,
       policy: &CheckPolicy,
   ) -> Vec<Finding> {
       // Implementation
   }
   ```

4. **Wire into the engine** in `crates/depguard-domain/src/checks/mod.rs`

5. **Add config support** in `crates/depguard-settings/src/model.rs` if needed

6. **Add tests**:
   - Unit tests in the check module
   - BDD scenario in `tests/features/checks.feature`
   - Golden fixture in `tests/fixtures/` if needed

7. **Update documentation** in `docs/checks.md`

## Architecture guidelines

### Domain purity

The `depguard-domain` crate must remain pure:

- No filesystem access
- No network calls
- No stdout/stderr
- No external process spawning
- All inputs via function parameters
- All outputs via return values

This enables:
- Easy unit testing with synthetic inputs
- Safe fuzzing without side effects
- Deterministic behavior regardless of environment

### Stable codes

Check IDs and codes are part of the public contract:

- Never rename existing IDs or codes
- Deprecate via aliases, not removal
- Every `(check_id, code)` pair must have an explain entry

### Determinism

Output must be deterministic:

- Use stable sorting (severity → path → line → check_id → code → message)
- Use canonical path normalization (`RepoPath`)
- Avoid floating timestamps
- Truncation must preserve sort order

## Pull request guidelines

### Before submitting

- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt --check`)
- [ ] No clippy warnings (`cargo clippy --all-targets --all-features`)
- [ ] Documentation is updated if needed
- [ ] Commit messages are clear and descriptive

### PR description

Include:
- **What** the PR does
- **Why** the change is needed
- **How** to test it
- Any **breaking changes**

### Review process

1. CI must pass
2. At least one maintainer review
3. Address feedback or discuss disagreements
4. Maintainer merges when ready

## Getting help

- Open an issue for bugs or feature requests
- Use discussions for questions
- Check existing issues before creating new ones

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
