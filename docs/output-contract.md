# Output Contract

> **Navigation**: [Quick Start](quickstart.md) | [Configuration](config.md) | [Checks](checks.md) | [CI Integration](ci-integration.md) | [Architecture](architecture.md) | [Troubleshooting](troubleshooting.md)

Overview of depguard's output surfaces, their stability guarantees, and how they relate to the cockpit ecosystem.

## Two-speed model

Depguard outputs fall into two stability tiers:

| Surface | Path | Schema | Stability |
|---|---|---|---|
| Cockpit report | `artifacts/depguard/report.json` | `sensor.report.v1` | Stable |
| PR comment | `artifacts/depguard/comment.md` | Markdown | Stable (cockpit comment ABI) |
| Extras (future) | `artifacts/depguard/extras/**` | Various | Fast-evolving, opt-in |

**Stable** surfaces follow the cockpit contract: breaking changes require a new schema version. **Fast-evolving** surfaces may change shape between minor releases and must be opted into explicitly.

## What consumers can rely on

- The envelope shape (`schema`, `tool`, `run`, `verdict`, `findings`, `data`) is governed by the sensor report schema.
- Finding `check_id` and `code` values are stable identifiers. See `contracts/docs/identity-and-codes.md`.
- Finding `data` payloads follow the shapes documented in `contracts/docs/finding-payload.md`. The `fix_action` tokens are stable and safe for actuator dispatch.
- Exit codes: 0 = pass, 2 = policy failure, 1 = tool error (cockpit mode: 0 = receipt written, 1 = failed).
- Deterministic ordering: findings sorted by severity, path, line, check_id, code, message.

## What may change

- **New check_ids and codes** — Additive; existing IDs are never renamed or removed.
- **New keys in `data`** — Additive; consumers must tolerate unknown keys.
- **New extras artifacts** — Opt-in only; never required for core workflow.
- **`fix_hint` text** — Human-readable; may be reworded between releases.
- **Internal v2 schema details** — The `depguard.report.v2` schema is an internal format. Cockpit consumers should use the `sensor.report.v1` envelope.

## Upgrade policy

- Major envelope changes (removing fields, renaming keys, changing semantics) require a new schema version.
- Additive changes (new keys, new check_ids, new fix_action tokens) are non-breaking.
- Deprecation follows the alias-only rule: old identifiers map to new ones; nothing is deleted.

## Reference

- Artifact layout: `contracts/docs/artifact-layout.md`
- Finding payloads: `contracts/docs/finding-payload.md`
- Identity and codes: `contracts/docs/identity-and-codes.md`
- Cockpit comment ABI: `contracts/docs/cockpit-comment-abi.md`
- Capabilities and missingness: `contracts/docs/capabilities-and-missingness.md`
