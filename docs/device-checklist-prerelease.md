# Pre-release device checklist (one pass before the final stable release)

> **Standing order (user, 2026-10-22):** development past v1.4 uses only the emulator and CI. **All device testing waits for one pass on the Kobo Forma before the final stable release.** Every version adds its device checks here instead of asking for an install. Nothing on this list has been run unless it is ticked with a date. Install for the pass follows `docs/install-plan-forma.md` (needs approval): serial check (<FORMA-SERIAL>), never sdb, checksum-verified backup, binary-only replace.

Installed on the Forma today: **1.4.0** (2026-10-22). Built and waiting: 1.4.1 … 1.4.6 (and the diagnostic 1.4.6-probe, draft PR #22).

## Logs to collect during the pass
- [ ] `/mnt/onboard/.adds/sigma-scribe/sigma-scribe.log` (perf lines: typing `perf:` p50/p95, `perf pointer:` p50/p95, count of `gl16-cleanup`).
- [ ] If the probe build is used: `gsensor-probe.log` (procedure in the install plan, "1.4.6-probe g-sensor log").

## Pointer and USB (v1.0.1–v1.0.4, v1.4.1; S10, S10.3, S10.4, S14.1)
- [ ] K400 touchpad circles over text and the title bar for 30 s: no trails; arrow responsive.
- [ ] `perf pointer:` p50/p95 a few ms; typing p95 with the pointer in use near 1.0.0 levels; about one `gl16-cleanup` per real pause.
- [ ] Natural vs Traditional two-finger scroll in note, preview, Files and Graph.
- [ ] Arrow stays solid black with a white outline over text, keyboard and title bar, Paper and Night, portrait and landscape; never white after hide/show.

## UI scale and icons (v1.1, S11)
- [ ] Legibility and fit at 200 % on the panel (title bar, sheets, drawer, Find, status bar); Lucide icons crisp.

## Orientation (v1.2, S12)
- [ ] Read-only g-sensor probe: node and codes per hold (see the probe procedure); map 0x19 / 0x1a.
- [ ] After the mapping and an approved `ENABLED = true` build: touch lands under the finger in all four rotations; page buttons right upside down; no turning mid-typing.

## Keyboard layouts (v1.3, S13)
- [ ] AltGr on the K400 and a 60 % board; dead keys from a real USB keyboard (FR, DE, PT).

## UI languages (v1.4, S14)
- [ ] The real `Kobo eReader.conf` is read; switching the Kobo's language changes the app's after a restart; FR / PT fit at 200 %.

## Settings and scroll bar (v1.4.2, S14.2)
- [ ] Settings legibility and tap accuracy at 200 %, checkbox targets included.
- [ ] Choice lists and Esc / back with a USB keyboard.
- [ ] Page buttons on a long Settings page in landscape.
- [ ] Editor scroll bar crisp with page buttons, wheel and typing; no grey residue.

## Keyboard size (v1.4.3–v1.4.5, S14.3–S14.5)
- [ ] At UI 200 % the keyboard is the 150 % size; typing is comfortable.
- [ ] Sizes 100 / 125 / 150 apply at once; Auto follows UI scale below 150 %.
- [ ] Landscape Split / Full / Centre fit; keys easy to hit.
- [ ] Existing settings load as before (Keyboard size shows Auto).
- [ ] Slider: tap and drag snap cleanly; with Auto off, drags are crisp (DU) with no grey edges.
- [ ] Auto on: slider grey, thumb at the Auto size; toggling Auto leaves no ghost; Auto off keeps the size.
- [ ] Left / Right on the K400 move the focused slider; the bigger focused thumb is noticeable.

## Glyphs (v1.4.6, S14.6)
- [ ] Files "Up to Files" and Help "Back to Help" arrows look right at 200 %; Graph list arrows legible.

## Write panel (v1.5.0, S15.0)
- [ ] Write opens the panel; ink follows a finger and a capacitive stylus with little lag; strokes are smooth enough (report rate while moving: note it).
- [ ] Live ink is crisp black (DU); ~0.6 s after lifting, one GL16 pass leaves no residue; no flashing.
- [ ] Fast ink (A2) on: faster? any residue? Decide whether A2 stays an option (plan §12.9 decision 6).
- [ ] Finger / Stylus widths look right; palm or second finger does not ink.
- [ ] Undo, Clear, Cancel, Insert; the note gets the `![ink](…)` line; the SVG and strokes files are on the Kobo next to the note and open on a computer.
- [ ] Landscape and UI 200 %: buttons easy to hit; closing the panel leaves no ink ghost.

## Ink preview, rename and contact size (v1.5.1, S15.1)
- [ ] Inserted ink shows in preview, crisp, no grey edges (Paper black, Night white); page turns over it leave no ghost.
- [ ] Renaming a note with ink keeps its images in preview; the `.ink/<new name>/` folder is on the Kobo, the old one gone.
- [ ] Moving a note with ink to another folder keeps its images; `.ink/` follows it.
- [ ] A strokes file written on the Forma has `"contact":"touch_major"` and four numbers per point (or `null` if the panel does not report it — note which, and the finger vs stylus values for Auto later).

## Release gates (S9)
- [ ] Install from the release tarball; update keeps notes; uninstall clean (S9 checklist).

New versions append their checks above this line, under their own heading.
