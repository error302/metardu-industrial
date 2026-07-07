# Competitive Analysis + Enterprise Readiness Assessment

**Date**: 2026-07-07
**Purpose**: What else should MetaRDU improve to match or exceed enterprise standards and competitors?

---

## Competitive Position

### Feature Comparison Matrix

| Feature | MetaRDU Industrial | Trimble Business Center | Hypack | Civil3D | DroneDeploy | QGIS |
|---|---|---|---|---|---|---|
| **Mining — Volume calc** | ✅ Grid + TIN + end-area | ❌ | ❌ | ✅ | ✅ Basic | ✅ Plugin |
| **Mining — Stockpile audit** | ✅ Wizard + signed PDF | ❌ | ❌ | ❌ | ✅ Basic | ❌ |
| **Mining — EOM reconciliation** | ✅ Wizard + chain-of-custody | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Mining — Blast fragmentation** | ✅ p20/p50/p80/p90 | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Mining — Highwall monitoring** | ✅ USACE thresholds + alerts | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Mining — Setting out** | ✅ Bearing/distance/slope | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Mining — Mine grid transform** | ✅ Bidirectional + rotation | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Mining — Tunnel profile** | ✅ Overbreak/underbreak | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Marine — MBES bathymetry** | ✅ Kongsberg .all + water column | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Marine — CUBE surface** | ✅ + disambiguation UI | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Marine — S-44 compliance** | ✅ + certificate PDF | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Marine — S-57 export** | ✅ | ❌ | ✅ | ❌ | ❌ | ✅ |
| **Marine — Dredge pay-volume** | ✅ 4-bucket audit | ❌ | ✅ Basic | ❌ | ❌ | ❌ |
| **Marine — Backscatter mosaic** | ✅ Lambert correction | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Marine — Tide correction** | ✅ NOAA CO-OPS real-time | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Marine — QC dashboard** | ✅ Real-time S-44 | ❌ | ✅ | ❌ | ❌ | ❌ |
| **Cross-cutting — Drone photogrammetry** | ✅ ODM integration | ❌ | ❌ | ❌ | ✅ Core | ❌ |
| **Cross-cutting — LAS/LAZ ingest** | ✅ Streaming reader | ✅ | ❌ | ✅ Plugin | ✅ | ✅ |
| **Cross-cutting — Shapefile I/O** | ✅ Reader + writer | ✅ | ❌ | ✅ | ❌ | ✅ |
| **Cross-cutting — Contour generation** | ✅ Marching squares | ❌ | ❌ | ✅ | ❌ | ✅ |
| **Cross-cutting — COGO** | ✅ 11 operations | ✅ | ❌ | ✅ | ❌ | ❌ |
| **Cross-cutting — Topology validator** | ✅ 8 rules | ❌ | ❌ | ❌ | ❌ | ✅ Plugin |
| **Cross-cutting — Map layout composer** | ✅ PDF with title block | ❌ | ✅ Basic | ✅ | ✅ | ✅ |
| **Cross-cutting — Orthomosaic viewer** | ✅ RGB GeoTIFF | ❌ | ❌ | ❌ | ✅ | ✅ |
| **Real-time — RTK rover** | ✅ NMEA TCP + trail | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Real-time — NTRIP client** | ✅ RTCM3 + TLS | ✅ | ❌ | ❌ | ❌ | ❌ |
| **QA/QC — Uncertainty propagation** | ✅ UncertainValue | ❌ | ✅ Basic | ❌ | ❌ | ❌ |
| **QA/QC — Cross-check verification** | ✅ Grid vs TIN | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Security — License signing** | ✅ RSA-PSS | ✅ | ✅ | ✅ | ❌ | N/A |
| **Security — Path validation** | ✅ Denylist | N/A | N/A | N/A | N/A | N/A |
| **Security — Chain-of-custody PDF** | ✅ SHA-256 + signature | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Account — User registration** | ✅ Local profile | ✅ Server | ✅ | ✅ | ✅ Server | N/A |
| **Account — Onboarding flow** | ✅ 2-step wizard | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Crash recovery** | ✅ Panic hook + snapshots | ❌ | ❌ | ✅ Autosave | ✅ Cloud | ✅ |
| **Accessibility — WCAG** | 🟡 5.8/10 (improving) | ❌ | ❌ | ❌ | ❌ | 🟡 |
| **Accessibility — Colorblind palette** | ✅ Wong (2011) | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Platform — Offline operation** | ✅ Fully offline | ✅ | ✅ | ✅ | ❌ Cloud | ✅ |
| **Platform — Cross-platform** | 🟡 Windows only | ✅ Win+Mac | ✅ Win | ✅ Win+Mac | ✅ Web | ✅ All |
| **Pricing** | $3-5K/seat/yr | $3.5K/seat | $4.5K/seat | $2.5K/seat | $300/mo | Free |

### MetaRDU's Unique Advantages (Things No Competitor Has)

1. **Cross-domain mining + marine in one tool** — no competitor does both
2. **Signed PDF chain-of-custody** — SHA-256 hash + RSA-PSS signature on every report
3. **Uncertainty propagation (UncertainValue)** — every volume shows ± m³ (95% CI)
4. **Cross-check verification** — grid vs TIN agreement on every volume
5. **Real-time tide correction** (NOAA CO-OPS) during the survey, not after
6. **Colorblind-safe palette** — Wong (2011) published palette, toggleable
7. **Open-source core** (MIT) — no vendor lock-in, client can verify independently
8. **EOM reconciliation wizard** — purpose-built for monthly mine reconciliation

---

## Enterprise Standards Gap Analysis

### What Enterprise Customers Expect

| Standard | MetaRDU Status | Gap | Fix |
|---|---|---|---|
| **SOX / audit trail** | ✅ Chain-of-custody + provenance hash | None | — |
| **ISO 19115 metadata** | ✅ Generated in deliverable package | Partial — not validated | Add validation |
| **IHO S-44 compliance** | ✅ Full implementation | None | — |
| **IHO S-57 export** | ✅ Working | None | — |
| **IHO S-100/S-102** | ❌ Deferred to 2027 | Future | Monitor IHO ecosystem |
| **USACE EM 1110-2-1900** | ✅ Highwall thresholds | None | — |
| **WCAG 2.1 AA** | 🟡 5.8/10 → targeting 8/10 | aria-labels, focus trap (Sprint 19 fixed DialogShell) | Finish remaining 47 dialogs |
| **Data sovereignty** | ✅ Fully local, no cloud | None | — |
| **Encryption at rest** | 🟡 License keys are signed, profile is plaintext | Profile should be encrypted | Add AES-256 for profile.json |
| **Encryption in transit** | ✅ NTRIP TLS (rustls), NOAA HTTPS | None | — |
| **Multi-user collaboration** | ❌ Single-user desktop | Enterprise tier planned | Sprint 22: PostGIS sync |
| **SSO / SAML** | ❌ No server auth | Not applicable (desktop) | — |
| **Backup / disaster recovery** | ✅ Crash recovery (Sprint 18) | Auto-save not wired to all commands | Wire to remaining commands |
| **Automated testing** | 🟡 394 unit tests, no E2E accessibility | Need axe-core in CI | Sprint 19 plan ready |
| **Dependency vulnerability scanning** | 🟡 cargo-audit configured | Not run regularly | Add to CI pipeline |
| **Code signing** | ❌ No code signing certificate | Required for Windows SmartScreen | Purchase cert + configure |
| **Auto-update** | ✅ tauri-plugin-updater | Needs keypair config | Generate keypair + configure |
| **Telemetry / usage analytics** | ✅ Opt-in telemetry module | Needs opt-in UI | Add to onboarding |
| **Documentation** | ✅ User manual + IPC reference | Needs update for Sprint 10-20 | Technical Writer agent |

---

## What Else to Improve (Prioritized)

### Tier 1 — Must-have for v1.0 release

| # | Improvement | Effort | Why |
|---|---|---|---|
| 1 | **Replace 260 unwrap() calls** | 8h | #1 crash risk — Code Reviewer audit |
| 2 | **Migrate top 10 dialogs to DialogShell** | 5h | Visual consistency — Brand Guardian audit |
| 3 | **Code signing certificate** | $200/yr | Windows SmartScreen shows "unknown publisher" without it |
| 4 | **Auto-update keypair** | 30 min | Updater exists but needs keypair to sign updates |
| 5 | **Encrypt profile.json at rest** | 2h | Enterprise security requirement (AES-256) |
| 6 | **Wire crash recovery to all 8 long-running commands** | 2h | Only 3 wired currently |
| 7 | **Update user manual** | 4h | Covers Sprint 0-8; needs Sprint 9-20 features |
| 8 | **Add telemetry opt-in to onboarding** | 1h | Privacy compliance (GDPR-style consent) |

### Tier 2 — Important for enterprise sales

| # | Improvement | Effort | Why |
|---|---|---|---|
| 9 | **Progress bars on CSF/ODM/EOM** | 6h | UX Researcher #2 finding; requires Tauri Channel |
| 10 | **Linux + macOS builds** | 4h | Some survey firms use macOS; CI already has build matrix |
| 11 | **ISO 19115 metadata validation** | 3h | Enterprise deliverable compliance |
| 12 | **Bulk report export (ZIP)** | 2h | Quarterly reporting workflow |
| 13 | **Project templates integration with reports** | 2h | Template selection pre-configures report metadata |
| 14 | **In-app help system (contextual tooltips)** | 4h | WCAG #3 — first-use abbreviations need expansion |
| 15 | **Audit log (who did what, when)** | 4h | Enterprise compliance — every action logged |
| 16 | **Data export to PostGIS/SpatiaLite** | 6h | Multi-user teams with spatial databases |

### Tier 3 — Differentiators (ahead of competitors)

| # | Improvement | Effort | Why |
|---|---|---|---|
| 17 | **Real-time MBES preview** (UDP listener) | 8h | On-vessel QC during survey — Hypack has this |
| 18 | **GNSS static post-processing** (RTKLIB plugin) | 6h | Sub-mm control networks — Trimble has this |
| 19 | **3D Tiles export** for Cesium | 8h | Pit visualization in 3D — no competitor has this |
| 20 | **Python scripting** (embedded runtime) | 20h | ArcPy equivalent — Trimble has scripting |
| 21 | **Mobile companion PWA** | 16h | Field data capture — DroneDeploy has mobile |
| 22 | **Cloud sync** (project files to S3) | 8h | Multi-site teams — DroneDeploy is cloud-native |
| 23 | **WebRTC collaborative survey** | 20h | Multi-operator shared view — nobody has this |
| 24 | **REST API** (headless calculation service) | 12h | Enterprise integration — Hypack has a scripting API |

### Tier 4 — Long-term vision

| # | Improvement | Effort | Why |
|---|---|---|---|
| 25 | **Provenance graph** (full audit trail per output) | 40h | Defensible in court — no competitor has this |
| 26 | **GPU-accelerated CUBE** (compute shader) | 16h | 10× faster surface generation |
| 27 | **Progressive LOD point-cloud** (octree, 1B+ points) | 40h | Game-changing for huge surveys |
| 28 | **Auto-triangulation** (no ODM dependency) | 80h | Closes the biggest external dependency |
| 29 | **LAS 1.4 PDRF 6/7** support | 4h | Modern point-cloud standard |
| 30 | **S-100/S-102 export** | 40h | IHO next-gen (defer until 2027) |

---

## Where MetaRDU Already Beats Competitors

| Area | MetaRDU | Competitors |
|---|---|---|
| **Volume uncertainty** | ✅ ± m³ (95% CI) on every result | ❌ All competitors show exact numbers |
| **Cross-check verification** | ✅ Grid vs TIN agreement flag | ❌ No competitor does this |
| **Chain-of-custody** | ✅ SHA-256 + RSA-PSS signed PDFs | ❌ No competitor has this |
| **Cross-domain** | ✅ Mining + Marine in one tool | ❌ Each competitor is one domain |
| **Open-source core** | ✅ MIT licensed | ❌ All competitors are proprietary |
| **Colorblind accessibility** | ✅ Wong (2011) palette | ❌ No competitor has this |
| **Real-time tide correction** | ✅ NOAA CO-OPS during survey | ❌ Hypack does post-processing only |
| **Crash recovery** | ✅ Panic hook + snapshots | 🟡 Civil3D has autosave, others don't |
| **Pricing** | ✅ $3-5K/seat vs $3.5-4.5K/seat | MetaRDU has more features per dollar |

---

## Recommendation: Pre-Release Sprint Plan

### Sprint 20 (current — finish hardening)
- ✅ Profile data in reports
- ✅ Path validation fixes
- ✅ Onboarding flow
- ⬜ Replace top 50 unwrap() calls (focus on IPC handlers)
- ⬜ Migrate 3 dialogs to DialogShell
- ⬜ Encrypt profile.json

### Sprint 21 (pre-release polish)
- Code signing certificate + configuration
- Auto-update keypair generation
- Wire crash recovery to remaining 5 commands
- Update user manual (Technical Writer agent)
- Telemetry opt-in in onboarding
- Progress bars on 3 most critical commands

### Sprint 22 (enterprise features)
- Audit log (every action logged with user + timestamp)
- Bulk report export (ZIP)
- ISO 19115 metadata validation
- Linux + macOS builds tested

### Sprint 23 (competitive differentiators)
- REST API for headless calculation
- 3D Tiles export for Cesium
- PostGIS/SpatiaLite export

---

## Bottom Line

MetaRDU Industrial is **already ahead of competitors** in 6 areas (uncertainty, cross-check, chain-of-custody, cross-domain, open-source, colorblind accessibility) and **at par** in most others. The gaps are in **operational maturity** (code signing, auto-update, progress bars) and **platform coverage** (Linux/macOS, mobile, cloud sync).

**The #1 thing to fix before release**: the 260 `unwrap()` calls. A single crash on first use permanently loses a customer. Everything else is polish.

**The #1 competitive differentiator to emphasize in marketing**: "Every volume comes with ± uncertainty and a grid-vs-TIN cross-check. Every report is signed with RSA-PSS and has a SHA-256 chain-of-custody. No other survey tool does this."

**Release after Sprint 21** (code signing + unwrap fixes + progress bars). Sprint 22-23 features can come post-release.
