# S0 tasks — Foundations & Forma bring-up (Plan B)

### S0-T01 — Full backup of the Forma
- **Status:** ✅ DONE 2026-10-20 (re-verified 22:1x after removing SQLite side files I had created)
### S0-T02 — Dev toolchain
- **Status:** ✅ DONE — Rust + armv7 musl, musl.cc gcc in ~/.local/toolchains (no sudo), gh (not logged in), Node
### S0-T03 — Workspace skeleton
- **Status:** ✅ DONE — core / render / device / app (Cobalt layers removed 2026-10-20)
### S0-T04 — CI
- **Status:** 🟡 WRITTEN — fmt, clippy, tests, ARMv7 + FBInk build; not run (no GitHub remote)
### S0-T05 — Desktop simulator at Forma size
- **Status:** ✅ DONE — live window `scripts/sim.sh` (mouse/keys/PageUp/PageDown, `--eink`), headless `sigma-emu` PNGs
### S0-T06 — Cobalt evaluation
- **Status:** ✅ CLOSED — installer refuses N782; SDK limits → Plan B (docs/s0-findings.md)
### S0-T07 — FBInk + evdev backend
- **Status:** 🟡 BUILT — compiles/links for ARM; untested on device
- **Acceptance:** blank/garbled screen never; touch, page keys, power decoded; full + partial refreshes visible.
### S0-T08 — Kobo packaging
- **Status:** ✅ DONE — `scripts/package-kobo.sh`, `packaging/kobo/.adds/{sigma-scribe,nm}`; launcher stops Nickel, reboots on exit
### S0-T09 — Install on the Forma
- **Status:** ⏸ WAITING for Armando (docs/install-plan-forma.md §1–3)
- **Deps:** S0-T01, S0-T08  **Estimate:** 0.5 h
### S0-T10 — On-device checklist + go/no-go
- **Status:** ⏸ after T09 (install plan §5)
### S0-T11 — Device notes for upstream (optional)
- **Status:** ⏸ TODO — e.g. Forma touch transform for FBInk/KOReader users; post only with explicit approval
