# depguard-repo-parser

Pure Cargo manifest parsing for depguard without filesystem access.

## Owns
- TOML string parsing for root and member manifests.
- Dependency, package, workspace, feature, and inline suppression extraction.

## Does not own
- Filesystem traversal.
- Repository discovery.
- Cache invalidation.
- Git integration.

## Public API
- `parse_root_manifest(manifest_path, text)` -> workspace deps + `ManifestModel`.
- `parse_member_manifest(manifest_path, text)` -> `ManifestModel`.
