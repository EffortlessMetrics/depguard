# depguard-inline-suppressions

## Problem
Real projects need narrow, scoped exceptions, but global suppression systems often become too coarse or undocumented.

## What this crate does
`depguard-inline-suppressions` parses comment-based suppression directives from manifest files and converts them into suppress rules that downstream checks can consume.

## Typical directive intent
- Suppress one check for one dependency
- Scope suppression to specific dependency entries
- Keep policy strict globally while allowing explicit local exceptions

## Usage pattern
1. Parse manifest text or comment block.
2. Extract and validate inline directives.
3. Merge suppressions into the policy evaluation context.

## Syntax direction
- Keep directives explicit and close to the dependency they modify.
- Prefer short-lived exceptions with clear rationale.

## Non-goals
- It does not execute dependency policy itself.
- It does not alter manifest semantics beyond suppression metadata.

## Related crates
- `depguard-domain`
- `depguard-app`
- `depguard-repo-parser`
