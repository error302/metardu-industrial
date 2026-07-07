#!/usr/bin/env python3
"""Batch 6: Migrate complex dialogs — PRESERVES all state + handlers.

The key difference from previous batches: this script ONLY replaces the
overlay/header/footer chrome. It does NOT touch any state variables,
async handler functions, or body content. The body is kept completely
intact between <DialogShell> and </DialogShell>.

Strategy:
1. Find overlay start (return ( + fixed inset-0)
2. Find body start (the div with overflow-y-auto that wraps the body)
3. Find footer start (border-t border-navy-border)
4. Find closing );
5. Replace everything from overlay_start to body_start with DialogShell opening
6. Keep body_start+1 through footer_start-1 (the actual body content)
7. Replace footer through closing with DialogShell closing + action buttons

CRITICAL: Do NOT remove any imports, state variables, or handler functions.
"""

import re
import sys

def migrate(filepath, title, icon_expr, icon_color_expr, max_width, subtitle, footer_hint, action_label=None, action_handler=None):
    """
    Migrate a complex dialog to DialogShell.
    
    action_label: If provided, adds a primary action button (e.g., "Generate", "Export")
    action_handler: The handler function name (e.g., "handleGenerate")
    """
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    # 1. Add DialogShell import if not present
    content = ''.join(lines)
    if 'DialogShell' not in content:
        for i in range(len(lines) - 1, -1, -1):
            if lines[i].strip().startswith('import ') and ';' in lines[i]:
                lines.insert(i + 1, 'import { DialogShell, DialogButton } from "@/components/dialog-shell";\n')
                break
    
    # 2. Remove useEscapeKey (DialogShell handles it)
    lines = [l for l in lines if 'useEscapeKey' not in l]
    lines = [l for l in lines if l.strip() != 'if (!open) return null;']
    
    # 3. Find overlay start — the `return (` before `fixed inset-0`
    overlay_start = None
    for i, line in enumerate(lines):
        if 'fixed inset-0' in line and 'z-50' in line:
            for j in range(i, max(i - 5, -1), -1):
                if 'return (' in lines[j]:
                    overlay_start = j
                    break
            if overlay_start is not None:
                break
    
    if overlay_start is None:
        return False, "no overlay found"
    
    # 4. Find the header close button (X icon) — this marks end of header
    header_end = None
    for i in range(overlay_start, min(overlay_start + 40, len(lines))):
        if '<X' in lines[i] and ('/>' in lines[i] or '</X>' in lines[i]):
            # Find the </button> after the X
            for j in range(i, min(i + 3, len(lines))):
                if '</button>' in lines[j]:
                    # Find the </div> that closes the header section
                    for k in range(j, min(j + 3, len(lines))):
                        if '</div>' in lines[k]:
                            header_end = k
                            break
                    break
            if header_end:
                break
    
    if header_end is None:
        return False, "no header end found"
    
    # 5. Find body start — the div with overflow-y-auto
    body_start = None
    for i in range(header_end, min(header_end + 10, len(lines))):
        if 'overflow-y-auto' in lines[i]:
            body_start = i
            break
    
    if body_start is None:
        # Maybe the body doesn't have overflow-y-auto — try finding the first
        # content div after the header
        for i in range(header_end, min(header_end + 10, len(lines))):
            if '<div' in lines[i] and 'className' in lines[i]:
                body_start = i
                break
    
    if body_start is None:
        return False, "no body found"
    
    # 6. Find footer — border-t border-navy-border
    footer_start = None
    for i in range(body_start, len(lines)):
        if 'border-t border-navy-border' in lines[i]:
            footer_start = i
            break
    
    # 7. Find closing ); — search from footer or from body+20
    closing_line = None
    search_from = footer_start if footer_start else body_start + 20
    for i in range(search_from, len(lines)):
        stripped = lines[i].strip()
        if stripped == ');' or stripped.endswith(');'):
            # Make sure this is the component's closing, not a nested one
            # Check if the next non-empty line is } (function close)
            for j in range(i + 1, min(i + 3, len(lines))):
                if lines[j].strip() == '}':
                    closing_line = i
                    break
            if closing_line:
                break
            # Or if there are no more lines
            if i == len(lines) - 1 or all(l.strip() == '' for l in lines[i+1:]):
                closing_line = i
                break
    
    if closing_line is None:
        return False, "no closing found"
    
    # Build the action buttons
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
    
    # Build DialogShell opening
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
    
    # Build the new file:
    # - Keep everything before overlay_start
    # - Add DialogShell opening
    # - Keep body content (body_start+1 through footer_start-1 or closing-2)
    # - Add DialogShell closing
    # - Keep everything after closing_line
    
    new_lines = lines[:overlay_start]
    new_lines.append(dialogshell_open)
    
    if footer_start:
        # Body content is from body_start+1 to footer_start-1
        # But we need to find the body closing </div> (the one before footer)
        body_close = footer_start - 1
        while body_close > body_start and '</div>' not in lines[body_close]:
            body_close -= 1
        
        # Skip the body opening <div> line (body_start) and body closing </div>
        new_lines.extend(lines[body_start + 1:body_close])
    else:
        # No footer — body goes from body_start+1 to closing_line-2
        # (skip body opening div and the 2 closing divs before );)
        new_lines.extend(lines[body_start + 1:closing_line - 1])
    
    new_lines.append('    </DialogShell>\n')
    new_lines.append('  );\n')
    new_lines.extend(lines[closing_line + 1:])
    
    content = ''.join(new_lines)
    
    # 8. Fix div balance
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    while closes > opens:
        content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
        opens = len(re.findall(r'<div', content))
        closes = len(re.findall(r'</div>', content))
    
    # 9. Remove X from lucide imports (the close button is now in DialogShell)
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    # 10. Remove unused step nav imports ONLY if truly unused
    for var in ['ArrowRight', 'ArrowLeft', 'STEP_LABELS', 'canNext']:
        uses = len(re.findall(rf'\b{var}\b', content))
        if uses <= 1:
            content = re.sub(rf'\b{var}, ', '', content)
            content = re.sub(rf', {var}\b', '', content)
            content = re.sub(rf'const {var}[^;]+;\n', '', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    return True, "ok"


DIALOGS = [
    ('src/components/backscatter-mosaic-dialog.tsx', 'Backscatter Mosaic', '<Grid3x3 className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-4xl', 'Gridded intensity mosaic', 'Lambert correction', 'Build', 'handleBuild'),
    ('src/components/cube-surface-dialog.tsx', 'CUBE Surface Generation', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Bathymetric surface', 'Hypothesis tracking', 'Generate', 'handleGenerate'),
    ('src/components/eom-auditor-dialog.tsx', 'EOM Volumetric Auditor', '<ShieldCheck className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-4xl', 'LAS to signed PDF', 'RSA-PSS license verification', None, None),
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation', 'Geometric features', None, None),
    ('src/components/monitoring-4d-dialog.tsx', '4D Pit Monitoring', '<Activity className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Multi-epoch differencing', 'Displacement time-series', 'Compute', 'handleCompute'),
    ('src/components/odm-pipeline-dialog.tsx', 'ODM Pipeline', '<Terminal className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Drone to point cloud', 'OpenDroneMap Docker', 'Run', 'handleRun'),
    ('src/components/pipeline-editor-dialog.tsx', 'Pipeline Editor', '<GitBranch className="h-4 w-4" />', 'colors.steelLight', 'max-w-3xl', 'Visual workflow builder', '11 actions + watch folders', 'Run', 'handleRun'),
    ('src/components/s44-certificate-dialog.tsx', 'S-44 Certificate', '<FileText className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Compliance certificate', 'TPU budget + per-order stats', 'Generate Cert', 'handleGenerateCert'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'ENC export', 'Wrecks + obstructions', 'Export', 'handleExport'),
    ('src/components/safety-report-dialog.tsx', 'Safety Inspection Report', '<ShieldAlert className="h-4 w-4" />', 'colors.fail', 'max-w-4xl', 'Hazard register + compliance', 'Regulator-ready text report', 'Generate', 'handleGenerate'),
    ('src/components/settings-dialog.tsx', 'Settings', '<Settings className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', 'Theme + CRS + density', 'Daylight/cabin mode', None, None),
]

if __name__ == '__main__':
    migrated = 0
    skipped = 0
    for config in DIALOGS:
        filepath = config[0]
        ok, msg = migrate(*config)
        if ok:
            print(f"  OK: {filepath}")
            migrated += 1
        else:
            print(f"  SKIP ({msg}): {filepath}")
            skipped += 1
    print(f"\nTotal: {migrated} migrated, {skipped} skipped")
