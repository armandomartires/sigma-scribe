# Performance — S7 (v0.7.0)

Target (S7 DoD): keystroke-to-ink < 150 ms at p95 on the Kobo Forma.

## Desktop measurement (2026-10-21)
`sigma-emu` release build, Forma metrics 1440×1920, `perf-type 300` (300 on-screen key taps; each sample = event handling + layout + draw + refresh-region computation, the same path `run()` times on the device):

| machine | p50 | p95 | max |
|---|---|---|---|
| i5-6300U (desktop) | 16.1–16.3 ms | 20.0–20.6 ms | 63.7–69.3 ms |

## On the Forma (not yet measured)
The Forma could not be used in S7 (no device access). The app now records every key event's handling time and, on exit, writes `perf: n=… p50=… p95=… max=…` to `.adds/sigma-scribe/sigma-scribe.log`. The next device session (v0.7 checklist) gives the real numbers. Caveats: the Forma's single Cortex-A9 class CPU is several times slower than the desktop, so the desktop p95 of ~20 ms could become ~100–200 ms; and the logged time ends when FBInk is asked to refresh, so the panel waveform (~fast-mode update) adds on top. If p95 ≥ 150 ms, the first optimisation is to redraw only the edited line band instead of the full canvas before diffing.

# Graph map layout — S9.1 (v1.0.0)

`perf-graph` in `tests/emu/v1.0-graph-map.script` (release build, i5-6300U, Forma metrics): **309 notes, 521 links → layout 21.4–21.7 ms, map draw ~5 ms** (empty cache). Cost is O(n²·160) for the repulsion, so it scales with the square of the linked notes.

Forma estimate (single Cortex-A9 class core, ~8–10× slower than this desktop as assumed for S7): layout ~0.17–0.22 s plus ~0.05 s drawing and the full refresh on open. The layout is computed only when the notes or links change (cache keyed by an FNV hash of the names and links) — reopening the Graph costs only the drawing. Decision: no background thread (S9.1-D3); if the device shows > 0.5 s at the user's library size, move `graphmap::layout` to a thread and show the list until it is ready. The real number comes from the device checklist.

# Preview layout time — S7.3 (2026-10-27)

Entering preview parses and lays out the whole note on the UI thread. `app/tests/preview_perf.rs` (release build, Forma metrics 1440×1920, notes of headings, lists, quotes, code, emphasis and wiki links) on the desktop:

| note | layout | pages |
|---|---|---|
| 10 KB | 6.8 ms | 8 |
| 50 KB | 30 ms | 37 |
| 100 KB | 60 ms | 73 |
| 250 KB | 163 ms | 181 |
| 500 KB | 337 ms | 361 |

About 0.65 ms per KB, linear. At the 8 to 10 times slowdown assumed for the Forma that is roughly 0.5 s for a 100 KB note and 3 s for 500 KB. **Not yet measured on the Forma.** With Settings → Logging on at Debug level, every preview entry logs `preview layout of <note> (<bytes> bytes): <n> ms`, so one preview of a large note on the device settles it. Run the desktop table again with:

    cargo test --release -p sigma-scribe --test preview_perf -- --ignored --nocapture
