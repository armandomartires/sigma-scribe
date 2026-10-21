# S12 — UI scale (draft)

> **DRAFT — planning only (2026-10-21). No development until the user approves.**

**Problem:** UI text is 17 brief px × (1440/1264) ≈ 19 px = **1.6 mm** on the Forma's 300 ppi panel (~4.6 pt) — too small; targets are 44 brief px ≈ 50 px = 4.2 mm (below the usual 7–9 mm finger guideline; the title bar was doubled in S9.2 as a stop-gap).

**Approach:** one **UI scale** setting — 100 / 125 / 150 / 175 / 200 % — that scales title bar, icons, sheets (Files, Options, Help, Graph), drawer, Find bar, status bar, keyboard key height and labels, and UI text together. The note text **Size** stays separate. Base sizes move from "brief px × width/1264" to **physical millimetres** × panel ppi (Forma 300 ppi = 11.81 px/mm; ppi from a device table, FBInk reports 300), so the same % looks the same on other Kobo models.

**At 100 %:** UI text 1.65 mm, target 4.2 mm (= today). **Default 175 %:** UI text ≈ 2.9 mm (~8 pt, comfortable reading at arm's length), targets ≈ 7.4 mm, title icons ≈ 8.5 mm (≈ today's S9.2 size, which becomes the 175 % look rather than a special case).

**Why % + mm instead of a raw DPI number:** DPI is a hardware fact, not a preference — the Forma is always 300; asking users to type "350 dpi" to get bigger buttons is confusing and invites broken values. A % is relative and obvious ("bigger"), has safe bounds, and the mm base keeps targets physically constant across 227/300 ppi devices. The overflow rule from S9.2 stays (gaps, then size, never < 44 px-equivalent 7 mm × 100 %).

**Work:** central `Ui` metrics struct replacing `px()` calls in all layouts; keyboard height per scale (landscape caps it so ≥ 6 text rows remain); persisted `ui_scale=175`; first run of 1.1 uses 175 %. **Estimate:** 3–4 d. **Risks:** layout regressions everywhere (mitigated by touch-target/overlap tests at every scale × orientation), fewer text rows with keyboard at 200 %, screenshot churn.
