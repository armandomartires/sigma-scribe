# S13 — Orientation: four rotations + Auto (draft)

> **DRAFT — planning only (2026-10-21). No development until the user approves.**

**Options → Orientation:** Portrait, Landscape, Portrait inverted, Landscape inverted, **Auto**.

**Research (Forma, fw 4.38):** the Forma has an accelerometer (mma8x5x). KOReader reads it from a raw input device as `EV_MSC / MSC_RAW` events, values 0x17 portrait-down, 0x18 portrait-up, 0x19 landscape-right, 0x1a landscape-left (0x1b/0x1c back/front ignored) — `misc_ntx_gsensor_protocol`, `Input:handleMiscEvNTX` in koreader `frontend/device/input.lua`, enabled for Kobo Frost (Forma) in `frontend/device/kobo/device.lua`; it offers "Ignore accelerometer rotation events". KOReader notes the events **may not be enabled unless Nickel ran in that power cycle** — Nickel always runs before a NickelMenu launch, so this should hold for us. Plato (Kobo reader) uses manual rotation. **Verdict: Auto is feasible**; to be confirmed with one read-only device probe (which `/dev/input/eventN` carries MSC_RAW; log the values while turning).

**Design:** device crate opens the gsensor device (detected by EV_MSC + MSC_RAW in `/proc/bus/input/devices`), emits `Event::Orientation(r)`; Auto applies it after 0.6 s of stability with one full refresh; manual modes ignore it. Rendering already rotates in `present()` (S4) — extend from 2 to 4 rotations and map touch for each; page-turn buttons keep "forward = lower button" per orientation (KOReader-style option later).

**Fallback** if no events on the device: the four manual options only, Auto hidden. **Estimate:** 2–3 d (+0.5 d probe). **Risks:** event device differs on 4.38; touch mapping for inverted modes (device-only); accidental rotation while writing in bed — Auto off by default (open question).
