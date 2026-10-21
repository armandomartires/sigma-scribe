# S8 tasks — USB-OTG keyboard & export

Estimates are working days. Status: all done in code and emulator 2026-10-21; on-device confirmation (host mode, hub) pending.

### S8-T00 — ✅ Route USB Ctrl/Shift combos through the S2.5 shortcut code
- **Acceptance:** same bindings as the on-screen Ctrl (A/C/X/V/Z/Y, word jump, selection).

### S8-T01 — ✅ evdev keyboard reader for `/dev/input/event*` (Forma USB host mode via powered hub)
- **Description:** evdev keyboard reader for `/dev/input/event*` (Forma USB host mode via powered hub).
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S7 done
- **Estimate:** 1 d

### S8-T02 — ✅ Keymap, modifiers, Ctrl+F etc.
- **Description:** Keymap, modifiers, Ctrl+F etc..
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S8-T01
- **Estimate:** 1 d

### S8-T03 — ✅ USB flag in status bar; in-app powered-hub note
- **Description:** USB flag in status bar; in-app powered-hub note.
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S8-T02
- **Estimate:** 0.5–1 d

### S8-T04 — ✅ Export docs (notes are already .md on USB; Syncthing optional later)
- **Description:** Export docs (notes are already .md on USB; Syncthing optional later).
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S8-T03
- **Estimate:** 0.5–1 d
