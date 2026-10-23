# Pre-release device checklist (one pass before the final stable release)

> **Standing order (user, 2026-10-22):** development past v1.4 uses only the emulator and CI. **All device testing waits for one pass on the Kobo Forma before the final stable release.** Every version adds its device checks here instead of asking for an install. Nothing on this list has been run unless it is ticked with a date. Install for the pass follows `docs/install-plan-forma.md` (needs approval): serial check (<FORMA-SERIAL>), never sdb, checksum-verified backup, binary-only replace.

Installed on the Forma today: **1.6.2 + main ea32b50** (2026-10-23, binary only; see the install plan run log). Earlier line, kept for the record: 1.4.0. Built and waiting: 1.4.1 … 1.4.6 (and the diagnostic 1.4.6-probe, draft PR #22).

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

## Letters and wiki relinks (v1.5.2, S15.2, S15.2a)
- [ ] Write → Letters: 5 boxes (6 landscape) with baselines; ink is crisp black (DU) in a box and does not spill into the next box.
- [ ] Write a few words box by box at a natural speed: each letter appears at the caret after ~0.8 s or as soon as the next box starts; nothing is lost when writing fast. Note how often the first guess is right (lowercase, capitals, digits, accents é è ê ç à ã õ, punctuation . , ' ? !).
- [ ] Candidate chips: tapping one replaces the letter; Space, Erase, Return, Draw, Done do what they say; the page behind updates without ghosts.
- [ ] Time a letter read on the device (the log or a stopwatch): should feel instant (estimate 1–3 ms recognition + one partial refresh).
- [ ] Train: write ~20 characters, Save each; Done; the first read after it is not slowed (fine-tuning runs in the background, est. 1.5–2 s for 126 samples); accuracy on those characters improves. `sigma-scribe/notes/.handwriting/samples.txt` exists on the Kobo and is plain text; Files does not show it.
- [ ] Settings → Handwriting → My handwriting shows the sample count; Delete removes the file and the folder.
- [ ] Finger vs capacitive stylus: does stroke width or contact change accuracy? Note for Auto (v1.5.x).
- [ ] Rename a note that other notes link to (`[[name]]`, `[[name|label]]`, `[[name#part]]`): the links in the other notes and the scratch pad now point at the new name; move a note to a folder: path links follow.

## Free writing and word mode (v1.5.3, S15.3)
- [ ] Write opens the new panel: candidate bar, 3 × 4 keys, blank area; all black and white, no grey left after strokes (DU), keys easy to hit with a finger; same in landscape.
- [ ] Print a few words with small gaps between letters: after ~1.2 s each is typed with a space; note how many characters are wrong (compare: 19–28 % on held-out synthetic fonts). Try capitals, digits with 123, accents with the globe on FR/PT.
- [ ] Write a word with joined letters: is it readable? Then turn on Word mode (experimental) and compare.
- [ ] Candidate bar: a word replaces the last reading; ‹ › page; × closes; ⌫, ↵, , . ? ! @ and space type at once.
- [ ] Timing: a printed word reads at once (estimate ≤ 25 ms), joined words ≤ 0.4 s.
- [ ] Settings → Letter boxes On brings back the 1.5.2 boxes; Train still works and returns to the free area.

## Gestures (v1.5.4, S15.4)
- [ ] Scratch out a half-written word: it disappears and nothing is typed. Scratch out an empty area right after a word was typed: the word and its space go.
- [ ] Write normally for a few minutes: does any letter (z, s, 8, m, w, a dash) get taken as a scratch-out or flick? Note which.
- [ ] Flick left in the middle of a line deletes one character; at a line start it joins the lines. Flick right types a space.
- [ ] Tap a word in the note above the panel: the caret moves there, the panel stays, the next word is typed there.
- [ ] Draw: a scratch-out erases only the strokes it crosses.

## Language override (v1.5.5, S15.5)
- [ ] With the Kobo in Portuguese: Settings → Language shows Override system language unticked and Interface language greyed "Português".
- [ ] Tick it: still Portuguese; Interface language → English switches the app to English at once; the Kobo's own language is unchanged.
- [ ] Restart the app (and the Kobo): the app stays English. Untick: back to Portuguese.

## Latin layouts (v1.6.0, S16.0)
- [ ] Settings → Keyboard & input → Layout lists 18 layouts over two pages; pick Polish (programmer): the on-screen Alt row shows ą ś ł ż ź ć ń ę ó.
- [ ] USB keyboard on Polish: AltGr+A = ą, Shift+AltGr+L = Ł. On Czech: Shift+´ (caron) then s = š. On Swedish: å ä ö keys type directly.
- [ ] Restart: the chosen layout is kept.

## Cyrillic and Greek layouts (v1.6.1, S16.1)
- [ ] Layout lists 21 layouts; pick Russian: the Full board shows ЙЦУКЕН, the Compact board fits 12 keys in its top row (readable, tappable).
- [ ] USB keyboard on Russian: keys type Cyrillic; AltGr+Q = q, Shift+AltGr+3 = #, Ctrl+Z (Я key) undoes.
- [ ] Ukrainian: the key above Enter types ґ; Greek: ; then ε types έ (tonos), Shift+; then ι types ϊ.
- [ ] Key labels in Cyrillic/Greek are readable (serif fallback font) in portrait and landscape.

## UI languages (v1.6.2, S16.2)
- [ ] With `CurrentLocale=pt-PT` the app starts in Portuguese; Settings → Language shows the system language.
- [ ] Override system language → Interface language lists 10 languages; pick Deutsch, Español, Italiano, Nederlands, Català, Dansk, Svenska in turn: titles and rows change at once and nothing is clipped (German and Dutch are the longest).
- [ ] ä ö ü ß å æ ø ñ à ç are drawn correctly on the panel in the UI font; About says the translation is machine-drafted.
- [ ] Landscape: the language list and Settings fit in every language.
- [ ] Switch the Kobo's own language to German, Spanish, Italian, Dutch, Catalan, Danish or Swedish and relaunch: the app follows (override off).

## Stray lines and Size slider (main after 1.6.2)
- [ ] Write → Letters / Train: write many small ticks and apostrophes; no long lines from the box's top corners appear.
- [ ] Afterwards read `.adds/sigma-scribe/sigma-scribe.log` over USB (ea32b50 ignores corner reports in the decoder: look for `corner report (…) ignored` lines and check that no stray lines remain): any `touch:` lines (second contact, slot or tracking-id change, jump, dropped move) show where stray reports came from; copy them into the install plan.
- [ ] Settings → Writing: Size is a slider with nine dots and no number; tapping and dragging change the note's text size at once; the default is the third dot.
- [ ] The Writing line on the Settings page shows only the typeface.

## Vietnamese and Korean typing (v1.7.0, v1.7.1)
- [ ] Layout → Vietnamese (Telex): `vieetj` → việt, `nguwowif` → người on the Full board and a USB keyboard; `aaa` → aa; a tone key after a moved caret starts a new word.
- [ ] Layout → Vietnamese (VNI): `Vie6t5` → Việt; digits still type digits after a space.
- [ ] Vietnamese letters and tones crisp at 100–200 % (Literata, no box glyphs).
- [ ] Korean needs the font pack first (docs/font-packs.md): `fonts/korean.ttf` in `.adds/sigma-scribe/`.
- [ ] Layout → Korean: jamo make syllables as you type, Backspace takes back one jamo; Shift gives ㅃ ㅉ ㄸ ㄲ ㅆ ㅒ ㅖ; AltGr types Latin and Markdown marks.
- [ ] Hangul legible at the body size; line wrap and the caret sit right on Hangul text; refresh while a syllable changes (no ghosting).
- [ ] Without the pack, Hangul draws as boxes (known; the layout is not hidden yet).

## Release gates (S9)
- [ ] Install from the release tarball; update keeps notes; uninstall clean (S9 checklist).

New versions append their checks above this line, under their own heading.
