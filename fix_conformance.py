#!/usr/bin/env python3
"""Fix the conformance test to use XML-escaped quotes."""

filepath = 'crates/depguard-render/tests/conformance.rs'

with open(filepath, 'rb') as f:
    content = f.read()

# The test assertion on line646 checks for literal 'serde' but the XML output escapes it
# We need to replace the assertion to check for 'serde' instead

# Use raw bytes to avoid escaping issues
old = b"assert!(output.contains(\"message=\\\"dependency 'serde' uses a wildcard version\"));"
new = b"assert!(output.contains(\"message=\\\"dependency 'serde' uses a wildcard version\"));"

print(f"Looking for: {old}")
print(f"Found in content: {old in content}")

if old in content:
    new_content = content.replace(old, new)
    with open(filepath, 'wb') as f:
        f.write(new_content)
    print("File updated successfully")
else:
    print("Pattern not found")
    # Debug - show what's actually there
    lines = content.split(b'\n')
    for i, line in enumerate(lines):
        if b"serde" in line and b"assert!" in line:
            print(f"Line {i+1}: {line}")
