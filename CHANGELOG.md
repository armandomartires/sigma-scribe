# Changelog

All notable changes to Sigma Scribe. Versions are tagged after they pass on the Kobo Forma.

## Unreleased
- **Fix: stray lines when writing letters.** A touch report outside the letter box (seen on the Forma as lines from the box's top corners to the pen point) was clamped to the box edge and drawn; it is now dropped. Cause of the stray reports not yet known: the app now logs odd touch input (second contact, slot or tracking-id change, jumps over 300 px, dropped moves) to `sigma-scribe.log`, capped at 400 lines per run.
- **Settings → Writing → Size is now a slider** (nine stops, no number, no unit, so it is not mistaken for points or pixels). The default is the third stop; values set in an older settings file stay and show at the nearest stop. The Writing line on the Settings page shows only the typeface. Leading and Margin are still steppers. Emulator: `tests/emu/v1.6.3-size-slider.script` (screenshots `docs/screenshots/v1.6.3/`); `v0.4-options.script` now taps `Size/4`, `Size/5` (unlabelled stops are `Size/1`…`Size/9`).
- **Settings → Display → Orientation is three buttons in the row** (Auto, Portrait, Landscape), like Paper and Drawer, instead of a row that opened a list. Auto no longer shows "(no sensor) — Portrait" in Settings (`orientation_status` is still tested). Emulator: `tests/emu/v1.6.3-orientation-buttons.script`; the eight older scripts that opened the list now tap `Orientation/Landscape` or `Orientation/Portrait` directly.
- **Cause of the stray lines found in the device log** (2026-10-23): the Forma's touch panel sends phantom reports at the four exact screen corners; every one of 77 logged jumps was corner to corner. The decoder now ignores any report on a corner pixel (never starts a touch from it, never lets it move the position, and does not let the last real position be overwritten by it). Letter boxes keep dropping reports outside the box as a second guard. Tests: `corner_reports_never_start_a_touch`, `corner_reports_inside_a_stroke_are_ignored_and_do_not_pollute_it` (`crates/device/src/evdev.rs`).
- Touch log: the panel repeats `tracking id 0` on every frame (323 of the first 400 log lines), so that line is now written only when the id changes; ignored corner reports have their own cap (100 lines per run).
- Test: `stray_reports_outside_the_box_do_not_draw_lines_to_its_corners` (`app/tests/letters.rs`).

## 1.9.0 — 2026-10-23 (built, emulator only, not installed, not tagged)
- **Hindi, Marathi, Nepali, Bengali, Tamil and Thai typing** (sprint S19.0, v1.9 wave W3b): four new layouts, "Hindi, Marathi, Nepali (InScript)" (one Devanagari layout serves all three), "Bengali (InScript)", "Tamil (InScript)" and "Thai (Kedmanee)", on the on-screen Full board and USB keyboards. InScript puts vowel signs on the base layer and independent vowels on Shift, and the same key positions serve every Indic script. Digits stay ASCII; AltGr is the US layer. These layouts have too many keys for the Compact board, so they use the Full board only. Vowel signs on key caps sit on a dotted circle.
- **Shaping:** Devanagari to Sinhala, Thai, Lao, Tibetan, Myanmar and Khmer text now takes the shaping path of v1.8 (rustybuzz): conjuncts (क्ष), vowel-sign reordering (हिन्दी), reph, Bengali and Tamil signs, Thai marks. The caret, taps and selection follow the glyph clusters. Joiners (ZWJ, ZWNJ) stay with the letters around them.
- **Clusters:** Left / Right and Delete move over a whole letter with its signs (क्षि, ก็, é); **Backspace takes one code point** (the sign, then the letter), as the plan's decision 10 says. Wrapping never ends a row inside a cluster, and for Thai (no spaces between words) never after a leading vowel (เ แ โ ไ ใ) or before ะ า ำ ๆ ฯ.
- Fonts bundled: Noto Sans Devanagari (229 KB), Bengali (202 KB), Tamil (73 KB), Thai (38 KB), SIL OFL. Devanagari and Bengali are drawn 20 % larger, Tamil 5 %, Thai 15 %, to sit with Literata.
- Not yet: Thai word breaking by dictionary (rows break between syllables, not at word boundaries); Tamil99 and phonetic layouts; Devanagari digits; Gurmukhi, Gujarati, Telugu, Kannada, Malayalam (fonts and InScript would follow the same pattern); interface catalogs for these languages (native review).
- Emulator: `tests/emu/v1.9.0-indic-thai.script`, screenshots `docs/screenshots/v1.9.0/`. Tests: cluster wrapping and Thai break rules (`wrap.rs`), caret by cluster and Backspace by code point (`buffer.rs`), conjunct width and caret stops, Thai wrapping (`shape.rs`), layout tables, glyph audit over all six new fonts. Interface catalogs gain the four layout names (machine-drafted).

## 1.8.0 — 2026-10-23 (built, emulator only, not installed, not tagged)
- **Arabic, Persian, Urdu and Hebrew** (sprint S18.0, v1.8 wave W3a): four new layouts (Arabic 101, Persian standard, Urdu phonetic, Hebrew standard) on the on-screen boards and USB keyboards; AltGr is the US layer, as for the other non-Latin layouts. Digits stay ASCII so Markdown lists work.
- **Right-to-left text in the editor:** a row with Arabic-script or Hebrew letters is split into bidi runs and shaped (letters join, marks sit on their letters, ligatures such as lam-alef form), drawn flush right when its first strong letter is right-to-left. Latin words and numbers inside keep their own order. The caret, taps and selections follow the glyphs, and **Left / Right swap in a right-to-left row** (on-screen and USB). Preview draws a right-to-left line flush right too.
- Fonts: Noto Naskh Arabic (176 KB) and Noto Sans Hebrew (27 KB) are bundled (SIL OFL), small enough not to need font packs. Naskh is drawn 18 % larger to sit with Literata. Libraries: rustybuzz and unicode-bidi (binary about +1 MB).
- Wrapping now finds the longest row that fits by binary search instead of growing it letter by letter (shaped text costs more to measure).
- Not yet: interface mirroring and Arabic/Hebrew/Persian/Urdu interface catalogs (need native review); Nastaliq for Urdu (Naskh is used); a row's direction comes from its own first strong letter, not the whole paragraph's; selection across a bidi boundary is one rectangle; Backspace and the arrows work by character, not by letter plus mark.
- Emulator: `tests/emu/v1.8.0-rtl.script`, screenshots `docs/screenshots/v1.8.0/`. Tests: `shape.rs` (direction, joining, caret and tap round trip, mixed order, wrapping), layout tables, glyph audit now covers Naskh and Hebrew. Interface catalogs gain the four layout names (machine-drafted).

## 1.7.1 — 2026-10-23 (built, emulator only, not installed, not tagged)
- **Korean typing** (sprint S17.1, v1.7 wave W2, second part): Dubeolsik on the on-screen boards (Full and Compact; Shift makes ㅃ ㅉ ㄸ ㄲ ㅆ ㅒ ㅖ) and on USB keyboards. Jamo compose into syllables as you type (compound vowels, compound finals, a final moving to the next syllable on a vowel); **Backspace takes back one jamo** (한 → 하 → ㅎ). AltGr is the US layer, as for Russian, Ukrainian and Greek, so Latin text and Markdown need no switching.
- **Font pack:** Hangul is not in Literata or Atkinson. The app now loads an optional script font pack (`SIGMA_FONT_PACK`, else `.adds/sigma-scribe/fonts/korean.ttf`, else the system Noto Sans CJK on a development machine) and uses it for any character the bundled faces lack. No pack on the Kobo means Hangul draws as empty boxes: the pack must be built and installed before the Korean layout is usable on the device (`docs/font-packs.md`).
- Not yet: the composing syllable is not underlined, the pack is not offered or hidden by the layout list, and there is no 한/A switch (AltGr does that job).
- Emulator: `tests/emu/v1.7.1-korean.script`, screenshots `docs/screenshots/v1.7.1/`. Tests: `hangul.rs` (composition, spelling round trip), the glyph audit skips jamo.

## 1.7.0 — 2026-10-23 (built, emulator only, not installed, not tagged)
- **Vietnamese typing, Telex and VNI** (sprint S17.0, v1.7 wave W2, first part): two new layouts on the US keys, "Vietnamese (Telex)" and "Vietnamese (VNI)", for the on-screen boards and USB. The word being typed is re-spelled as keys arrive, so a mark or tone can follow the letter it belongs to (`vieetj` → việt, `nguwowif` → người, VNI `Vie6t5`), and a repeated key takes it back (`aaa` → aa). Tones follow the modern rule (hòa, thủy, quả; gi and qu handled).
- Moving the caret or typing a space or punctuation starts a new word. No spell check: typing English words under Telex can turn `s f r x j` into tones; switch to the US layout for that.
- No new font: Literata and Atkinson already carry the Vietnamese letters.
- Emulator: `tests/emu/v1.7.0-vietnamese.script`, screenshots `docs/screenshots/v1.7.0/`. Interface catalogs gain the three layout names in all ten languages (machine-drafted).

## 1.6.2 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **UI languages: German, Spanish, Italian, Dutch, Catalan, Danish, Swedish** (sprint S16.2, decision 60; v1.6 wave W1, third part): the interface follows the Kobo `CurrentLocale` for `de es it nl ca da sv`, or Settings → Language → Interface language; 10 languages in all with English, French and Portuguese. Fi, nb, tr, pl, cs, ro move to v1.6.3.
- Catalogs are machine-drafted, best effort (About says so); `assets/i18n/*.txt`, each checked against French for identical keys. German "Move" shortened to fit.
- Internal: language table (`REAL`) drives locale lookup, catalogs and the picker; unused statics removed.
- Emulator: `tests/emu/v1.6.2-ui-languages.script`. Screenshots `docs/screenshots/v1.6.2/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build, PR #36 release run 39831043595) `46787595ce50d17c9b9b7b0218032f59368244c49ba1193eaca45cfab8f54011`; tarball `dbbfeece6eb5f83d0e2f10a51382e4758ba93f64c43743f9ec4619f8f4aa42f6`; binary 4,518,432 bytes (1.6.1: 4,417,072; **+101,360 bytes**, the seven catalogs).

## 1.6.1 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Russian, Ukrainian and Greek keyboard layouts** (sprint S16.1, decision 59; v1.6 wave W1, second part): ЙЦУКЕН (xkb ru), Ukrainian (xkb ua, ґ on the key above Enter, ₴ on Shift+\`) and Greek (xkb gr, tonos ´ and dialytika ¨ dead keys: ά έ ή ί ό ύ ώ, ϊ ϋ); 21 layouts in all.
- On these three, **AltGr is the US layout** (Shift+AltGr its Shift layer): Latin letters, `#`, `[[`, `]]` and other Markdown marks without switching layouts; Ctrl shortcuts keep the US letter (Ctrl+Я = Ctrl+Z).
- Compact board: Cyrillic rows have 12 / 11 / 10 letters; the keys narrow to fit. Greek compact has no tonos key (Full board and USB do).
- Fonts: Literata already covers Cyrillic and Greek; Atkinson (key labels, UI) does not, so those labels fall back to Literata per character (decision 47). No new font, no size cost.
- Settings key `layout=` gains `ru`, `ua`, `gr`; older binaries fall back to US.
- Emulator: `tests/emu/v1.6.1-cyrillic-greek.script`. Screenshots `docs/screenshots/v1.6.1/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `7e9d8cbf4a299bab91a81e0abae6735392fca9307098db489caf2b858635d59e`; tarball `02f12be1b47731577ddfd0ea2b768c1eb6fc28e333c093253946a359fbf3a5a4`; binary 4,417,072 bytes (1.6.0: 4,414,072; **+3,000 bytes**).

## 1.6.0 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Eleven more keyboard layouts** (sprint S16.0, decision 58; v1.6 wave W1, first part): US International, Belgian AZERTY, Danish, Norwegian, Swedish / Finnish, Polish (programmer), Czech QWERTZ, Romanian, Turkish Q, Spanish (Latin America) and Portuguese (Brazil), for the on-screen board and USB keyboards; 18 layouts in all (Settings → Keyboard & input → Layout, two pages).
- **Shift + AltGr** types the capital of an AltGr letter (Polish Ą Ę Ł Ó Ś Ź Ż Ć Ń).
- New dead keys **caron** (ˇ: č ď ě ň ř š ť ž) and **ring** (°: å ů); the acute now also composes ń ś ź ĺ ŕ.
- Settings key `layout=` gains `us-intl`, `be`, `dk`, `no`, `se`, `pl`, `cz`, `ro`, `tr`, `latam`, `br`; older binaries fall back to US for these.
- Emulator: `tests/emu/v1.6.0-latin-layouts.script`. Screenshots `docs/screenshots/v1.6.0/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `f0241692a62f5c57ac295182a469862bd878514867d4bdb1c8a6d258f14133f9`; tarball `f6ed0f851ab1aa75f9363732ac89b19698831574d27febd1daeed3786ae28266`; binary 4,414,072 bytes (1.5.5: 4,408,864; **+5,208 bytes**).

## 1.5.5 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Settings → Language → Override system language** (sprint S15.5, decision 57; user report: with the Kobo in Portuguese the language could not be changed). A checkbox, **off by default**: off, the app follows the Kobo's language (`CurrentLocale`, read-only, never written) and Interface language is greyed out showing it; on, Interface language opens a list (English, Français, Português) and the choice is saved. Turning it on starts from the language in use.
- New settings keys `ui_lang_override=` (default `off`) and `ui_lang=` (default `en`); older files load with the override off, older binaries ignore the keys.
- Emulator: `tests/emu/v1.5.5-lang-override.script`. Screenshots `docs/screenshots/v1.5.5/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `bcf02bdd55892e1cdf3416c687e7cdf30c8169573af2759cde62bb92320b8909`; tarball `ab5930a75a35af670e0b4304bf1d7c012d0b9f048bbd953e7721ef4245e83bfa`; binary 4,408,864 bytes (1.5.4: 4,406,432; **+2,432 bytes**).

## 1.5.4 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Gestures in the Write panel** (sprint S15.4, decision 56):
  - **Scratch out** ink you have not finished to erase it; scratch out on an empty area to delete the word just typed (or the word before the caret); in Draw a scratch-out erases the strokes it crosses.
  - A long flat **flick left** is Backspace, and at the start of a line it **joins** it to the line above; a **flick right** types a space.
  - **Tap the note** above the panel to move the caret there; the panel stays open.
- Emulator: `tests/emu/v1.5.4-gestures.script`. Screenshots `docs/screenshots/v1.5.4/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `2d609ca1c95e81feea8a86c215ad6b21c10fea51c594de4e13edfc674b2db420`; tarball `184f5cdfc1139a2532879d5a8dad3ea2e62ea7b8393c86f5e8b475709c8f9da2`; binary 4,406,432 bytes (1.5.3: 4,402,160; **+4,272 bytes, +0.1 %**).

## 1.5.3 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **New Write panel** (sprint S15.3, decision 55), in the style of Boox's handwriting keyboard: a candidate bar across the top (‹ › for more, × to close), a 3 × 4 block of keys on the left (⇄ Draw, ⌫, ↵, `,` `.` `?`, 123, `!`, `@`, globe, space) and a **large blank area** to write in freely. Pure black and white, every key ≥ 44 px, portrait and landscape.
- **Free writing:** write letters or words anywhere in the area; 1.2 s after you stop, the ink is cut into characters automatically and typed at the caret, followed by a space. Tap a word in the candidate bar to replace the last reading. **123** reads digits only; the **globe** limits letters to English, French or Portuguese.
- **Word mode (experimental), off by default** (decision 54): reads every word against a word list (built-in EN/FR/PT plus the words in your notes). It missed its accuracy gate (24–28 % character errors on held-out fonts against ≤ 15 %), so it is a setting; free writing already uses the word list for letters that touch.
- **Letter boxes** (the 1.5.2 boxes) are now a setting, off by default. Train still uses boxes.
- Settings → Keyboard & input → Handwriting: Ink pen, Fast ink (A2), Letter boxes, Word mode (experimental), My handwriting (Samples, Train, Delete). New keys `ink_boxes=`, `ink_words=`.
- CI prints the handwriting figures (`--nocapture`, step "Handwriting figures").
- Emulator: `tests/emu/v1.5.3-word-mode.script`. Screenshots `docs/screenshots/v1.5.3/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `448c830c2f98f87f64ecb77860247c731ca1cc3e5b5b617a6d07d9f30e26f169`; tarball `1f71d3b4d4799bc83599f3991f596f8a4f45dd7b309154944b8b1b03809214b2`; binary 4,402,160 bytes (1.5.2: 4,217,704; **+184,456 bytes, +4.4 %**: word lists, the new icons, the free reader and word decoder).

## 1.5.2 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Letters** (sprint S15.2, decision 53): in the Write panel, tap **Letters** and write one character per box; it is typed at the caret. Five candidates appear after each letter; tap one to replace it. Space, Erase, Return, Draw (back to free ink), Done. Works offline with the app's own small model (105 kB), English, French and Portuguese letters with accents, digits and punctuation (126 characters).
- **Train** (opt-in): write samples of your own hand, one character at a time; they are saved only in `notes/.handwriting/samples.txt` and the model adapts to them on the device. Settings → Keyboard & input → Handwriting → **My handwriting** shows the count and **Delete** removes them.
- The default model is trained only on synthetic ink drawn from OFL fonts with random jitter (`tools/hwr-train`, reproducible). Held-out synthetic results: 97.4 % top-1 on new jitter of the training fonts; 82.5 % top-1 and 98.4 % top-5 on two fonts it never saw. Est. 1–3 ms per character on the Forma.
- **Wiki links follow rename and move** (sprint S15.2a, decision 52, PR #28): renaming or moving a note rewrites `[[links]]` to it in every other note and the scratch pad, keeping aliases (`[[note|label]]`), headings (`[[note#part]]`), path or bare-name form and the extension; code is left alone.
- The Write panel's Draw view gains the Letters button (left of Undo).
- Emulator: `tests/emu/v1.5.2-letters.script`, `tests/emu/v1.5.2-relink.script`. Screenshots `docs/screenshots/v1.5.2/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `8fe47dda4f9caec2c04420c5f34f7ec5944f9443ec4c5266ecf6891cd3e98040`; tarball `b215d8394923232d3176390b860ac4dc2b5cfabb3aab42518065729f5c2d1423`; binary 4,217,704 bytes (1.5.1: 4,053,608; **+164,096 bytes, +4.0 %**: the model is 104,876 bytes, the rest recogniser and panel code).

## 1.5.1 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Ink in preview** (sprint S15.1, decision 51): inserted ink blocks are drawn in preview instead of `[image: ink]`, rasterised with resvg to **1-bit** (pure black on Paper, pure white on Night), so they never leave grey for DU. Only `.ink/` images are read; a missing file keeps the placeholder. Insert now puts the image in its own paragraph.
- **Rename and move keep ink:** the note's `.ink/<name>/` folder moves with it and the note's links are rewritten (also in the open note).
- **Contact size** in the strokes file when the touch panel reports `ABS_MT_TOUCH_MAJOR` (format version 2); without it the file says `"contact":null` and points stay `[x, y, t]`.
- Roadmap: character boxes move to 1.5.2.
- Emulator: `tests/emu/v1.5.1-ink-preview-rename.script`. Screenshots `docs/screenshots/v1.5.1/`. Device checks: `docs/device-checklist-prerelease.md`.
- Device binary sha256 (CI build) `4f55e94be77d5fd6af7e8fbe471789cf4d89ae56ab930635759ac90e13c323fd`; tarball `c4dd63aeb4bc1e4954f8b44df35f3686cb04c39f1c995091610f80a922608093`; binary 4.05 MB (1.5.0: 3.15 MB, +0.9 MB for resvg).

## 1.5.0 — 2026-10-22 (built, emulator/CI only, not installed, not tagged)
- **Write panel** (sprint S15.0, decisions 49–50, plan §12): a **Write** button in the status bar opens a pure white panel with **pure black ink** for a finger or a capacitive stylus. Live strokes go out as small **DU** updates that are never waited for; 0.6 s after a stroke one **GL16** pass cleans the inked area.
- **Undo** (last stroke, or a Clear), **Clear**, **Cancel**, **Insert**. Insert saves the drawing next to the note as `.ink/<note>/ink-NNN.svg` plus `ink-NNN.strokes.json` (raw points for later recognition) and adds `![ink](.ink/<note>/ink-NNN.svg)` at the caret.
- Settings → Keyboard & input → **Handwriting**: Ink pen Finger (1.2 mm) / Stylus (0.6 mm); **Fast ink (A2)**, an experimental 1-bit waveform, **off by default**. New keys `ink_pen=`, `ink_a2=`; older settings load unchanged.
- Not yet: ink drawn in preview (shows `[image: ink]`), `.ink/` following a renamed note, recognition (v1.5.1).
- Emulator: `tests/emu/v1.5.0-ink-panel.script`. Screenshots `docs/screenshots/v1.5.0/`. Device checks: `docs/device-checklist-prerelease.md` (decision 48).
- Device binary sha256 (CI build) `b4c9957410bdb7fe093aaa95dcf2def73167f57f720e69fe9954c6def78108fa`; tarball `f23eeccf6024e369772d97b57c76a204511b71deb883177edf15532bb06cbaae`.

## 1.4.6 — 2026-10-22 (built, not installed, not tagged)
- **Fix: empty box before "Up to Files"** (sprint S14.6, decision 47). Atkinson Hyperlegible has no "←" (U+2190), so the Files crumb drew a box. The crumb, and Help's "Back to Help", now draw the Lucide **arrow-left** icon (resvg-baked, like the title-bar icons) instead of a text glyph.
- **Glyph audit:** every UI string in EN, FR, PT and the pseudo-long test locale, plus every non-ASCII character in the UI code's string literals, was checked against the five bundled faces. Also missing from Atkinson: → ↑ ↓ (Graph list "→ / ← links", USB hint "Help → …", keyboard labels). Text drawing now falls back per character to the first bundled face that has the glyph (Literata has the arrows), so nothing draws as a box. New test `app/tests/glyphs.rs` fails on any character no bundled face can draw.
- Emulator: `tests/emu/v1.4.6-arrow-glyph.script`. Screenshots `docs/screenshots/v1.4.6/` (before / after).
- Device binary sha256 (CI build) `e884f1e71ab1661184171b76270665c19b839ce115f9673e34592b2d8daedacd`; tarball `acf2551db89ecf20d02dd5361e3dab79c87d8ac5e0ab5937d3e9e095cd39e48a`.

## 1.4.5 — 2026-10-22 (built, not installed, not tagged)
- **Keyboard size slider look** (sprint S14.5, decision 46, user feedback on 1.4.4): the slider sits straight on the Settings page: no white (Night: black) panel and no focus frame.
- While **Auto is on** the whole slider (dotted track, hollow dots, hollow thumb, labels) is drawn in a static **grey**; it appears with the page's greyscale refresh (opening the page, toggling Auto). When **Auto is off** it is **solid ink** (black on Paper, white on Night), so moving it is DU-safe: only the thumb and the bold label change. Toggling Auto redraws the row in greyscale, so no grey or black is left behind (tested).
- **Focus** (after a tap, for Left / Right on a USB keyboard): a bigger thumb with a thicker ring instead of a frame.
- Settings key, behaviour and tap targets (≥ 44 px) unchanged.
- Emulator: `tests/emu/v1.4.5-slider-grey.script`. Screenshots `docs/screenshots/v1.4.5/`.
- Device binary sha256 (CI build) `9fcbbed743bbe3d64b886ce4c3206f483b77dd0bd5c91d5d02892a3aa46e3a3e`; tarball `7e77ede427f317bc1a9a3e9b6fcf5b038584c7c8b2eba123fb80a48debdf7744`.

## 1.4.4 — 2026-10-22 (built, not installed, not tagged)
- **Keyboard size control** (sprint S14.4, decision 45, user request): the 1.4.3 choice page is replaced, inline on Settings → Keyboard & input (section "Keyboard size"), by an **Auto keyboard size** checkbox (on by default) and a horizontal **slider** with three dots, 100% / 125% / 150%.
- Slider: a tap or a drag snaps to the nearest dot; once tapped it has focus, and Left / Right (USB keyboard) move it. While Auto is on the slider is disabled and drawn muted (dotted track, hollow dots and thumb) with the thumb on the size Auto gives. Turning Auto off keeps that size; moving the slider then sets it.
- Pure black and white (0x00 / 0xFF, text thresholded), so it is DU-safe, inverted on Night pages; every stop is a tap target of at least 44 px. Tested in EN / FR / PT / pseudo-long, portrait and landscape, at UI scales 100–200 % (`app/tests/keyboard_slider.rs`).
- The slider is a reusable widget (`crates/render/src/slider.rs`); other numeric settings are not converted yet.
- **Settings carry over:** `keyboard_size=` is unchanged (Auto = `auto`).
- Emulator: `tests/emu/v1.4.4-keyboard-slider.script` (new `press-on` / `move-on` commands aim at named controls); `v1.4.3-keyboard-size.script` updated. Screenshots `docs/screenshots/v1.4.4/`.
- Device binary sha256 (CI build) `085e9b1b6128379a8d0032e860edd2c4d6baa85f8cc66dc38b7e282593448548`; tarball `f024c73f07d64591eded70f544e10868e348f01a73679d6675ca463143163181`.

## 1.4.3 — 2026-10-22 (built, not installed, not tagged)
- **Keyboard size** (sprint S14.3, decision 44, user report: the on-screen keyboard is too big above 150 % UI scale; 125–150 % is fine). The keyboard has its own scale, separate from the UI scale. New setting under Settings → Keyboard & input: **Keyboard size** Auto / 100% / 125% / 150%, a choice page. **Auto (default)** follows the UI scale up to 150 %, so at the default 200 % UI the keyboard is the 150 % size while the rest of the interface stays at 200 %.
- The keys, gaps, key labels, arrows and the landscape Split / Full / Centre bar use the keyboard scale; key targets stay at least 44 px. Tested: every size fits in portrait and landscape at UI scales 100–200 % in EN, FR, PT and the pseudo-long locale (`app/tests/keyboard_size.rs`).
- **Settings carry over:** new key `keyboard_size=auto|100|125|150`; files without it (1.4.2 and older) load as Auto. 1.4.0–1.4.2 ignore the key, so a rollback still reads the file.
- Emulator: `tests/emu/v1.4.3-keyboard-size.script`. Screenshots `docs/screenshots/v1.4.3/` (keyboard at 200 % UI before and after).
- Device binary sha256 (CI build) `a3bef0bcd147c10c4b4e810773ad38ee93262dbd93056994f1502675b7f17b2c`; tarball `1dd829d9fa17014e04c6751934befcb8cdcda9cefab200204a251158974b2b88`.

## 1.4.2 — 2026-10-22 (built, not installed, not tagged)
- Settings redesign (sprint S14.2, decision 43; proposal PR #16, approved): the Aa icon opens a Kobo-style **Settings** page with six categories: Writing, Display, Keyboard & input, Language, USB & devices, Help & about. Each category shows a summary and opens its own page. Each page has a back arrow, a small centred "Settings", a large serif page title, grey section bands with small capitals, and serif rows (label on the left, value on the right) with thin separators. Long pages turn with the page buttons or by tapping "Page i of n".
- Controls: on/off settings are **checkboxes** ("☐ On"), two-way choices stay **inline buttons**, numbers keep − / + steppers, and longer lists (Orientation, Layout, Keyboard tone, Tab width) open a choice page that goes back when you pick. The back arrow or Esc goes up one level; X closes from any page. Help pages are under Help & about (the ? icon still opens Help).
- **Settings carry over:** the `.sigma-settings` keys are unchanged, so 1.4.0 / 1.4.1 settings load as they were (tested).
- **Scroll position bar:** a thin bar on the editor page's right edge (Kobo Settings style) shows where you are in the note and how much of it is on screen. It is pure black and white, so it is DU-safe; it inverts on Night pages and is only sent to the panel when the note scrolls. It is display-only and hidden when the note fits.
- Emulator: `tests/emu/v1.4.2-settings.script`; old scripts' `Row/Cell` names (`Paper/Night`, `Layout/+`, `Orientation/Landscape`) are reached through the pages automatically. Screenshots `docs/screenshots/v1.4.2/`.
- Device binary sha256 (CI build) `1e2ca1c93790ca2cec935ddaeaff4547055958068aaf04552608f615d5d41cb1`; tarball `4cf42621f9c414e531443ccd26f860f591c966cff8a4d1c691bc24c4458685a2`.

## 1.4.1 — 2026-10-22 (built, not installed, not tagged)
- Pointer always black (sprint S14.1, decision 42): the mouse arrow is now pure black (0x00) with a 2 px pure white (0xFF) outline in Paper and Night, with no grey pixels. 1.4.0 drew it in the theme's ink and paper: in Night that is a near-white arrow (233), and in Paper the greys 28 / 240 are values the 2-level DU waveform used for pointer moves does not define, so the panel could show the arrow white.
- Packaging: `THIRD-PARTY.txt` (Lucide ISC notice) is part of the next device update, next to the binary (`docs/install-plan-forma.md`).
- Emulator: `tests/emu/v1.4.1-pointer-black.script`; screenshots `docs/screenshots/v1.4.1/`.
- Device binary sha256 (CI build) `ffdd50b8fb0035fbd4d2443677344b44a351d413c5ba26d4dd1c7931289f3a9e`; tarball `334295fd02d9f4ad20f1cf88f22125b86c9685bb8b48ea068229528cd62ee8d9`.

## 1.4.0 — 2026-10-22 (built, not installed, not tagged) — first stable release
- Interface languages (sprint S14, decision 41): **English, Français, Português (PT-PT)**. The language follows the Kobo: at start the app reads `CurrentLocale` from `[ApplicationPreferences]` in `/mnt/onboard/.kobo/Kobo/Kobo eReader.conf` (read-only, never written); `fr*` → French, `pt*` → Portuguese, anything else, missing or unreadable → English. Help → App info shows the language.
- Every sheet, button, key label, status text, Help page and the Markdown reference are translated. The French and Portuguese catalogs (`assets/i18n/fr.txt`, `pt.txt`) are **machine-drafted, best effort, and need a native review**; English is the source and the fallback for anything missing. Note names, paths and note text are never translated.
- Long translated hints in Options are shortened with "…" instead of running under the buttons (all languages).
- Help → USB keyboard: the old "US layout" line now points to Options → Layout.
- Not yet: an Options language override, thin-space thousands in word counts, the right-to-left mirroring switch (with v1.6; a pseudo-RTL locale is in the tests).
- Emulator: `tests/emu/v1.4-languages.script` (steps `locale-conf`, `language`, `expect-language`); screenshots `docs/screenshots/v1.4/`.
- Device binary sha256 (CI build) `2f89c5fd559ffbb1c8bd070213992e38bb4e7e6b8bc5f97bbd10eabefc85dd1d`; tarball `6b63d580544c97df14039b1d3ef88f412d2c4faea74ea5c26e96c2eb5a7a8cf3`.

## 1.3.0 — 2026-10-22 (built, not installed, not tagged)
- Keyboard layouts (sprint S13, decision 40): Options → **Layout** US / UK / French AZERTY / German QWERTZ / Spanish / Portuguese / Italian, for the on-screen Full board and USB keyboards together (`layout=`). The Full board shows each layout's keys (with the extra key left of Z where it exists); Alt gives the AltGr characters (€, @, { } …). Compact gets AZERTY / QWERTZ letters. Dead keys ` ´ ^ ~ ¨ compose with the next letter (ê, ñ, ü …). On USB, right Alt is AltGr and Ctrl shortcuts keep their letter (Ctrl+Z is Z wherever it sits).
- Not yet: the long-press accent popup on Compact and the candidate bar (deferred, S13-D3); Nordic, Dutch and Brazilian layouts.
- Emulator: `tests/emu/v1.3-layouts.script` (`usb-type` follows the layout); screenshots `docs/screenshots/v1.3/`.
- Device binary sha256 (CI build) `51bfc10303d2fcd740fecc550a9a895116efb11209493c047404866be9f8451b`; tarball `6747e4f6c2c816d03f3e6fe64d15907a822b6fb56fe76562446f29bcdc061cb1`.

## 1.2.0 — 2026-10-22 (built, not installed, not tagged)
- Orientation (sprint S12, decision 39): Options → Orientation **Auto** (default) / Portrait / Landscape and **Upside down** Off/On — four rotations. Touch, page buttons (swapped upside down) and partial refreshes follow the rotation. Auto turns the screen after a reading holds 0.6 s, with one full refresh, never while a finger is down or within 1 s of typing; the rotation is remembered. Settings from 1.1 and earlier move to Auto unless landscape was chosen.
- The accelerometer is **off in this build** (safe default): the decoder is in place but waits for a device check, so Auto keeps the last rotation and Options says "Auto (no sensor) — Portrait".
- Emulator: `tests/emu/v1.2-orientation.script` (steps `sensor`, `tilt`, `expect-rotation`, `expect-orientation-status`); screenshots `docs/screenshots/v1.2/`.
- Device binary sha256 (CI build) `7618f3e599815f9221cde891e483fc76d5e7eae724c71bb6b149af5b3ca74fb2`; tarball `ee5c5d615814f3d82fd2d3122a9bf4b27c209322c760f41da9791cca46d21fdb`.

## 1.1.0 — 2026-10-22 (built, not installed, not tagged)
- UI scale (sprint S11, decision 38): Options → **UI scale** 100 / 125 / 150 / 175 / **200 %** (default 200 %, saved as `ui_scale=`). It scales the title bar, icons, sheets (Files, Options, Help, Graph), drawer, Find bar, status bar and keyboard together; the note text **Size**, margins, preview and selection handles keep the device scale. The keyboard takes at most 42 % of the height in portrait and 45 % in landscape. Touch targets stay ≥ 44 px at the device scale at every step.
- Icons are now **Lucide** (ISC, decision 35): the SVGs are baked with resvg at build time into greyscale masks, one per UI-scale step (stroke 2.25 for e-ink), and drawn by one blit; Options keeps its 'Aa'. Credit in the README, Help → App info and `THIRD-PARTY.txt`.
- Emulator: `tests/emu/v1.1-ui-scale.script`; screenshots `docs/screenshots/v1.1/`.
- Device binary sha256 (CI build) `917a0c11e9723256904a4bdbb00188d287b6873b9bafa39a56836ae31179213f`; tarball `94d8335156ccb47e92a67363f5349cc9bbe7894e982960ac6d1129a24c9b7750`.

## 1.0.4 — 2026-10-22 (built, not installed, not tagged)
- Pointer regression fixed (sprint S10.4): pointer moves no longer redraw the whole screen (1.0.3 log: ~170 ms CPU per move, events queued behind it); a pointer-only frame restores and redraws just the arrow. Idle hiding uses processing time, so a backlog no longer makes the arrow flicker on and off. The Graph selection pill is GL16 again (1.0.3 sent it as DU).
- Device binary sha256 (CI build) `681a6d1f800a996ecef3ce8beea33434269fb43316af6f95c57bc14ea1b46b2e`; tarball `7da86927b84fbf69e96580827c677cd967a0094e003482caabeefd5a49f9a86d`.

## 1.0.3 — 2026-10-22 (built, not installed, not tagged)
- Pointer lag fixed (sprint S10.3, decision S10.3-D2): the moving pointer now uses the fast DU waveform and is never waited for; one non-flashing GL16 cleanup over its path runs when it hides (3 s idle, typing, touching), clearing trails. The blocking `fbink_wait_for_complete` every 20th move (1.0.1/1.0.2) is gone, so pointer updates can no longer hold up typing. Regions stay ≥ 16×16. Device log: `perf pointer: du …` every 20th move, `perf pointer: gl16-cleanup …` per hide, and a pointer p50/p95 summary at exit.
- Drawer in preview (decision S10.3-D1, amends S5-D1): Contents, To-dos and Scratch stay open in preview; the page is laid out beside the drawer. Contents jumps to the preview page of a heading; a to-do checkbox toggles (atomic write + reindex) and re-renders the preview — the one edit allowed from preview; the Scratch pad stays editable while focused, the note stays read-only. Not shown over Files, Options, Graph, Help or rename.
- Emulator: `tests/emu/v1.0.3-drawer-preview.script`; screenshots `docs/screenshots/v1.0.3/`.
- Device binary sha256 (CI build) `339b62613cf13b51c7e140ac9c85887b5780677028b759799aa6d04178b49be0`; tarball `b6d4dd52bd56aa07ebfcb005f7c651b111a5ad3e1f261f72ae6f209f7d07a6d4`.

## 1.0.2 — 2026-10-21 (built, not installed, not tagged)
- Graph map look (sprint S10.2): links are plain lines (no arrowheads); notes are small circles (rounded rectangles, radius = height/2); the selected note grows into a pill with its full name, in one non-flashing GL16 region; the current note stays filled; the selection's neighbours keep a larger, thicker ring and ink links. Labels sit beside the dots: always for the current note and the neighbours, for every note only at zooms with room (decision S10.2-D1). Every node hits within ≥ 44 × 44 brief px.
- Back navigation: a session history (note, write/preview, caret, scroll, preview page; cap 50) is pushed when a note is opened from a wiki link or backlink in preview, the Graph (map Open / second tap, list rows), To-dos, Contents jumps and Files. A Back arrow appears at the far left of the title bar (before Files) only while the history is not empty; Esc (when nothing else is open) and Alt+Left on a USB keyboard also go back. The title bar reserves the slot, so it still fits in portrait and landscape.
- Emulator: `tests/emu/v1.0.2-map-back.script`, control `Back`.
- Device binary sha256 (CI build) `5e14620bfeb40ca937f7ff06f9b9bf08b2df4af0c556f2a49c41e9c668703376`; tarball `1b9c2192c6f0e6d858a8ba5803cb93080dc7e663e91091b26a302da2f098e22f`.

## 1.0.1 — 2026-10-21 (built, not installed, not tagged)
- Pointer ghost trails fixed (sprint S10): every pointer move is now ONE update over the union of the old and new arrow (≥ 16×16) on a new `Refresh::Pointer` class mapped to FBInk `WFM_GL16` (grey-accurate, no flash) instead of `WFM_AUTO` (DU/A2, which left residue on the grey paper). When the arrow hides (3 s idle, typing, touching) one non-flashing GC16 partial covers its whole path since the last cleanup. Throttle stays 150 ms. Device log: `perf pointer: gl16 WxH submit+complete N ms` every 20th update.
- Reverse scroll: Options → **Scroll: Traditional / Natural** (default Traditional = 1.0). Natural flips every wheel / two-finger notch: note, preview, sheets, Graph zoom. Saved as `scroll=natural|traditional`.
- Label: Options → **USB keyboard & mouse**; Help → **USB keyboard, mouse & export**; About text updated. Settings key `usb` unchanged.
- Test notes pack (S10.1): sources in `tests/test-notes/`, `scripts/make-test-notes.sh` builds `dist/test-notes/` (316 files, incl. generated `graph-300/`) and a reproducible tarball; emulator `copy-notes DIR` and `tests/emu/v1.0.1-test-notes.script`.
- Emulator e-ink model: small Fast updates to mid greys now keep 30 % of the old pixel (DU-like), so trails show in `SIGMA_EINK=1`.
- Device binary sha256 (CI build) `ffc2e119e569e4364e565b7779d4e650183bb6107bc46e4bc982356650306567`; tarball `3aa18e6b2d0514f9b446a3e520d05307d7632b0ac40ba0b6376db6b8137ff03c`.

## 1.0.0 — 2026-10-21 (release candidate: built, not installed, not tagged, not released)
- Release prep (sprint S9): reproducible tarball (fixed order, owner, mode, time; `gzip -n`), `scripts/check-package.sh` (only `.adds/`, no notes, required files, ARM static, hash list), `THIRD-PARTY.txt` in the package (AGPL, FBInk GPL-3.0+ at 886f25f1, OFL fonts, NickelMenu not included).
- `.github/workflows/release.yml`: builds and checks the package with the pinned FBInk on every PR and push to main (artifact = release candidate); on a `v*` tag pushed by the maintainer it checks tag = Cargo version and creates a **draft** GitHub Release. It never tags or publishes.
- README rewritten (features, screenshots, install/update/uninstall), `docs/install.md` for users, `docs/release/` (release notes draft, MobileRead draft, Forma developer notes draft, release checklist), v1.0 screenshot tour `tests/emu/v1.0-tour.script`.
- Graph map (sprint S9.1): the Graph sheet opens on a drawn map of notes (labelled boxes) and links (straight lines with arrowheads), with a Map / List toggle (the list stays). Deterministic layout in `sigma-core` (seeded force-directed, 160 iterations; orphans on an outer ring), cached until the links change. Opens fitted to the whole graph, centred on the current note (filled box). Tap a note to select it and emphasise its neighbours; tap it again, or **Open**, to open it in preview. Pan by dragging or with the page buttons (Partial), zoom − / + over 4 levels (Full). Mouse: wheel zooms, click selects, drag pans. Large graphs show dots with labels only where they fit (current, selected, neighbours); zoom in to read the rest. 300 notes: layout 22 ms on the desktop (~0.2 s estimated on the Forma, once per link change).
- Title bar (sprint S9.2): icons twice the size (glyphs and targets; 100 px portrait, 134 px landscape; bar 114 / 152 px) and in the mockup's order after the file name: Files, Preview, Find, Scratch, To-dos, Contents, Graph, Options, Help (was mirrored). Sheets' close X moved to the rightmost slot. Help is now drawn in preview too. Overflow rule for narrow panels: smaller gaps, then smaller icons (never below 44 px); not needed on the Forma.
- Emulator: `tap-on Map|List|Zoom+|Zoom-|Open|Node:PATH`, `expect-graph map|list`, `expect-selected PATH|none`, `expect-zoom N`, `make-notes N`, `perf-graph [MAX_MS]`; script `tests/emu/v1.0-graph-map.script`.
- Docs corrected: the keyboard that worked on this Forma before was a wired 60% keyboard (most likely with KOReader), not the K400 Plus; host-mode switch compared with KOReader's.
- Device binary sha256 (CI build, the release candidate, with S9.2) `bb38a083b0925ced9adc61690430d0f768509a1cf284a0c533cf5839e570dad1`; tarball `edba7f6fb63d0147f0464addb8249ecb1946da4514e0823ac9cf6575e07b8c58`; local build `6d4316335944ae6b96726b16187d76cf7a6dad86fc0dc99442acb1a44bd24f9f`. (Earlier candidates: S9 `f2424114…f093d4`, S9.1 `22d8faac…7e52`.)

## 0.8.1 — 2026-10-21 (built, not yet installed or tagged)
- USB mouse and touchpad (sprint S8.1), including the touchpad half of a combined device such as the Logitech K400 Plus on its Unifying receiver: detected by EV_REL + REL_X/REL_Y; one reader per device feeds both keymap and pointer decoder. Touch panel and buttons are never pointers.
- On-screen arrow (outlined, Paper and Night): Fast partial refreshes of the changed area only (≥ 16×16), at most one redraw per 150 ms, hidden after 3 s still and when typing or touching.
- Left click = tap everywhere; drag on the note selects (same selection and bar as touch); double-click selects a word; right/middle click do nothing.
- Wheel / two-finger scroll (when sent as REL_WHEEL): note moves 3 rows per notch; preview, Files, Graph and Help turn a page.
- Unplug safe (also mid-drag); Help → USB keyboard & export has a Mouse and touchpad section.
- Emulator: `pointer plug|unplug`, `pointer-to`, `pointer-to-word`, `pointer-before`, `pointer-after`, `pointer-to-control`, `pointer-move`, `click`, `double-click`, `right-click`, `mouse-down`, `mouse-up`, `wheel`, `expect-pointer`; script `tests/emu/v0.8.1-pointer.script`.
- Device binary sha256 `21103122de5602e77c954ffb6dc0c303d645bf5cf91e6ea1f2129f048579f562`.

## 0.8.0 — 2026-10-21 (built, not yet installed or tagged)
- USB keyboard (Options → USB keyboard On): switches the Forma's port to USB host mode while on (back to normal on Off or exit), finds keyboards in `/proc/bus/input/devices` with hot-plug (1 s poll), reads them through a US keymap with Shift, Caps Lock, Ctrl, Alt and auto-repeat.
- Ctrl / Shift / Alt combinations go through the on-screen keyboard's shortcut code: Ctrl+Z/Y/A/C/X/V/F, Shift+arrows, Ctrl+arrows, Home/End, Ctrl+Home/End, PgUp/PgDn; in the note, Scratch, Find and rename fields. The on-screen keyboard is unaffected.
- Status bar: `USB` when a keyboard is connected, `no USB` when the setting is on but none is found.
- Help → USB keyboard & export: powered-hub note, shortcuts, export (copy the notes folder over USB), troubleshooting.
- Unplugging is safe; any failure (no host mode, no keyboard) is logged and leaves touch and buttons working.
- Desktop window: Ctrl+letter, Shift/Ctrl+arrows, Home/End send the same chords.
- Emulator: `usb plug|unplug`, `usb-type`, `usb-key`, `expect-usb`; script `tests/emu/v0.8-usb-keyboard.script`.
- Device binary sha256 `b533680ed86d175b49821cdb1312e9dc279bc123c557d8146423edbef903be27`.

## 0.7.0 — 2026-10-21 (built, not yet installed or tagged)
- Help sheet (`?`, leftmost in the title bar, write and preview): App info (version, licence, author, notes folder, buttons, USB keyboard, credits) and a paginated Markdown reference rendered by the preview engine; "← Back to Help", Esc goes back, page buttons turn.
- Hardening: a note that cannot be read (e.g. invalid UTF-8) opens read-only and is never overwritten; atomic save verified by a kill-during-save test (10 SIGKILLs, file always whole).
- Touch-target audit: every control ≥ 44 px scaled in portrait and landscape; drawer heading rows and the To-dos Filter button enlarged.
- Rotation pass: orientation round trip keeps text, caret and valid refresh regions; landscape screenshots of every sheet.
- Performance: key-event latency logged on exit (`perf: n … p50 … p95 … max`) to the device log; emulator `perf-type N`; report in `docs/perf.md`.
- Emulator: `expect-mode help`, `expect-help`, `perf-type`; script `tests/emu/v0.7-help-hardening.script`.
- Device binary sha256 `b8d76998234d71b6cf5e8bb626dda66738365f14ab198b0e8cbdfba7fe2f7789`.

## 0.6.0 — 2026-10-21 (built, not yet installed or tagged)
- Find bar (magnifier in the title bar, or Ctrl+F): Find and Replace fields, count (`Find` / `None` / `i of n`), Prev, Next, Replace, All, Aa (match case, off by default). A one-line selection seeds the query; Esc or the X closes it; opening it from preview switches to write. Current note only; Replace All is one undo step.
- Wiki links in preview: tap a known `[[note]]` to open it (in preview); tap a missing one (dashed) to create it, empty, beside the linking note (or in the folder the link names) and open it in write mode.
- Backlinks at the bottom of the preview: the notes that link here, each tappable.
- Graph sheet (connected-nodes icon): a list of every note with its outgoing (→) and incoming (←) links; tap a row to open the note. The scratch pad is not a node.
- Title bar order now Files, Preview, Find, Scratch, To-dos, Contents, Graph, Options (brief §3; Help comes in S7).
- Emulator: `expect-find`, `expect-mode find|graph`, `tap-on Link:LABEL`, `tap-on Graph:PATH`, Find-bar controls; script `tests/emu/v0.6-find-wiki.script`.
- Device binary sha256 `a9c2922daceefbbeae9b870e11ceb12e04cf5e1bbd2030733f59d361cf8f734c`.

## 0.5.0 — 2026-10-21 (built, not yet installed or tagged)
- Side drawer in write mode: three title-bar icons (Contents, To-dos, Scratch), any combination open, stacked with equal heights; side from Options → Drawer.
- Contents: the open note's headings, indented by level; tap jumps the caret to that line.
- To-dos: every `- [ ]` / `- [x]` across all notes and the scratch pad, filter All / Open / Done; the box toggles (only that line is rewritten on disk), the text opens the note at that line.
- Scratch: a side buffer typed with the keyboard, saved to the hidden `notes/.scratchpad.md` (not in Files, cannot be deleted); kept across note switches and restarts.
- Background indexer for to-dos across files; refresh regions stay ≥ 16×16.
- Emulator: `expect-scratch`, `expect-file-contains`, `expect-line`; script `tests/emu/v0.5-drawer.script`.
- Device binary sha256 `1ada5ae93504d287bb685e15e9cb3a15288cb65073dc233b220cd4293ec01f8f`.

## 0.4.0 — 2026-10-21
- Options sheet (`Aa` in the title bar): Paper, Type, Line, Size, Leading, Margin, Tab key, Tab width, Keyboard, Keyboard tone, Drawer, USB keyboard, Orientation.
- Night mode: the same page inverted, full refresh on switch; keyboard tone Theme / Light / Dark.
- Line mode: 8–16 words per line, fitted with the real face; Size and Margin move each other.
- Landscape (Options → Orientation): locked portrait-width column with gutters when Line is on; keyboards Centre / Split / Full with a number or arrow pad on either side.
- Every setting persists in `notes/.sigma-settings` (v0.2.1 files still load).
- Tab key follows Options (spaces 2/4/8 or a tab).
- Emulator: `--eink` also for headless runs; new script `tests/emu/v0.4-options.script`.
- Device binary sha256 `689b84c4e763119cbdfa19eb07ade52d5709cdf034bf971fc2e5116e97c02043`.

## 0.3.0 — 2026-10-21
- Markdown preview: eye / pencil toggle, read-only, paged with the page-turn buttons.

## 0.2.3 — 2026-10-21
- Touch selection (long-press, handles, action bar, double-tap); refresh regions ≥ 16×16 px and non-fatal refresh failures. (0.2.2 skipped.)

## 0.2.1 — 2026-10-21
- Full Windows-style on-screen keyboard (default), sticky modifiers, Ctrl shortcuts, undo/redo.

## 0.2.0 — 2026-10-20
- Library: Files sheet, folders, new / rename / move / delete, last-opened note.

## 0.1.0 — 2026-10-20
- Editor with on-screen keyboard and autosave to plain `.md`.
