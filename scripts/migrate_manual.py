#!/usr/bin/env python3
"""Manually migrate the 12 remaining complex dialogs.

Each dialog follows this pattern:
  Lines [overlay_start]: return (
  Lines [overlay_start+1]: <div className="fixed inset-0...">
  Lines [overlay_start+2]: <div onClick={stopPropagation} className="...">
  Lines [header_start]: <div className="border-b border-navy-border px-5 py-3">
  Lines [header_start+1]: <h2>...title...</h2>
  Lines [close_btn]: <button onClick={onClose}><X/></button>
  Lines [close_btn+1]: </div>  (closes header)
  Lines [body_start]: <div className="flex-1 overflow-y-auto p-5...">
  ... body content ...
  Lines [body_end]: </div>  (closes body)
  Lines [footer_start]: <div className="border-t border-navy-border px-5 py-3">
  ... footer content (buttons) ...
  Lines [footer_end]: </div>  (closes footer)
  Lines [footer_end+1]: </div>  (closes inner container)
  Lines [footer_end+2]: </div>  (closes overlay)
  Lines [footer_end+3]: );

Strategy: replace lines overlay_start through body_start (exclusive) with DialogShell opening,
keep body_start+1 through body_end (exclusive) as body content,
replace footer through ); with DialogShell closing.
"""

import re

def migrate(filepath, title, icon, icon_color, max_width, subtitle, footer_hint,
            action_label=None, action_handler=None, action_disabled=None):
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    # Add import
    if 'DialogShell' not in ''.join(lines):
        for i in range(len(lines) - 1, -1, -1):
            if lines[i].strip().startswith('import ') and ';' in lines[i]:
                lines.insert(i + 1, 'import { DialogShell, DialogButton } from "@/components/dialog-shell";\n')
                break
    
    # Remove useEscapeKey
    lines = [l for l in lines if 'useEscapeKey' not in l]
    lines = [l for l in lines if l.strip() != 'if (!open) return null;']
    
    # Find key line numbers
    overlay_start = None
    header_close_div = None  # the </div> after the close button
    body_start = None
    body_end = None  # the </div> that closes the body div
    footer_start = None
    closing_paren = None  # the ); line
    
    for i, line in enumerate(lines):
        if overlay_start is None and 'fixed inset-0' in line:
            # Search backward for 'return ('
            for j in range(i, max(i-5, -1), -1):
                if 'return (' in lines[j]:
                    overlay_start = j
                    break
        
        if overlay_start is not None and header_close_div is None:
            if '<X' in line and '/>' in line:
                # Find the </div> after the close button
                for j in range(i, min(i+5, len(lines))):
                    if '</button>' in lines[j]:
                        for k in range(j, min(k+3, len(lines)) if 'k' in dir() else min(j+3, len(lines))):
                            pass
                        for k in range(j, min(j+3, len(lines))):
                            if '</div>' in lines[k]:
                                header_close_div = k
                                break
                        break
        
        if header_close_div is not None and body_start is None:
            if 'overflow-y-auto' in line:
                body_start = i
    
    # Find body_end: the </div> before the footer
    if body_start is not None:
        for i in range(body_start + 1, len(lines)):
            if 'border-t border-navy-border' in lines[i]:
                footer_start = i
                # body_end is the line before footer_start that has </div>
                for j in range(footer_start - 1, body_start, -1):
                    if '</div>' in lines[j]:
                        body_end = j
                        break
                break
    
    # Find closing );
    if footer_start is not None:
        for i in range(footer_start, len(lines)):
            if lines[i].strip() == ');':
                closing_paren = i
                break
    
    if None in (overlay_start, header_close_div, body_start, body_end, footer_start, closing_paren):
        missing = []
        if overlay_start is None: missing.append('overlay')
        if header_close_div is None: missing.append('header_close')
        if body_start is None: missing.append('body_start')
        if body_end is None: missing.append('body_end')
        if footer_start is None: missing.append('footer')
        if closing_paren is None: missing.append('closing')
        return False, f"missing: {','.join(missing)}"
    
    # Build action buttons
    if action_label and action_handler:
        disabled_expr = f' disabled={{{action_disabled}}}' if action_disabled else ''
        actions = f'''actions={{
        <>
          <DialogButton variant="primary" onClick={{{action_handler}}}{disabled_expr}>{action_label}</DialogButton>
          <DialogButton variant="secondary" onClick={{onClose}}>Close</DialogButton>
        </>
      }}'''
    else:
        actions = '''actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }'''
    
    # Build DialogShell opening (replaces overlay_start through body_start)
    dso = f'''return (
    <DialogShell
      open={{open}}
      onClose={{onClose}}
      title="{title}"
      icon={{{icon}}}
      iconColor={{{icon_color}}}
      maxWidth="{max_width}"
      subtitle="{subtitle}"
      footerHint="{footer_hint}"
      {actions}
    >
'''
    
    # Build new file:
    # 1. Everything before overlay_start
    # 2. DialogShell opening
    # 3. Body content (body_start+1 through body_end, exclusive)
    # 4. DialogShell closing
    # 5. );
    # 6. Everything after closing_paren
    
    new_lines = lines[:overlay_start]
    new_lines.append(dso)
    new_lines.extend(lines[body_start + 1:body_end])
    new_lines.append('    </DialogShell>\n')
    new_lines.append('  );\n')
    new_lines.extend(lines[closing_paren + 1:])
    
    content = ''.join(new_lines)
    
    # Fix div balance
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    while closes > opens:
        content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
        opens = len(re.findall(r'<div', content))
        closes = len(re.findall(r'</div>', content))
    
    # Remove X from lucide imports
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    return True, "ok"


DIALOGS = [
    ('src/components/backscatter-mosaic-dialog.tsx', 'Backscatter Mosaic Builder', '<Grid3x3 className="h-4 w-4" />', 'colors.marine', 'max-w-4xl', 'Gridded intensity mosaic', 'Lambert correction + mean/max gridding', 'Build Mosaic', 'handleBuild', 'loading || !filePath.trim()'),
    ('src/components/cube-surface-dialog.tsx', 'CUBE Surface Generation', '<Waves className="h-4 w-4" />', 'colors.marine', 'max-w-2xl', 'Bathymetric surface from soundings', 'Hypothesis tracking + disambiguation', 'Generate', 'handleGenerate', 'loading'),
    ('src/components/monitoring-4d-dialog.tsx', '4D Pit Monitoring', '<Activity className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Multi-epoch surface differencing', 'Per-cell displacement time-series', 'Compute', 'handleCompute', 'loading'),
    ('src/components/odm-pipeline-dialog.tsx', 'ODM Pipeline', '<Terminal className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Drone photos to point cloud', 'OpenDroneMap Docker integration', 'Run', 'handleRun', 'running'),
    ('src/components/safety-report-dialog.tsx', 'Safety Inspection Report', '<ShieldAlert className="h-4 w-4" />', 'colors.fail', 'max-w-4xl', 'Hazard register + compliance', 'Regulator-ready text report', 'Generate', 'handleGenerate', 'loading'),
    ('src/components/s44-certificate-dialog.tsx', 'S-44 Certificate', '<FileText className="h-4 w-4" />', 'colors.marine', 'max-w-2xl', 'Compliance certificate', 'TPU budget + per-order stats', 'Generate', 'handleGenerateCert', 'generating'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marine', 'max-w-lg', 'IHO S-57 ENC export', 'Wrecks + obstructions + depth contours', 'Export', 'handleExport', 'exporting'),
]

for config in DIALOGS:
    ok, msg = migrate(*config)
    print(f'  {"OK" if ok else "SKIP"} ({msg}): {config[0]}')
