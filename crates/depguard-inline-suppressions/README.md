# depguard-inline-suppressions

Pure parsing utilities for depguard inline suppression comments.

This crate provides deterministic, IO-free parsing of inline suppression annotations in `Cargo.toml` files. It enables developers to suppress specific findings on a per-dependency basis.

## Purpose

The inline-suppressions crate:
- Parses `# depguard: allow(...)` comments from TOML source
- Supports both inline and preceding-line comment forms
- Normalizes suppression tokens to canonical check IDs
- Remains completely pure with no filesystem access

## Supported Syntax

### Inline Comment Form

Suppress findings on the same line as the dependency:

```toml
serde = "*" # depguard: allow(deps.no_wildcards)
```

### Preceding Comment Form

Suppress findings with a comment on the line(s) above:

```toml
# depguard: allow(deps.no_wildcards)
serde = "*"
```

### Multiple Suppressions

Suppress multiple checks or codes in a single comment:

```toml
# depguard: allow(deps.no_wildcards, deps.path_safety)
my-crate = { path = "../my-crate", version = "*" }
```

### Short Form

Check IDs can be abbreviated (the `deps.` prefix is optional):

```toml
# depguard: allow(no_wildcards, wildcard_version)
serde = "*"
```

## Public API

```rust
/// Parse inline suppression tokens for a dependency declaration line.
///
/// Returns a sorted, deduplicated list of suppression tokens.
///
/// # Arguments
/// * `source` - The full TOML source text
/// * `line` - 1-based line number of the dependency declaration
///
/// # Returns
/// Vector of suppression tokens (check IDs or codes)
pub fn parse_inline_suppressions(source: &str, line: u32) -> Vec<String>;
```

## Usage Example

```rust
use depguard_inline_suppressions::parse_inline_suppressions;

let source = r#"
[dependencies]
# depguard: allow(no_wildcards)
serde = "*"

# depguard: allow(path_safety, absolute_path)
local-crate = { path = "../local" }
"#;

// Parse suppressions for line 4 (serde dependency)
let suppressions = parse_inline_suppressions(source, 4);
assert_eq!(suppressions, vec!["deps.no_wildcards"]);

// Parse suppressions for line 7 (local-crate dependency)
let suppressions = parse_inline_suppressions(source, 7);
assert!(suppressions.contains(&"deps.path_safety".to_string()));
```

## Token Normalization

The parser normalizes tokens to their canonical form:

1. If the token contains a `.` (e.g., `deps.no_wildcards`), it's used as-is
2. If the token matches a known check ID prefix (e.g., `no_wildcards` → `deps.no_wildcards`), it's expanded
3. Otherwise, the token is used as-is (typically a code like `wildcard_version`)

## Design Constraints

- **No filesystem access**: Source text is provided as input
- **Deterministic**: Same input always produces same output
- **Panic-free**: Handles any input gracefully
- **Line-based**: Uses 1-based line numbers matching TOML conventions

## Dependencies

| Crate | Purpose |
|-------|---------|
| `depguard-types` | Check ID lookup for token normalization |

## Testing

The crate includes property tests using proptest to ensure robustness:

```bash
cargo test -p depguard-inline-suppressions
```

## Related Crates

- [`depguard-repo-parser`](../depguard-repo-parser/) - Uses this crate during manifest parsing
- [`depguard-domain`](../depguard-domain/) - Consumes suppression data during evaluation
- [`depguard-types`](../depguard-types/) - Provides check ID registry
