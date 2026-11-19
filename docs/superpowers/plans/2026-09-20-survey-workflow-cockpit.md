# Survey Workflow Cockpit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the MetaRDU desktop workspace into a map-first, domain-aware survey workflow cockpit without changing processing or IPC behavior.

**Architecture:** Keep `WorkspaceShell` as the owner of dialog state, survey-store access, and action callbacks. Move workspace presentation into small components with explicit prop contracts: workflow guidance, sidebar navigation, survey brief, and status telemetry. The workflow rail is advisory, mapping each domain and stage to an existing action rather than introducing a new processing path.

**Tech Stack:** React 19, TypeScript, Tailwind CSS 4, Zustand, Lucide React, Tauri 2, OpenLayers.

**Spec:** `docs/superpowers/specs/2026-09-20-survey-workflow-cockpit-design.md`

## Global Constraints

- Keep the map as the dominant work surface.
- Preserve every existing dialog action, callback, and domain visibility rule.
- Use existing MetaRDU colors, 8 px spacing, compact desktop density, visible focus states, and reduced-motion support.
- Treat the workflow rail as guidance, never a gate; direct sidebar and command-palette access must remain.
- Use semantic buttons with accessible labels and text-backed status states.
- Do not change Rust/IPC processing, file formats, project schemas, or calculation algorithms.
- Do not add decorative gradients, glass effects, or marketing-style panels.

---

## File Structure

- Create: `src/components/workflow-rail.tsx` — advisory workflow stages, domain-aware recommended action labels, and prerequisite messaging.
- Create: `src/components/survey-brief-panel.tsx` — survey readiness, staged-file metrics, project facts, recommendations, and profile preview.
- Create: `src/components/workspace-sidebar.tsx` — searchable, collapsible domain tool navigation and workspace utility actions.
- Create: `src/components/workspace-status-bar.tsx` — live cursor telemetry, CRS, domain, undo/redo, shortcuts, UTC clock.
- Modify: `src/screens/workspace-shell.tsx` — retain orchestration state and callbacks while composing extracted workspace components.
- Modify: `src/index.css` — add only shared workspace primitives required by the extracted components.
- Modify: `docs/manual/USER_MANUAL.md` — explain the workflow rail and survey brief.

## Task 1: Add the Advisory Workflow Rail

**Files:**
- Create: `src/components/workflow-rail.tsx`
- Modify: `src/screens/workspace-shell.tsx`

**Interfaces:**
- Consumes: `DomainMode` from `@/lib/tokens`.
- Produces: `WorkflowRail`, accepting `{ domain, stage, onStageChange, actions }`.

- [ ] **Step 1: Define the workflow contract**

```ts
export type WorkflowStage = "ingest" | "validate" | "process" | "review" | "deliver";

export interface WorkflowActions {
  ingest: () => void;
  validate: () => void;
  process: () => void;
  review: () => void;
  deliver: () => void;
}

export interface WorkflowRailProps {
  domain: DomainMode;
  stage: WorkflowStage;
  onStageChange: (stage: WorkflowStage) => void;
  actions: WorkflowActions;
}
```

- [ ] **Step 2: Implement a fixed, keyboard-accessible five-stage rail**

Render a `<nav aria-label="Survey workflow">` containing five `<button type="button">` elements. Each button shows its stage label, uses `aria-current="step"` only for the selected stage, calls `onStageChange(stage)`, and invokes the matching action from `actions`. Use Lucide icons for ingest, validate, process, review, and deliver.

- [ ] **Step 3: Map stages to existing shell actions**

```ts
const workflowActions: WorkflowActions = {
  ingest: () => setProjectOpen(true),
  validate: () => activeDomain === "mining" ? setCsfOpen(true) : setS44Open(true),
  process: () => activeDomain === "mining" ? setVolumeCalcOpen(true) : setCubeOpen(true),
  review: () => activeDomain === "mining" ? setEomAuditorOpen(true) : setS44Open(true),
  deliver: () => setDeliverableOpen(true),
};
```

For `both`, use labels that describe both domain paths and retain direct access to every specialist dialog.

- [ ] **Step 4: Verify the rail**

Run: `npm.cmd run build`

Expected: TypeScript completes with no errors and the Vite production bundle is emitted.

- [ ] **Step 5: Commit the isolated rail change**

```bash
git add src/components/workflow-rail.tsx src/screens/workspace-shell.tsx
git commit -m "feat(workspace): add advisory survey workflow rail"
```

## Task 2: Extract the Survey Brief Panel

**Files:**
- Create: `src/components/survey-brief-panel.tsx`
- Modify: `src/screens/workspace-shell.tsx`

**Interfaces:**
- Consumes: `DomainMode`, `ProfileLine`, the existing survey-store file type, and existing profile/action callbacks.
- Produces: `SurveyBriefPanel`, accepting `{ domain, epsg, files, activeFileId, onSelectFile, onRemoveFile, profile, actions }`.

- [ ] **Step 1: Define the actions contract**

```ts
type SurveyBriefActions = {
  classify: () => void;
  volume: () => void;
  audit: () => void;
  cube: () => void;
  s44: () => void;
  deliver: () => void;
};
```

Import the file type directly from `@/stores/survey-store` rather than duplicating it.

- [ ] **Step 2: Move current survey status behavior**

Move file count, loaded/error/probing counts, byte total, point total, latest ingest, and bounds checks to `SurveyBriefPanel`. Preserve behavior, with text states limited to `Ready for ingest`, `Probing headers`, `Needs review`, and `Ready for processing`.

- [ ] **Step 3: Render the accessible survey brief**

Use an `<aside aria-label="Survey brief">` with readiness, points/data/errors, staged files, CRS/domain/coverage/last-ingest facts, domain-aware next actions, and a profile preview when active. Removal buttons must carry a specific accessible name such as `Remove survey file`.

- [ ] **Step 4: Compose the extracted panel**

Replace inline `RightPanel` use with `<SurveyBriefPanel />`. Preserve `removeFile`, `setActiveFile`, and profile behavior unchanged.

- [ ] **Step 5: Verify empty and staged states**

Run: `npm.cmd run build`

Then use browser preview for an empty workspace and a staged-file state. Expected: status text, counts, file selection, removal, and next-action buttons render without overflow.

- [ ] **Step 6: Commit the isolated survey brief change**

```bash
git add src/components/survey-brief-panel.tsx src/screens/workspace-shell.tsx
git commit -m "refactor(workspace): extract survey brief panel"
```

## Task 3: Extract and Prioritize Sidebar Navigation

**Files:**
- Create: `src/components/workspace-sidebar.tsx`
- Modify: `src/screens/workspace-shell.tsx`
- Modify: `src/index.css`

**Interfaces:**
- Consumes: `DomainMode`, current sidebar action callbacks, visual icons, and `railMode` / `onDismiss` state.
- Produces: `WorkspaceSidebar`, accepting `{ domain, railMode, onDismiss, actions }`.

- [ ] **Step 1: Define a declarative sidebar item model**

```ts
type WorkspaceTool = {
  id: string;
  label: string;
  category: "Project" | "Marine & Subsea" | "Mining Operations" | "GIS & Geodesy" | "Automation & System";
  icon: React.ReactNode;
  onClick: () => void;
  domain?: "mining" | "marine";
  badge?: string;
};
```

- [ ] **Step 2: Move all current tool entries and domain rules**

Move the current tool array and filter logic into `WorkspaceSidebar`. Preserve each identifier, label, icon, badge, callback, and mining/marine visibility rule. Group categories in this order: Project, active domain, GIS & Geodesy, Automation & System.

- [ ] **Step 3: Persist collapsed section state**

Use `localStorage` key `metardu.sidebar.sections` with a JSON record keyed by the five category names. Initialise Project and the active-domain group expanded; initialise GIS and Automation collapsed. Update the stored record after each section toggle.

- [ ] **Step 4: Keep utility actions and improve focus clarity**

Keep New Survey Project, search, rail mode, settings, shortcuts, saved views, daylight theme, and colorblind access. Replace the current orange gradient in changed sidebar controls with the existing flat accent token. Keep native titles where a custom tooltip does not exist.

- [ ] **Step 5: Verify sidebar behavior**

Run: `npm.cmd run build`

In browser preview, verify search, collapsed-state persistence after reload, mining visibility, marine visibility, both-domain visibility, rail mode, and narrow drawer mode. Every previously reachable tool must remain reachable.

- [ ] **Step 6: Commit the isolated sidebar change**

```bash
git add src/components/workspace-sidebar.tsx src/screens/workspace-shell.tsx src/index.css
git commit -m "refactor(workspace): organize survey tool navigation"
```

## Task 4: Extract Header and Status Telemetry

**Files:**
- Create: `src/components/workspace-status-bar.tsx`
- Modify: `src/screens/workspace-shell.tsx`
- Modify: `src/index.css`

**Interfaces:**
- Consumes: `Map | null`, `DomainMode`, current CRS, `useUndoStore`, and the shortcuts callback.
- Produces: `WorkspaceStatusBar`, accepting `{ domain, epsg, map, onOpenShortcuts }`.

- [ ] **Step 1: Move status telemetry**

Preserve the pointer-move subscription, one-second UTC refresh, readiness, coordinates, scale, CRS, domain, undo/redo, commands, shortcuts, UTC, and version.

```ts
useEffect(() => {
  if (!map) return;
  const onMouseMove = (event: { coordinate: number[] }) => {
    // Update coordinates and scale from event.coordinate and map resolution.
  };
  map.on("pointermove", onMouseMove);
  return () => map.un("pointermove", onMouseMove);
}, [map]);
```

- [ ] **Step 2: Tighten status-bar button semantics**

Use Lucide `Undo2` and `Redo2` for undo/redo, retain explanatory titles and disabled states, and keep text-based coordinate, scale, and CRS readouts visible.

- [ ] **Step 3: Keep the native title bar stable**

Keep `TitleBar` in `workspace-shell.tsx` because it owns Tauri window controls and layout profiles. Apply only deterministic token-based visual adjustments, then compose the workflow rail near it. Do not change native window-control behavior.

- [ ] **Step 4: Verify interactive telemetry**

Run: `npm.cmd run build`

In browser preview, move the map cursor, test undo/redo disabled/enabled states, open keyboard shortcuts, and verify UTC advances. Expected: no console errors or layout shift.

- [ ] **Step 5: Commit the isolated telemetry change**

```bash
git add src/components/workspace-status-bar.tsx src/screens/workspace-shell.tsx src/index.css
git commit -m "refactor(workspace): extract survey telemetry status bar"
```

## Task 5: Integrate, Review, and Verify the Cockpit

**Files:**
- Modify: `src/screens/workspace-shell.tsx`
- Modify: `src/index.css`
- Modify: `docs/manual/USER_MANUAL.md`

**Interfaces:**
- Consumes: `WorkflowRail`, `WorkspaceSidebar`, `SurveyBriefPanel`, and `WorkspaceStatusBar`.
- Produces: a composed workspace shell with unchanged dialog callback ownership.

- [ ] **Step 1: Compose components in the existing grid**

Replace inline workspace regions in this order: title/header, workflow rail, sidebar, map, survey brief, status bar. Keep `MapCanvas`, file-drop overlay, CRS banner, map overlays, live-stream panel, and lazy dialogs in the shell.

- [ ] **Step 2: Remove only dead local helpers and imports**

Delete inline `Sidebar`, `SidebarSection`, `SidebarItem`, `RightPanel`, `MetricTile`, `StatTile`, and `StatusBar` only after their imported replacements compile. Keep helpers with their closest consumer.

- [ ] **Step 3: Update user-facing workspace documentation**

Update the workspace overview to describe the workflow rail and survey brief, and state that specialist tools remain accessible from the sidebar and command palette.

- [ ] **Step 4: Run deterministic checks**

Run: `npm.cmd run build`

Run: `npx --yes @21st-dev/cli@latest review src/screens/workspace-shell.tsx src/components/workflow-rail.tsx src/components/workspace-sidebar.tsx src/components/survey-brief-panel.tsx src/components/workspace-status-bar.tsx --json`

Expected: build succeeds. Apply only deterministic, low-ambiguity review fixes; report any CLI or authentication failure rather than claiming a clean review.

- [ ] **Step 5: Run visual and interaction acceptance checks**

Use browser preview at desktop and narrow widths. Check mining, marine, and both-domain empty states; staged files; workflow actions; sidebar search/collapse; panel toggles; map cursor telemetry; command palette; focus visibility; and no truncated or overlapping controls.

- [ ] **Step 6: Commit the integrated workspace refactor**

```bash
git add src/screens/workspace-shell.tsx src/index.css src/components/workflow-rail.tsx src/components/workspace-sidebar.tsx src/components/survey-brief-panel.tsx src/components/workspace-status-bar.tsx docs/manual/USER_MANUAL.md
git commit -m "feat(workspace): deliver survey workflow cockpit"
```

## Plan Self-Review

| Spec requirement | Plan task |
| --- | --- |
| Map remains dominant | Tasks 1 and 5 preserve the current map grid and panel toggles |
| Domain-aware workflow guidance | Task 1 |
| Asset and tool navigator | Task 3 |
| Survey readiness and next action | Task 2 |
| Live status telemetry | Task 4 |
| Accessibility and responsive behavior | Tasks 1-5 verification steps |
| No IPC/calculation changes | Global constraints and Task 5 composition boundary |
| 21st review | Task 5 deterministic check |

The plan assigns every approved requirement and defines component contracts before later tasks consume them.
