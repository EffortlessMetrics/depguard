# Artifact Layout Convention

This document describes the standard artifact layout for cockpit ecosystem sensors.

## Directory Structure

```
artifacts/
├── <sensor>/
│   ├── report.json          # Primary sensor report (sensor.report.v1)
│   ├── comment.md           # Optional: PR comment in Markdown
│   └── annotations.txt      # Optional: GitHub Actions annotations
├── cockpit/
│   └── report.json          # Director aggregation (cockpit.report.v1)
└── buildfix/
    └── plan.json            # Actuator fix plan (buildfix.plan.v1)
```

## Sensor Output Path

Each sensor writes to `artifacts/<sensor-name>/report.json`:

| Sensor | Path |
|--------|------|
| depguard | `artifacts/depguard/report.json` |
| license-checker | `artifacts/license-checker/report.json` |
| vuln-scanner | `artifacts/vuln-scanner/report.json` |

## Exit Code Semantics

### Standard Mode

| Code | Meaning |
|------|---------|
| 0 | Pass (no policy violations) |
| 1 | Tool/runtime error |
| 2 | Policy failure (violations found) |

### Cockpit Mode (`--mode cockpit`)

| Code | Meaning |
|------|---------|
| 0 | Receipt successfully written (regardless of verdict) |
| 1 | Failed to write receipt |

Cockpit mode enables pipeline orchestration: the director reads receipts from all sensors and makes the final pass/fail decision. Individual sensor exit codes don't fail the pipeline.

## Artifact Pointers

Sensors can declare additional artifacts in the report's `artifacts` array:

```json
{
  "artifacts": [
    {
      "type": "comment",
      "path": "artifacts/depguard/comment.md",
      "format": "text/markdown"
    },
    {
      "type": "annotation",
      "path": "artifacts/depguard/annotations.txt",
      "format": "text/plain"
    }
  ]
}
```

### Artifact Types

| Type | Description |
|------|-------------|
| `comment` | PR/MR comment content |
| `annotation` | IDE/CI annotations |
| `extra` | Any additional output |

## CI Integration Example

```yaml
# GitHub Actions
jobs:
  sensors:
    steps:
      - name: Run depguard
        run: depguard check --mode cockpit

      - name: Run license-checker
        run: license-checker --mode cockpit

      - name: Aggregate results
        run: cockpit aggregate artifacts/*/report.json

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: cockpit-reports
          path: artifacts/
```

## No Green By Omission

The `run.capabilities` block in sensor reports enables "No Green By Omission" policy:

```json
{
  "run": {
    "capabilities": {
      "git": { "status": "missing", "reason": "Not a git repository" },
      "config": { "status": "available" }
    }
  }
}
```

A director can flag a passing sensor as suspicious if critical capabilities are degraded, ensuring that incomplete analysis doesn't silently pass.
