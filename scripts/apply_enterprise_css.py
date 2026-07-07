#!/usr/bin/env python3
"""Apply enterprise CSS classes across all dialog components.

Replaces:
- Result card patterns → add card-enterprise class
- Form input patterns → add input-enterprise class  
- Table patterns → add table-enterprise class
- Button patterns → add btn-enterprise class
"""

import re
import os

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    changes = []
    
    # 1. Apply card-enterprise to result/stat cards
    # Pattern: rounded-md border p-2 style={{ borderColor: ... background: ... }}
    # These are the KPI/stat tiles in dialogs
    if 'card-enterprise' not in content:
        # Match: <div className="rounded-md border p-2" or p-2.5 or p-3
        # that have style with borderColor + background
        content = re.sub(
            r'className="rounded-md border p-2(\.\d)?"(?=\s+style=\{\{ borderColor)',
            r'className="card-enterprise rounded-md border p-2\1"',
            content
        )
        # Match: <div className="rounded-md border p-3"
        content = re.sub(
            r'className="rounded-md border p-3"(?=\s+style=\{\{ borderColor)',
            r'className="card-enterprise rounded-md border p-3"',
            content
        )
        # Match: <div className="rounded-md border p-2.5"
        content = re.sub(
            r'className="rounded-md border p-2\.5"(?=\s+style=\{\{ borderColor)',
            r'className="card-enterprise rounded-md border p-2.5"',
            content
        )
        if 'card-enterprise' in content and 'card-enterprise' not in original:
            changes.append('card-enterprise')
    
    # 2. Apply input-enterprise to form inputs
    # Pattern: <input ... className="... rounded-md border border-navy-border bg-navy-base ..."
    if 'input-enterprise' not in content:
        # Match text/number/email/tel inputs with the standard border pattern
        content = re.sub(
            r'(className=")([^"]*border border-navy-border bg-navy-base[^"]*focus:[^"]*")',
            r'\1input-enterprise \2',
            content
        )
        # Also match: border border-navy-border bg-navy-base without focus
        content = re.sub(
            r'(className=")(rounded[^"]*border border-navy-border bg-navy-base[^"]*")',
            r'\1input-enterprise \2',
            content
        )
        if 'input-enterprise' in content and 'input-enterprise' not in original:
            changes.append('input-enterprise')
    
    # 3. Apply table-enterprise to data tables
    # Pattern: <table className="w-full text-left text-[10px]"> or similar
    if 'table-enterprise' not in content:
        content = re.sub(
            r'(<table\s+className=")(w-full[^"]*)(")',
            r'\1table-enterprise \2\3',
            content
        )
        if 'table-enterprise' in content and 'table-enterprise' not in original:
            changes.append('table-enterprise')
    
    # 4. Apply btn-enterprise to DialogButton (already styled, just add class)
    if 'btn-enterprise' not in content:
        # Add btn-enterprise to DialogButton usage
        content = re.sub(
            r'(<DialogButton\s+variant=")',
            r'\1btn-enterprise ',
            content
        )
        # Fix: the above puts it in the wrong place. DialogButton doesn't take className.
        # Instead, add it to the DialogButton component itself.
        # Revert that change
        content = re.sub(r'variant="btn-enterprise ', 'variant="', content)
        
        # Actually, let's add btn-enterprise to the DialogButton component definition
        # in dialog-shell.tsx instead of each usage.
        # Skip this for individual files.
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        return changes
    return []

# Process all dialog/wizard/tool files
import glob
files = glob.glob('src/components/*dialog*.tsx') + glob.glob('src/components/*wizard*.tsx') + glob.glob('src/components/*tool*.tsx')

total_changes = {'card-enterprise': 0, 'input-enterprise': 0, 'table-enterprise': 0}

for filepath in sorted(files):
    changes = process_file(filepath)
    if changes:
        for c in changes:
            total_changes[c] = total_changes.get(c, 0) + 1
        print(f"  {', '.join(changes)}: {filepath}")

print(f"\nTotals: {total_changes}")
