#!/usr/bin/env python3
"""Migrate a dialog from hand-rolled boilerplate to DialogShell."""

import re
import sys

def migrate(filepath, title, icon_var, icon_color_var, max_width, subtitle, footer_hint):
    with open(filepath, 'r') as f:
        content = f.read()

    original = content

    # 1. Add DialogShell import after last import line
    if 'DialogShell' not in content:
        # Find all import lines
        imports = list(re.finditer(r'^import .+;$', content, re.MULTILINE))
        if imports:
            last = imports[-1]
            content = content[:last.end()] + '\nimport { DialogShell, DialogButton } from "@/components/dialog-shell";' + content[last.end():]

    # 2. Remove standalone X import usage in the close button
    # The pattern is: <button onClick={onClose} ...><X className="h-4 w-4" /></button>
    # DialogShell has its own close button, so we remove the header block entirely

    # 3. Replace overlay + container + header with DialogShell opening
    # Pattern varies slightly per file but core is:
    #   return (
    #     <div className="fixed inset-0 z-50 ...">
    #       <div onClick={stopPropagation} className="flex max-h-... ...">
    #         {/* Header */}
    #         <div className="flex items-center justify-between border-b ...">
    #           <h2 ...>TITLE</h2>
    #           <button onClick={onClose} ...><X .../></button>
    #         </div>
    #         {/* Body */}
    #         <div className="flex-1 overflow-y-auto p-5">

    # Find the return ( with fixed inset-0
    overlay_match = re.search(
        r'return \(\s*\n\s*<div\s+[^>]*fixed inset-0[^>]*>\s*\n\s*<div\s+[^>]*onClick=\{\(e\) => e\.stopPropagation\(\)\}[^>]*>\s*\n\s*(?:/\* Header \*/|/\*[^*]*\*/)?\s*\n?\s*<div\s+className="flex items-center justify-between border-b border-navy-border px-5 py-3">\s*\n\s*<h2[^>]*>.*?</h2>\s*\n\s*<button[^>]*onClick=\{onClose\}[^>]*>\s*\n?\s*<X\s+className="h-4 w-4"\s*/>\s*\n?\s*</button>\s*\n\s*</div>\s*\n\s*(?:/\* Body \*/|/\*[^*]*\*/)?\s*\n?\s*<div\s+className="flex-1 overflow-y-auto p-5">',
        content,
        re.DOTALL
    )

    if not overlay_match:
        # Try a looser pattern
        overlay_match = re.search(
            r'return \(\s*\n\s*<div\s+[^>]*fixed inset-0[^>]*>\s*\n\s*<div\s+[^>]*onClick=\{[^}]*stopPropagation[^}]*\}[^>]*>\s*\n.*?border-b border-navy-border px-5 py-3.*?</button>\s*\n\s*</div>\s*\n.*?<div\s+className="flex-1 overflow-y-auto p-5">',
            content,
            re.DOTALL
        )

    if not overlay_match:
        print(f"  SKIP: couldn't find overlay pattern")
        return False

    # Build the DialogShell opening
    dialogshell_open = f'''return (
    <DialogShell
      open={{open}}
      onClose={{onClose}}
      title="{title}"
      icon={{{icon_var}}}
      iconColor={{{icon_color_var}}}
      maxWidth="{max_width}"
      subtitle="{subtitle}"
      footerHint="{footer_hint}"
      actions={{
        <>
          <DialogButton variant="secondary" onClick={{onClose}}>Close</DialogButton>
        </>
      }}
    >'''

    content = content[:overlay_match.start()] + dialogshell_open + content[overlay_match.end():]

    # 4. Replace the footer + closing divs with DialogShell closing
    # Pattern: </div> (body) {/* Footer */} <div border-t...> ... </div> </div> </div> );
    footer_match = re.search(
        r'\s*</div>\s*\n\s*(?:/\* Footer \*/|/\*[^*]*\*/)?\s*\n?\s*<div\s+className="flex items-center justify-between border-t border-navy-border px-5 py-3">.*?</div>\s*\n\s*</div>\s*\n\s*</div>\s*\n\s*\);',
        content,
        re.DOTALL
    )

    if footer_match:
        content = content[:footer_match.start()] + '\n    </DialogShell>\n  );' + content[footer_match.end():]
    else:
        # Try without the body closing div
        footer_match = re.search(
            r'(?:/\* Footer \*/|/\*[^*]*\*/)?\s*\n?\s*<div\s+className="flex items-center justify-between border-t border-navy-border px-5 py-3">.*?</div>\s*\n\s*</div>\s*\n\s*\);',
            content,
            re.DOTALL
        )
        if footer_match:
            content = content[:footer_match.start()] + '\n    </DialogShell>\n  );' + content[footer_match.end():]
        else:
            print(f"  WARNING: couldn't find footer pattern — manual fix needed")

    # 5. Remove useEscapeKey (DialogShell has its own)
    content = re.sub(r'useEscapeKey\(onClose, open\);\s*\n', '', content)
    content = re.sub(r'if \(!open\) return null;\s*\n', '', content)

    # 6. Remove unused X import
    content = re.sub(r'\bX, ', '', content)
    content = re.sub(r', X\b', '', content)
    content = re.sub(r'import \{ X \} from "lucide-react";\n', '', content)

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"  MIGRATED: {filepath}")
        return True
    else:
        print(f"  NO CHANGES: {filepath}")
        return False

if __name__ == '__main__':
    # Dredge Audit Wizard
    migrate(
        'src/components/dredge-audit-wizard.tsx',
        'Dredge Pay-Volume Audit',
        '<Anchor className="h-4 w-4" />',
        'colors.marineTurquoise',
        'max-w-3xl',
        '4-bucket categorization · branded PDF',
        'Pre-dredge vs post-dredge vs design template',
    )

    # Stockpile Audit Wizard
    migrate(
        'src/components/stockpile-audit-wizard.tsx',
        'Stockpile Inventory Audit',
        '<Boxes className="h-4 w-4" />',
        'colors.industrialOrange',
        'max-w-3xl',
        'Volume + tonnage + branded PDF',
        'Flat or previous-survey baseline',
    )

    # EOM Reconciliation Wizard
    migrate(
        'src/components/eom-reconciliation-wizard.tsx',
        'EOM Reconciliation',
        '<Calculator className="h-4 w-4" />',
        'colors.industrialOrange',
        'max-w-2xl',
        'Monthly production reconciliation',
        'Compare actual vs mine plan',
    )

    # Highwall Monitoring Wizard
    migrate(
        'src/components/highwall-monitoring-wizard.tsx',
        'Highwall Deformation Monitoring',
        '<ShieldAlert className="h-4 w-4" />',
        'colors.fail',
        'max-w-3xl',
        'USACE EM 1110-2-1900 compliant',
        'Per-cell displacement time-series + alerts',
    )

    # Cross-Section Profiler Wizard
    migrate(
        'src/components/cross-section-profiler-wizard.tsx',
        'Cross-Section Profiler',
        '<Ruler className="h-4 w-4" />',
        'colors.marineTurquoise',
        'max-w-3xl',
        'Channel design compliance',
        'Bilinear DEM sampling + under/over-dredge',
    )
