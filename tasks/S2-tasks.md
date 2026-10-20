# S2 tasks — Library (v0.2.0)

Estimates are working days. Branch `feat/v0.2-library`. Desktop/emulator status as of 2026-10-20; on-device check pending.

### S2-T01 — Library model + safe paths (core)
- **Status:** ✅ DONE (`crates/core/src/library.rs`)
- **Description:** `NotePath` (relative, `/`-separated) and `Library` over one root: `/mnt/onboard/sigma-scribe/notes` on the Kobo, `SIGMA_NOTES` override.
- **Acceptance:** rejects `..`, `.`, empty parts, hidden parts, backslashes, control chars; a leading `/` is library-relative, never absolute; lists only `.md`/`.txt` (any case), hides dotfiles and temp files; folders first then notes, case-insensitive; unit tests.
- **Deps:** S1  **Estimate:** 1 d

### S2-T02 — File operations with collision and safety rules (core)
- **Status:** ✅ DONE (unit tests `create_list_and_collisions`, `rename_…`, `move_rules`, `delete_rules`)
- **Acceptance:** create note/folder never overwrites (`untitled.md`, `untitled 2.md`, … case-insensitive, vfat-safe); rename cleans the name (`a/b` → `a b`, `../../evil` → `evil.md`, empty → `untitled`), keeps `.md`/`.txt` or adds `.md`, suffixes on collision, handles case-only renames; move refuses a folder into itself or its children and a missing destination, suffixes on collision; delete removes a note or a folder with its contents, refuses the scratch file, the root, and anything that would leave zero notes; writes are atomic (tmp → fsync → rename → dir fsync).
- **Deps:** S2-T01  **Estimate:** 1 d

### S2-T03 — Files sheet (render + app)
- **Status:** ✅ DONE (`crates/render/src/files.rs`; screenshots `v0.2-files.png`, `v0.2-folder.png`)
- **Acceptance:** folder icon in the editor title bar opens a full-page sheet (full refresh); rows ≥ 76 brief px with name and `N words` / `Folder`; open note marked (bar + bold); tap folder enters, `← Up to …` crumb goes up; tap note opens it in the editor (current note saved first); page-turn keys page the list; X / Esc closes; all targets ≥ 44 brief px (render tests).
- **Deps:** S2-T02  **Estimate:** 1 d

### S2-T04 — More → Rename / Move / Delete (armed)
- **Status:** ✅ DONE (screenshots `v0.2-move.png`, `v0.2-delete-armed.png`, `v0.2-files-after.png`)
- **Acceptance:** More reveals three actions under the row; Delete arms (`Tap again to delete`, inverted) and only a second Delete tap on the same row deletes — any other tap disarms; deleting the open note opens another note and the deleted file is not re-created on save/quit (app test); Move lists destination folders (not the current parent, not the folder itself/children) with Cancel; errors (e.g. last note) show as a message line.
- **Deps:** S2-T03  **Estimate:** 1 d

### S2-T05 — Rename in place from the title bar (was S1-T11)
- **Status:** ✅ DONE (screenshot `v0.2-rename-title.png`, app test)
- **Acceptance:** tap the file name → field with caret, Cancel and OK; OSK and USB keyboard type into the field; Return/OK/blur confirm, Cancel/Esc discard; unsaved edits are saved before the rename; the open note follows the rename, including when its folder is renamed or moved.
- **Deps:** S2-T02  **Estimate:** 0.5 d

### S2-T06 — New note / New folder
- **Status:** ✅ DONE
- **Acceptance:** New note creates `untitled.md` in the folder shown, opens it with the title field in rename; New folder creates `new folder` and starts in rename; collisions suffix.
- **Deps:** S2-T03  **Estimate:** 0.5 d

### S2-T07 — Seed library + last-opened restore
- **Status:** ✅ DONE (app test `seeds_library_and_restores_last_opened_note`)
- **Acceptance:** first run with no notes writes `field-notes.md`, `drafts/chapter-01.md`, `scratch.txt`, `.scratchpad.md`; an existing library (e.g. the v0.1 note on the Forma) is never re-seeded or overwritten; last-opened note stored in `notes/.sigma-last-opened` and restored on launch; falls back to `field-notes.md` / first note if it was deleted over USB.
- **Deps:** S2-T02  **Estimate:** 0.5 d

### S2-T08 — Emulator script, screenshots, CI, ARM package
- **Status:** ✅ DONE — `tests/emu/v0.2-library.script` (also run in CI), `docs/screenshots/v0.2-*.png`, `dist/sigma-scribe-0.2.0-kobo.tar.gz`
- **Acceptance:** both v0.1 and v0.2 scripts pass; static ARMv7 binary builds; package has the same layout as 0.1.0 (upgrade = replace `.adds/sigma-scribe/sigma-scribe`).
- **Deps:** S2-T03…T07  **Estimate:** 0.5 d

### S2-T09 — On-device v0.2 checklist
- **Status:** ✅ PASS on Forma 2026-10-21 (user report). Tagged `v0.2.0`.
- **Acceptance:** v0.1 note still there and opens on launch; create nested folder, move a note into it, rename from the title bar and from More, armed delete; last note cannot be deleted; reboot restores the last note; all files visible as `.md`/`.txt` on the USB mount under `sigma-scribe/notes`; names with spaces survive vfat.
- **Deps:** S2-T08  **Estimate:** 0.5 d
