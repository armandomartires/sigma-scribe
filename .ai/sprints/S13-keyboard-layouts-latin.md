# S13 — v1.3: Keyboard layouts (Latin) for on-screen and USB

> **Status:** ✅ DONE (passed on the Forma, confirmed 2026-10-27); built 2026-10-22 (v1.3.0, decision 40, not tagged). As built: US, UK, FR AZERTY, DE QWERTZ, ES, PT, IT; tables hand-written from xkb basic variants (S13-D1); one composer for both keyboards (S13-D2); Options → Layout applies to both (no USB override yet). **Deferred (S13-D3):** Compact long-press accent popup + candidate bar slot, Nordic / NL / ABNT2. Tests `app/tests/layouts.rs`, `crates/core/src/layouts.rs`, glyph coverage in `crates/render/src/fonts.rs`; script `tests/emu/v1.3-layouts.script`; shots `docs/screenshots/v1.3/`. Device check: AltGr on the K400 / a 60 % board; dead keys from a real USB keyboard.

**Scope v1.3:** US (today), **FR AZERTY**, **PT** (Portugal; Brazil ABNT2 easy to add), **DE QWERTZ**, **ES**; easy extras from the same tables: UK, IT, Nordic (SE/NO/DK/FI), NL (US-intl). Options → Layout (applies to both keyboards; USB can override if wanted).

**Approach:** layout tables (keycode → base / Shift / AltGr / Shift+AltGr, dead keys) generated once from xkb `symbols/` data by a small dev script and checked in as Rust data (no runtime xkb); dead keys ´ ` ^ ~ ¨ compose with the next letter (fallback: the accent then the letter). The on-screen Full layout gets per-locale rows (AZERTY/QWERTZ letter positions, extra keys like ç, ñ, ß) and a long-press accent popup for Compact; USB uses the same table so Ctrl shortcuts keep their **letter** meaning (Ctrl+Z is Z, wherever it sits).
**Fonts:** Literata and Atkinson already cover Latin-1 + Latin Extended-A — no size impact.
**Estimate:** 4–5 d. **Risks:** OSK width for extra keys in Full; AltGr on 60% boards via Fn; test effort (emulator scripts per layout).

**Framework role:** v1.3 introduces the input pipeline used by all later input work: `KeySource (OSK | USB | handwriting) → layout table → composer (dead keys; later IME) → editor`, plus a **candidate bar** slot above the keyboard (first used for accent long-press). Handwriting (v1.5) and the CJK IMEs (v1.7, v1.8) plug into the composer and candidate bar.
