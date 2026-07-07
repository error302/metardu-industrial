#!/usr/bin/env python3
"""Batch 5: Migrate remaining simpler dialogs.

These dialogs were reverted because the script removed the `loading` state
but the async handler functions still referenced `setLoading`. This version
PRESERVES all state variables and async functions — it only replaces the
overlay/header/footer chrome, leaving the body completely intact.
"""

import re
import sys

def migrate(filepath, title, icon_expr, icon_color_expr, max_width, subtitle, footer_hint):
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    # 1. Add DialogShell import
    content = ''.join(lines)
    if 'DialogShell' not in content:
        for i in range(len(lines) - 1, -1, -1):
            if lines[i].strip().startswith('import ') and ';' in lines[i]:
                lines.insert(i + 1, 'import { DialogShell, DialogButton } from "@/components/dialog-shell";\n')
                break
    
    # 2. Remove useEscapeKey import + call + if(!open) return null
    lines = [l for l in lines if 'useEscapeKey' not in l]
    lines = [l for l in lines if l.strip() != 'if (!open) return null;']
    
    # 3. Find overlay start (return ( + fixed inset-0)
    overlay_start = None
    for i, line in enumerate(lines):
        if 'fixed inset-0' in line and 'z-50' in line:
            for j in range(i, max(i-5, -1), -1):
                if 'return (' in lines[j]:
                    overlay_start = j
                    break
            if overlay_start:
                break
    
    if overlay_start is None:
        return False, "no overlay"
    
    # 4. Find the close button </button> that ends the header
    # The header has: <div border-b...> <h2>title</h2> <button onClick={onClose}><X/></button> </div>
    header_end = None
    for i in range(overlay_start, min(overlay_start + 30, len(lines))):
        if '<X' in lines[i] and '/>' in lines[i]:
            # Find the next </div> after this — that's the end of the header
            for j in range(i, min(i + 5, len(lines))):
                if '</div>' in lines[j] and '</button>' in ''.join(lines[i:j+1]):
                    header_end = j
                    break
            if header_end:
                break
            # Or just find </button> then </div>
            for j in range(i, min(i + 5, len(lines))):
                if '</button>' in lines[j]:
                    for k in range(j, min(j + 3, len(lines))):
                        if '</div>' in lines[k]:
                            header_end = k
                            break
                    break
            if header_end:
                break
    
    if header_end is None:
        return False, "no header end"
    
    # 5. Find the body start — the <div className="flex-1 overflow-y-auto..."> after header
    body_start = None
    for i in range(header_end, min(header_end + 10, len(lines))):
        if 'overflow-y-auto' in lines[i]:
            body_start = i
            break
    
    if body_start is None:
        return False, "no body"
    
    # 6. Find footer (border-t border-navy-border)
    footer_start = None
    for i in range(body_start, len(lines)):
        if 'border-t border-navy-border' in lines[i] and ('px-5 py-3' in lines[i] or 'px-5 py-2' in lines[i]):
            footer_start = i
            break
    
    # 7. Find closing );
    closing_line = None
    search_start = footer_start if footer_start else body_start + 20
    for i in range(search_start, len(lines)):
        if lines[i].strip() == ');' or lines[i].strip().endswith(');'):
            closing_line = i
            break
    
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
      actions={{
        <DialogButton variant="secondary" onClick={{onClose}}>Close</DialogButton>
      }}
    >
'''
    
    # Strategy: Replace overlay_start through body_start (exclusive) with DialogShell opening
    # Keep body_start line and everything until footer/closing
    # Replace footer through closing with DialogShell closing
    
    if footer_start and closing_line:
        # Find body closing </div> (the one just before footer)
        body_close = footer_start - 1
        while body_close > body_start and '</div>' not in lines[body_close]:
            body_close -= 1
        
        new_lines = lines[:overlay_start]
        new_lines.append(dialogshell_open)
        # Skip the body opening <div> line, keep content, skip body closing </div>
        new_lines.extend(lines[body_start + 1:body_close])
        new_lines.append('    </DialogShell>\n')
        new_lines.append('  );\n')
        new_lines.extend(lines[closing_line + 1:])
    elif closing_line:
        # No footer — just replace overlay+header with DialogShell, keep body
        new_lines = lines[:overlay_start]
        new_lines.append(dialogshell_open)
        new_lines.extend(lines[body_start + 1:closing_line])
        new_lines.append('    </DialogShell>\n')
        new_lines.append('  );\n')
        new_lines.extend(lines[closing_line + 1:])
    else:
        return False, "no closing"
    
    content = ''.join(new_lines)
    
    # 8. Fix div balance — remove extra </div> before </DialogShell>
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    while closes > opens:
        content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
        opens = len(re.findall(r'<div', content))
        closes = len(re.findall(r'</div>', content))
    
    # 9. Remove X from lucide imports (only if it was the close button icon)
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    # 10. Remove unused step nav imports (ArrowRight, ArrowLeft, Download, STEP_LABELS, canNext)
    # ONLY if they appear in the file but aren't used in the body
    for var in ['ArrowRight', 'ArrowLeft', 'Download', 'STEP_LABELS', 'canNext']:
        uses = len(re.findall(rf'\b{var}\b', content))
        if uses <= 1:  # Only in the declaration/import
            content = re.sub(rf'\b{var}, ', '', content)
            content = re.sub(rf', {var}\b', '', content)
            content = re.sub(rf'const {var}[^;]+;\n', '', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    return True, "ok"


DIALOGS = [
    ('src/components/setout-tool-dialog.tsx', 'Setting Out & Markout', '<Crosshair className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-4xl', 'Bearing + distance from reference', 'Total station / RTK setout'),
    ('src/components/stockpile-change-dialog.tsx', 'Stockpile Change Detection', '<History className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-5xl', 'Cut/fill heat map', 'Median rasterization'),
    ('src/components/tidal-datum-dialog.tsx', 'Tidal Datum Converter', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-3xl', 'MLLW/MSL/CD/LAT/NAVD88', 'Offset-based conversion'),
    ('src/components/tide-gauge-dialog.tsx', 'Tide Gauge', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-4xl', 'NOAA CO-OPS + TCP', '6-min water level observations'),
    ('src/components/triage-dialog.tsx', 'Mission Data Triage', '<FolderOpen className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-4xl', 'Field data validation', 'EXIF GPS + CRS mismatch'),
    ('src/components/tunnel-profile-dialog.tsx', 'Tunnel Profile Analyzer', '<SquareDashed className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-4xl', 'Area + overbreak/underbreak', 'SVG cross-section preview'),
    ('src/components/blast-report-wizard.tsx', 'Blast Fragmentation Report', '<Bomb className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-3xl', 'p20/p50/p80/p90 + muck volume', 'Design vs actual'),
    ('src/components/density-gates-tool.tsx', 'Density Gates', '<Activity className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-3xl', 'Coverage validator', 'S-44 density compliance'),
    ('src/components/machine-control-tool.tsx', 'Machine Control Compiler', '<Cpu className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-3xl', 'DXF to Leica/Trimble/Topcon', '.svd / .tp3 / .top'),
    ('src/components/tidal-spline-tool.tsx', 'Tidal Spline Corrector', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-3xl', 'Cubic spline interpolation', 'Tide gauge to soundings'),
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
