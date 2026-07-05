# Release Checklist

Pre-flight checklist for cutting a MetaRDU Industrial release.
Every item MUST be ticked before tagging.

---

## Pre-release (1 week before)

- [ ] **Security audit** — re-read `SECURITY.md` and verify all
      "Known Gaps" are either fixed or explicitly accepted for this
      release with a documented rationale.
- [ ] **License key rotation** — if this is a major release, generate
      a new RSA-2048 keypair. Update `src-tauri/src/keys/license_pub.pem`
      with the new public key. Store the private key offline (NOT in
      the repo). Document the rotation in `CHANGELOG.md`.
- [ ] **Dependency audit** — run `cargo audit` on every crate.
      Fix or document every advisory. `cargo install cargo-audit`
      if not already installed.
- [ ] **NPM audit** — run `npm audit --audit-level=moderate`. Fix or
      document every finding.

---

## Code freeze

- [ ] **All CI green on `main`** — the `CI` workflow must pass on
      the commit you intend to tag. Check at
      https://github.com/error302/metardu-industrial/actions
- [ ] **No `TODO`/`FIXME` in production paths** —
      `rg "TODO|FIXME" --type rust --type ts -g '!**/target/**'`
      should return only items explicitly deferred to a future
      release with a tracking issue.
- [ ] **Version bump** — update `version` in:
      - `src-tauri/tauri.conf.json`
      - `src-tauri/Cargo.toml`
      - `package.json`
      - `crates/metardu-core/Cargo.toml`
      - `metardu-eom-cli/Cargo.toml`
      - `metardu-verify/Cargo.toml`
      All six must match. Use [semver](https://semver.org): bump
      patch for bugfixes, minor for new features, major for breaking
      changes.
- [ ] **CHANGELOG.md** — add a new section under `## [Unreleased]`
      → rename it to `## [vX.Y.Z] - YYYY-MM-DD`. Move the empty
      `## [Unreleased]` section back to the top for the next cycle.
- [ ] **Git tag** — `git tag -a vX.Y.Z -m "Release vX.Y.Z"` and
      `git push origin vX.Y.Z`. The tag triggers the `release.yml`
      workflow.

---

## Build & sign

The `release.yml` GitHub Actions workflow handles this automatically
when a tag is pushed. It produces:

- Windows MSI + NSIS installers (x64)
- macOS DMG + .app (arm64 + x64)
- Linux .deb + .AppImage (x64)
- `metardu-worker` standalone binary for each platform
- `metardu-eom-cli` standalone binary for each platform
- `metardu-verify` standalone binary for each platform

**Manual verification after the build completes:**

- [ ] Download each artifact and install on a clean VM
- [ ] **Smoke test each platform:**
      - App launches without errors
      - Splash → modules → onboarding → workspace flow works
      - Open a sample LAS file — appears on map
      - Run Volume Calc — produces sane numbers
      - Run EOM Auditor on a small LAS — generates signed PDF
      - Run metardu-verify on the generated PDF — verifies OK
      - Open Settings → change theme → persists across restart
      - Open each of the 33 dialogs — no crash, Escape closes
- [ ] **Verify the auto-updater is OFF** (it's a stub — see
      `src-tauri/src/updater.rs`). The "Check for updates" button
      should return "up to date" silently.
- [ ] **Verify the license system** — generate a trial license
      with `metardu-eom-cli sign-license`, install it, confirm
      the app shows "Active" status. Tamper with the license
      file, confirm the app shows "Invalid".

---

## Distribution

- [ ] **GitHub Release** — create a release at
      https://github.com/error302/metardu-industrial/releases/new
      targeting the tag. Attach all build artifacts. Paste the
      CHANGELOG section as the release notes.
- [ ] **Website update** — if applicable, update the download page
      with links to the new artifacts.
- [ ] **Customer notification** — email customers with active
      licenses. Include:
      - What changed (CHANGELOG summary)
      - Security fixes (if any)
      - How to update (manual re-download until auto-updater is wired)
      - New license key if rotated (attach their re-issued license file)

---

## Post-release

- [ ] **Bump version to `X.Y.Z+1-dev`** in all six version files
      (see above) on a new branch, merge to `main`. This prevents
      confusion about whether a build is "the release" or "post-release
      dev".
- [ ] **Create a GitHub milestone** for the next release and move
      all unfinished issues to it.
- [ ] **Retrospective** — add a section to `docs/RETROSPECTIVE.md`
      (create if missing) noting what went well, what didn't, and
      what to change in this checklist for next time.

---

## Emergency hotfix procedure

If a critical security bug is found after release:

1. Branch from the release tag: `git checkout -b hotfix/vX.Y.Z+1 vX.Y.Z`
2. Fix the bug with the minimal possible change
3. Bump the patch version in all six files
4. Add a `## [vX.Y.Z+1] - YYYY-MM-DD (hotfix)` section to CHANGELOG
5. Tag and push: `git tag -a vX.Y.Z+1 -m "Hotfix: <one-line summary>"`
6. Merge the hotfix branch back to `main` (it should be a fast-forward
   if main hasn't diverged, otherwise a merge commit)
7. Notify customers immediately via email + GitHub Security Advisory

Do NOT bundle other fixes into a hotfix. Hotfixes must be small
enough to review in 5 minutes.
