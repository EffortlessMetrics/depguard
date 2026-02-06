# Cockpit Comment ABI

Rules for PR comment rendering by the cockpit director.

## Section markers

Each sensor's comment output uses begin/end markers:

```
<!-- cockpit:section:begin:{sensor_name}:{section_type} -->
... content ...
<!-- cockpit:section:end:{sensor_name}:{section_type} -->
```

## Section order

The cockpit director renders sections in this order:

1. Summary table
2. Verdict badge
3. Sensor details (per-sensor, ordered by policy)
4. Capabilities warnings
5. Truncation notice (if applicable)

## Character limits

- Total comment body: 65536 characters (GitHub API limit)
- Per-sensor detail section: configurable via `budgets.per_sensor_cap`
- Truncated content includes a `...truncated...` marker

## Sensor comment integration

- The cockpit director links to each sensor's `comment.md` artifact
- It does NOT splice or stitch sensor markdown into the comment body
- Each sensor is responsible for its own comment formatting
