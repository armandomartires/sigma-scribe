# S12 — v1.2: Orientation, four rotations + Auto

> **Status:** BUILT 2026-10-22 (v1.2.0, decision 39, not installed, not tagged). As built: Orientation Auto / Portrait / Landscape + Upside down Off/On (S12-D1, instead of a 5-value cycler); ≤ 1.1 settings move to Auto (S12-D2); g-sensor decoder and node finder in `crates/device/src/gsensor.rs` with `ENABLED = false` (S12-D3) — the read-only probe was not done (no device access). Tests `app/tests/orientation.rs`; script `tests/emu/v1.2-orientation.script` (pretend sensor); shots `docs/screenshots/v1.2/`.
>
> **Device check list:** (1) read-only probe: `/proc/bus/input/devices` has an MSC_RAW node; codes seen when turning the Forma; (2) map 0x19/0x1a to the right landscape; set `ENABLED = true` and wire the reader; (3) touch lands under the finger in all four rotations; (4) page buttons forward/back feel right upside down; (5) no turning mid-typing.

**Options → Orientation:** **Auto (default, user decision)**, Portrait, Landscape, Portrait inverted, Landscape inverted (a cycler at UI scale 200 %).

**Research (Forma, fw 4.38):** accelerometer (mma8x5x) readable from userspace as `EV_MSC / MSC_RAW` events, values 0x17 portrait-down, 0x18 portrait-up, 0x19 landscape-right, 0x1a landscape-left (0x1b/0x1c ignored) — KOReader `Input:handleMiscEvNTX` (`frontend/device/input.lua`), `misc_ntx_gsensor_protocol` for Kobo Frost/Forma (`frontend/device/kobo/device.lua`), with an "Ignore accelerometer rotation events" option. KOReader notes the events may only be enabled if Nickel ran in that power cycle — always true for a NickelMenu launch. Plato uses manual rotation only.

**Behaviour:** Auto applies a new reading after 0.6 s of stability, with one full refresh; Auto is **paused while a finger/stylus is down and for 1 s after typing**, so the screen does not turn mid-sentence. The last applied orientation is remembered.
**Fallback (user decision):** if no accelerometer device is found at start (no MSC_RAW in `/proc/bus/input/devices`) or no event arrives, the app uses the **last manual orientation** and shows "Auto (no sensor) — Portrait" in Options; Auto stays selectable.
**Work:** gsensor reader in the device crate (`Event::Orientation`), 4-way rotation in `present()` and touch mapping, page buttons per orientation; one read-only device probe first (0.5 d). **Estimate:** 3 d + probe. **Risks:** medium — event node on 4.38, inverted touch mapping (device-only), unwanted turns (pause rule).
