# S4 tasks — Options & Line mode

Estimates are working days. Status 2026-10-21: ✅ all done — passed on the Forma (landscape direction confirmed), tagged `v0.4.0`.

### S4-T01 — Options sheet rows (Paper, Type, Size, Leading, Margin, Tab, Tab width, Keyboard tone, Drawer, USB keyboard)
- **Status:** ✅ DONE (`crates/render/src/options.rs`, `crates/core/src/settings.rs`; tests: every control hits in both orientations, rows follow the brief)
- **Description:** Options sheet rows (Paper, Type, Size, Leading, Margin, Tab, Tab width, Keyboard tone, Drawer, USB keyboard).
- **Acceptance:** works as specified in .ai/design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S3 done
- **Estimate:** 1 d

### S4-T02 — Day/Night with full refresh
- **Status:** ✅ DONE (Night theme everywhere, full refresh on switch, keyboard tone; render + app tests) — on-device ✅
- **Description:** Day/Night with full refresh.
- **Acceptance:** works as specified in .ai/design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S4-T01
- **Estimate:** 1 d

### S4-T03 — Line mode fitting with real measurement; locked 1440 column + gutter in landscape
- **Status:** ✅ DONE (`crates/core/src/linefit.rs`; fake-measurer tests + real-font N=8…16 portrait/landscape; locked column + gutters) — on-device ✅
- **Description:** Line mode fitting with real measurement; locked 1440 column + gutter in landscape.
- **Acceptance:** works as specified in .ai/design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S4-T02
- **Estimate:** 0.5–1 d

### S4-T04 — Landscape keyboard: Centre / Split / Full + num/arrow pad
- **Status:** ✅ DONE (Centre/Split/Full + number/arrow pad, left/right; no overlaps, every key hittable, portrait key size) — on-device ✅
- **Description:** Landscape keyboard: Centre / Split / Full + num/arrow pad.
- **Acceptance:** works as specified in .ai/design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S4-T03
- **Estimate:** 0.5–1 d

### S4-T05 — Persist all settings
- **Status:** ✅ DONE (`.sigma-settings` key=value, v0.2.1 compatible; restart test in app tests and emulator script)
- **Description:** Persist all settings.
- **Acceptance:** works as specified in .ai/design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S4-T04
- **Estimate:** 0.5–1 d
