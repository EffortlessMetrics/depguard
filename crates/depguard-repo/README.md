# depguard-repo

Repository adapters for workspace discovery and Cargo manifest parsing.

This crate is the filesystem boundary between depguard application logic and on-disk workspace data.

## Owns

- Workspace manifest discovery (`discover_manifests`)
- Building the domain `WorkspaceModel` from real files (`build_workspace_model`)
- Diff scope filtering via caller-provided changed file paths (`ScopeInput`)
- Fuzz-facing parser entry points that should never panic (`fuzz` module)
- TOML parsing delegation to `depguard-repo-parser` (pure parser microcrate)

## Public API

- `discover_manifests(repo_root: &Utf8Path) -> anyhow::Result<Vec<RepoPath>>`
- `build_workspace_model(repo_root: &Utf8Path, scope: ScopeInput) -> anyhow::Result<WorkspaceModel>`
- `fuzz::parse_root_manifest`, `fuzz::parse_member_manifest`, `fuzz::expand_globs`

## Design Constraints

- Filesystem I/O is allowed
- External subprocess calls are not owned by this crate
- Output must remain deterministic for identical repository state

## Cargo-Compatible Glob Semantics

The workspace member glob expansion follows Cargo's actual behavior for edge cases:

### Supported Features

1. **Double-star (`**`) patterns**: Matches zero or more directory components.
   ```toml
   [workspace]
   members = ["crates/**"]  # Matches crates/a, crates/foo/bar, etc.
   ```

2. **Exclusion patterns**: Patterns starting with `!` in the members list exclude previously matched paths.
   ```toml
   [workspace]
   members = ["crates/*", "!crates/excluded"]
   ```

3. **Empty member lists**: When `[workspace]` is present with no members, all `Cargo.toml` files are included.

4. **Non-existent paths**: Patterns that match no files are silently ignored.

5. **Relative path normalization**: Both `./path` and `path` forms are handled equivalently.

6. **Combined exclude mechanisms**: Both `exclude` field and `!` prefix patterns work together.

### Known Deviations from Cargo

1. **Circular workspace references**: Not currently detected. Cargo errors during resolution; this implementation may include nested workspaces.

2. **default-members**: The `default-members` field is not honored during discovery. All matched members are included.

3. **Path dependencies with workspace inheritance**: Complex inheritance scenarios may behave differently.

### Reference

- [Cargo Workspace Documentation](https://doc.rust-lang.org/cargo/reference/manifest.html#the-workspace-section)
