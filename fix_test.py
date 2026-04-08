#!/usr/bin/env python3
"""Fix the conformance test to use XML-escaped quotes."""

filepath = 'crates/depguard-render/tests/conformance.rs'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# The test assertion checks for literal 'serde' but the XML output escapes it
# We need to replace the assertion to check for 'serde' instead

old_str = "assert!(output.contains(\"message=\\\"dependency 'serde' uses a wildcard version\"));"
new_str = "assert!(output.contains(\"message=\\\"dependency 'serde' uses a wildcard version\"));"

if old_str in content:
    content = content.replace(old_str, new_str)
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print("File updated successfully")
else:
    print("Pattern not found")
    # Show the actual content around line646
    lines = content.split('\n')
    for i, line in enumerate(lines[643:650], start=644):
        print(f"{i}: {repr(line)}")
