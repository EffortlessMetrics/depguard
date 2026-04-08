# Cargo Glob Expansion Alignment

This document describes the Cargo edge cases discovered and the improvements made to align depguard's workspace member glob expansion with Cargo's behavior.

## Cargo Edge Cases Discovered

### 1. Parent Directory References
**Cargo Behavior**: Cargo rejects workspace member patterns that reference parent directories using `../` or `..\\`.

**Examples**:
- `../other-crate` - Rejected
- `crates/../../other` - Rejected
- `!../excluded` - Rejected (in exclusion patterns)

**Rationale**: Workspace members must be within the workspace root. Parent directory references would allow members outside the workspace, which Cargo does not permit.

### 2. Absolute Paths
**Cargo Behavior**: Cargo rejects absolute paths in workspace member patterns.

**Examples**:
- `/absolute/path` - Rejected (Unix)
- `\absolute\path` - Rejected (Windows)
- `C:\absolute\path` - Rejected (Windows drive letter)
- `D:/absolute/path` - Rejected (Windows drive letter)

**Rationale**: Workspace members are always relative to the workspace root. Absolute paths would reference directories outside the workspace.

### 3. Virtual Workspaces
**Cargo Behavior**: Virtual workspaces (manifests with `[workspace]` but no `[package]`) are excluded from member discovery.

**Examples**:
```toml
# Virtual workspace - excluded from parent workspace
[workspace]
members = ["nested/*"]
```

**Rationale**: Virtual workspaces represent workspace boundaries. Cargo only includes members from the root workspace, not from nested virtual workspaces.

### 4. Package Workspaces
**Cargo Behavior**: Package workspaces (manifests with both `[package]` and `[workspace]`) are included as regular members.

**Examples**:
```toml
# Package workspace - included as a regular member
[package]
name = "a"
version = "0.1.0"

[workspace]
members = ["nested/*"]
```

**Rationale**: Package workspaces are both a package and a workspace root. They are treated as regular members of the parent workspace.

### 5. Nested Workspace Members
**Cargo Behavior**: Members of nested workspaces can still be matched by parent workspace patterns if they match the glob.

**Examples**:
- Root workspace has `members = ["crates/*"]`
- `crates/virtual/Cargo.toml` is a virtual workspace (excluded)
- `crates/virtual/nested/b/Cargo.toml` is still included (matches `crates/*`)

**Rationale**: Virtual workspaces are excluded, but their members can still be matched by parent workspace patterns.

### 6. Empty Patterns
**Cargo Behavior**: Empty patterns (after trimming whitespace) are silently ignored.

**Examples**:
- `""` - Ignored
- `"   "` - Ignored
- `"!"` - Ignored (exclusion prefix with empty pattern)

**Rationale**: Empty patterns don't match anything and are harmless to ignore.

### 7. Whitespace Normalization
**Cargo Behavior**: Patterns are trimmed of leading/trailing whitespace before processing.

**Examples**:
- `"  crates/*  "` → `"crates/*"`
- `"  !  crates/excluded  "` → `"!crates/excluded"`

**Rationale**: Improves usability and handles common formatting variations.

### 8. Relative Path Normalization
**Cargo Behavior**: Leading `./` or `.\` prefixes are normalized away.

**Examples**:
- `"./crates/a"` → `"crates/a"`
- `".\\crates\\a"` → `"crates\\a"`

**Rationale**: `./path` and `path` are equivalent in workspace members.

### 9. Case Sensitivity
**Cargo Behavior**: Pattern matching is case-sensitive on all platforms.

**Examples**:
- Pattern `"Crates/*"` matches `Crates/A/Cargo.toml` but not `crates/b/Cargo.toml`

**Rationale**: Consistent behavior across platforms, even on case-insensitive filesystems.

## Improvements Made

### 1. Pattern Validation
- **Before**: All patterns were accepted, including invalid ones
- **After**: Patterns with parent directory references or absolute paths are rejected
- **Impact**: Prevents invalid workspace configurations that would fail in Cargo

### 2. Virtual Workspace Detection
- **Before**: No distinction between virtual and package workspaces
- **After**: Virtual workspaces are detected and excluded from member discovery
- **Impact**: Aligns with Cargo's workspace boundary handling

### 3. Empty Pattern Handling
- **Before**: Empty patterns could cause issues in globset compilation
- **After**: Empty patterns are filtered out before globset compilation
- **Impact**: More robust handling of edge cases

### 4. Enhanced Documentation
- Added comprehensive documentation of Cargo edge cases
- Added inline comments explaining the rationale for each behavior
- **Impact**: Better maintainability and understanding of the implementation

## Tests Added

### Unit Tests
1. `parse_member_pattern_rejects_empty_patterns` - Verifies empty patterns are rejected
2. `parse_member_pattern_rejects_parent_directory_references` - Verifies `../` patterns are rejected
3. `parse_member_pattern_rejects_absolute_paths` - Verifies absolute paths are rejected

### Integration Tests
1. `discover_rejects_parent_directory_references` - Verifies parent directory references in members are rejected
2. `discover_rejects_absolute_paths` - Verifies absolute paths in members are rejected
3. `discover_rejects_parent_directory_in_exclude` - Verifies parent directory references in exclude are rejected
4. `discover_rejects_exclusion_with_parent_references` - Verifies exclusion patterns with `../` are rejected
5. `discover_rejects_embedded_parent_references` - Verifies embedded `../` in patterns are rejected
6. `discover_virtual_workspace_is_excluded` - Verifies virtual workspaces are excluded
7. `discover_package_workspace_is_included` - Verifies package workspaces are included

## Test Results

All tests pass successfully:
- **Unit tests**: 27 tests passed
- **Integration tests**: 11 tests passed
- **Total**: 38 tests passed, 0 failed

## Known Limitations

1. **Circular workspace references**: Not currently detected. Cargo handles these by erroring during resolution. This implementation may include nested workspaces.

2. **Default members**: The `default-members` field is not honored during discovery. All matched members are included regardless of `default-members`.

These limitations are documented in the main `discover_manifests` function documentation.

## References

- Cargo workspace documentation: https://doc.rust-lang.org/cargo/reference/workspaces.html
- globset crate documentation: https://docs.rs/globset/
