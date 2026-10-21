# S12 — v1.2: Orientation, four rotations + Auto (draft)

> **DRAFT — planning only (updated 2026-10-21 21:24 with the user's answers). No development until the user approves.**

**Options → Orientation:** **Auto (default, user decision)**, Portrait, Landscape, Portrait inverted, Landscape inverted (a cycler at UI scale 200 %).

**Research (Forma, fw 4.38):** accelerometer (mma8x5x) readable from userspace as `EV_MSC / MSC_RAW` events, values 0x17 portrait-down, 0x18 portrait-up, 0x19 landscape-right, 0x1a landscape-left (0x1b/0x1c ignored) — KOReader `Input:handleMiscEvNTX` (`frontend/device/input.lua`), `misc_ntx_gsensor_protocol` for Kobo Frost/Forma (`frontend/device/kobo/device.lua`), with an "Ignore accelerometer rotation events" option. KOReader notes the events may only be enabled if Nickel ran in that power cycle — always true for a NickelMenu launch. Plato uses manual rotation only.

**Behaviour:** Auto applies a new reading after 0.6 s of stability, with one full refresh; Auto is **paused while a finger/stylus is down and for 1 s after typing**, so the screen does not turn mid-sentence. The last applied orientation is remembered.
**Fallback (user decision):** if no accelerometer device is found at start (no MSC_RAW in `/proc/bus/input/devices`) or no event arrives, the app uses the **last manual orientation** and shows "Auto (no sensor) — Portrait" in Options; Auto stays selectable.
**Work:** gsensor reader in the device crate (`Event::Orientation`), 4-way rotation in `present()` and touch mapping, page buttons per orientation; one read-only device probe first (0.5 d). **Estimate:** 3 d + probe. **Risks:** medium — event node on 4.38, inverted touch mapping (device-only), unwanted turns (pause rule).
