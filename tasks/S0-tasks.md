# S0 tasks — Foundations & Forma gate

Estimates are working days. Status: all TODO. Order matters: nothing is written to the Kobo before S0-T01 is verified.

### S0-T01 — Full backup of the Forma
- **Description:** Mount `KOBOeReader` read-only, `rsync -a` the whole user partition (incl. hidden `.kobo`, `.adds`, `.kobo-images`) to `~/backups/kobo-forma-<FORMA-SERIAL>/<date>/`, write a sha256 manifest. See docs/deployment.md §1.
- **Acceptance:** file count and sha256 manifest of the copy match the device; `KoboReader.sqlite` opens with sqlite3; backup path recorded.
- **Deps:** —  **Estimate:** 0.5 d

### S0-T02 — Dev toolchain
- **Description:** Install rustup (stable), targets `armv7-unknown-linux-gnueabihf`/`musleabihf`, `gcc-arm-linux-gnueabihf`, Node (for `tools/app-contribute.mjs`), clone BandarLabs/Cobalt, build `kobo` CLI.
- **Acceptance:** `kobo --version`, `cargo build --target armv7…` of the template succeed.
- **Deps:** —  **Estimate:** 0.5 d

### S0-T03 — App skeleton in repo
- **Description:** Copy `templates/app/` → `app/` in sigma-scribe, crate `kobo-sigma-scribe`, id `sigma-scribe`; workspace crates `core`, `ui`, `app`; platform trait (`Display`, `Input`, `Store`, `TextMeasure`).
- **Acceptance:** `cargo test` passes; `kobo dev` shows hello screen.
- **Deps:** S0-T02  **Estimate:** 1 d

### S0-T04 — CI
- **Description:** GitHub Actions: fmt, clippy -D warnings, test, ARMv7 static build artifact.
- **Acceptance:** green run on push to main.
- **Deps:** S0-T03  **Estimate:** 0.5 d

### S0-T05 — Forma metrics for simulator
- **Description:** Add 1440×1920 / 1920×1440 Forma metrics (Cobalt tests default to `CLARA_BW_METRICS`).
- **Acceptance:** simulator renders at Forma size; layout tests run with Forma metrics.
- **Deps:** S0-T03  **Estimate:** 0.5 d

### S0-T06 — Install Cobalt + NickelMenu on the Forma
- **Description:** Charge device; install via Cobalt browser installer or `install.sh` (writes `.kobo/KoboRoot.tgz` + NickelMenu); restart; accept the untested-device notice.
- **Acceptance:** Cobalt entry in Kobo menu; launcher opens; exit returns to Nickel.
- **Deps:** S0-T01 (verified)  **Estimate:** 0.5 d

### S0-T07 — Read-only doctor + attended device tests
- **Description:** Run Cobalt's read-only doctor/probe; record identity, framebuffer, touch ranges. Attended: display stages, touch transform, page buttons, rotation, exit, recovery.
- **Acceptance:** results in `docs/s0-findings.md`; all checks pass or failures described.
- **Deps:** S0-T06  **Estimate:** 1 d

### S0-T08 — Spikes
- **Description:** Text input + caret drawing, measured text with Literata/Atkinson, partial-refresh latency, page-key events, raw evdev from a USB-OTG keyboard via powered hub.
- **Acceptance:** each spike has a measured result and a recommended approach.
- **Deps:** S0-T05, S0-T07  **Estimate:** 1.5 d

### S0-T09 — Hello-world on device
- **Description:** Sideload the skeleton build to the Forma (dev flow per Cobalt DEVELOPING.md).
- **Acceptance:** launches, draws, responds to touch and page keys, exits cleanly.
- **Deps:** S0-T07  **Estimate:** 0.5 d

### S0-T10 — Go/No-go
- **Description:** Decide Cobalt (Plan A) vs FBInk + NickelMenu (Plan B) on S0 evidence; optionally open a Forma device thread on BandarLabs/Cobalt with the probe results (needs user OK, it's public).
- **Acceptance:** decision + rationale written into plan.md §Decisions.
- **Deps:** S0-T08, S0-T09  **Estimate:** 0.25 d

### S0-T11 — Draft Forma device report for Cobalt (do not post)
- **Description:** From S0-T07 evidence, draft a Forma (N782, code 377, fw 4.38.23697, kernel 4.1.15) device report in the format of Cobalt `docs/DEVICES.md` rows / device-thread issues: identity, framebuffer, touch ranges/transform, page-button codes, rotation, display stages, exit/recovery results. Save as `docs/cobalt-forma-device-report.md`.
- **Acceptance:** draft complete and reviewed by Armando. **Must not be posted (issue/PR/comment) without explicit user approval.**
- **Deps:** S0-T07, S0-T10  **Estimate:** 0.25 d
