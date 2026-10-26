# Pre-release device checklist (one pass before the final stable release)

> **Standing order (user, 2026-10-22):** development past v1.4 uses only the emulator and CI. **All device testing waits for one pass on the Kobo Forma before the final stable release.** Every version adds its device checks here instead of asking for an install. Nothing on this list has been run unless it is ticked with a date. Install for the pass follows `docs/install-plan-forma.md` (needs approval): serial check (<FORMA-SERIAL>), never sdb, checksum-verified backup, binary-only replace.

Installed on the Forma: **local build of `v1.13.1-write-fixes` (PR 48), head `a3baa08`**, binary sha256 `298e92ae…d981` (2026-10-25; the 1.11.1 label is unchanged; dictionaries and notices from 99d6887; see the install plan run log). **Special-language sections (Cyrillic/Greek, Vietnamese, Korean, Arabic and Hebrew, Indic and Thai, Japanese, Chinese) wait for feedback from native users; they are not release gates** (user, 2026-10-25). Run order: `docs/device-pass-run-order.md`.

## Logs to collect during the pass
- [x] `/mnt/onboard/.adds/sigma-scribe/sigma-scribe.log` (perf lines: typing `perf:` p50/p95, `perf pointer:` p50/p95, count of `gl16-cleanup`). — 2026-10-24: pulled after the 1.11.1 pass (copy in `~/backups/kobo-forma-N7828B0052845/2026-10-24-device-pass-log/`).
- [ ] If the probe build is used: `gsensor-probe.log` (procedure in the install plan, "1.4.6-probe g-sensor log").

## Pointer and USB (v1.0.1–v1.0.4, v1.4.1; S10, S10.3, S10.4, S14.1)
- [x] K400 touchpad circles over text and the title bar for 30 s: no trails; arrow responsive.
- [x] `perf pointer:` p50/p95 a few ms; typing p95 with the pointer in use near 1.0.0 levels; about one `gl16-cleanup` per real pause. — 2026-10-24 (1.11.1, 109 keys, 452 moves): pointer move-to-submit p50 3.2 ms, p95 14.9 ms, max 179 ms; typing p50 64.7 ms, p95 104.0 ms, max 276 ms; 154 `gl16-cleanup` (session was ~14 h mostly idle, so cleanups per real pause not measurable: the log has no per-line times).
- [x] Natural vs Traditional two-finger scroll in note, preview, Files and Graph.
- [x] Arrow stays solid black with a white outline over text, keyboard and title bar, Paper and Night, portrait and landscape; never white after hide/show.

## UI scale and icons (v1.1, S11)
- [x] Legibility and fit at 200 % on the panel (title bar, sheets, drawer, Find, status bar); Lucide icons crisp.

## Orientation (v1.2, S12)
- [x] Read-only g-sensor probe, 2026-10-23: the Forma sends raw EV_ABS, not MSC_RAW; fixed in the Unreleased build (Auto orientation reads the accelerometer).
- [x] Auto turns the screen in all four ways on the Forma; touch lands under the finger in each; page buttons right upside down; no turning mid-typing.
- [x] Settings → Display → Orientation: the three buttons (Auto, Portrait, Landscape) work and show the choice.

## Keyboard layouts (v1.3, S13)
- [x] AltGr on the K400 and a 60 % board; dead keys from a real USB keyboard (FR, DE, PT).

## UI languages (v1.4, S14)
- [x] The real `Kobo eReader.conf` is read; switching the Kobo's language changes the app's after a restart; FR / PT fit at 200 %.

## Settings and scroll bar (v1.4.2, S14.2)
- [x] Settings legibility and tap accuracy at 200 %, checkbox targets included.
- [x] Choice lists and Esc / back with a USB keyboard.
- [x] Page buttons on a long Settings page in landscape.
- [x] Editor scroll bar crisp with page buttons, wheel and typing; no grey residue.

## Keyboard size (v1.4.3–v1.4.5, S14.3–S14.5)
- [x] At UI 200 % the keyboard is the 150 % size; typing is comfortable.
- [x] Sizes 100 / 125 / 150 apply at once; Auto follows UI scale below 150 %.
- [x] Landscape Split / Full / Centre fit; keys easy to hit.
- [x] Existing settings load as before (Keyboard size shows Auto).
- [x] Slider: tap and drag snap cleanly; with Auto off, drags are crisp (DU) with no grey edges.
- [x] Auto on: slider grey, thumb at the Auto size; toggling Auto leaves no ghost; Auto off keeps the size.
- [x] Left / Right on the K400 move the focused slider; the bigger focused thumb is noticeable.

## Glyphs (v1.4.6, S14.6)
- [x] Files "Up to Files" and Help "Back to Help" arrows look right at 200 %; Graph list arrows legible.

## Write panel (v1.5.0, S15.0)
- [x] Write opens the panel; ink follows a finger and a capacitive stylus with little lag; strokes are smooth enough (report rate while moving: note it).
- [x] Live ink is crisp black (DU); ~0.6 s after lifting, one GL16 pass leaves no residue; no flashing.
- [x] Fast ink (A2) on: faster? any residue? Decide whether A2 stays an option (plan §12.9 decision 6).
- [x] Finger / Stylus widths look right; palm or second finger does not ink.
- [/] Undo, Clear, Cancel, Insert; the note gets the `![ink](…)` line; the SVG and strokes files are on the Kobo next to the note and open on a computer.
- [x] Landscape and UI 200 %: buttons easy to hit; closing the panel leaves no ink ghost.

## Ink preview, rename and contact size (v1.5.1, S15.1)
- [x] Inserted ink shows in preview, crisp, no grey edges (Paper black, Night white); page turns over it leave no ghost.
- [x] Renaming a note with ink keeps its images in preview; the `.ink/<new name>/` folder is on the Kobo, the old one gone.
- [x] Moving a note with ink to another folder keeps its images; `.ink/` follows it.
- [ ] A strokes file written on the Forma has `"contact":"touch_major"` and four numbers per point (or `null` if the panel does not report it — note which, and the finger vs stylus values for Auto later).

## Letters and wiki relinks (v1.5.2, S15.2, S15.2a)
- [x] Write → Letters: 5 boxes (6 landscape) with baselines; ink is crisp black (DU) in a box and does not spill into the next box.
- [x] Write a few words box by box at a natural speed: each letter appears at the caret after ~0.8 s or as soon as the next box starts; nothing is lost when writing fast. Note how often the first guess is right (lowercase, capitals, digits, accents é è ê ç à ã õ, punctuation . , ' ? !).
- [x] Candidate chips: tapping one replaces the letter; Space, Erase, Return, Draw, Done do what they say; the page behind updates without ghosts.
- [x] Time a letter read on the device (the log or a stopwatch): should feel instant (estimate 1–3 ms recognition + one partial refresh).
- [x] Train: write ~20 characters, Save each; Done; the first read after it is not slowed (fine-tuning runs in the background, est. 1.5–2 s for 126 samples); accuracy on those characters improves. `sigma-scribe/notes/.handwriting/samples.txt` exists on the Kobo and is plain text; Files does not show it.
- [x] Settings → Handwriting → My handwriting shows the sample count; Delete removes the file and the folder.
- [x] Finger vs capacitive stylus: does stroke width or contact change accuracy? Note for Auto (v1.5.x).
- [x] Rename a note that other notes link to (`[[name]]`, `[[name|label]]`, `[[name#part]]`): the links in the other notes and the scratch pad now point at the new name; move a note to a folder: path links follow.

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
- [x] With the Kobo in Portuguese: Settings → Language shows Override system language unticked and Interface language greyed "Português".
- [x] Tick it: still Portuguese; Interface language → English switches the app to English at once; the Kobo's own language is unchanged.
- [x] Restart the app (and the Kobo): the app stays English. Untick: back to Portuguese.

## Latin layouts (v1.6.0, S16.0)
- [x] Settings → Keyboard & input → Layout lists 18 layouts over two pages; pick Polish (programmer): the on-screen Alt row shows ą ś ł ż ź ć ń ę ó.
- [x] USB keyboard on Polish: AltGr+A = ą, Shift+AltGr+L = Ł. On Czech: Shift+´ (caron) then s = š. On Swedish: å ä ö keys type directly.
- [x] Restart: the chosen layout is kept.

## Cyrillic and Greek layouts (v1.6.1, S16.1)
> **Deferred:** waits for feedback from native users of this language; not a release gate (user, 2026-10-25). The items stay open until someone who writes it has tried them.
- [ ] Layout lists 21 layouts; pick Russian: the Full board shows ЙЦУКЕН, the Compact board fits 12 keys in its top row (readable, tappable).
- [ ] USB keyboard on Russian: keys type Cyrillic; AltGr+Q = q, Shift+AltGr+3 = #, Ctrl+Z (Я key) undoes.
- [ ] Ukrainian: the key above Enter types ґ; Greek: ; then ε types έ (tonos), Shift+; then ι types ϊ.
- [ ] Key labels in Cyrillic/Greek are readable (serif fallback font) in portrait and landscape.

## UI languages (v1.6.2, S16.2)
- [x] With `CurrentLocale=pt-PT` the app starts in Portuguese; Settings → Language shows the system language.
- [x] Override system language → Interface language lists 10 languages; pick Deutsch, Español, Italiano, Nederlands, Català, Dansk, Svenska in turn: titles and rows change at once and nothing is clipped (German and Dutch are the longest).
- [x] ä ö ü ß å æ ø ñ à ç are drawn correctly on the panel in the UI font; About says the translation is machine-drafted.
- [c] Landscape: the language list and Settings fit in every language.
- [c] Switch the Kobo's own language to German, Spanish, Italian, Dutch, Catalan, Danish or Swedish and relaunch: the app follows (override off).

## Stray lines and Size slider (main after 1.6.2)
- [x] Write → Letters / Train: write many small ticks and apostrophes; no long lines from the box's top corners appear.
- [x] Afterwards read `.adds/sigma-scribe/sigma-scribe.log` over USB (ea32b50 ignores corner reports in the decoder: look for `corner report (…) ignored` lines and check that no stray lines remain): any `touch:` lines (second contact, slot or tracking-id change, jump, dropped move) show where stray reports came from; copy them into the install plan. — 2026-10-24: 100 `corner report (1439,0) ignored` lines (more not logged, cap), 2 jumps got through ((542,995)→(451,692) and (1019,1912)→(1066,832), neither corner to corner); no tracking-id spam; no stray lines reported by the user.
- [x] Settings → Writing: Size is a slider with nine dots and no number; tapping and dragging change the note's text size at once; the default is the third dot.
- [x] The Writing line on the Settings page shows only the typeface.

## Vietnamese and Korean typing (v1.7.0, v1.7.1)
> **Deferred:** waits for feedback from native users of this language; not a release gate (user, 2026-10-25). The items stay open until someone who writes it has tried them.
- [ ] Layout → Vietnamese (Telex): `vieetj` → việt, `nguwowif` → người on the Full board and a USB keyboard; `aaa` → aa; a tone key after a moved caret starts a new word.
- [ ] Layout → Vietnamese (VNI): `Vie6t5` → Việt; digits still type digits after a space.
- [ ] Vietnamese letters and tones crisp at 100–200 % (Literata, no box glyphs).
- [ ] Korean needs the font pack first (docs/font-packs.md): `fonts/korean.ttf` in `.adds/sigma-scribe/`.
- [ ] Layout → Korean: jamo make syllables as you type, Backspace takes back one jamo; Shift gives ㅃ ㅉ ㄸ ㄲ ㅆ ㅒ ㅖ; AltGr types Latin and Markdown marks.
- [ ] Hangul legible at the body size; line wrap and the caret sit right on Hangul text; refresh while a syllable changes (no ghosting).
- [ ] Without the pack, Hangul draws as boxes (known; the layout is not hidden yet).

## Arabic, Persian, Urdu, Hebrew (v1.8.0)
> **Deferred:** waits for feedback from native users of this language; not a release gate (user, 2026-10-25). The items stay open until someone who writes it has tried them.
- [ ] Layout → Arabic: type a few words on the Full board and a USB keyboard; letters join, marks sit right, the row sits flush right.
- [ ] Mixed with Latin and numbers: order looks right; the caret sits where the next letter appears.
- [ ] Left / Right move the way the text reads in a right-to-left row; tap places the caret at the letter touched; selection highlight covers the right letters.
- [ ] Persian (گ چ پ ژ, ZWNJ key), Urdu (ٹ ڈ ڑ ں ے ھ), Hebrew: letters present and legible at 100–200 %.
- [ ] Arabic size next to Latin (Naskh is drawn 18 % larger): too big or too small?
- [ ] Wrapping a long Arabic paragraph; scrolling; page turn; no slowdown while typing.
- [ ] Preview of an Arabic paragraph: flush right, legible.
- [ ] Refresh behaviour while typing right to left (no ghosting at the caret).

## Indic scripts and Thai (v1.9.0)
> **Deferred:** waits for feedback from native users of this language; not a release gate (user, 2026-10-25). The items stay open until someone who writes it has tried them.
- [ ] Layout → Hindi, Marathi, Nepali (InScript): type words on the Full board and a USB keyboard; conjuncts (क्ष, न्द), the short ि sign before its letter, shirorekha joined.
- [ ] Caret and Left / Right move over a whole letter with its signs; Backspace removes one sign at a time; a tap lands between letters, never inside one.
- [ ] Bengali, Tamil, Thai: letters and signs present and legible at 100–200 %; Thai tone marks above, not colliding.
- [ ] Key caps: vowel signs on a dotted circle, legible; the Compact key does nothing on these layouts.
- [ ] Sizes next to Latin (Devanagari, Bengali 120 %, Tamil 105 %, Thai 115 %): too big or small?
- [ ] Line height with tall scripts (Devanagari headline and signs below): no clipping between rows.
- [ ] Wrapping long Hindi and Thai paragraphs; scrolling; no slowdown while typing.
- [ ] Preview of a Hindi and a Thai paragraph.

## Japanese (v1.10.0)
> **Deferred:** waits for feedback from native users of this language; not a release gate (user, 2026-10-25). The items stay open until someone who writes it has tried them.
- [ ] The CJK font pack is on the Kobo (`fonts/cjk.ttf`); kana and kanji draw, not boxes.
- [ ] Layout → Japanese (romaji): `watashihagakuseidesu` → わたしはがくせいです underlined; Space → 私は学生です; Space again cycles; Enter commits (no newline); Esc steps back.
- [ ] The candidate bar: chips legible, a tap picks and commits, the text above is not hidden under it; refresh while typing and converting (no ghosting on the underline).
- [ ] The あ / A key switches to direct input and back; Backspace takes back a kana; digits end the composition.
- [ ] Korean: Backspace on a USB keyboard removes one letter at a time; the syllable being composed is underlined.
- [ ] Phrases (v1.10.1): `watashihagakuseidesu` + Space → two phrases; Left / Right change the heavier underline; Shift + Right moves the boundary; a tap on a candidate moves on to the next phrase.
- [ ] Han / kana size and line height next to Latin; wrapping a long Japanese paragraph (rows break at any kana).

## Chinese (v1.11.0)
> **Deferred:** waits for feedback from native users of this language; not a release gate (user, 2026-10-25). The items stay open until someone who writes it has tried them.
- [ ] The CJK font pack is on the Kobo (`fonts/cjk.ttf`, rebuilt for Chinese) and the dictionaries `dict/pinyin-hans.txt`, `pinyin-hant.txt` are there.
- [ ] Layout → Chinese Simplified (pinyin): `woaizhongguo` → 我爱中国 on the bar; Space writes it; `nihao` then 1–9 picks; Left / Right move the choice; a candidate that covers the start leaves the rest.
- [ ] Time from the first letter to the candidates (the dictionary loads once, about 3 MB); typing speed afterwards; no ghosting on the candidate bar.
- [ ] Punctuation in full width; `3.14` keeps its point; Enter keeps the pinyin; Esc removes it; 中 / A key.
- [ ] Chinese Traditional (pinyin): `woaizhongguo` → 我愛中國; `tushuguan` → 圖書館.

## Scroll bar, Experimental features and Backlight (Unreleased after 1.11.1)
- [x] Editor and Settings show the Kobo-style bar: hairline track in the right margin, hollow rounded thumb; it follows scrolling and Settings sub-pages; no grey, no ghost when it moves (Paper and Night) — device pass 2026-10-24/25 (user: everything passed).
- [x] Settings → Experimental features is off after a fresh install; the Layout list then has no Japanese, Chinese, Korean, Arabic, Persian, Urdu, Hebrew, Indic or Thai entries, and Handwriting has no Fast ink or Word mode rows. Turning it on shows them; turning it off while Japanese is the layout puts US back — device pass 2026-10-24/25 (user: everything passed).
- [ ] An older settings file that used Japanese (or Word mode, or Fast ink) comes up with Experimental features already on.
- [x] Display → Backlight appears on the Forma and moves the front light in nine steps from off to full; the log has `light: sysfs …` lines for every device and `light: using <name>`. If the light does not respond or the wrong light moves (the warm one), copy the `light:` lines and set `SIGMA_LIGHT_DEVICE=<name>` in `launch.sh` — device pass 2026-10-24/25 (user: everything passed).
- [ ] The light keeps its level when the app exits to the stock reader.

## Kobo style keyboard (Unreleased after 1.11.1)
- [x] Settings → Keyboard & input → Enhanced keyboard off: the keyboard becomes the Kobo-style board (number row, three letter rows, bottom row with `#&+`, `ÉÀÇ`, a bracket Space, ← →, a dark Return); every key is easy to hit with a finger at 200 % and the keys read clearly — device pass 2026-10-24/25 (user: everything passed).
- [x] Shift is one-shot (the arrow key inverts); letters, digits, `@` and `.` type as shown; Back deletes; ← → move the caret; Return starts a new line — device pass 2026-10-24/25 (user: everything passed).
- [x] `#&+` shows the symbols layer (`£ € ¡ ¿ « » ¢ ¥ • °` present), `ÉÀÇ` the accents layer; Shift there switches capitals and lower case; the two bottom-left keys always name the other two layers — device pass 2026-10-24/25 (user: everything passed).
- [x] Landscape: the board is centred at portrait key size and there is no Split / Full / Centre bar; nothing else is on the board — device pass 2026-10-24/25 (user: everything passed).
- [x] Enhanced keyboard on: Compact and Full as before and no `#&+` or `ÉÀÇ` anywhere on them — device pass 2026-10-24/25 (user: everything passed).
- [x] A layout the Kobo board has no room for (Japanese, Chinese, Indic, Thai) keeps the Full board with the setting off — device pass 2026-10-24/25 (user: everything passed).

## Write panel keys, Insert and Undo (Unreleased after 1.11.1)
- [x] Write: the row above the writing area shows Home, End, PgUp, PgDn, Ins and Undo; each is easy to hit with a finger at 200 %; the writing area is still comfortable (portrait and landscape) — device pass 2026-10-24/25 (user: everything passed).
- [x] Home / End move the caret along the line; PgUp / PgDn move by a screen; a reading still pending is typed first; the candidate bar clears — device pass 2026-10-24/25 (user: everything passed).
- [x] Ins: the status bar shows `OVR`; typing with the panel keys, the keyboard and a USB keyboard replaces the next character and never eats a line break; Ins again goes back; Korean and Japanese layouts still insert — device pass 2026-10-24/25 (user: everything passed).
- [x] USB keyboard: the Insert key toggles it — device pass 2026-10-24/25 (user: everything passed).
- [x] Undo after a reading is typed takes the text back (not only unread ink); with ink pending it removes the last stroke first — device pass 2026-10-24/25 (user: everything passed).

## Write, Sketch, switches, sliders, Help and splash (v1.13.1 builds, 2026-10-24/25)
- [x] The app is **Σscribe** (NickelMenu entry, About) and the author **SIGMArecon**; the NickelMenu tab reads **My Apps** with the grid icon after a reboot — user, 2026-10-24/25 ("Name OK", "All looks great").
- [x] Splash: the name, the author and "Loading…" over a flat bar appear right after the start, before the editor — user pass; everything reported done on 2026-10-25.
- [x] Write panel: Undo, Redo, Erase, Return on one row and Home, End, Ins, PgUp, PgDn on the next, all icons except Ins; Redo brings back text that Undo took. Letter boxes: Write/Sketch left, Train and Done right, the keys centred, candidates on top. Sketch has the Line button and the footer note — user pass; everything reported done on 2026-10-25.
- [x] The title bar works while a writing panel is open (Files, Settings, Back, the drawer icons close the panel and act) — user pass; everything reported done on 2026-10-25.
- [x] Enhanced keyboard: rounded keys with icons for Backspace, Shift, Return, Space, Tab, Home, End, PgUp, PgDn; the Compact key reads "Comp."; Full in landscape has the `+ − * /` column and a Return key on the number pad — user pass; everything reported done on 2026-10-25.
- [x] One scroll bar, the same size in the editor, Preview, Help and Settings; the thumb can be dragged and a tap above or below it turns a screen or a page — user pass; everything reported done on 2026-10-25.
- [x] Settings: toggle switches for every on/off row, radio buttons on the layout and language lists with clear space before the scroll bar, Dark mode switch, sliders with the value over the thumb (Size, Leading, Margin, UI scale, Keyboard size, Backlight), notes under greyed rows — user pass; everything reported done on 2026-10-25.
- [x] UI scale changes only when the finger lifts — user pass; everything reported done on 2026-10-25.
- [x] The title bar has the Settings gear and no Help icon; Help opens from Settings → Help & about and looks like Settings (back arrow in the bar, topic as title, "Page i of n") — user pass; everything reported done on 2026-10-25.
- [x] Backlight slider: the log shows one `light:` line per step, 0 → 8 → 0 on `mxc_msp430.0` (no repeats) — device log, 2026-10-25 02:00 session, exit 0, no errors.
- [x] Landscape with Line length on: the desk beside the page has a fine grain; the page margins elsewhere are plain paper — user pass; everything reported done on 2026-10-25.

## Release gates (S9)
- [ ] Install from the release tarball; update keeps notes; uninstall clean (S9 checklist). Still open; the local-build installs above were updates over the previous binary.
- Not a gate: the special-language sections wait for native users' feedback (see the top of this file).

New versions append their checks above this line, under their own heading. The assistant fills this file in as builds are installed and tried (user, 2026-10-25); items are ticked only on the user's word or the device log, with the source named.
