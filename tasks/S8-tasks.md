# S8 tasks — USB-OTG keyboard & export

Estimates are working days. Status: all TODO.

### S8-T01 — evdev reader for `/dev/input/event*` (Cobalt capability or platform code; Plan B native)
- **Description:** evdev reader for `/dev/input/event*` (Cobalt capability or platform code; Plan B native).
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S7 done
- **Estimate:** 1 d

### S8-T02 — Keymap, modifiers, Ctrl+F etc.
- **Description:** Keymap, modifiers, Ctrl+F etc..
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S8-T01
- **Estimate:** 1 d

### S8-T03 — USB flag in status bar; in-app powered-hub note
- **Description:** USB flag in status bar; in-app powered-hub note.
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S8-T02
- **Estimate:** 0.5–1 d

### S8-T04 — Export docs (USB folder; Syncthing via Cobalt Sync optional)
- **Description:** Export docs (USB folder; Syncthing via Cobalt Sync optional).
- **Acceptance:** works as specified in design-brief.md and the sprint DoD; covered by a test or on-device checklist item.
- **Deps:** S8-T03
- **Estimate:** 0.5–1 d
