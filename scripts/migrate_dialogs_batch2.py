#!/usr/bin/env python3
"""Batch migrate dialogs to DialogShell — Sprint 22.

Improved version that handles more edge cases and collects all
dialog configs in one place.
"""

import re
import sys

def migrate(filepath, title, icon_expr, icon_color_expr, max_width, subtitle, footer_hint):
    with open(filepath, 'r') as f:
        content = f.read()
    
    original = content
    
    # 1. Add DialogShell import
    if 'DialogShell' not in content:
        imports = list(re.finditer(r'^import .+;$', content, re.MULTILINE))
        if imports:
            last = imports[-1]
            content = content[:last.end()] + '\nimport { DialogShell, DialogButton } from "@/components/dialog-shell";' + content[last.end():]
    
    # 2. Remove useEscapeKey
    content = re.sub(r'import \{ useEscapeKey \} from "[^"]+";\n', '', content)
    content = re.sub(r'useEscapeKey\(onClose, open\);\s*\n', '', content)
    content = re.sub(r'if \(!open\) return null;\s*\n', '', content)
    
    # 3. Replace overlay + header with DialogShell opening
    # Match: return ( <div fixed inset-0...> <div stopProp...> [header with h2 + close button] <div body>
    pattern = re.compile(
        r'return \(\s*\n\s*<div\s+[^>]*fixed inset-0[^>]*>\s*\n\s*<div\s+[^>]*onClick=\{[^}]*stopPropagation[^}]*\}[^>]*>\s*\n'
        r'.*?border-b border-navy-border px-5 py-3.*?</h2>\s*\n'
        r'.*?<button[^>]*onClick=\{onClose\}[^>]*>\s*\n?\s*<X\s+className="h-4 w-4"\s*/>\s*\n?\s*</button>\s*\n\s*</div>\s*\n'
        r'(?:/\*[^*]*\*/\s*)?\n?\s*<div\s+className="flex-1 overflow-y-auto p-5">',
        re.DOTALL
    )
    
    replacement = f'''return (
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
    >'''
    
    content, count = pattern.subn(replacement, content)
    if count == 0:
        print(f"  SKIP (overlay not found): {filepath}")
        return False
    
    # 4. Replace footer + closing with DialogShell closing
    footer_pattern = re.compile(
        r'\s*</div>\s*\n\s*(?:/\* Footer \*/)?\s*\n?\s*<div\s+className="flex items-center justify-between border-t border-navy-border px-5 py-3">.*?</div>\s*\n\s*</div>\s*\n\s*\);',
        re.DOTALL
    )
    
    content, fcount = footer_pattern.subn('\n    </DialogShell>\n  );', content)
    
    if fcount == 0:
        # Try without the body closing div
        footer_pattern2 = re.compile(
            r'\s*(?:/\* Footer \*/)?\s*\n?\s*<div\s+className="flex items-center justify-between border-t border-navy-border px-5 py-3">.*?</div>\s*\n\s*</div>\s*\n\s*\);',
            re.DOTALL
        )
        content, fcount = footer_pattern2.subn('\n    </DialogShell>\n  );', content)
    
    if fcount == 0:
        # Try just replacing </div></div>); at the end
        content = re.sub(r'\s*</div>\s*\n\s*</div>\s*\n\s*\);\s*$', '\n    </DialogShell>\n  );', content)
    
    # 5. Remove X from lucide imports
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    # 6. Fix div balance — remove one extra </div> before </DialogShell> if needed
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    if closes > opens:
        content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
    
    # 7. Remove unused vars that were used by old footer
    content = re.sub(r'const \[reportGenerated, setReportGenerated\][^;]+;\n', '', content)
    content = re.sub(r'setReportGenerated\(true\);\n', '', content)
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"  OK: {filepath}")
        return True
    else:
        print(f"  NO CHANGES: {filepath}")
        return False


# Batch configuration for 10 dialogs
DIALOGS = [
    ('src/components/svp-editor-dialog.tsx', 'SVP Editor (Sound Velocity)', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Sound velocity profile for ray tracing', 'Depth vs speed curve editor'),
    ('src/components/vessel-config-dialog.tsx', 'Vessel Configuration', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Lever-arm offsets for TPU calculation', 'IMU to transducer to GNSS'),
    ('src/components/cube-surface-dialog.tsx', 'CUBE Surface Generation', '<Waves className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Bathymetric surface from soundings', 'Hypothesis tracking + disambiguation'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'IHO S-57 electronic navigational chart', 'Wrecks, obstructions, depth contours'),
    ('src/components/monitoring-4d-dialog.tsx', '4D Pit Monitoring', '<Activity className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Multi-epoch surface differencing', 'Per-cell displacement time-series'),
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation analysis', 'Geometric feature extraction'),
    ('src/components/odm-pipeline-dialog.tsx', 'ODM Photogrammetry Pipeline', '<Terminal className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Drone photos to point cloud', 'OpenDroneMap Docker integration'),
    ('src/components/benchmark-dialog.tsx', 'Performance Benchmark', '<Gauge className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', '8 benchmarks with p95 timing', 'Throughput + latency measurement'),
    ('src/components/telemetry-dialog.tsx', 'Telemetry & Crash Reporter', '<Activity className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', 'Opt-in usage statistics + crash dumps', 'Anonymous — no personal data'),
    ('src/components/update-checker-dialog.tsx', 'Check for Updates', '<RefreshCw className="h-4 w-4" />', 'colors.steelLight', 'max-w-lg', 'Auto-updater with signed releases', 'RSA-PSS signed update packages'),
]

if __name__ == '__main__':
    for config in DIALOGS:
        print(f"\n--- {config[0]} ---")
        migrate(*config)
