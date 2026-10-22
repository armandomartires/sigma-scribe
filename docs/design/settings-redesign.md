# Settings redesign — PROPOSAL (not approved, not built)

> **Status:** proposal for Armando's approval, 2026-10-22. Nothing in the app changes until it is approved. Mocks are static images (`docs/design/mock-settings.py`), not emulator screenshots.

## Goal
Make Options feel like the Kobo Forma's own Settings: a list of categories, each opening its own page, instead of one long sheet with 17 rows of segmented buttons.

## What is known about Nickel's Settings (firmware 4.x)
These points come from public sources: the *Kobo Forma User Guide* (cdn.kobo.com, EN), Kobo help pages and user write-ups (e.g. code.mendhak.com/kobo-customizations).
- You reach Settings from the Menu / More icon, then **Settings**. It opens as a **list of categories**: Accounts, Date and time, Energy saving and privacy, Language and dictionaries, Reading settings, Device information, OverDrive and others. Each category opens its own page.
- Pages group rows under **section headings**, for example "Under 'Button Controls', tap Inverted" and "Under Dictionary, tap Edit".
- Rows have the **label on the left and the control on the right**: "beside 'Ask for PIN…', tap the **checkbox** on the right", "beside Repair your Kobo account, tap **Repair**", and **dropdowns** for choices (for example Font Face, and values such as "Header: Off", "Button Controls: Inverted").

**Not verified.** I found no source I could check for these, so the proposal takes them as *our* choices, not as copies of Nickel:
- the exact back-arrow and chevron artwork;
- the title bar layout;
- separator weight;
- fonts;
- whether 4.38 uses switches anywhere. The Forma guide describes **checkboxes**, not switches.

This gap is the first open question below.

## Proposed structure
Root **Settings** page: one row per category. Each row shows the category name, a short summary of its current value (muted) and a chevron.

| Category | Rows (current Options → new control) |
|---|---|
| **Writing** | Typeface (Literata / Atkinson → choice page) · Size, Leading, Margin (steppers) · Fixed words per line (switch) + Words per line (stepper) · Tab key inserts (Tab / Spaces → choice) · Tab width (2 / 4 / 8 → choice) |
| **Display** | Page (Paper / Night → choice) · UI scale (stepper, 100–200 %) · Orientation (Auto / Portrait / Landscape → choice) · Upside down (switch) · Drawer side (Left / Right → choice) |
| **Keyboard & input** | Keyboard (Compact / Full → choice) · Layout (7 layouts → radio list) · Keyboard tone (Follow page / Light / Dark → choice) |
| **Language** | Interface language: "English (from the Kobo)", read-only for now; the override (S14-D2) would become a radio list here |
| **USB & devices** | USB keyboard & mouse (switch, with the powered-hub hint) · Scroll direction (Traditional / Natural → choice) · optionally a read-only "Connected" row with the device name from the log |
| **Help & about** | App info, Markdown reference, USB help: the current Help pages move here. The `?` title-bar icon stays as a shortcut |

**Controls**
- **Switch** for on/off settings: two states, one tap. We could use a checkbox instead to match the Forma guide exactly; see the open questions.
- **Stepper** (− value +) for numbers.
- **Choice page** for two or more named values: a sub-page with a radio list, where a tap selects the value and goes back.
- Choices with only 2–3 short values could instead stay as segmented buttons on the row. That is fewer taps, but less Kobo-like.

**Mocks** in `docs/screenshots/proposal-settings/`:
- `1-settings-root.png`
- `2-writing.png`
- `3-keyboard-input.png`
- `4-layout-radio.png`
- `5-usb-devices.png`

## Interaction
- **Tap** a category opens its page; tap a choice row opens its radio list; tap a radio row selects it, saves immediately (as today) and returns.
- **Back:** the ← in the title bar, **Esc** on a USB keyboard, or the **page-back button** goes up one level. **X** closes Settings from any level and returns to the note.
- The title shows where you are ("Writing", "Layout"). There is no breadcrumb, matching one page per level.
- Settings stay instant: no Save or Cancel. A changed value repaints the row and shows the effect when you return to the note. Display changes (Night, UI scale, orientation) apply on the spot with one full refresh, as now.
- Emulator and test addresses keep working: `tap-on Layout/+` becomes `tap-on Keyboard & input` → `tap-on Layout` → `tap-on French (AZERTY)`. Old scripts need updating (planned, mechanical).

## Fit at 200 % UI scale
- Rows are 150 px at 200 % on the Forma, well above the 44 px minimum. That gives about 11 rows on a portrait page and about 7 in landscape. The largest page, Writing, has 8 rows plus 3 section headings, which fits in portrait. In landscape it needs page-turn paging, using the same `page i of n` pattern as Help.
- Labels get the full width minus the control. There are no hints under labels except where they matter (USB power, Layout). This removes today's long hints that had to be shortened with "…" in French and Portuguese.
- The pseudo-long language test (+40 %) covers every page in both orientations, as it does for Options now.

## Cost and risk
- About 2–3 days: a generic page/row renderer (switch, stepper, chevron, radio), a navigation stack, and porting the 17 rows.
- Tests and emulator scripts need updating.
- Risk is low; the settings file format does not change.
- More taps for frequent toggles such as Paper/Night. To offset that, the `Aa` icon could open the Display page directly; see the open questions.

## Open questions for approval
1. Switches, or checkboxes like the Forma guide describes? A quick look at the device's Settings page (any page) settles it: a photo is enough.
2. Should short choices (Paper/Night, Left/Right, Tab/Spaces) be choice pages (Kobo-like) or stay as inline segmented buttons (fewer taps)?
3. Should `Aa` open the Settings root, or a quick page with Typeface/Size/Paper?
4. Should the release number for this be v1.5 (bumping handwriting to v1.6), or v1.4.2?
