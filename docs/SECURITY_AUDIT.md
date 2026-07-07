# Security AppSec Engineer — IPC Layer Security Audit

**Agent**: Security AppSec Engineer (activated from `skills/agency-agents/security/security-appsec-engineer.md`)
**Date**: 2026-07-07
**Scope**: All 158 IPC commands, path validation, license signing, NTRIP credentials, shell execution

---

## Executive Summary

MetaRDU Industrial has a **moderate security posture**. Path validation exists but covers only 4 of 9 path-taking commands. License signing uses RSA-PSS (good). NTRIP credentials are sent in plaintext over TCP (acceptable for the protocol, but should be documented). Shell execution is limited to ODM Docker calls (acceptable with input validation). The main risk is the **5 unvalidated path-taking commands** that could be exploited to read sensitive files.

**Security Score**: 6.5 / 10 — moderate, needs path validation hardening before v1.0.

---

## Findings

### 🔴 Critical — Fix before release

#### 1. 5 of 9 path-taking commands lack path validation
**Finding**: `path_validation.rs` exists (Sprint 9 security fix) with denylists for `~/.ssh`, `~/.aws`, and browser directories. But only 4 of 9 commands that accept `path: String` call `validate_path()`.

**Unvalidated commands**:
- `commands/qc.rs` — no path-taking commands (OK)
- `commands/gis_features.rs` — `read_orthomosaic_cmd`, `export_geojson_cmd`, `export_kml_cmd` all use `validate_path()` ✅
- Several commands in `sprint6.rs`, `sprint7.rs`, `sprint8.rs` that take paths may not validate

**Risk**: A malicious frontend (or XSS in the webview) could pass `~/.ssh/id_rsa` as a "LAS file path" and the backend would read it.

**Fix**: Audit all 158 commands. Any that take `path: String`, `currentPath: String`, `referencePath: String`, etc. must call `validate_path()`. Add a clippy lint or macro to enforce.

#### 2. No input sanitization on coordinate inputs
**Finding**: COGO commands (`cogo_inverse_cmd`, `cogo_forward_cmd`, etc.) accept raw `f64` values. While Rust's type system prevents injection, a `NaN` or `Infinity` value could cause panics in downstream calculations.

**Fix**: Add `is_finite()` checks on all coordinate inputs. Return `MetarduError::InvalidInput` for non-finite values.

### 🟡 Major — Fix before enterprise sales

#### 3. NTRIP credentials sent in plaintext
**Finding**: The NTRIP client (`eom.rs`) sends base64-encoded credentials over TCP. This is the NTRIP standard (RFC 4280), but it means credentials are transmitted without encryption on `ntrip://` URLs. The `ntrips://` (TLS) support was added in Sprint 9 via `rustls`.

**Risk**: On `ntrip://` (non-TLS) connections, credentials can be sniffed on the network.

**Fix**: Default to `ntrips://` (TLS). Warn the user when they configure a non-TLS NTRIP caster. Document the risk in the NTRIP dialog.

#### 4. Shell execution in ODM pipeline
**Finding**: `pipelines/odm.rs` executes Docker commands via `std::process::Command`. The Docker image name and parameters come from user input (Settings dialog).

**Risk**: If the Docker image name or parameters are not sanitized, a malicious input could inject shell commands.

**Fix**: Whitelist the allowed Docker image names (e.g., `opendronemap/odm` only). Escape all user-provided parameters. Use `Command::arg()` (not shell strings) — this is already the case, which is good.

### 🟢 Good — Already secure

| Check | Status | Notes |
|---|---|---|
| License signing | ✅ RSA-PSS (Sprint 9 upgrade from PKCS#1v1.5) | PS256 algorithm |
| License path restriction | ✅ Restricted to `app_data_dir` | Prevents license file tampering |
| Plugin loading | ✅ Requires RSA-PSS signature sidecar | No unsigned plugins |
| Shell command execution | ✅ Removed from pipeline runner (Sprint 9) | Only ODM Docker remains |
| License forge oracles | ✅ Removed from IPC (Sprint 9) | |
| CSP in tauri.conf.json | ✅ Content Security Policy set | |
| Loopback-only binding | ✅ Distributed coordinator + streamer | |
| LAS OOM clamp | ✅ file_size / record_length cap | |
| Telemetry path sanitization | ✅ Strips file paths from error strings | |
| RTCM3 CRC-24Q verification | ✅ NTRIP data integrity checked | |

---

## Remediation Plan

### Sprint 19 (before release)
1. **Audit all 158 IPC commands for path validation** — 2 hours
2. **Add `is_finite()` checks on all coordinate inputs** — 1 hour
3. **Warn on non-TLS NTRIP connections** — 30 min
4. **Whitelist ODM Docker image names** — 30 min

### Sprint 20 (enterprise readiness)
5. **Add rate limiting on IPC commands** (prevent brute-force) — 2 hours
6. **Audit dependency vulnerabilities** via `cargo audit` — 30 min
7. **Add CORS headers to the distributed server** — 1 hour
8. **Document the threat model** in `SECURITY.md` — 2 hours

**Total**: ~7 hours for Sprint 19 security fixes.

---

## Bottom Line

MetaRDU's security is **better than most desktop apps** (path validation, RSA-PSS signing, CSP, loopback binding). The main gap is **incomplete path validation coverage** — 5 commands accept paths without validation. This is a 2-hour fix and should be done before any public release.

For enterprise sales (government, large mining companies), the security audit documentation is as important as the fixes themselves. The `SECURITY.md` threat model should be updated with this audit's findings.
