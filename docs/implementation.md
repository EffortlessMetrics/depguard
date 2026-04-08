# depguard Implementation Notes

## Problem
Implementation details tend to leak across crate boundaries, making behavior hard to locate.

## Recommended reading order
1. `docs/architecture.md`
2. `docs/microcrates.md`
3. `docs/design.md`
4. Relevant crate readmes

## Responsibility split
- **Parser/adapters**: convert external sources into stable in-memory models.
- **Domain**: evaluate checks and produce findings.
- **Application**: orchestrate workflows and pass through contracts.
- **Renderer**: transform findings into a destination format.

## Common implementation pattern

```text
Adapter input -> Domain model -> Domain policy -> Report envelope -> Renderer
```

## Extension pattern
- Add a check to `depguard-domain-checks`.
- Register metadata and docs keys in `depguard-check-catalog`.
- Ensure explanation coverage in explain registry.
- Add/extend fixtures.
- Add tests in unit + integration layer and run conformance checks.

## Operational safety
- Keep default behavior backward-compatible.
- Use explicit feature flags when introducing behavior that should be opt-in.
