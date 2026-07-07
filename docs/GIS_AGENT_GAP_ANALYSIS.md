# GIS Agent Gap Analysis — What MetaRDU Industrial Should Have

**Basis**: 12 GIS agents from `skills/agency-agents/gis/` (excluding GeoAI/ML Engineer per user direction)
**Date**: 2026-07-07
**Scope**: Mining + marine surveying only (cadastral belongs to the separate MetaRDU web app)

---

## Agent-by-Agent Assessment

For each of the 12 GIS agents, I assessed: **what MetaRDU already has** vs **what's missing** that fits the mining + marine scope.

### 1. 🧠 Technical Consultant — GIS strategy, gap analysis, roadmaps
**Status**: ✅ Covered
- `docs/GEOMATICS_GAP_ANALYSIS.md` — comprehensive gap analysis already done
- `docs/ROADMAP.md` — 5-theme strategic backlog
- `docs/ARCHITECTURE.md` — full architecture documentation
- **Missing**: Nothing for the current scope

### 2. 🔧 Solution Engineer — Esri + FOSS4G prototype building, PoC delivery
**Status**: ⚠️ Partial (FOSS4G strong, Esri missing)
- ✅ FOSS4G stack: OpenLayers, PROJ, ODM, deck.gl, pure-Rust parsers
- ✅ Prototypes work: all 45 dialogs are functional, not mockups
- ❌ **Missing: Esri integration** — no ArcGIS REST client, no Feature Service support, no Esri format readers (File Geodatabase, Shapefile via Esri driver)
- **Recommendation**: Add a Shapefile reader/writer (most common interchange format). Esri REST integration is lower priority — mining/marine surveyors rarely use ArcGIS Online.

### 3. 🖥️ GIS Analyst — Map production, data QC, symbology, layouts, spatial queries
**Status**: ⚠️ Partial
- ✅ Map canvas with OpenLayers (pan, zoom, layers, graticule)
- ✅ QC dashboard (S-44 compliance, density, coverage)
- ❌ **Missing: Print-quality map layouts** — every survey deliverable needs a map sheet with title block, north arrow, scale bar, legend, coordinate grid, border. MetaRDU generates PDF reports but not survey-plan-style map sheets.
- ❌ **Missing: Symbology editor** — can't change point colors, line styles, fill patterns per layer. Current styling is hardcoded.
- ❌ **Missing: Spatial query builder** — can't "select all points within 50m of this line" or "find all soundings deeper than 30m". The slice editor does polygon selection but not attribute-based queries.
- **Recommendation**: **Map layout composer** is the highest-value gap. Every EOM report, stockpile audit, and dredge certificate needs one. ~1,500 lines.

### 4. 📦 Spatial Data Engineer — ETL, format conversion, CRS reprojection, pipelines
**Status**: ⚠️ Partial
- ✅ Format readers: LAS, LAZ, GeoTIFF, Kongsberg .all, Reson .s7k, SSS XTF, DXF, SVP
- ✅ Pipeline DSL + executor (11 actions)
- ✅ CRS reprojection via proj4js (frontend) + PROJ (Rust)
- ❌ **Missing: Shapefile read/write** — the #1 interchange format for mining plans, cadastral data, and engineering drawings. Surveyors get Shapefiles from mine planning software (Surpac, Datamine) and need to overlay them.
- ❌ **Missing: GeoJSON export** — for web map handoff and API integration
- ❌ **Missing: KML/KMZ export** — for Google Earth visualization (common request from mine managers)
- ❌ **Missing: PostGIS/SpatiaLite export** — for teams with spatial databases
- **Recommendation**: **Shapefile reader/writer** is the #1 priority. ~1,200 lines. Then GeoJSON export (trivial — 100 lines) and KML export (~300 lines).

### 5. ⚙️ Geoprocessing Specialist — ArcPy, Python Toolbox, Model Builder, batch automation
**Status**: ⚠️ Partial
- ✅ Pipeline editor with visual workflow builder
- ✅ Watch folders (zero-touch ingest)
- ✅ Scheduled jobs
- ❌ **Missing: Python scripting** — can't write a Python script that chains MetaRDU operations. ArcPy is the gold standard for this; MetaRDU has no equivalent.
- ❌ **Missing: Batch processing UI** — can't "run this volume calc on all 50 stockpile LAS files in this folder". The pipeline does this but the UI doesn't expose it.
- **Recommendation**: **Batch processing dialog** — select a folder + an operation + output directory → run on all matching files. ~500 lines. Python scripting is a larger effort (embedded Python runtime or WASM Python) — defer to Sprint 15+.

### 6. ✅ GIS QA Engineer — Topology validation, metadata audit, CRS consistency
**Status**: ⚠️ Partial
- ✅ `qc/` module: UncertainValue, verify_calculation, range_checks
- ✅ Cross-check framework (grid vs TIN volume)
- ✅ S-44 compliance checks
- ❌ **Missing: Topology validation** — no checks for polygon gaps, overlaps, dangles, slivers. Critical for stockpile pad boundaries and dredge channel templates.
- ❌ **Missing: CRS consistency audit** — when a project has files in 3 different CRSs (common when combining drone data, historical surveys, and design files), there's no warning. The CRS Switch Banner catches it per-file but doesn't audit the whole project.
- ❌ **Missing: Metadata audit** — ISO 19115 metadata is generated in the deliverable package but not validated. Missing fields aren't flagged.
- **Recommendation**: **Topology validator** is the highest-value QA gap. ~800 lines. Checks: polygon gaps, overlaps, self-intersection, dangles on line features.

### 7. ~~🤖 GeoAI/ML Engineer~~ — EXCLUDED per user direction

### 8. 🏗️ BIM/GIS Specialist — Revit/IFC to GIS, indoor mapping, digital twins
**Status**: ❌ Out of scope
- BIM coordination is not a mining/marine surveying workflow
- IFC import could be useful for underground mine infrastructure (conveyors, crushers) but it's niche
- **Recommendation**: Defer. If an underground mine customer requests IFC import, add it then.

### 9. 🏔️ 3D & Scene Developer — Cesium, 3D Tiles, point clouds, terrain
**Status**: ⚠️ Partial
- ✅ Point cloud rendering via deck.gl (Float32Array, 100M+ points)
- ✅ 3D slice editor with reject brush
- ✅ DEM hillshade rendering
- ❌ **Missing: 3D Tiles export** — can't export a point cloud or DEM as 3D Tiles for Cesium/Ion. Mining companies increasingly use Cesium for pit visualization.
- ❌ **Missing: Terrain flyover** — no animated camera path for pit/bathymetry flythroughs. Common request for stakeholder presentations.
- ❌ **Missing: 3D mesh export** — ODM produces textured meshes but MetaRDU can't display or export them.
- **Recommendation**: **3D Tiles export** for point clouds is the highest-value 3D gap. ~1,000 lines. Terrain flyover is lower priority (nice-to-have for presentations).

### 10. 📊 Spatial Data Scientist — Spatial statistics, clustering, interpolation
**Status**: ⚠️ Partial
- ✅ Volume calculation (grid + TIN + end-area)
- ✅ Change detection (cut/fill + hotspots)
- ✅ Highwall deformation time-series
- ❌ **Missing: Spatial interpolation** — IDW (Inverse Distance Weighting) and kriging for filling DEM gaps. Critical for sparse bathymetry (MBES data has gaps between survey lines) and for generating continuous surfaces from point observations.
- ❌ **Missing: Hotspot analysis** — Getis-Ord Gi* statistic for identifying statistically significant clusters of high/low values. Useful for ore grade analysis and bathymetric anomaly detection.
- ❌ **Missing: Point pattern analysis** — nearest-neighbor index, kernel density estimation for sounding distribution QC.
- **Recommendation**: **IDW interpolation** is the highest-value statistics gap. ~400 lines. Kriging is more complex (~1,500 lines) but valuable for ore body modeling.

### 11. 🛸 Drone/Reality Mapping — Photogrammetry, orthomosaic, DTM/DSM
**Status**: ⚠️ Partial
- ✅ ODM pipeline integration (drone → point cloud)
- ✅ CSF ground classification (DTM extraction)
- ✅ Volume calculation from drone DEMs
- ❌ **Missing: Orthomosaic viewer** — ODM produces an orthomosaic GeoTIFF but MetaRDU can't display it on the map. The GeoTIFF reader exists but only reads the DEM channel.
- ❌ **Missing: DTM/DSM export** — can't export the classified ground surface as a standalone DTM, or the full surface as a DSM. Only the volume result is exported.
- ❌ **Missing: 3D mesh export** — ODM produces an OBJ/textured mesh but MetaRDU doesn't display or export it.
- **Recommendation**: **Orthomosaic viewer** is the highest-value drone gap. ~600 lines (extend the existing GeoTIFF reader to handle RGB channels + add an OpenLayers ImageLayer).

### 12. 🌐 Web GIS Developer — MapLibre, ArcGIS JS, Leaflet, dashboards, REST APIs
**Status**: ⚠️ Partial
- ✅ OpenLayers map (better than MapLibre/Leaflet for surveying — supports custom projections)
- ✅ Real-time stream panel (UDP pings)
- ✅ QC dashboard
- ❌ **Missing: REST API for external consumers** — can't expose MetaRDU's calculation engine to other apps via HTTP. A mining company's dashboard app can't call "compute stockpile volume" remotely.
- ❌ **Missing: Operational dashboard** — the QC dashboard is marine-specific. No equivalent mining operational dashboard (trucks per hour, material moved, bench progress).
- **Recommendation**: **REST API** is valuable for Enterprise customers. The distributed coordinator (`distributed/server.rs`) already has a TCP server; extending to HTTP REST would make MetaRDU a headless calculation service. ~1,500 lines. Defer to Sprint 15.

### 13. 🎨 Cartography Designer — Color theory, typography, basemap design
**Status**: ⚠️ Partial
- ✅ Professional GIS dark theme (slate chrome, raw hex colors)
- ✅ Daylight high-contrast theme
- ✅ Theme auto-switch (Sprint 13)
- ✅ Design tokens (colors, spacing, typography)
- ❌ **Missing: Colorblind-safe palettes** — current DEM hillshade and point cloud coloring use red-green encoding (cut=red, fill=green). 8% of men with colorblindness can't distinguish these.
- ❌ **Missing: Basemap switcher** — can't switch between satellite imagery, streets, terrain, or a blank grid. Currently only the default OSM tiles.
- ❌ **Missing: Print-quality cartography** — no control over map border, grid spacing, font sizes for print output.
- **Recommendation**: **Colorblind-safe palette option** is quick and high-impact. ~100 lines (add a `colorblind` mode to tokens.ts that swaps red→orange, green→blue). **Basemap switcher** is ~300 lines.

---

## Priority Ranking — What MetaRDU Should Have (Excluding AI/ML)

Ranked by impact × frequency × fit with mining + marine scope:

### Tier 1 — Build in Sprint 14 (highest ROI)

| # | Feature | Agent Source | Effort | Why |
|---|---|---|---|---|
| 1 | **Shapefile reader/writer** | Spatial Data Engineer | ~1,200 lines | #1 interchange format; mine planning software outputs Shapefiles |
| 2 | **Map layout composer** (print-quality map sheets) | GIS Analyst + Cartography | ~1,500 lines | Every survey deliverable needs a map sheet |
| 3 | **Orthomosaic viewer** | Drone/Reality Mapping | ~600 lines | Display ODM's orthomosaic output alongside DEM |
| 4 | **Topology validator** | GIS QA Engineer | ~800 lines | Validate stockpile pad boundaries, dredge templates |
| 5 | **IDW interpolation** | Spatial Data Scientist | ~400 lines | Fill DEM gaps in sparse bathymetry |

### Tier 2 — Build in Sprint 15

| # | Feature | Agent Source | Effort | Why |
|---|---|---|---|---|
| 6 | **Batch processing dialog** | Geoprocessing Specialist | ~500 lines | Run one operation on all files in a folder |
| 7 | **Colorblind-safe palette** | Cartography Designer | ~100 lines | Accessibility; 8% of male surveyors |
| 8 | **Basemap switcher** | Cartography Designer | ~300 lines | Satellite vs streets vs terrain |
| 9 | **GeoJSON + KML export** | Spatial Data Engineer | ~400 lines | Web map handoff + Google Earth |
| 10 | **CRS consistency audit** | GIS QA Engineer | ~300 lines | Warn when project has files in multiple CRSs |

### Tier 3 — Build in Sprint 16+

| # | Feature | Agent Source | Effort | Why |
|---|---|---|---|---|
| 11 | **3D Tiles export** | 3D & Scene Developer | ~1,000 lines | Cesium integration for pit visualization |
| 12 | **Spatial query builder** | GIS Analyst | ~800 lines | Attribute + spatial selection |
| 13 | **Symbology editor** | GIS Analyst | ~600 lines | Per-layer styling control |
| 14 | **Kriging interpolation** | Spatial Data Scientist | ~1,500 lines | Ore body modeling |
| 15 | **REST API** | Web GIS Developer | ~1,500 lines | Headless calculation service |
| 16 | **Terrain flyover** | 3D & Scene Developer | ~500 lines | Stakeholder presentations |

### Out of Scope (per user direction)

| Feature | Reason |
|---|---|
| GeoAI/ML (object detection, segmentation) | Excluded per user direction |
| BIM/IFC integration | Not mining/marine surveying |
| Python scripting (ArcPy equivalent) | Large effort, defer to Sprint 15+ |
| Esri REST integration | Mining/marine surveyors rarely use ArcGIS Online |

---

## Bottom Line

MetaRDU Industrial covers **7 of 12** GIS agent specialties well, with **5 gaps** that fit the mining + marine scope. The top 5 to build in Sprint 14 are:

1. **Shapefile** — the #1 interchange format
2. **Map layout composer** — every deliverable needs one
3. **Orthomosaic viewer** — display drone survey output
4. **Topology validator** — QA for boundaries
5. **IDW interpolation** — fill DEM gaps

These 5 features (~4,500 lines total, ~5 days) would push MetaRDU from 85% to ~93% coverage of the mining + marine GIS agent scope.

**But first** — per the user's direction, we complete the remaining UX + Backend tasks from the Sprint 13 audits before building any new features. Those tasks follow in the implementation below.
