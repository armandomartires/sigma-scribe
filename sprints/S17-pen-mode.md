# S17 — Pen mode (handwriting / sketch) with finger or stylus (draft)

> **DRAFT — planning only (2026-10-21). No development until the user approves.**

**Requirement (user, 2026-10-21 21:05):** pen mode must work with a **finger** as well as with a **capacitive stylus**. The Forma has no active pen (no Wacom digitizer), so both arrive as the same capacitive multitouch contacts; the only differences are contact size and steadiness.

## What the panel gives us
The touch driver already sends `ABS_MT_TOUCH_MAJOR` and/or `ABS_MT_PRESSURE` (we use them today only to detect lift, `crates/device/src/evdev.rs`). On the Forma these values are believed to scale with contact area, but their range and resolution are **unverified** — step 0 is a read-only probe on the device: log `TOUCH_MAJOR`/`PRESSURE`/`WIDTH_MAJOR` for a fingertip, a flat finger, a palm and a capacitive stylus (typical 6 mm rubber tip and a 1–2 mm fine tip).

## Input mode: Finger / Stylus / Auto
Options → Pen input: **Finger** · **Stylus** · **Auto** (default **Finger**, the safe choice until the probe proves Auto).
- **Auto:** classify each new contact by its first ~3 samples of contact size: small/steady → stylus; medium → finger; large or growing fast → palm (ignored). Thresholds come from the probe and are adjustable (calibration screen: "touch with your finger, then with the stylus"). If the panel's size values turn out to be coarse or constant, Auto is hidden and the explicit toggle stays.
- **Toolbar toggle** in pen mode (one tap) so the user can switch without opening Options.

## Palm rejection, designed for both
| Rule | Stylus | Finger |
|---|---|---|
| First contact wins; later contacts during a stroke are ignored | ✓ | ✓ |
| Contact bigger than the palm threshold → ignored, and suppresses input for 300 ms | ✓ | ✓ (higher threshold, so a flat finger still draws) |
| Two-finger contact = pan/zoom the canvas, never ink | — (stylus + palm counts as one) | ✓ |
| "Writing hand" side (Left/Right): contacts starting in the lower outer corner while a stroke is active are ignored | ✓ | optional |
| Minimum stroke length / dwell (ignore 1–2 sample taps unless Dot is on) | 2 px | 6 px (fingers jitter on landing) |

## Stroke width and smoothing
- Width presets **Fine / Medium / Bold** in mm, chosen per input: Stylus default Fine 0.6 mm; Finger default Medium 1.2 mm (a fingertip covers 8–10 mm, so a thin line would wander from where the user feels it is).
- No pressure-variable width (capacitive "pressure" is contact area, not force — unreliable for width); constant width per preset.
- Smoothing: light one-euro / moving-average filter, stronger for Finger; finger strokes drawn at the **centroid** with an optional small upward offset setting (0–3 mm) so the ink shows past the fingertip.
- Rendering: e-ink pen latency — draw new segments with fast DU (black ink on white canvas cell is fine for DU), one merged GL16/GC16 cleanup when the stroke ends (same pattern as the S10 pointer fix).

## Scope questions (need the user)
Pen mode was not in the v1.1 list given to me before; where does it live — a sketch block inside a note (saved as PNG/SVG next to the `.md` and linked), a separate drawing page, or handwriting in the margin? Erase gesture (stylus back is not detectable capacitively → eraser tool button)? Release target (v1.1 or later)?

**Estimate:** probe 0.5 d; input classification + palm rules 2 d; canvas, strokes, save/load, toolbar 4–5 d → **~1.5 weeks**. **Risks:** high on Auto (depends on panel data quality; mitigated by the explicit toggle), medium on latency and finger accuracy.
