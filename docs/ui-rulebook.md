# UI rulebook

The rules every screen, control and change to Σscribe's interface follows. They come from `design-brief.md` (the page), the Kobo's own interface (`docs/ui-reference/README.md`, the chrome) and what the device taught us (`plan.md` decisions, `docs/device-checklist-prerelease.md`). When a rule here conflicts with a mock, **the rule wins for behaviour and the Kobo reference wins for chrome**; the page itself follows the design brief. A change that breaks a rule says so in its sprint note and in the changelog.

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
2. A checkbox and its "On" read as one target. A slider has one target per stop, each ≥ 44 px.
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

1. **Top bar:** back arrow left (not on the root), small centred "Settings", close × right; no rule under it. Page **title** large below, **"Page i of n"** italic at the right of the title when the page has more than one.
2. **Bands:** a full-width grey wash with an upper-case heading, 24 px high. A band is never the last thing on a page, and **a band with no visible row is not drawn** (no empty "Light" band on a Kobo without a light).
3. **Rows** are 52 px (sliders 84 px), a hairline rule between rows, label left, control right, 20 px side padding.
   - on/off: **checkbox + italic "On"** (a filled square when on). The Kobo uses a toggle switch on a few pages; we use checkboxes everywhere, so one habit covers the app.
   - two or three exclusive choices: **inline buttons**, the selected one filled in ink with the label inverted.
   - a number: **− value +** stepper, or a **slider** when the range is a handful of stops.
   - a longer list: a **Pick** row (value in italics) that opens a **choice page** (a list with a checkbox per choice); back returns to the category.
   - read-only: an italic value; disabled: `MUTED` grey value, no target.
4. **Root** lists the categories, each with a one-line summary of its state. A category holds bands; keep related rows together, put rarely used or risky ones in their own category (Experimental features).
5. **Hidden, not greyed,** when a row cannot apply on this device (Backlight without a front light) or belongs to a feature that is switched off (Fast ink and Word mode while Experimental features is off). Greyed only when the row *could* apply and a setting explains why not (Interface language while the Kobo's language is followed).
6. Settings are saved to `.sigma-settings` as `key=value`; a new key has a default and old files load unchanged. A setting that is only a request to the device (Backlight) is not saved.
7. Help text under a row is plain Literata in the mute colour and wraps; no marketing words.

## 6. Controls

1. **Scroll bar** (`scrollbar.rs`; editor and Settings): a **1 px hairline track** down the right margin, 12 px wide strip, and a **hollow, rounded (capsule) thumb** with a 2 px outline, at least 40 px tall, paper inside. The editor's thumb size and place follow the visible share of the note; Settings' follow the sub-page (`n` pages, thumb = one). Not drawn when everything fits. Display only, black and white.
2. **Slider** (`slider.rs`): a track with a dot per stop and a thumb; taps and drags snap to the nearest stop; Left/Right move it when focused; focus = a bigger thumb with a thicker ring. Enabled = solid ink (DU-safe); disabled = static grey, hollow dots and thumb. End labels only where they help ("Off" and "Full" on Backlight; none on Size).
3. **Icons** are Lucide (ISC), baked at build time with resvg into one greyscale mask per scale step (stroke ≈ 2.25), drawn by `icons::draw_icon`; no hand-drawn glyph and no icon font. A new icon is a Lucide name in `build.rs`. The Aa icon keeps its letters.
4. **Buttons** are outlined rounded rectangles (keys of the writing panel) or filled/outlined cells (inline buttons); the pressed or selected state is ink with an inverted label. The selected toolbar icon is a filled disc with a white glyph (the Kobo's).
5. **Popovers and sheets** open with a `Full` refresh when they cover the page; a popover that covers less uses `Partial`.

## 7. The writing panel

1. The panel covers the bottom 45 % (portrait) or 55 % (landscape), pure white with a 2 px black rule on top; ink is pure black.
2. A candidate bar on top (‹ words › ×), a key grid on the left (≥ 44 px keys), a row of caret and page keys (Home, End, PgUp, PgDn, Ins, Undo) above an open writing area that stays at least 55 % of the width. Pending ink is committed before a key acts.
3. Nothing in the panel is grey; keys are outlines and 1-bit text (`text_1bit`).

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

1. Look at `docs/ui-reference/` for the Kobo's version of the thing; copy its structure, not its pixels.
2. Use the existing widget (`Kind`, `slider`, `scrollbar`, `icons`) before writing a new one; a new widget gets a module, tests for geometry at every scale and for the two-colour rule, and a line in section 6.
3. Add or change the English keys in all nine catalogs.
4. Write an emulator script (`tests/emu/vX.Y-*.script`) with `shot` lines for the new look in portrait (and landscape or Night when it differs), run it, and look at the PNGs.
5. Tests: layout at 100/150/200 % and both orientations, targets ≥ 44, no overlap, no grey in fast regions, behaviour.
6. `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` and every emulator script pass before the PR.
7. Add the device checks to `docs/device-checklist-prerelease.md`; there is **one** device pass before the stable release (plan decision 48).
8. Changelog entry under Unreleased; a decision in `plan.md` when a rule changes.

## 11. Kobo reference patterns not built yet

Seen in `docs/ui-reference/`; use them when the feature comes, in this form.

1. **Text buttons:** a grey wash rectangle (the `wash` token), an italic Literata label, no outline, ≥ 44 px high; one primary button per page ("Continue", "Done", "Cancel"). Plain actions inside a list are italic text rows between hairlines.
2. **Dialogs:** a 1 px frame, a serif title, the body, wash buttons left-aligned under it; never dimmed or shadowed.
3. **Progress bar:** a flat bar, dark filled part and light remainder, no animation between updates (a step per refresh).
4. **Paged tour or help:** top bar (back, centred title, close), a line illustration, a serif headline, a sans paragraph, "Page i of n" italic at the bottom centre with ‹ › at the corners.
5. **On-screen keyboard, Kobo style** (built in v1.13; Settings → Enhanced keyboard off): number row always on top, letters in three rows, a bottom row with symbols key, accents key, an underline-bracket space bar, ← → and one dark action key; three layers (letters, symbols, accents), Shift switches case on every layer; keys are flat (no outlines) in sans; targets ≥ 44 px. It is an alternative **style**: with it on, nothing else is on the keyboard (no layout switch, no modifiers, no landscape bar); with Enhanced keyboard on, nothing on the Compact or Full board mentions it. Layouts without room for it keep the Full board.
6. **Language names** are shown in their own script (the Kobo's list), whatever the interface language.
7. **Help as a modal dialog:** a 1 px frame over the page (the page stays drawn, no dimming), a close × inside the frame's top right corner, a dotted circle behind the illustration, headline, paragraph, "Page i of n" and ‹ › at the bottom; the last page of a legend ends with a Done wash button. An **icon legend** page lists icon left, name right, one per row.
