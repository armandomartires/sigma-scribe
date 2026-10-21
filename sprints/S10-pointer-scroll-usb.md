# S10 — v1.0.1: pointer trails, Reverse scroll, USB label (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**

**Release:** **v1.0.1** on its own (user decision 2026-10-21) · **Estimate:** 1.75 d + device check · **Depends on:** 1.0.0 device results

## 1. Bug: the pointer leaves ghost trails (device, K400 Plus, 1.0.0)
**Cause (from the code):** each move is a `Refresh::Fast`, which FBInk maps to `WFM_AUTO`; on the Forma's EPDC that picks DU/A2 for small updates. DU drives pixels only to black or white, so (a) erasing the arrow back to the grey paper (#f0, 240) and the anti-aliased/grey outline leaves residue, and (b) the old and new rects can go out as separate regions from `diff_regions` (gap 24, up to 8), so the erase and the draw use different update markers.

**Options considered**
| Option | Trails | Speed | Flash | Notes |
|---|---|---|---|---|
| A. Merged old ∪ new rect, one update, **GL16** | gone (grey-accurate, no flash) | ~260 ms per update | none | recommended |
| B. Keep DU/A2, periodic GC16 cleanup | reduced, residue between cleanups | fastest | small flash on cleanup | fallback |
| C. XOR cursor | n/a | fast | none | rejected: no hardware cursor; XOR over greys is unreadable; still needs refresh |
| D. Pure black/white arrow + white box under it with DU | partly | fast | none | paper is grey, so a white box looks wrong |

**Recommendation: A + a cleanup.** Pointer moves send exactly one region = union(old arrow rect, new arrow rect) padded to ≥16 px, refreshed with GL16 (new `Refresh::Pointer` class mapped to `WFM_GL16`); when the arrow hides (3 s idle / typing) one GC16 partial over the bounding box of its path since the last cleanup. Keep the 150 ms throttle (raise to 250 ms if GL16 updates queue up on the device).
**Tests:** emulator e-ink mode models DU residue; regions per move == 1; cleanup region covers the path; device check: move in circles for 30 s over text → no trails.
**Risk:** GL16 latency makes the arrow feel laggy → measure; fallback B.

## 2. Reverse scroll
Options row **Scroll: Traditional / Natural** (default Traditional = 1.0 behaviour). Inverts wheel / two-finger notches everywhere (note rows, preview/sheet pages, Graph zoom). Persisted in `.sigma-settings` (`scroll=natural`). 0.5 d, low risk.

## 6. "USB keyboard & mouse"
Rename the Options row, Help page ("USB keyboard, mouse & export") and status help text; setting key unchanged for compatibility. 0.25 d.

## Device checklist
- [ ] Circles with the K400 touchpad over text and the title bar: no trails; arrow still responsive
- [ ] Natural vs Traditional on two-finger scroll in note, preview, Files and Graph
