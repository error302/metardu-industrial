# Agency Agents — Skill Library (MetaRDU subset)

Installed from `https://github.com/msitarzewski/agency-agents` on 2026-07-07.

Only the agents relevant to MetaRDU Industrial (mining + marine + GIS surveying) are installed. The full upstream catalog has 200+ agents across 20+ divisions; the rest were skipped to keep the skill inventory focused.

## How to Use

Other agents can `Read` any `.md` file below and follow its instructions to take on that role. The activation format is the upstream "agency-agents" convention — each file is a complete agent definition.

## Installed Agents

### GIS (13 agents) — `gis/`

The full GIS team from the upstream repo. Directly relevant to MetaRDU's mining + marine surveying scope.

| Agent | Use Case |
|---|---|
| `gis-solution-engineer` | End-to-end GIS solution design (CRS, workflows, vendor selection) |
| `gis-spatial-data-engineer` | Spatial data pipelines, ETL, format conversion |
| `gis-geoprocessing-specialist` | Geoprocessing tool chains, raster/vector analysis |
| `gis-analyst` | Spatial analysis, query, attribution |
| `gis-qa-engineer` | **Use this for QA/QC audits** — spatial data quality, accuracy assessment, error propagation |
| `gis-cartography-designer` | Map design, symbology, layout |
| `gis-3d-scene-developer` | 3D scene building, point-cloud visualization |
| `gis-web-gis-developer` | Web GIS frontends (OpenLayers, Leaflet, MapLibre) |
| `gis-bim-specialist` | BIM ↔ GIS integration (IFC, CityGML) |
| `gis-drone-reality-mapping` | Drone photogrammetry, SfM, reality capture |
| `gis-technical-consultant` | Client-facing technical advisory |
| `gis-spatial-data-scientist` | Spatial statistics, clustering, regression |
| `gis-geoai-ml-engineer` | GeoAI/ML — note: MetaRDU's roadmap removed AI/ML from feature scope, but this agent is still useful for advisory |

### Design (9 agents) — `design/`

UI/UX specialists. The `design-ui-designer` is the primary agent for UI audits and visual fixes.

| Agent | Use Case |
|---|---|
| `design-ui-designer` | **UI audit + visual fixes** — call this for layout obstruction issues |
| `design-ux-architect` | Information architecture, navigation flows |
| `design-ux-researcher` | User research, persona validation |
| `design-brand-guardian` | Brand consistency, visual identity enforcement |
| `design-visual-storyteller` | Narrative visual design |
| `design-image-prompt-engineer` | Image generation prompts |
| `design-inclusive-visuals-specialist` | Accessibility (WCAG, color contrast) |
| `design-persona-walkthrough` | Walkthrough reviews against user personas |
| `design-whimsy-injector` | Delightful micro-interactions (use sparingly for pro tools) |

### Spatial Computing (6 agents) — `spatial-computing/`

AR/XR specialists. Useful when MetaRDU's AR companion (already scaffolded in `ar_companion.rs`) gets built out.

| Agent | Use Case |
|---|---|
| `xr-interface-architect` | XR interaction architecture |
| `xr-immersive-developer` | XR app development |
| `xr-cockpit-interaction-specialist` | Vehicle/operator cockpit UI (relevant for survey vessels) |
| `visionos-spatial-engineer` | Apple Vision Pro spatial computing |
| `macos-spatial-metal-engineer` | macOS Metal 3D rendering |
| `terminal-integration-specialist` | Terminal/CLI tool integration |

## Activation Pattern

To activate an agent, the parent agent should:

1. `Read` the agent's `.md` file from this directory.
2. Follow the activation prompt at the top of the file (the upstream format includes an `<agent_activation>` block).
3. Adopt the agent's role, tools, and constraints for the duration of the task.
4. Return a single consolidated report when the task is complete.

## Source

- Upstream: https://github.com/msitarzewski/agency-agents
- License: See upstream LICENSE file (MIT-style)
- Commit at install: latest `main` as of 2026-07-07
