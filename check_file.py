#!/usr/bin/env python3
filepath = 'crates/depguard-render/tests/conformance.rs'
content = open(filepath, 'rb').read()

# Find all occurrences
search1 = b"'serde'"
search2 = b"'serde'"

pos1 = content.find(search1)
pos2 = content.find(search2)

print(f"Position of 'serde': {pos1}")
print(f"Position of 'serde': {pos2}")

# Show line646
lines = content.split(b'\n')
if len(lines) > 645:
    print(f"\nLine646: {lines[645]}")
