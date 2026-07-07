# Skills Audit — Upgrade Opportunities for MetaRDU Industrial

**Date**: 2026-07-07
**Scope**: All installed agency-agents skills (GIS, design, spatial-computing, engineering)
**Purpose**: Identify which agents can be activated to upgrade and improve the app beyond what's already been done

---

## Installed Skills Inventory

| Division | Count | Installed |
|---|---|---|
| GIS | 13 | ✅ All 13 (excluding GeoAI/ML per user direction) |
| Design | 9 | ✅ All 9 |
| Spatial Computing | 6 | ✅ All 6 |
| Engineering | 34 | ✅ All 34 |
| **Total** | **62** | |

---

## Already Activated Agents

| Agent | Sprint | What It Did |
|---|---|---|
| UI Designer | 12-13 | Dialog shell, component standardization, map overlay audit |
| UX Researcher | 13 | Workflow friction audit (12 findings, Nielsen scorecard) |
| Backend Architect | 13 | IPC architecture audit (10 findings, timeout + error fixes) |
| GIS QA Engineer | 15-16 | Topology validator methodology |
| Spatial Data Engineer | 16 | Shapefile ETL methodology |

---

## High-Value Agents Not Yet Activated

### Tier 1 — Activate Next (highest ROI)

#### 1. 🎨 Brand Guardian (`design-brand-guardian.md`)
**What it does**: Enforces visual consistency across the product — color, typography, spacing, iconography.
**Why MetaRDU needs it**: The app has 45+ dialogs with 6 historical button-padding variants. Sprint 12 created `DialogShell` but only 2 dialogs have been migrated. The Brand Guardian would do a full audit of every dialog + screen against the design tokens and flag every inconsistency.
**Upgrade opportunity**: A "design system compliance" report listing every component that deviates from `tokens.ts` + `DialogShell` + `DialogButton`.
**Activate for**: A full visual-consistency audit pass before v1.0.

#### 2. 🎨 Inclusive Visuals Specialist (`design-inclusive-visuals-specialist.md`)
**What it does**: Ensures WCAG AA/AAA compliance — color contrast, focus indicators, screen reader support.
**Why MetaRDU needs it**: Sprint 17 added a colorblind palette, but WCAG compliance goes further — contrast ratios on every text/background pair, keyboard navigation completeness, ARIA labels on all interactive elements. The UX Researcher audit found aria-labels on only 6 of 45 dialogs.
**Upgrade opportunity**: A WCAG compliance report with per-component contrast ratios + ARIA coverage + keyboard navigation test results.
**Activate for**: Accessibility audit before government/enterprise sales (often a procurement requirement).

#### 3. 🏗️ Code Reviewer (`engineering-code-reviewer.md`)
**What it does**: Systematic code review for quality, security, performance, maintainability.
**Why MetaRDU needs it**: The codebase is now ~30,000 lines of Rust + ~25,000 lines of TypeScript across 16 sprints. No systematic code review has been done. The Backend Architect found 10 issues; the Code Reviewer would find more at the implementation level (not just architecture).
**Upgrade opportunity**: A code-review report covering: unsafe Rust usage, unwrap() calls that could panic, TypeScript `any` types, missing error handling, dead code, performance anti-patterns.
**Activate for**: A pre-v1.0 code quality audit.

#### 4. 🏗️ SRE (`engineering-sre.md`)
**What it does**: Reliability, monitoring, incident response, runbooks.
**Why MetaRDU needs it**: The app has no crash recovery (if a calculation panics, the app dies). No error reporting beyond the telemetry dialog. An SRE agent would design the crash-recovery + error-reporting pipeline.
**Upgrade opportunity**: A reliability runbook + crash recovery design (auto-save project state before long operations, recover from panics, report crashes to the telemetry backend).
**Activate for**: Designing the crash recovery system before field deployment.

#### 5. 📦 Database Optimizer (`engineering-database-optimizer.md`)
**What it does**: Database schema, indexing, query optimization.
**Why MetaRDU needs it**: The project file format is JSON (`.metardu`). As projects grow (50+ files, 100+ reports), loading + saving becomes slow. The Database Optimizer would evaluate whether to migrate to SQLite (already a dependency via SpatiaLite) for project state.
**Upgrade opportunity**: A schema design for SQLite-backed project files with indexed file lookups + report history.
**Activate for**: Project file format optimization.

### Tier 2 — Activate in Sprint 18+

#### 6. 🏗️ Software Architect (`engineering-software-architect.md`)
**What it does**: High-level architecture review, design patterns, technical debt.
**Why MetaRDU needs it**: The app has grown organically across 16 sprints. The Software Architect would review the module boundaries, dependency graph, and identify refactoring opportunities (e.g., the `commands/` module has 16 submodules — should they be consolidated?).
**Upgrade opportunity**: An architecture refactoring roadmap.

#### 7. 🎨 UX Architect (`design-ux-architect.md`)
**What it does**: Information architecture, navigation flows, user journey maps.
**Why MetaRDU needs it**: The sidebar has 8 sections + 50+ items. The command palette has 50+ actions. The UX Architect would re-evaluate the information architecture — is the sidebar organization optimal? Should some dialogs be tabs instead?
**Upgrade opportunity**: A reorganized sidebar + dialog consolidation proposal.

#### 8. 📦 Data Engineer (`engineering-data-engineer.md`)
**What it does**: Data pipeline design, ETL, data quality.
**Why MetaRDU needs it**: The watch-folder pipeline + batch processing (Sprint 17) are ETL pipelines. The Data Engineer would review the pipeline DSL + executor for robustness.
**Upgrade opportunity**: Pipeline error handling + retry logic + data quality checks.

#### 9. 🏗️ DevOps Automator (`engineering-devops-automator.md`)
**What it does**: CI/CD, automation, deployment pipelines.
**Why MetaRDU needs it**: The GitHub Actions CI has 8 jobs but no staging environment, no automated release process, no canary testing. The DevOps Automator would design the release pipeline.
**Upgrade opportunity**: A staged release pipeline (dev → staging → production) with automated regression tests.

#### 10. ✅ Testing — Accessibility Auditor (`testing-accessibility-auditor.md`)
**What it does**: Automated + manual accessibility testing.
**Why MetaRDU needs it**: Complements the Inclusive Visuals Specialist — the auditor runs actual accessibility tests (axe-core, keyboard navigation, screen reader) rather than heuristic evaluation.
**Upgrade opportunity**: An accessibility test suite that runs in CI.

### Tier 3 — Niche but Useful

#### 11. 🏗️ Technical Writer (`engineering-technical-writer.md`)
**What it does**: Documentation, API references, user guides.
**Why MetaRDU needs it**: The docs are extensive (`docs/manual/IPC_REFERENCE.md` is 643 lines) but not all new Sprint 12-17 commands are documented. The Technical Writer would update the IPC reference + write a user manual.
**Upgrade opportunity**: Updated IPC reference + a "Getting Started" guide for new surveyors.

#### 12. 🏗️ Git Workflow Master (`engineering-git-workflow-master.md`)
**What it does**: Branch strategy, commit conventions, release management.
**Why MetaRDU needs it**: The git history has 16+ sprints of commits. The Git Workflow Master would design a branching strategy for v1.0 (feature branches, release branches, hotfix branches).
**Upgrade opportunity**: A git workflow document + release branching strategy.

#### 13. 🎨 Persona Walkthrough (`design-persona-walkthrough.md`)
**What it does**: Walks through the app as a specific persona to find friction.
**Why MetaRDU needs it**: The UX Researcher created 3 personas (Sarah, James, Maria) but didn't do full walkthroughs. The Persona Walkthrough agent would do a click-by-click walkthrough as each persona.
**Upgrade opportunity**: Persona-specific friction reports.

#### 14. 🏗️ Senior Developer (`engineering-senior-developer.md`)
**What it does**: Mentoring, code quality, best practices.
**Why MetaRDU needs it**: A senior developer review of the Rust code for idiomatic patterns (avoiding `clone()`, using `&str` vs `String`, proper error propagation).
**Upgrade opportunity**: A Rust idioms refactoring guide.

---

## Spatial Computing Agents (Niche)

| Agent | Potential Use |
|---|---|
| XR Interface Architect | Design the AR companion app's interaction model (currently scaffolded in `ar_companion.rs`) |
| XR Immersive Developer | Build the AR companion for HoloLens/Magic Leap (pit visualization in AR) |
| XR Cockpit Interaction Specialist | Design the on-vessel dashboard layout for survey vessels |
| visionOS Spatial Engineer | Build a Vision Pro app for 3D pit visualization (future product) |
| macOS Spatial Metal Engineer | Optimize 3D rendering for macOS (if Linux/macOS builds are added) |
| Terminal Integration Specialist | Build a CLI version of MetaRDU for headless server use |

**Recommendation**: Defer all spatial-computing agents until the AR companion is prioritized (Sprint 18+). The XR Cockpit Interaction Specialist is the most relevant for MetaRDU's marine surveying use case.

---

## Skills Not Yet Installed (From Upstream)

The upstream `agency-agents` repo has 200+ agents. The following divisions are NOT installed and could be useful:

| Division | Useful Agents | Why |
|---|---|---|
| Testing | Performance Benchmarker, API Tester, Reality Checker | CI test suite expansion |
| Security | AppSec Engineer, Penetration Tester, Compliance Auditor | Pre-v1.0 security audit |
| Support | Analytics Reporter, Executive Summary Generator | Field usage analytics |

**Recommendation**: Install the Testing + Security divisions before v1.0. The Security division is especially important — the app handles license keys, file paths, and network connections (NTRIP, NOAA API).

---

## Recommendation: Sprint 18 Skill Activation Plan

| Sprint | Agent | Deliverable |
|---|---|---|
| 18 | Brand Guardian | Full visual-consistency audit + migration plan for remaining 43 dialogs |
| 18 | Code Reviewer | Code quality report (Rust + TypeScript) with severity-ranked issues |
| 19 | Inclusive Visuals Specialist | WCAG AA compliance report + remediation plan |
| 19 | SRE | Crash recovery design + error reporting pipeline |
| 20 | Software Architect | Architecture refactoring roadmap |
| 20 | UX Architect | Sidebar reorganization + dialog consolidation proposal |
| 20 | Technical Writer | Updated IPC reference + user manual |

**Install before Sprint 18**: Testing division (7 agents) + Security division (10 agents) from upstream.

---

## Bottom Line

MetaRDU has 62 skills installed but has only activated 5. The highest-value activations for the next 2 sprints are:

1. **Brand Guardian** — finish the dialog standardization that DialogShell started
2. **Code Reviewer** — catch implementation-level bugs the Backend Architect didn't find
3. **Inclusive Visuals Specialist** — WCAG compliance for government/enterprise sales
4. **SRE** — crash recovery for field reliability
5. **Testing + Security divisions** — install + activate for pre-v1.0 hardening

Each activation is ~1-2 hours of agent time and produces a concrete deliverable (audit report + remediation plan). The pattern is proven: the UX Researcher + Backend Architect audits in Sprint 13 directly drove the Sprint 14-17 improvements.
