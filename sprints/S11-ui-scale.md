# S11 — v1.1: UI scale

> **Status:** BUILT 2026-10-22 (v1.1.0, decision 38, not installed, not tagged). Planned 2026-10-21 21:24 with the user's answers. As built: chrome scales by the % over the device scale (short side / 1264 brief px, equal to the mm base on the Forma; per-ppi mm base deferred); keyboard capped at 42 % / 45 % of the height; Lucide icons baked per step (decision 35). Tests `app/tests/ui_scale.rs`; script `tests/emu/v1.1-ui-scale.script`; shots `docs/screenshots/v1.1/`. Device check still needed.

**Setting:** Options → UI scale 100 / 125 / 150 / 175 / **200 % (default, user decision)**. Scales title bar, icons, sheets (Files, Options, Help, Graph), drawer, Find bar, status bar, keyboard key height and labels, and UI text together; the note text **Size** stays separate. Base sizes are physical mm × panel ppi (Forma 300 ppi = 11.81 px/mm), not "brief px × width/1264", so landscape no longer grows the UI by 1920/1440.

**Why % + mm, not a DPI number:** DPI is a hardware fact (the Forma is always 300); typing "600 dpi" to get bigger buttons is confusing and invites broken values. A % is relative and bounded; the mm base keeps sizes physically equal across 227/300 ppi Kobos.

**What 200 % means on the Forma** (100 % = today's 1.0 sizes): UI text 39 px ≈ 3.3 mm (~9.4 pt); targets 100 px ≈ 8.5 mm; title icons 100 px (= today's S9.2 size, now simply the 200 % value of a 44-unit target); title bar 114 px; status bar 118 px; keyboard key height 132 px.

## Fit check at 200 % (computed from the current layout code)
| Area | Portrait 1440×1920 | Landscape 1920×1440 | Verdict |
|---|---|---|---|
| Title bar: 9 icons 100 px + gaps | 986 px of 1440, title keeps 438 px (~17 bold chars at 39 px) | 986 of 1920, title 918 px | fits; long names ellipsize earlier |
| Full keyboard (5 rows) | 762 px tall; page keeps ~926 px (≈ 27 note rows at the default note size) | 762 + 114 (Split/Full/Centre bar) = 876 px; page keeps **~330 px ≈ 9 note rows** | **tight in landscape** |
| Full keyboard key width | 16 units → ~86 px wide × 132 tall: tall narrow keys; single glyphs fit, "Shift"/"Return" fit on wide keys | keys sized from the short side, same as portrait | fits, looks tall → cap key height at 1.5× when width/unit < key height |
| Compact keyboard (4 rows) | ~620 px | ~720 px with bar; page ~490 px | fits |
| Status bar text "1 234 words · line 56 · Paper · saved" + Keys + USB | ~700 px + 145 px | plenty | fits |
| Options sheet | 2–3 choice cells per row fit; **Orientation now has 5 choices ("Landscape inverted" ≈ 360 px)** and UI scale has 5 → do not fit in one row | 5 short cells fit | **tight → Orientation and UI scale rows become a value + ‹ › cycler (or a 2-line row)** |
| Files sheet | row 2× tall → ~11 rows per page instead of ~22; More/rename fit | ~8 rows | fits, more paging |
| Graph toolbar (Map, List, Open, −, +) | ~1 010 px | fits | fits; footer text ellipsizes |
| Find bar (2 fields + Prev/Next/Replace/All/Aa/X) | tool row ~1 150 px | fits | **tight in portrait** → icons for Prev/Next, text for Replace/All |
| Drawer side pane (landscape, Line on) | n/a | 40 % width, rows 2× tall, "N more…" earlier | fits, shorter lists |
| Help pages | half the lines per page, twice the pages | | fits |

**Mitigations:** key-height cap (keyboard ≤ 45 % of the screen height in landscape, ≤ 42 % portrait); cyclers for 5-value rows; S9.2 overflow rule generalised (gaps, then size, never < 100 % size); pseudo-layout tests at every scale × orientation × theme for overlap and ≥ 7 mm targets.
**Estimate:** 4–5 d. **Risks:** medium — touches every layout; landscape writing space with the Full keyboard at 200 %; screenshot/emulator churn.

## Note (2026-10-22): Lucide icons (plan decision 35)
User-approved, not urgent, done alongside or right after this sprint: replace the ~11 custom `draw_*_icon` functions in `crates/render` with Lucide icons (ISC). Bake the SVGs with resvg at build time into small greyscale images, one per icon per UI-scale step (100/125/150/175/200 %, no runtime scaling), embed them and draw with one blit function. Keep the Options 'Aa' letters. Stroke ~2 px at device size, avoid fine detail (e-ink). Add Lucide's ISC LICENSE (repo + package THIRD-PARTY) and a credit line in README and About.
