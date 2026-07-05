# Security Policy

## Supported Versions

MetaRDU Industrial is pre-1.0 software. Security fixes are applied
only to the latest `main` branch — there are no backport releases
yet. Once we ship 1.0, this section will list the supported version
window.

| Version | Supported          |
|---------|--------------------|
| `main`  | ✅ latest commit   |
| tags    | ✅ latest tag only  |
| branches| ❌ not supported    |

## Reporting a Vulnerability

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please report vulnerabilities privately:

1. **Preferred:** use GitHub's private vulnerability reporting at
   https://github.com/error302/metardu-industrial/security/advisories/new
2. **Alternative:** email the maintainer directly (see the commit
   history for the contact).

Please include:
- A description of the vulnerability and its impact
- Steps to reproduce, including any proof-of-concept code
- Affected versions (commit SHA or tag)
- Suggested fix, if you have one

### Response Timeline

- **Acknowledgement:** within 48 hours
- **Initial assessment:** within 7 days
- **Fix or mitigation:** within 30 days for high-severity issues,
  90 days for low-severity
- **Public disclosure:** after a fix is released, coordinated with
  the reporter

## Threat Model

MetaRDU Industrial is a **desktop application** used by surveyors in
the field. The threat model assumes:

- **The user's machine is trusted.** We do not defend against
  malware running on the same machine — the app has the same
  privileges as the user.
- **Survey data files may be untrusted.** LAS/GeoTIFF/XTF files come
  from third-party sensors and software. We parse them in pure Rust
  (no unsafe blocks in the parsers) but bugs in parsing can still
  crash the app. We do NOT currently sandbox file parsing — see
  "Known Gaps" below.
- **The license system is a deterrent, not a hard boundary.** A
  determined attacker with a debugger can bypass the license check.
  Our goal is to make casual piracy harder, not to stop nation-state
  adversaries.
- **Network access is opt-in.** NTRIP/RTCM3 corrections and the
  (future) auto-updater are the only outbound network calls. The
  app works fully offline.

## Security-Relevant Components

### License signing (`crates/metardu-core/src/mining/license.rs`)

- **Scheme:** RSASSA-PSS over SHA-256 (algorithm tag "PS256")
- **Legacy compat:** RSASSA-PKCS1-v1_5-SHA256 (tag "RS256") still
  accepted for licenses issued before the PSS migration
- **Key size:** RSA-2048 (minimum recommended for PSS-SHA256)
- **Bundled public key:** `src-tauri/src/keys/license_pub.pem`
  (committed to the repo — this is intentional, it's a PUBLIC key)
- **Private key:** generated per-issuing-authority, NEVER committed
  to the repo. Stored offline on the signing machine.

**Migration timeline:** the legacy RS256 path will be removed in
v0.3.0. Customers with old licenses should re-request a PSS-signed
license before then.

### NTRIP/RTCM3 parsing (`crates/metardu-core/src/ntrip/mod.rs`)

- **CRC-24Q verification:** every incoming RTCM frame is verified
  against its CRC-24Q. Corrupt frames are dropped and the parser
  resyncs on the next 0xD3 preamble.
- **No outbound data exfiltration:** the NTRIP client only sends
  the auth header (base64-encoded username:password) and the
  mountpoint request. No telemetry, no telemetry, no telemetry.

### Tauri security

- **CSP:** `default-src 'self'` — see `tauri.conf.json`. Inline
  styles are allowed (Tailwind needs them); inline scripts are not.
- **Capabilities:** minimal — only `core:default`, `shell:allow-open`,
  `dialog:allow-open`, `dialog:allow-save`. No filesystem or
  process spawn permissions beyond what Tauri's core provides.
- **No `unsafe-eval`** anywhere in the CSP.

## Known Gaps (Pre-1.0)

These are known security gaps that are NOT yet fixed:

1. **File parsing is not sandboxed.** A malicious LAS/GeoTIFF/XTF
   file can crash the app (and potentially exploit parser bugs).
   The `wasm_sandbox` module exists but is not wired to the file
   parsers. **Mitigation:** only open files from trusted sources.

2. **Auto-updater requires configuration.** `src-tauri/src/updater.rs`
   is now wired to `tauri-plugin-updater` (Ed25519 signature
   verification), but requires a signing keypair + endpoint to be
   configured before it can deliver updates. See `RELEASE.md` §"Auto-
   updater setup". Until configured, the "Check for Updates" dialog
   shows "auto-update not available" with a link to GitHub Releases.

3. **No certificate pinning on NTRIP.** The NTRIP client uses raw
   TCP without TLS. RTCM corrections over the public internet are
   vulnerable to MITM. **Mitigation:** only connect to NTRIP
   casters on trusted networks, or behind a VPN.

4. **License private key history.** The current `license_pub.pem`
   was generated in commit `92e2152`. We have NOT audited the full
   git history to confirm the private key was never accidentally
   committed during development. **Mitigation:** rotate the
   signing key before commercial launch.

5. **No fuzzing.** The file parsers (LAS, GeoTIFF, XTF, S7K, .all)
   have unit tests but no fuzz testing. **Mitigation:** planned
   for v0.2.0 via `cargo-fuzz`.

## Hardening Checklist (Pre-1.0)

Before tagging 1.0, the following MUST be done:

- [ ] Wire the auto-updater (see `src-tauri/src/updater.rs` doc comment)
- [ ] Rotate the license signing keypair
- [ ] Add fuzzing harnesses for all file parsers
- [ ] Audit all `unsafe` blocks (currently zero, but new deps may add some)
- [ ] Penetration test the IPC surface (especially file-path handling)
- [ ] Verify CSP doesn't break any feature in production use
- [ ] Add TLS support to the NTRIP client
