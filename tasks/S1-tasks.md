# S1 tasks — Editor core (v0.1.0)

Estimates are working days. Updated 2026-10-20 22:4x CEST — re-implemented on Plan B (FBInk / own renderer).

### S1-T01 — Text buffer with caret/selection
- **Status:** ✅ DONE (`crates/core/src/buffer.rs`)
- **Acceptance:** insert/backspace/delete-forward on UTF-8 never splits a char; selection replaced by typing and removed by Back/Del; ←/→ collapse a selection; ↑/↓ keep the char column across logical lines; `line_number()` 1-based; `revision()` changes only on edits. 6 unit tests.
- **Deps:** S0-T03  **Estimate:** 1 d

### S1-T02 — Soft wrap by measured width
- **Status:** ✅ DONE (`crates/core/src/wrap.rs`)
- **Acceptance:** `wrap()` keeps hard breaks and empty lines, breaks after spaces, splits words wider than a row, covers every byte exactly once; width comes from a `TextMeasure` (mono for Cobalt, glyph advances later); caret at a soft break maps to the next row; `offset_at` maps (row,col)→byte. 6 unit tests. ↑/↓ and page keys move by visual row (`sigma_ui::move_vertical`, 1 test).
- **Deps:** S1-T01  **Estimate:** 1 d

### S1-T03 — Page render (title bar, page, status bar)
- **Status:** ✅ DONE (`crates/render/src/editor.rs`) — Literata 20 px×1.139, leading 1.5, margin 28 px×1.139, brief day tokens, 1 px rules; caret 2 px ink bar.
- **Acceptance:** fits 1440×1920; caret row kept in view; status `N words · line L   Paper · saved [Keys]`; tests in sigma-render.
- **Deps:** S1-T02  **Estimate:** 1 d

### S1-T04 — Portrait on-screen keyboard
- **Status:** ✅ DONE (`crates/core/src/keyboard.rs`, drawn by sigma-render; 33 keys ≥50 px, all hit-tested)
- **Acceptance:** rows exactly `q…p / a…l / Shift z…m Back / 123 Tab Del Space Return`; `123`↔`ABC` symbol layer `1–0 / # * - [ ] ( ) \` / / . , ! ? : | ~ >`; Shift is one-shot and outlined while armed; Del = delete forward, Shift+Del = backspace; Tab inserts 4 spaces (default); keys addressed positionally (`kb.rRcC`), positions stable across Shift/layer. 6 core + 1 ui tests; drive script types via the keys.
- **Deps:** S1-T01  **Estimate:** 1.5 d

### S1-T05 — Keys toggle
- **Status:** ✅ DONE — top-bar `Keys` / `Hide keys`; page grows when hidden (screenshot `v0.1-keys-hidden.png`).
- **Deps:** S1-T04  **Estimate:** 0.25 d

### S1-T06 — Autosave
- **Status:** ✅ DONE — core `Autosave` + `FsNoteStore` (atomic tmp→fsync→rename→dir fsync, path-escape refused) writing `field-notes.md` to `/mnt/onboard/sigma-scribe/notes` on the Kobo (`target/emu-notes` on desktop); seed written on first run; saved on every edit and on power.
- **Acceptance:** status reaches `saved` after typing; text survives app restart (emulator test); failure shows `not saved: reason` and retries on next edit (unit test).
- **Open:** on-device power-cut test (S1-T10).
- **Deps:** S1-T01  **Estimate:** 1 d

### S1-T07 — Page-turn buttons scroll
- **Status:** ✅ DESKTOP (PageUp/PageDown, test) · device: evdev 193 back / 194 forward, check on Forma.
- **Deps:** S1-T03  **Estimate:** 0.25 d

### S1-T08 — Seed content
- **Status:** 🟡 PARTIAL — `field-notes.md` seeded on first run; `drafts/chapter-01.md`, `scratch.txt`, `.scratchpad.md` arrive with the library in S2.
- **Deps:** S1-T06  **Estimate:** 0.25 d

### S1-T09 — Tap-to-place caret
- **Status:** ✅ DONE — tap on the page places the caret at the measured position (test); hardware-keyboard arrows also move it. The Cobalt-era arrow row is gone.
- **Deps:** S1-T03  **Estimate:** 0.5 d

### S1-T12 — Hardware keyboard input path
- **Status:** ✅ DESKTOP — `HwKey` events (chars, Backspace, Delete, Return, Tab, arrows) from the window; same path for USB-OTG in S8.

### S1-T10 — On-device v0.1 checklist
- **Status:** ⏸ after S0-T09 install (docs/install-plan-forma.md §5).
- **Acceptance:** sprint file checklist: 500 words via OSK without missed keys; caret survives scroll; kill app mid-typing keeps last save; reboot restores note; page keys scroll; keystroke-to-ink measured.
- **Deps:** S0-T09  **Estimate:** 0.5 d

### S1-T11 — Rename in title bar
- **Status:** ⏭ MOVED to S2 (needs the library and a text-entry screen).
