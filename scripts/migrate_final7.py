#!/usr/bin/env python3
"""Migrate the final 7 complex dialogs by reading exact line numbers
and replacing the chrome while keeping ALL body content intact.
"""
import re

def migrate_by_lines(filepath, title, icon, icon_color, max_width, subtitle, footer_hint,
                     action_label=None, action_handler=None, action_disabled=None,
                     extra_after_header=""):
    """Migrate by finding exact structural lines and replacing them."""
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
    
    # Find structural lines
    return_line = None      # return (
    body_open_line = None   # <div className="flex-1 overflow-y-auto p-5">
    footer_open_line = None # <div className="...border-t border-navy-border...">
    close_paren_line = None # );
    
    for i, line in enumerate(lines):
        if return_line is None and 'fixed inset-0' in line:
            for j in range(i, max(i-5, -1), -1):
                if 'return (' in lines[j]:
                    return_line = j
                    break
        
        if return_line is not None and body_open_line is None:
            if 'overflow-y-auto' in line and ('flex-1' in line or 'p-5' in line):
                body_open_line = i
        
        if body_open_line is not None and footer_open_line is None:
            if 'border-t border-navy-border' in line:
                footer_open_line = i
        
        if footer_open_line is not None and close_paren_line is None:
            if line.strip() == ');':
                close_paren_line = i
    
    # If no footer, try to find closing from body
    if footer_open_line is None and body_open_line is not None:
        for i in range(body_open_line + 1, len(lines)):
            if lines[i].strip() == ');':
                close_paren_line = i
                break
    
    if None in (return_line, body_open_line, close_paren_line):
        missing = []
        if return_line is None: missing.append('return')
        if body_open_line is None: missing.append('body')
        if close_paren_line is None: missing.append('closing')
        return False, f"missing: {','.join(missing)}"
    
    # Find body_end: the </div> just before footer (or before closing if no footer)
    body_end = None
    if footer_open_line:
        for i in range(footer_open_line - 1, body_open_line, -1):
            if '</div>' in lines[i]:
                body_end = i
                break
    else:
        # Find last </div> before close_paren
        for i in range(close_paren_line - 1, body_open_line, -1):
            if '</div>' in lines[i]:
                body_end = i
                break
    
    if body_end is None:
        body_end = close_paren_line - 1
    
    # Build actions
    if action_label and action_handler:
        disabled_str = f' disabled={{{action_disabled}}}' if action_disabled else ''
        actions = f'''actions={{
        <>
          <DialogButton variant="primary" onClick={{{action_handler}}}{disabled_str}>{action_label}</DialogButton>
          <DialogButton variant="secondary" onClick={{onClose}}>Close</DialogButton>
        </>
      }}'''
    else:
        actions = '''actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }'''
    
    extra = f'\n      {extra_after_header}' if extra_after_header else ''
    
    dso = f'''return (
    <DialogShell
      open={{open}}
      onClose={{onClose}}
      title="{title}"
      icon={{{icon}}}
      iconColor={{{icon_color}}}
      maxWidth="{max_width}"
      subtitle="{subtitle}"{extra}
      footerHint="{footer_hint}"
      {actions}
    >
'''
    
    # Build new file
    new_lines = lines[:return_line]
    new_lines.append(dso)
    # Body content: from body_open_line+1 to body_end (exclusive)
    new_lines.extend(lines[body_open_line + 1:body_end])
    new_lines.append('    </DialogShell>\n')
    new_lines.append('  );\n')
    new_lines.extend(lines[close_paren_line + 1:])
    
    content = ''.join(new_lines)
    
    # Fix div balance (single pass, no loop)
    opens = len(re.findall(r'<div', content))
    closes = len(re.findall(r'</div>', content))
    if closes > opens:
        for _ in range(closes - opens):
            content = re.sub(r'\s*</div>\n(\s*</DialogShell>)', r'\n\1', content, count=1)
    elif opens > closes:
        for _ in range(opens - closes):
            content = content.replace('    </DialogShell>', '        </div>\n    </DialogShell>')
    
    # Remove X from imports
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    return True, "ok"


# Process all 7 remaining files
DIALOGS = [
    ('src/components/eom-auditor-dialog.tsx', 'EOM Volumetric Auditor', '<ShieldCheck className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-5xl', 'LAS to signed PDF volume report', 'CSF + IDW + 2.5D matrix + SHA-256 audit trail', None, None, None),
    ('src/components/ml-classification-dialog.tsx', 'ML Classification', '<Brain className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-2xl', 'Habitat + fragmentation analysis', 'Geometric feature extraction', None, None, None),
    ('src/components/pipeline-editor-dialog.tsx', 'Pipeline Editor', '<GitBranch className="h-4 w-4" />', 'colors.steelLight', 'max-w-3xl', 'Visual workflow builder', '11 actions + watch folders', 'Run', 'handleRun', 'running'),
    ('src/components/s44-certificate-dialog.tsx', 'S-44 Certificate', '<FileText className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-2xl', 'Compliance certificate PDF', 'TPU budget + per-order stats', 'Generate', 'handleGenerateCert', 'generating'),
    ('src/components/s57-export-dialog.tsx', 'S-57 Export', '<Anchor className="h-4 w-4" />', 'colors.marineTurquoise', 'max-w-lg', 'IHO S-57 ENC export', 'Wrecks + obstructions + depth contours', 'Export', 'handleExport', 'exporting'),
    ('src/components/settings-dialog.tsx', 'Settings', '<Settings className="h-4 w-4" />', 'colors.steelLight', 'max-w-2xl', 'Theme + CRS + density', 'Daylight/cabin mode', None, None, None),
    ('src/components/blast-report-wizard.tsx', 'Blast Fragmentation Report', '<Bomb className="h-4 w-4" />', 'colors.industrialOrange', 'max-w-3xl', 'p20/p50/p80/p90 + muck volume', 'Design vs actual', None, None, None),
]

for config in DIALOGS:
    ok, msg = migrate_by_lines(*config)
    print(f'  {"OK" if ok else "SKIP"} ({msg}): {config[0]}')
