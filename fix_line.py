#!/usr/bin/env python3
filepath = 'crates/depguard-render/tests/conformance.rs'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# The test assertion needs to check for 'serde' instead of 'serde'
# because the JUnit XML output escapes single quotes

# Find and show what we're looking for
import re
matches = list(re.finditer(r"dependency .serde. uses", content))
for m in matches:
    print(f"Match at {m.start()}: {repr(m.group())}")

# Replace the pattern in the test assertion only
# The line looks like: assert!(output.contains("message=\"dependency 'serde' uses a wildcard version"));
# We need to change it to: assert!(output.contains("message=\"dependency 'serde' uses a wildcard version"));

# Use a more precise pattern
old = '''assert!(output.contains("message=\\"dependency 'serde' uses a wildcard version"));'''
new = '''assert!(output.contains("message=\\"dependency 'serde' uses a wildcard version"));'''

print(f"\nLooking for:\n{repr(old)}")
print(f"\nFound in content: {old in content}")

if old in content:
    content = content.replace(old, new)
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print("\nFile updated successfully!")
else:
    print("\nPattern not found - trying alternative approach")
    # Try line by line
    lines = content.split('\n')
    for i, line in enumerate(lines):
        if "dependency" in line and "serde" in line and "assert!" in line:
            print(f"\nLine {i+1}:\n{repr(line)}")
            # Replace the single quotes around serde
            new_line = line.replace("'serde'", "'serde'")
            if new_line != line:
                lines[i] = new_line
                print(f"New line:\n{repr(new_line)}")
    
    content = '\n'.join(lines)
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print("\nFile updated with alternative approach!")
