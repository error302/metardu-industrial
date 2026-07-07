#!/usr/bin/env python3
"""Batch 6b: Migrate the 8 remaining complex dialogs.

These dialogs have complex structures (tabs, async handlers, loading states)
that previous scripts broke. This version uses a simple, robust approach:
find the overlay div, find the body div, find the footer, and replace
ONLY the chrome — keeping ALL body content, state, and handlers intact.
"""

import re

def migrate(filepath, title, icon_expr, icon_color_expr, max_width, subtitle, footer_hint, action_label=None, action_handler=None):
    with open(filepath, 'r') as f:
        content = f.read()
    
    # 1. Add DialogShell import
    if 'DialogShell' not in content:
        # Find last import
        imports = list(re.finditer(r'^import .+;$', content, re.MULTILINE))
        if imports:
            pos = imports[-1].end()
            content = content[:pos] + '\nimport { DialogShell, DialogButton } from "@/components/dialog-shell";' + content[pos:]
    
    # 2. Remove useEscapeKey
    content = re.sub(r'import \{ useEscapeKey \} from "[^"]+";\n', '', content)
    content = re.sub(r'useEscapeKey\(onClose, open\);\s*\n', '', content)
    content = re.sub(r'if \(!open\) return null;\s*\n', '', content)
    
    # 3. Find the overlay pattern: return ( \n <div ... fixed inset-0
    # Use a broader search — some dialogs use onClick={onClose} on the same line
    overlay_match = re.search(
        r'return \(\s*\n\s*<div\s+[^>]*(?:fixed inset-0|className="fixed)[^>]*>',
        content
    )
    
    if not overlay_match:
        # Try: the div might not have "fixed inset-0" literally
        # Look for the pattern: return ( <div className with backdrop-blur
        overlay_match = re.search(
            r'return \(\s*\n\s*<div\s+className="[^"]*backdrop-blur[^"]*"[^>]*>',
            content
        )
    
    if not overlay_match:
        return False, "no overlay"
    
    overlay_start = overlay_match.start()
    
    # 4. Find the end of the header section (the close button with X)
    # Pattern: <button onClick={onClose}>...<X .../>...</button>
    # Then the </div> that closes the header
    header_end_match = re.search(
        r'<button[^>]*onClick=\{onClose\}[^>]*>\s*<X\s+className="h-4 w-4"\s*/>\s*</button>\s*</div>',
        content[overlay_start:]
    )
    
    if not header_end_match:
        # Try without the className on X
        header_end_match = re.search(
            r'<button[^>]*onClick=\{onClose\}[^>]*>\s*<X[^/]*/>\s*</button>\s*</div>',
            content[overlay_start:]
        )
    
    if not header_end_match:
        return False, "no header end"
    
    header_end = overlay_start + header_end_match.end()
    
    # 5. Find the body start — the next <div> after the header
    # Usually: <div className="flex-1 overflow-y-auto p-5">
    body_match = re.search(
        r'<div\s+className="[^"]*overflow-y-auto[^"]*"',
        content[header_end:]
    )
    
    if not body_match:
        # Try any div after header
        body_match = re.search(r'<div\s+className="', content[header_end:header_end+200])
    
    if not body_match:
        return False, "no body"
    
    body_start = header_end + body_match.start()
    
    # 6. Find the footer — border-t border-navy-border
    footer_match = re.search(
        r'<div\s+className="[^"]*border-t border-navy-border[^"]*"',
        content[body_start:]
    )
    
    # 7. Find the closing ); — the last one before the function ends
    # Search from footer or from body+100
    search_start = body_start + (footer_match.start() if footer_match else 100)
    
    # Find the LAST ");" before the next function definition or end of file
    closing_matches = list(re.finditer(r'\);', content[search_start:]))
    
    if not closing_matches:
        return False, "no closing"
    
    # The closing ); should be followed by \n} (end of the component function)
    closing_line = None
    for m in reversed(closing_matches):
        pos = search_start + m.end()
        # Check if next non-empty line is }
        after = content[pos:pos+20].strip()
        if after.startswith('}'):
            closing_line = search_start + m.start()
            break
    
    if closing_line is None:
        # Just use the last one
        closing_line = search_start + closing_matches[-1].start()
    
    # 8. Build the replacement
    if action_label and action_handler:
        actions = f'''actions={{
        <>
          <DialogButton variant="primary" onClick={{{action_handler}}}>{action_label}</DialogButton>
          <DialogButton variant="secondary" onClick={{onClose}}>Close</DialogButton>
        </>
      }}'''
    else:
        actions = '''actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }'''
    
    dialogshell_open = f'''return (
    <DialogShell
      open={{open}}
      onClose={{onClose}}
      title="{title}"
      icon={{{icon_expr}}}
      iconColor={{{icon_color_expr}}}
      maxWidth="{max_width}"
      subtitle="{subtitle}"
      footerHint="{footer_hint}"
      {actions}
    >
'''
    
    # Extract body content: from after the body opening div to before the footer/closing
    if footer_match:
        footer_start = body_start + footer_match.start()
        # Find the </div> just before the footer
        body_content = content[body_start:footer_start]
        # Remove the body opening div and its closing </div>
        # The body opening div is the first <div...> and the closing is the last </div> before footer
        body_content = re.sub(r'^<div\s+[^>]*>\s*', '', body_content, count=1)
        body_content = re.sub(r'\s*</div>\s*$', '', body_content, count=1)
    else:
        # No footer — body goes to closing
        body_content = content[body_start:closing_line]
        body_content = re.sub(r'^<div\s+[^>]*>\s*', '', body_content, count=1)
        body_content = re.sub(r'\s*</div>\s*$', '', body_content, count=1)
    
    # Build the new content
    new_content = content[:overlay_start]
    new_content += dialogshell_open
    new_content += body_content
    new_content += '\n    </DialogShell>\n  );\n'
    new_content += content[closing_line + 2:]  # skip the ");"
    
    # 9. Fix div balance
    opens = len(re.findall(r'<div', new_content))
    closes = len(re.findall(r'</div>', new_content))
    while closes > opens:
        new_content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', new_content, count=1)
        opens = len(re.findall(r'<div', new_content))
        closes = len(re.findall(r'</div>', new_content))
    
    # 10. Remove X from lucide imports
    new_content = re.sub(r'\bX, ', '', new_content)
    new_content = re.sub(r', X\b', '', new_content)
    
    # 11. Remove unused step nav imports
    for var in ['ArrowRight', 'ArrowLeft', 'STEP_LABELS', 'canNext']:
        uses = len(re.findall(rf'\b{var}\b', new_content))
        if uses <= 1:
            new_content = re.sub(rf'\b{var}, ', '', new_content)
            new_content = re.sub(rf', {var}\b', '', new_content)
            new_content = re.sub(rf'const {var}[^;]+;\n', '', new_content)
    
    with open(filepath, 'w') as f:
        f.write(new_content)
    
    return True, "ok"


DIALOGS = [
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation', 'Geometric features', None, None),
    ('src/components/monitoring-4d-dialog.tsx', '4D Pit Monitoring', '<Activity className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Multi-epoch differencing', 'Displacement time-series', 'Compute', 'handleCompute'),
    ('src/components/odm-pipeline-dialog.tsx', 'ODM Pipeline', '<Terminal className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Drone to point cloud', 'OpenDroneMap Docker', 'Run', 'handleRun'),
    ('src/components/pipeline-editor-dialog.tsx', 'Pipeline Editor', '<GitBranch className="h-4 w-4" />', 'colors.steelLight', 'max-w-3xl', 'Visual workflow builder', '11 actions + watch folders', 'Run', 'handleRun'),
    ('src/components/s44-certificate-dialog.tsx', 'S-44 Certificate', '<FileText className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Compliance certificate', 'TPU budget + stats', 'Generate', 'handleGenerateCert'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'ENC export', 'Wrecks + obstructions', 'Export', 'handleExport'),
    ('src/components/safety-report-dialog.tsx', 'Safety Inspection Report', '<ShieldAlert className="h-4 w-4" />', 'colors.fail', 'max-w-4xl', 'Hazard register + compliance', 'Regulator-ready report', 'Generate', 'handleGenerate'),
]

if __name__ == '__main__':
    for config in DIALOGS:
        filepath = config[0]
        ok, msg = migrate(*config)
        status = "OK" if ok else "SKIP"
        print(f"  {status} ({msg}): {filepath}")
