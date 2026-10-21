# S9 tasks — Release

Status 2026-10-21: T02, T03 done; T05, T06 drafts written (files only, not posted); T01 (device regression) and T04 (tag + release) wait for approval and the device.

### ⏳ S9-T01 — Final regression on the Forma
- **Acceptance:** all sprint checklists pass on fw 4.38.23697.  **Estimate:** 1 d
### ✅ S9-T02 — Release workflow
- **Description:** `.github/workflows/release.yml` on tags: build FBInk (pinned), ARM binary, `package-kobo.sh`, upload tarball + sha256.
- **Acceptance:** tagged build reproduces the local package hashes.  **Deps:** S9-T01  **Estimate:** 1 d
### ✅ S9-T03 — Docs & screenshots
- **Acceptance:** README install/uninstall, licences (AGPL-3.0, FBInk GPL-3.0+, OFL), author Sigma Recon, screenshots.  **Estimate:** 0.5 d
### ⏳ S9-T04 — GitHub Release v1.0.0
- **Acceptance:** release published by Armando (needs `gh auth login` + approval).  **Deps:** S9-T02  **Estimate:** 0.25 d
### ✅ (draft) S9-T05 — MobileRead announcement draft
- **Acceptance:** draft reviewed; **posted only with explicit approval**.  **Estimate:** 0.25 d
### ✅ (draft) S9-T06 — Forma device notes (optional)
- **Acceptance:** touch transform / waveform notes for other Forma devs; posted only with approval.  **Estimate:** 0.25 d
