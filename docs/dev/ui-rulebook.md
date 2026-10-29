# UI rulebook

The rules every screen, control and change to Σscribe's interface follows. They come from `.ai/design-brief.md` (the page), the Kobo's own interface (`docs/dev/ui-reference/README.md`, the chrome) and what the device taught us (`.ai/plan.md` decisions, `docs/dev/device-checklist-prerelease.md`). When a rule here conflicts with a mock, **the rule wins for behaviour and the Kobo reference wins for chrome**; the page itself follows the design brief. A change that breaks a rule says so in its sprint note and in the changelog.

Numbers in "px" are **brief px** (a 1264 px wide page) unless marked *device px*; the renderer scales them (`Scale::px`, 1440 px wide Forma = ×1.139) and multiplies chrome by the UI scale (100–200 %, default 200 %).

## 1. E-ink first

1. **Fast regions are pure black and white.** Anything that moves or changes on its own (pointer, live ink, scroll bar, enabled slider, typing caret) is drawn with exactly `0x00` and `0xFF` (inverted at night) so the DU waveform renders it cleanly. No anti-aliased edge, no grey. Static grey is allowed only where a greyscale refresh follows (the disabled slider, `MUTED` = `0x88`).
2. **Refresh classes** (`Refresh`): `Fast` for keys and caret, `Pointer` and `Ink` (DU, never waited for), `PointerCleanup` (one non-flashing GL16 pass when the moving thing stops), `Partial` for a bigger change (keyboard toggled, a page turned, a setting changed), `Full` (flashing) only for theme, rotation, opening a sheet and the first paint.
3. **The input loop never waits for the panel.** No code path blocks on refresh completion.
4. **Regions are at least 16 × 16 device px** (the panel refuses smaller ones); the frame diff sends only what changed.
5. **No animation, no glow, no shadow, no transparency.** State changes are instant and drawn in ink.
6. Only what changed is sent: a control whose look can change without user input (a clock, a spinner) does not exist.

## 2. Touch

1. **Every target is at least 44 px (device px at the device scale), 44 × 44.** Check it at every UI scale (100/125/150/175/200 %) and in both orientations; `app/tests/ui_scale.rs` and the panel layout tests do.
2. A switch and its state word read as one target; a radio button is one target. A slider has one target per stop, each ≥ 44 px.
3. Keyboard keys are ≥ 44 px at the keyboard's own scale (Auto = UI scale up to 150 %), and the keyboard never covers more than 42 % (portrait) or 45 % (landscape) of the height.
4. Page buttons turn pages everywhere there is paging (note, preview, Settings sub-pages, Graph); never put a destructive action on one.

## 3. Colour

| Role | Day | Night |
|---|---|---|
| Paper | `#f3f0e8` (luma 240) | `#111111` (17) |
| Ink | `#1c1c1c` (28) | `#ece8df` (233) |
| Rule | `#c9c2b4` (195) | `#3c3c3c` (60) |
| Mute | `#5e584e` (89) | `#a8a294` (161) |
| Wash | `#e7e1d4` (226) | `#1c1c1c` (28) |

Night is the same drawing inverted, never a grey card. Use the `Theme` tokens, never a literal grey. Fast regions (rule 1.1) ignore the tokens and use black/white.

## 4. Type

1. **Literata** (serif) for titles, row labels and values; **Atkinson Hyperlegible** for buttons, bands, keys and captions. Values and actions are **italic** (Literata Italic); selected buttons are bold.
2. Sizes come from `ui_px()`: body 1×, page title 1.5×, the small centred "Settings" 0.9×, "Page i of n" 0.9× italic. The note text has its own Size (not the UI scale).
3. Band headings are **upper-case small** (the Kobo's), page titles sentence case, labels sentence case with a colon on Settings rows ("Layout:").
4. A character no bundled face has falls back per character (`Fonts::face_for`); `app/tests/glyphs.rs` fails on a UI character nothing can draw. CJK, Hangul, kana and the Han ranges come from the optional font pack and are exempt.

## 5. Settings pages (the Kobo pattern)

1. **Top bar:** back arrow left (not on the root), small centred "Settings" ("Help" on the Help screens), close × right; no rule under it. Page **title** large below, **"Page i of n"** italic at the right of the title when the page has more than one. Settings and Help share this header (`page_header`, `draw_page_header`): a Help sub-page is the same screen with its topic as the title and the back arrow in the bar; there is no separate crumb row and no footer line.
2. **Bands:** a full-width grey wash with an upper-case heading, 24 px high. A band is never the last thing on a page, and **a band with no visible row is not drawn** (no empty "Light" band on a Kobo without a light).
3. **Rows** are 52 px (sliders 84 px), a hairline rule between rows, label left, control right, 20 px side padding.
   - on/off: a **toggle switch** (`switch.rs`), after the Kobo's own: an upper-case state word (ON / OFF, translated) and a capsule. On = a filled capsule with a paper knob ringed in ink at the right; off = an outlined capsule with a ringed knob at the left. One target, 44 px high, flush right. Every on/off setting uses it (`Kind::Check`). A single-choice list is not on/off: it uses radio buttons.
   - two or three exclusive choices: **inline buttons**, the selected one filled in ink with the label inverted.
   - a number: **− value +** stepper, or a **slider** when the range is a handful of stops (Size, Leading, Margin, UI scale, Keyboard size, Backlight, Warmth).
   - a longer list: a **Pick** row (value in italics) that opens a **choice page**: a list with a **radio button** per choice (`radio.rs`: a ring, with a dot inside the chosen one, flush right, 44 px target); back returns to the category.
   - read-only: an italic value; disabled: `MUTED` grey value, no target.
4. **Root** lists the categories, each with a one-line summary of its state. A category holds bands; keep related rows together, put rarely used or risky ones in their own category (Experimental features).
5. **Hidden, not greyed,** when a row cannot apply on this device (Backlight without a front light) or belongs to a feature that is switched off (Fast ink and Word mode while Experimental features is off). Greyed only when the row *could* apply and a setting explains why not (Interface language while the Kobo's language is followed, Upside down on Auto, Words per line with Line off, Tab width with Tab key on Tab, Keyboard size with Auto on). A greyed row says why in a **note** under its label (`DRow::note`, small Atkinson in the mute colour); no other row has one.
6. Settings are saved to `.sigma-settings` as `key=value`; a new key has a default and old files load unchanged. A setting that is only a request to the device (Backlight) is not saved, unless the device cannot be asked for it later (Warmth: the stock reader never reads the mixer back, so the step is saved and sent at start). Each line of the file is one entry of `settings::FIELDS` (key, how it is read, how it is written); a setting with a fixed set of values lists them once in its `Words` table (the file's word and the Settings label), which both the file and the row's cells use.
7. Notes (rule 5) are one short line, no marketing words. Rows that are live carry no help text; the Help screen explains the rest.

## 6. Controls

1. **Scroll bar** (`scrollbar.rs`; editor, Preview, Help and Settings, all through `scrollbar::bar` and `scrollbar::Metrics`): a **1 px hairline track** down the right margin and a **hollow, rounded (capsule) thumb**. Every size lives in `Metrics` (handle 17 px, 1.5 handle widths from the edge, outline 2.5 px, thumb at least 40 px, inset 8 px), in brief px scaled by the **page** scale, never the UI scale, so the bar is the same size on every screen and at every UI scale. The editor's thumb follows the visible share of the note; Preview, Help and Settings follow the page (`n` pages, thumb = one). Not drawn when everything fits. Black and white on the paper (on the desk tone beside a locked column). It answers a finger everywhere: drag the thumb, or tap above or below its centre to move a screen (editor) or a page (Settings, Help, Preview) back or forward; the touch zone is the strip widened to the page edge (`scrollbar::zone`, `scrollbar::Paged`). Pages keep their content `Metrics::content_gap` clear of the bar (`content_inset`).
2. **Toggle switch** (`switch.rs`) and **radio button** (`radio.rs`): see §5.3. All sizes (`switch::Metrics`), the target and the drawing live in that module, so one change reaches every switch. Pure ink and paper.
3. **Slider** (`slider.rs`), after the Kobo's brightness slider: a **thick track** up to a **hollow ring thumb** and a **hairline** after it; no dot per stop. **Every slider shows its current value over the thumb** and updates it as the thumb moves (Size "20", Leading "1.5", Margin "28", UI scale "150 %", Keyboard size "125%", Backlight "Off" / "25 %" / "Full", Warmth "Cool" / "60 %" / "Warm"); sliders with an `ends` flag (UI scale, Keyboard size, Backlight, Warmth) also label the two ends on the same line. Taps and drags snap to the nearest stop, one target per stop ≥ 44 px (**UI scale is the one exception: it rescales the whole screen, so its thumb and value follow the finger but the scale changes only when the finger lifts**); Left/Right move it when focused; focus = a bigger thumb with a thicker ring. Enabled = solid ink and 1-bit text (DU-safe); disabled = static grey (`MUTED`).
4. **Icons** are Lucide (ISC), baked at build time with resvg into one greyscale mask per scale step (stroke ≈ 2.25), drawn by `icons::draw_icon`; no hand-drawn glyph and no icon font. A new icon is a Lucide name in `build.rs`. The Aa icon keeps its letters.
5. **Buttons** are outlined rounded rectangles (keys of the writing panel) or filled/outlined cells (inline buttons); the pressed or selected state is ink with an inverted label. The selected toolbar icon is a filled disc with a white glyph (the Kobo's).
6. **Popovers and sheets** open with a `Full` refresh when they cover the page; a popover that covers less uses `Partial`.

## 7. The writing panel

1. The panel covers the bottom 45 % (portrait) or 55 % (landscape), pure white with a 2 px black rule on top; ink is pure black.
2. **Write** (free writing): a candidate bar on top (‹ words › ×), a key grid on the left (Sketch swap, `, . ? ! @`, 123, language, Space; ≥ 44 px keys) and, above an open writing area that stays at least 55 % of the width, two rows: **Undo, Redo, Erase, Return** and **Home, End, Ins, PgUp, PgDn**. Pending ink is committed before a key acts.
3. **Letter boxes** use the same look and the same two rows: Write/Sketch left, Undo, Redo, Erase, Return, Space centred, Train and Done right; the nav row centred; the candidate row on top.
4. **Sketch** (was Draw): Write on the left; Undo, Clear, Pen and **Line** (shows or hides the dotted guide) centred; Cancel and Insert (the one dark key) right; a footer line says a sketch is added as an image link and Preview shows it whole.
5. Icons: Undo, Redo, Erase, Return, Space, Home, End, PgUp, PgDn and Tab are Lucide icons (§6.3), as on the enhanced keyboard; Ins and words stay text.
6. Nothing in the panel is grey; keys are rounded outlines and 1-bit text (`text_1bit`). The enhanced keyboard (Compact and Full) uses the same rounded outlines, and its number pad has a **+ − * /** column and a Return key.

## 8. Words

1. Every user-facing string is an **English key** passed through `tr()`; it has a line in all nine catalogs (`assets/i18n/*.txt`), machine-drafted where nobody native has checked it (About says so). A missing key shows English, a catalog test fails on a key that exists in one catalog and not another.
2. Sentence case; no trailing full stops on labels; "On"/"Off" for states; ellipsis only for text that is cut.
3. Names of things stay the user's: files are shown as typed; notes are `.md`.

## 9. Behaviour

1. **No surprises on the user's data:** edits autosave; destructive actions are armed (a second tap); undo works for typed text; nothing is sent anywhere.
2. **Experimental means switched off by default.** A feature that is not fully tested lives behind Settings → Experimental features and says so in its name (Word mode (experimental)); an old settings file that already used one keeps it on (the switch starts on).
3. **Device side effects are requests, not state.** Backlight, USB host mode and the like are sent to the device by the app loop (`sync_*`), logged, and restored or left as the stock reader leaves them. A failure is logged and the UI carries on.
4. **Orientation and scale:** every screen works in portrait, landscape and upside down, at every UI scale.

## 10. How a UI change is made (the checklist)

1. Look at `docs/dev/ui-reference/` for the Kobo's version of the thing; copy its structure, not its pixels.
2. Use the existing widget (`Kind`, `slider`, `scrollbar`, `icons`) before writing a new one; a new widget gets a module, tests for geometry at every scale and for the two-colour rule, and a line in section 6.
3. Add or change the English keys in all nine catalogs.
4. Write an emulator script (`tests/emu/vX.Y-*.script`) with `shot` lines for the new look in portrait (and landscape or Night when it differs), run it, and look at the PNGs.
5. Tests: layout at 100/150/200 % and both orientations, targets ≥ 44, no overlap, no grey in fast regions, behaviour.
6. `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` and every emulator script pass before the PR.
7. Add the device checks to `docs/dev/device-checklist-prerelease.md`; there is **one** device pass before the stable release (plan decision 48).
8. Changelog entry under Unreleased; a decision in `.ai/plan.md` when a rule changes.

## 11. Kobo reference patterns not built yet

Seen in `docs/dev/ui-reference/`; use them when the feature comes, in this form.

1. **Text buttons:** a grey wash rectangle (the `wash` token), an italic Literata label, no outline, ≥ 44 px high; one primary button per page ("Continue", "Done", "Cancel"). Plain actions inside a list are italic text rows between hairlines.
2. **Dialogs:** a 1 px frame, a serif title, the body, wash buttons left-aligned under it; never dimmed or shadowed.
3. **Progress bar** (built, on the splash screen: `Splash`): a flat bar as in the Kobo's loading screen: "Loading…" in the serif, left-aligned, over a thin bar, dark where it is done and the rule tone for the rest; no outline, no animation between updates (a step per refresh, only the bar region refreshes). The splash is drawn the moment the panel opens, in the saved paper and rotation: name, author, then the word and the bar.
4. **Paged tour or help:** top bar (back, centred title, close), a line illustration, a serif headline, a sans paragraph, "Page i of n" italic at the bottom centre with ‹ › at the corners.
5. **On-screen keyboard, Kobo style** (built in v1.13; Settings → Enhanced keyboard off): number row always on top, letters in three rows, a bottom row with symbols key, accents key, an underline-bracket space bar, ← → and one dark action key; three layers (letters, symbols, accents), Shift switches case on every layer; keys are flat (no outlines) in sans; targets ≥ 44 px. It is an alternative **style**: with it on, nothing else is on the keyboard (no layout switch, no modifiers, no landscape bar); with Enhanced keyboard on, nothing on the Compact or Full board mentions it. Layouts without room for it keep the Full board.
6. **Language names** are shown in their own script (the Kobo's list), whatever the interface language.
7. **Help as a modal dialog:** a 1 px frame over the page (the page stays drawn, no dimming), a close × inside the frame's top right corner, a dotted circle behind the illustration, headline, paragraph, "Page i of n" and ‹ › at the bottom; the last page of a legend ends with a Done wash button. An **icon legend** page lists icon left, name right, one per row.

## 12. Page and desk

1. The page's side margins are plain paper. Only **landscape with Line length on** shows a desk: the strips beside the locked column are the `gutter` tone (darker by day, lighter at night) with a fixed fine grain (a hash of the pixel's place, the same every frame). The document is a page on the desk, with its normal text margin inside.
2. The scroll bar sits on the desk tone beside a locked column and on the paper otherwise.

## 13. Names

1. The app is **Σscribe** (NickelMenu entry, window, About); the author is **SIGMArecon**. The NickelMenu tab is **My Apps** with its own grid icon (`experimental:menu_main_15505_*` in `nm/sigma-scribe`, icons under `.adds/sigma-scribe/icons/`). File, folder and binary names stay `sigma-scribe`.

## 14. The title bar

1. Right-aligned icons, left to right: Files, Preview, Find, Scratch, To-dos, Contents, Graph, **Settings** (the Lucide gear). There is no Help icon: Help opens from Settings → Help & about. On the sheets (Files, Graph, Help, Settings) the Close × takes the rightmost slot.
