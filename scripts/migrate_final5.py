#!/usr/bin/env python3
"""Migrate remaining 5 complex dialogs — one at a time, no shell quoting issues."""
import re

def do_migrate(filepath, title, icon, icon_color, max_width, subtitle, footer_hint, action_label=None, action_handler=None):
    with open(filepath, 'r') as f:
        content = f.read()
    
    if 'DialogShell' not in content:
        imports = list(re.finditer(r'^import .+;$', content, re.MULTILINE))
        pos = imports[-1].end()
        content = content[:pos] + '\nimport { DialogShell, DialogButton } from "@/components/dialog-shell";' + content[pos:]
    
    content = re.sub(r'import \{ useEscapeKey \} from "[^"]+";\n', '', content)
    content = re.sub(r'useEscapeKey\(onClose, open\);\s*\n', '', content)
    content = re.sub(r'if \(!open\) return null;\s*\n', '', content)
    
    om = re.search(r'return \(\s*\n\s*<div\s+[^>]*fixed inset-0[^>]*>', content)
    if not om:
        return False, "no overlay"
    overlay_start = om.start()
    
    he = re.search(r'<button[^>]*onClick=\{onClose\}[^>]*>\s*<X[^/]*/>\s*</button>\s*</div>', content[overlay_start:])
    if not he:
        return False, "no header end"
    header_end = overlay_start + he.end()
    
    bm = re.search(r'<div\s+className="[^"]*overflow-y-auto[^"]*"', content[header_end:])
    if not bm:
        return False, "no body"
    body_start = header_end + bm.start()
    
    fm = re.search(r'<div\s+className="[^"]*border-t border-navy-border[^"]*"', content[body_start:])
    footer_start = body_start + fm.start() if fm else None
    
    cm = list(re.finditer(r'\);', content[(footer_start or body_start):]))
    closing_line = None
    for m in reversed(cm):
        pos = (footer_start or body_start) + m.end()
        if content[pos:pos+20].strip().startswith('}'):
            closing_line = (footer_start or body_start) + m.start()
            break
    if not closing_line and cm:
        closing_line = (footer_start or body_start) + cm[-1].start()
    if not closing_line:
        return False, "no closing"
    
    if action_label and action_handler:
        actions = 'actions={\n        <>\n          <DialogButton variant="primary" onClick={' + action_handler + '}>' + action_label + '</DialogButton>\n          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>\n        </>\n      }'
    else:
        actions = 'actions={\n        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>\n      }'
    
    dso = 'return (\n    <DialogShell\n      open={open}\n      onClose={onClose}\n      title="' + title + '"\n      icon={' + icon + '}\n      iconColor={' + icon_color + '}\n      maxWidth="' + max_width + '"\n      subtitle="' + subtitle + '"\n      footerHint="' + footer_hint + '"\n      ' + actions + '\n    >\n'
    
    if footer_start:
        bc = content[body_start:footer_start]
        bc = re.sub(r'^<div\s+[^>]*>\s*', '', bc, count=1)
        bc = re.sub(r'\s*</div>\s*$', '', bc, count=1)
    else:
        bc = content[body_start:closing_line]
        bc = re.sub(r'^<div\s+[^>]*>\s*', '', bc, count=1)
        bc = re.sub(r'\s*</div>\s*$', '', bc, count=1)
    
    nc = content[:overlay_start] + dso + bc + '\n    </DialogShell>\n  );\n' + content[closing_line + 2:]
    
    opens = len(re.findall(r'<div', nc))
    closes = len(re.findall(r'</div>', nc))
    while closes > opens:
        nc = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', nc, count=1)
        opens = len(re.findall(r'<div', nc))
        closes = len(re.findall(r'</div>', nc))
    
    nc = re.sub(r'\bX, ', '', nc)
    nc = re.sub(r', X\b', '', nc)
    
    with open(filepath, 'w') as f:
        f.write(nc)
    return True, "ok"

# Process each file
files = [
    ('src/components/pipeline-editor-dialog.tsx', 'Pipeline Editor', '<GitBranch className="h-4 w-4" />', 'colors.steelLight', 'max-w-3xl', 'Visual workflow builder', '11 actions + watch folders', 'Run', 'handleRun'),
    ('src/components/s44-certificate-dialog.tsx', 'S-44 Certificate', '<FileText className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Compliance certificate', 'TPU budget + stats', 'Generate', 'handleGenerateCert'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'ENC export', 'Wrecks + obstructions', 'Export', 'handleExport'),
    ('src/components/safety-report-dialog.tsx', 'Safety Inspection Report', '<ShieldAlert className="h-4 w-4" />', 'colors.fail', 'max-w-4xl', 'Hazard register', 'Regulator-ready report', 'Generate', 'handleGenerate'),
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation', 'Geometric features', None, None),
]

for config in files:
    ok, msg = do_migrate(*config)
    print(f'  {"OK" if ok else "SKIP"} ({msg}): {config[0]}')
