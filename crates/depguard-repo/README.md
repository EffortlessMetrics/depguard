# depguard-repo

Repository adapters for workspace discovery and Cargo manifest parsing.

This crate is the filesystem boundary between depguard application logic and on-disk workspace data.

## Owns

- Workspace manifest discovery (`discover_manifests`)
- Building the domain `WorkspaceModel` from real files (`build_workspace_model`)
- Diff scope filtering via caller-provided changed file paths (`ScopeInput`)
- Fuzz-facing parser entry points that should never panic (`fuzz` module)

## Public API

- `discover_manifests(repo_root: &Utf8Path) -> anyhow::Result<Vec<RepoPath>>`
- `build_workspace_model(repo_root: &Utf8Path, scope: ScopeInput) -> anyhow::Result<WorkspaceModel>`
- `fuzz::parse_root_manifest`, `fuzz::parse_member_manifest`, `fuzz::expand_globs`

## Design Constraints

- Filesystem I/O is allowed
- External subprocess calls are not owned by this crate
- Output must remain deterministic for identical repository state
