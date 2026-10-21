# S14 — Keyboard layouts (Latin) for on-screen and USB (draft)

> **DRAFT — planning only (2026-10-21). No development until the user approves.**

**Scope v1.1:** US (today), **FR AZERTY**, **PT** (Portugal; Brazil ABNT2 easy to add), **DE QWERTZ**, **ES**; easy extras from the same tables: UK, IT, Nordic (SE/NO/DK/FI), NL (US-intl). Options → Layout (applies to both keyboards; USB can override if wanted).

**Approach:** layout tables (keycode → base / Shift / AltGr / Shift+AltGr, dead keys) generated once from xkb `symbols/` data by a small dev script and checked in as Rust data (no runtime xkb); dead keys ´ ` ^ ~ ¨ compose with the next letter (fallback: the accent then the letter). The on-screen Full layout gets per-locale rows (AZERTY/QWERTZ letter positions, extra keys like ç, ñ, ß) and a long-press accent popup for Compact; USB uses the same table so Ctrl shortcuts keep their **letter** meaning (Ctrl+Z is Z, wherever it sits).
**Fonts:** Literata and Atkinson already cover Latin-1 + Latin Extended-A — no size impact.
**Estimate:** 4–5 d. **Risks:** OSK width for extra keys in Full; AltGr on 60% boards via Fn; test effort (emulator scripts per layout).
