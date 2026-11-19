# MetaRDU Survey Workflow Cockpit

## Goal

Refactor the desktop workspace into a map-first operations cockpit for mining and marine surveyors. The redesign must make the current survey state, its next defensible action, and available domain tools easy to understand without removing existing workflows.

## Evidence

The application already contains the core capabilities surveyors expect: LAS/GeoTIFF/MBES ingest, mining classification and volumes, CUBE and S-44 checks, GIS tools, automation, and client deliverables. The workspace exposes those capabilities through a dense sidebar with more than 40 tools, while long-running and compliance-critical steps need clearer status and sequencing.

The project already provides the required visual basis: dark field-ready chrome, mining amber, marine teal, status colors, an 8 px spacing scale, compact density, visible focus styles, and reduced-motion support.

## Chosen Direction

Use a shared desktop shell with domain-aware workflow guidance.

- The map remains the dominant canvas.
- The header becomes the operational context bar: project, domain, layout, command access, and panel controls.
- A compact workflow rail surfaces the current stage: Ingest, Validate, Process, Review, Deliver. It does not replace specialist tools; it gives surveyors a clear starting point.
- The left sidebar becomes an asset and tool navigator. It keeps the existing domain actions, but privileges the current workflow and collapses inactive groups.
- The right panel becomes a survey brief: readiness, staged data, QA signals, project facts, and next recommended actions.
- The bottom status bar continues to provide live coordinates, scale, CRS, domain, undo/redo, shortcuts, and UTC time.

## Information Architecture

```text
Title / workflow context
  Project name | domain | workflow stage | layout | commands | panel controls

Desktop workspace
  Left: project assets, layers, contextual tools, settings
  Center: map and survey geometry
  Right: survey brief, files, QC state, next action, profile when active
  Bottom: live survey telemetry
```

### Workflow stages

| Stage | Mining entry action | Marine entry action | Shared outcome |
| --- | --- | --- | --- |
| Ingest | LAS/LAZ, drone, GeoTIFF | MBES, side-scan, tide, vessel data | Data staged with CRS and bounds |
| Validate | Ground classification, control checks | Density, TPU, tidal/SVP checks | Explicit pass, warning, or failure state |
| Process | DEM, volume, stockpile or change analysis | CUBE, cleaning, backscatter or dredge progress | Reproducible derived surface or result |
| Review | EOM audit, highwall or 4D review | S-44 review, QC dashboard | Operator signs off on flags |
| Deliver | Signed volume report, machine-control output | S-57, reporting package | Traceable client deliverable |

The stage rail must highlight the recommended action but never block direct access to another operation. A surveyor handling an exception should be able to open any existing dialog via the sidebar or command palette.

## Component Boundaries

The current `workspace-shell.tsx` owns application state, layout, navigation, panels, and dialog wiring. Retain it as the orchestration layer, while extracting presentational workspace pieces with explicit props:

- `WorkspaceHeader`: project identity, domain state, layout selection, commands, panel visibility.
- `WorkflowRail`: stage selection and domain-aware shortcuts.
- `WorkspaceSidebar`: project actions, collapsible tool groups, search, and utility actions.
- `SurveyBriefPanel`: live ingest metrics, staged files, project facts, recommendations, profile preview.
- `WorkspaceStatusBar`: coordinate and scale telemetry, CRS, undo/redo, shortcuts, UTC.

Existing dialogs and processing callbacks remain owned by `WorkspaceShell` initially, avoiding a risky change to IPC or survey-store behavior.

## Interaction Rules

- Every action is a semantic button with a visible focus state and accessible label.
- Workflow actions have a concise purpose and only appear as enabled when their prerequisites are satisfied. Disabled actions explain the missing prerequisite in a tooltip or helper text.
- The survey brief uses status color only as a secondary cue; text always states the condition.
- Empty states guide the first viable action using the active domain’s supported formats.
- Existing responsive behavior remains: narrow screens use a drawer sidebar and can hide the right panel. Desktop remains the primary target.
- No decorative gradients, glass effects, or marketing-style panels are introduced. Values use existing design tokens wherever practical.

## Initial Implementation Scope

1. Extract the workspace header, sidebar, survey brief, and status bar from the shell without changing existing actions.
2. Add the workflow rail and connect it to current entry-point dialogs.
3. Make tool groups persistently collapsible and prioritize the active domain.
4. Upgrade the survey brief to communicate ingest/QC readiness, staged files, CRS, coverage, and recommended next steps.
5. Remove duplicated hard-coded chrome values in the changed workspace pieces in favor of existing color tokens or shared utility classes.

## Explicitly Deferred

- Rust/IPC progress reporting for long-running jobs.
- Result persistence for every dialog.
- Recent-file pickers, range validation, and unsaved-result recovery.
- New field tools or changes to calculation algorithms.
- A public 21st.dev theme publication.

These are important product follow-ups, but they should not be mixed into a workspace-architecture refactor.

## Verification

- TypeScript production build completes.
- The 21st UI review is run against changed workspace files; only deterministic safe fixes are applied automatically.
- Browser verification covers mining, marine, and both-domain empty states, plus a populated file state.
- Keyboard focus, panel toggles, direct tool access, command palette access, and narrow-layout sidebar behavior are checked.

## Risks and Mitigations

- **Regression risk from a large shell**: extract visual components incrementally and retain callback ownership in the shell.
- **Too much workflow prescription**: provide guidance, not a mandatory wizard; specialist tools remain discoverable.
- **Insufficient map space on laptops**: the rail stays compact, both panels remain independently collapsible, and the existing responsive drawer behavior is preserved.
