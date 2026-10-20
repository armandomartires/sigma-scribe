# S0 tasks — Foundations & Forma gate

Estimates are working days. Status per task below. Order matters: nothing is written to the Kobo before S0-T01 is verified.

### S0-T01 — Full backup of the Forma
- **Status:** ✅ DONE 2026-10-20 — sha256 manifests match, sqlite integrity ok (docs/s0-findings.md)
- **Description:** Mount `KOBOeReader` read-only, `rsync -a` the whole user partition (incl. hidden `.kobo`, `.adds`, `.kobo-images`) to `~/backups/kobo-forma-<FORMA-SERIAL>/<date>/`, write a sha256 manifest. See docs/deployment.md §1.
- **Acceptance:** file count and sha256 manifest of the copy match the device; `KoboReader.sqlite` opens with sqlite3; backup path recorded.
- **Deps:** —  **Estimate:** 0.5 d

### S0-T02 — Dev toolchain
- **Status:** ✅ DONE — Rust targets, clippy/rustfmt, Node 20, kobo CLI 0.3.27, user-space ARM musl gcc (no sudo), gh (not logged in)
- **Description:** Install rustup (stable), targets `armv7-unknown-linux-gnueabihf`/`musleabihf`, `gcc-arm-linux-gnueabihf`, Node (for `tools/app-contribute.mjs`), clone BandarLabs/Cobalt, build `kobo` CLI.
- **Acceptance:** `kobo --version`, `cargo build --target armv7…` of the template succeed.
- **Deps:** —  **Estimate:** 0.5 d

### S0-T03 — App skeleton in repo
- **Status:** ✅ DONE — workspace crates/core (no Cobalt dep), crates/ui, app (`kobo-sigma-scribe`, id `sigma-scribe`); kobo-sdk pinned by git rev
- **Description:** Copy `templates/app/` → `app/` in sigma-scribe, crate `kobo-sigma-scribe`, id `sigma-scribe`; workspace crates `core`, `ui`, `app`; platform trait (`Display`, `Input`, `Store`, `TextMeasure`).
- **Acceptance:** `cargo test` passes; `kobo dev` shows hello screen.
- **Deps:** S0-T02  **Estimate:** 1 d

### S0-T04 — CI
- **Status:** 🟡 WRITTEN — .github/workflows/ci.yml; not run yet (no GitHub remote)
- **Description:** GitHub Actions: fmt, clippy -D warnings, test, ARMv7 static build artifact.
- **Acceptance:** green run on push to main.
- **Deps:** S0-T03  **Estimate:** 0.5 d

### S0-T05 — Forma metrics for simulator
- **Status:** 🟡 PARTIAL — `FORMA_METRICS` used in layout tests; Cobalt has no Forma sim profile (needs a kobo-profile port), screenshots use libra-2-388
- **Description:** Add 1440×1920 / 1920×1440 Forma metrics (Cobalt tests default to `CLARA_BW_METRICS`).
- **Acceptance:** simulator renders at Forma size; layout tests run with Forma metrics.
- **Deps:** S0-T03  **Estimate:** 0.5 d

### S0-T06 — Install Cobalt + NickelMenu on the Forma
- **Status:** ⏸ BLOCKED — needs Armando holding the Forma
- **Description:** Charge device; install via Cobalt browser installer or `install.sh` (writes `.kobo/KoboRoot.tgz` + NickelMenu); restart; accept the untested-device notice.
- **Acceptance:** Cobalt entry in Kobo menu; launcher opens; exit returns to Nickel.
- **Deps:** S0-T01 (verified)  **Estimate:** 0.5 d

### S0-T07 — Read-only doctor + attended device tests
- **Status:** ⏸ BLOCKED — needs device
- **Description:** Run Cobalt's read-only doctor/probe; record identity, framebuffer, touch ranges. Attended: display stages, touch transform, page buttons, rotation, exit, recovery.
- **Acceptance:** results in `docs/s0-findings.md`; all checks pass or failures described.
- **Deps:** S0-T06  **Estimate:** 1 d

### S0-T08 — Spikes
- **Status:** 🟡 PARTIAL — desktop spikes done (text input via terminal grid + caret, positional OSK, page keys, store limits); device latency/evdev pending
- **Description:** Text input + caret drawing, measured text with Literata/Atkinson, partial-refresh latency, page-key events, raw evdev from a USB-OTG keyboard via powered hub.
- **Acceptance:** each spike has a measured result and a recommended approach.
- **Deps:** S0-T05, S0-T07  **Estimate:** 1.5 d

### S0-T09 — Hello-world on device
- **Status:** ⏸ BLOCKED — needs device
- **Description:** Sideload the skeleton build to the Forma (dev flow per Cobalt DEVELOPING.md).
- **Acceptance:** launches, draws, responds to touch and page keys, exits cleanly.
- **Deps:** S0-T07  **Estimate:** 0.5 d

### S0-T10 — Go/No-go
- **Status:** 🟡 INPUT READY — recommendation in docs/s0-findings.md; awaiting device tests + decision
- **Description:** Decide Cobalt (Plan A) vs FBInk + NickelMenu (Plan B) on S0 evidence; optionally open a Forma device thread on BandarLabs/Cobalt with the probe results (needs user OK, it's public).
- **Acceptance:** decision + rationale written into plan.md §Decisions.
- **Deps:** S0-T08, S0-T09  **Estimate:** 0.25 d

### S0-T11 — Draft Forma device report for Cobalt (do not post)
- **Status:** ⏸ TODO after T07
- **Description:** From S0-T07 evidence, draft a Forma (N782, code 377, fw 4.38.23697, kernel 4.1.15) device report in the format of Cobalt `docs/DEVICES.md` rows / device-thread issues: identity, framebuffer, touch ranges/transform, page-button codes, rotation, display stages, exit/recovery results. Save as `docs/cobalt-forma-device-report.md`.
- **Acceptance:** draft complete and reviewed by Armando. **Must not be posted (issue/PR/comment) without explicit user approval.**
- **Deps:** S0-T07, S0-T10  **Estimate:** 0.25 d
