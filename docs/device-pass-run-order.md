# Device pass: run order

A companion to `docs/device-checklist-prerelease.md`. That file lists *what* to check, grouped by version. This one says *in what order* to do it, so you install once, set up once and keep every test next to the ones that share its equipment. Tick the boxes in the checklist, not here.

## Before you start (stale notes in the checklist to fix first)
- The checklist header says the Forma runs "1.6.2 + main ea32b50", but the changelog says 1.10.1 was installed with `fonts/cjk.ttf` (Japanese checks reported fine). Read the real state first: Help → App info on the device, or the install plan's run log. Then correct the header.
- Orientation, first item: the g-sensor probe was run on 2026-10-23 and the fix is built (Unreleased in the changelog; the sensor sends raw EV_ABS, not MSC_RAW). Replace that item with the check in session 1.
- Install the **latest build** (1.11.1 plus Unreleased) once, following `docs/install-plan-forma.md`: serial check, never sdb, verified backup, binary-only replace, with you present. Testing older builds only repeats work.
- Also put on the Kobo before the pass: `fonts/cjk.ttf` (Japanese and Chinese, and Korean), the dictionaries (`dict/skk-jisyo.txt`, `pinyin-hans.txt`, `pinyin-hant.txt`) and any Korean pack (`docs/font-packs.md`).
- Collect `sigma-scribe.log` at the end of every session, not only at the end of the pass (first item of the checklist).

## What you need
Forma on charge, the K400 Plus with its receiver and a powered hub or OTG cable, a 60 % wired board if you have one, a capacitive stylus, a computer with USB, about 4 to 5 hours in total.

## Session 1: Basics on the panel (about 45 min, no accessories)
Do these first. If any fails, later sessions are built on it.
1. UI scale and icons: legibility at 200 %, Lucide icons crisp.
2. Settings and scroll bar: legibility, checkbox targets, page buttons in landscape, scroll bar crisp.
3. Keyboard size: the 150 % size at UI 200 %, all sizes, Auto, the slider and its grey state, landscape Split / Full / Centre.
4. Glyphs: the back arrows in Files, Help and the Graph list.
5. Size slider (Settings → Writing): nine dots, the third is the default, text size changes at once.
6. **Orientation (new fix):** Auto turns the screen on the Forma in all four ways; touch lands under the finger in each; page buttons are right upside down; nothing turns while you type. Also the three Orientation buttons in Display.
7. Language override: the Portuguese, English and restart sequence.

## Session 2: Handwriting (about 60 min, finger and stylus)
Run in the order the features were built: Write panel → ink in preview, rename and contact size → Letters and relinks → free writing and word mode → gestures. Then the stray-lines checks (write many ticks and apostrophes; read the log for `corner report ignored`). While you write, note the report rate, A2 residue, finger vs stylus accuracy and which letters trigger a scratch-out or flick. These notes decide the open product decisions (A2, Auto pen).

## Session 3: USB keyboard, pointer and layouts (about 60 min, K400 and a 60 % board)
1. Pointer and USB: touchpad trails, `perf pointer` p50 / p95, two-finger scroll Natural vs Traditional, arrow black in Paper and Night.
2. Choice lists and Esc / back from the keyboard; Left / Right on the focused slider.
3. AltGr and dead keys: FR, DE, PT, then Polish, Czech, Swedish, then Russian, Ukrainian, Greek.
4. Layout list pages (the checklist counts 21 layouts; the real number will be higher now), the chosen layout kept after a restart.

## Session 4: Interface languages (about 30 min)
Set `CurrentLocale` through the Kobo's own language: Portuguese, then French, then German, Spanish, Italian, Dutch, Catalan, Danish, Swedish. Check fit and clipped text at 200 % in portrait and landscape, the special letters, and that About says "machine-drafted". French and Portuguese need a native review anyway (decision 31); mark what a reader would change.

## Session 5: Scripts that need font packs (about 90 min)
Order by how much equipment each needs, easiest first:
1. Vietnamese (Telex, VNI) and Korean: no accessory beyond the font pack; Korean Backspace on a USB keyboard.
2. Japanese: romaji → kana → kanji, phrases, the candidate bar, the あ / A key, Han and kana size and line height.
3. Chinese pinyin (Simplified, then Traditional): time to the first candidates (the dictionary loads once), no ghosting on the bar, full-width punctuation.
4. Arabic, Persian, Urdu, Hebrew: joining, right-to-left caret, Left / Right, selection, wrapping, preview.
5. Indic scripts and Thai: conjuncts, the signs before letters, caret over a letter with its signs, line height, wrapping.

## Session 6: Release gates (about 30 min)
Install from the release tarball (v1.0.0 is public; build the tarball for the version you release), update keeps notes, uninstall leaves nothing. Then fill in the results below and decide the final stable tag.

## After the pass
- Tick the boxes with the date in the checklist, copy log findings into the install plan, and open one issue per failure with the log line.
- Product decisions waiting on your notes: A2 on or off, Auto pen from contact size, Arabic and Indic size factors, word mode's off-by-default status, French and Portuguese review.
- Entries Nickel indexed before 1.11.1 stay in the Kobo library until removed (`docs/install.md`).

## Order logic
Basics before accessories, because a broken panel result invalidates the rest; handwriting early while you are fresh and the finger and stylus are warm; keyboard layouts and interface languages together because they share a Settings path and a restart; the script packs last because each needs a large file on the device and the heaviest typing.
