#!/usr/bin/env python3
"""Batch 4: Migrate remaining dialogs to DialogShell.

Handles dialogs with more complex structures (tabs, multiple sections,
non-standard footers) by using a more flexible line-based approach.
"""

import re
import sys

def migrate(filepath, title, icon_expr, icon_color_expr, max_width, subtitle, footer_hint):
    with open(filepath, 'r') as f:
        lines = f.readlines()
    
    content = ''.join(lines)
    
    # 1. Add DialogShell import
    if 'DialogShell' not in content:
        for i in range(len(lines) - 1, -1, -1):
            if lines[i].strip().startswith('import ') and ';' in lines[i]:
                lines.insert(i + 1, 'import { DialogShell, DialogButton } from "@/components/dialog-shell";\n')
                break
    
    # 2. Remove useEscapeKey
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
    
    # 4. Find body start (flex-1 overflow-y-auto or overflow-y-auto p-5)
    body_start = None
    for i in range(overlay_start, len(lines)):
        if 'overflow-y-auto' in lines[i] and ('p-5' in lines[i] or 'p-4' in lines[i]):
            body_start = i
            break
    
    if body_start is None:
        return False, "no body"
    
    # 5. Find footer (border-t border-navy-border px-5 py-3)
    footer_start = None
    for i in range(body_start, len(lines)):
        if 'border-t border-navy-border' in lines[i] and ('px-5 py-3' in lines[i] or 'px-5 py-2' in lines[i]):
            footer_start = i
            break
    
    # 6. Find closing );
    closing_line = None
    search_start = footer_start if footer_start else body_start
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
    
    if footer_start and closing_line:
        body_content_start = body_start + 1
        body_close = footer_start - 1
        while body_close > body_start and '</div>' not in lines[body_close]:
            body_close -= 1
        
        new_lines = lines[:overlay_start]
        new_lines.append(dialogshell_open)
        new_lines.extend(lines[body_content_start:body_close])
        new_lines.append('    </DialogShell>\n')
        new_lines.append('  );\n')
        new_lines.extend(lines[closing_line + 1:])
    elif closing_line:
        new_lines = lines[:overlay_start]
        new_lines.append(dialogshell_open)
        new_lines.extend(lines[body_start + 1:closing_line])
        new_lines.append('    </DialogShell>\n')
        new_lines.append('  );\n')
        new_lines.extend(lines[closing_line + 1:])
    else:
        return False, "no closing"
    
    content = ''.join(new_lines)
    
    # 7. Fix div balance
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    while closes > opens:
        content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
        opens = len(re.findall(r'<div', content))
        closes = len(re.findall(r'</div>', content))
    
    # 8. Remove X from lucide imports
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    return True, "ok"


DIALOGS = [
    # Sprint 10-17 dialogs (already have some structure)
    ('src/components/backscatter-mosaic-dialog.tsx', 'Backscatter Mosaic', '<Grid3x3 className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-4xl', 'Gridded intensity mosaic', 'Lambert correction + mean/max gridding'),
    ('src/components/cube-disambiguation-dialog.tsx', 'CUBE Disambiguation', '<Layers3 className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-3xl', 'Hypothesis selection UI', 'Resolve ambiguous depth cells'),
    ('src/components/cube-surface-dialog.tsx', 'CUBE Surface Generation', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Bathymetric surface', 'Hypothesis tracking'),
    ('src/components/eom-auditor-dialog.tsx', 'EOM Volumetric Auditor', '<ShieldCheck className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-4xl', 'LAS to signed PDF', 'RSA-PSS license verification'),
    ('src/components/mbes-survey-dialog.tsx', 'MBES Survey Reader', '<FileSearch className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-5xl', 'Kongsberg .all ingest', 'Bathymetry + position + attitude'),
    ('src/components/mine-grid-dialog.tsx', 'Mine Grid Transform', '<Grid3x3 className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Bidirectional CRS transform', 'Rotation + scale'),
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation', 'Geometric features'),
    ('src/components/monitoring-4d-dialog.tsx', '4D Pit Monitoring', '<Activity className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Multi-epoch differencing', 'Displacement time-series'),
    ('src/components/ntrip-dialog.tsx', 'NTRIP Client', '<Radio className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'RTCM3 correction stream', 'TCP + TLS + base64 auth'),
    ('src/components/odm-pipeline-dialog.tsx', 'ODM Pipeline', '<Terminal className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Drone to point cloud', 'OpenDroneMap Docker'),
    ('src/components/pipeline-editor-dialog.tsx', 'Pipeline Editor', '<GitBranch className="h-4 w-4" />', 'colors.steelLight', 'max-w-3xl', 'Visual workflow builder', '11 actions + watch folders'),
    ('src/components/plugin-marketplace-dialog.tsx', 'Plugin Marketplace', '<Package className="h-4 w-4" />', 'colors.steelLight', 'max-w-3xl', 'Registry + install + search', 'SHA-256 verified plugins'),
    ('src/components/qc-dashboard-dialog.tsx', 'QC Dashboard', '<Activity className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-4xl', 'Real-time S-44 compliance', 'Density + coverage + uncertainty'),
    ('src/components/rover-stream-dialog.tsx', 'RTK Rover Stream', '<Satellite className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-3xl', 'NMEA over TCP', 'GGA + RMC + 5Hz polling'),
    ('src/components/s44-certificate-dialog.tsx', 'S-44 Certificate', '<FileText className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Compliance certificate PDF', 'TPU budget + per-order stats'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'ENC export', 'Wrecks + obstructions'),
    ('src/components/safety-report-dialog.tsx', 'Safety Inspection Report', '<ShieldAlert className="h-4 w-4" />', 'colors.fail', 'max-w-4xl', 'Hazard register + compliance', 'Regulator-ready PDF'),
    ('src/components/settings-dialog.tsx', 'Settings', '<Settings className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', 'Theme + CRS + density', 'Daylight/cabin mode'),
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
