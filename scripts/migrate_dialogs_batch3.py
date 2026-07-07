#!/usr/bin/env python3
"""Batch migrate dialogs to DialogShell — improved v3.

Uses a simpler line-by-line approach instead of complex regex.
"""

import re
import sys

def migrate(filepath, title, icon_expr, icon_color_expr, max_width, subtitle, footer_hint):
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    content = ''.join(lines)
    
    # 1. Add DialogShell import
    if 'DialogShell' not in content:
        # Find last import line
        for i in range(len(lines) - 1, -1, -1):
            if lines[i].strip().startswith('import ') and ';' in lines[i]:
                lines.insert(i + 1, 'import { DialogShell, DialogButton } from "@/components/dialog-shell";\n')
                break
    
    # 2. Remove useEscapeKey import + call + if(!open) return null
    lines = [l for l in lines if 'useEscapeKey' not in l]
    lines = [l for l in lines if l.strip() != 'if (!open) return null;']
    
    # 3. Find overlay start line (return ( + fixed inset-0)
    overlay_start = None
    for i, line in enumerate(lines):
        if 'fixed inset-0' in line and 'z-50' in line:
            # Search backwards for 'return ('
            for j in range(i, max(i-5, -1), -1):
                if 'return (' in lines[j]:
                    overlay_start = j
                    break
            if overlay_start:
                break
    
    if overlay_start is None:
        print(f"  SKIP (no overlay): {filepath}")
        return False
    
    # 4. Find body start line (flex-1 overflow-y-auto p-5)
    body_start = None
    for i in range(overlay_start, len(lines)):
        if 'flex-1 overflow-y-auto' in lines[i] or 'overflow-y-auto p-5' in lines[i]:
            body_start = i
            break
    
    if body_start is None:
        print(f"  SKIP (no body): {filepath}")
        return False
    
    # 5. Find footer start line (border-t border-navy-border px-5 py-3)
    footer_start = None
    for i in range(body_start, len(lines)):
        if 'border-t border-navy-border' in lines[i] and 'px-5 py-3' in lines[i]:
            footer_start = i
            break
    
    # 6. Find the closing ); after footer
    closing_line = None
    if footer_start:
        for i in range(footer_start, len(lines)):
            if lines[i].strip() == ');':
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
    
    # Replace overlay + header with DialogShell opening
    # Keep body_start line and everything after it (but we need to remove the <div className="flex-1..."> opening too)
    
    if footer_start and closing_line:
        # Replace from overlay_start to body_start with DialogShell opening
        # Then keep body content
        # Then replace footer_start to closing_line with DialogShell closing
        
        # Find the body opening div and skip it
        body_div_line = body_start
        body_content_start = body_start + 1
        
        # Find the body closing </div> (the one before footer)
        body_close = footer_start - 1
        while body_close > body_start and '</div>' not in lines[body_close]:
            body_close -= 1
        
        new_lines = lines[:overlay_start]  # everything before return (
        new_lines.append(dialogshell_open)  # DialogShell opening
        new_lines.extend(lines[body_content_start:body_close])  # body content (skip body div opening + closing)
        new_lines.append('    </DialogShell>\n')  # DialogShell closing
        new_lines.append('  );\n')
        new_lines.extend(lines[closing_line + 1:])  # everything after );
        
        content = ''.join(new_lines)
    else:
        # No footer found — just replace overlay + header
        new_lines = lines[:overlay_start]
        new_lines.append(dialogshell_open)
        new_lines.extend(lines[body_start + 1:])  # skip body div opening
        content = ''.join(new_lines)
    
    # 7. Remove X from lucide imports
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    # 8. Fix div balance
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    if closes > opens:
        content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
    
    # 9. Remove unused vars
    content = re.sub(r'const \[reportGenerated, setReportGenerated\][^;]+;\n', '', content)
    content = re.sub(r'setReportGenerated\(true\);\n', '', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    print(f"  OK: {filepath}")
    return True


DIALOGS = [
    ('src/components/svp-editor-dialog.tsx', 'SVP Editor', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Sound velocity profile', 'Ray tracing correction'),
    ('src/components/vessel-config-dialog.tsx', 'Vessel Configuration', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Lever-arm offsets', 'IMU to transducer to GNSS'),
    ('src/components/cube-surface-dialog.tsx', 'CUBE Surface Generation', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Bathymetric surface', 'Hypothesis tracking'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'ENC export', 'Wrecks + obstructions'),
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation', 'Geometric features'),
    ('src/components/odm-pipeline-dialog.tsx', 'ODM Pipeline', '<Terminal className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Drone to point cloud', 'OpenDroneMap Docker'),
    ('src/components/benchmark-dialog.tsx', 'Performance Benchmark', '<Gauge className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', '8 benchmarks + p95 timing', 'Throughput measurement'),
    ('src/components/telemetry-dialog.tsx', 'Telemetry & Crash', '<Activity className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', 'Opt-in usage stats', 'Anonymous, no personal data'),
    ('src/components/update-checker-dialog.tsx', 'Check for Updates', '<RefreshCw className="h-4 w-4" />', 'colors.steelLight', 'max-w-lg', 'Signed auto-updater', 'RSA-PSS packages'),
    ('src/components/license-manager-dialog.tsx', 'License Manager', '<Key className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', 'RSA-PSS signed licenses', 'Core/Pro/Enterprise/Trial'),
    ('src/components/project-manager-dialog.tsx', 'Project Manager', '<FolderOpen className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Save/load .metardu files', 'Auto-save + versioning'),
    ('src/components/deliverable-package-wizard.tsx', 'Deliverable Package', '<Package className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'ZIP + ISO 19115 metadata', 'Branded manifest'),
]

if __name__ == '__main__':
    for config in DIALOGS:
        print(f"\n--- {config[0]} ---")
        migrate(*config)
