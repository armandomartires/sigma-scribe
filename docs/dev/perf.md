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

About 0.65 ms per KB, linear, on the desktop.

On the Forma (2026-10-28, build 5b976fe, Settings → Logging at Debug, one preview of each note, `perf-100k.md` and `perf-500k.md` from `sigma-scribe-backups/perf-notes/`):

| note | desktop | Forma | Forma ÷ desktop |
|---|---|---|---|
| 100 KB (102,700 B) | 60 ms | 929 ms | 15× |
| 500 KB (512,236 B) | 337 ms | 1,905 ms | 5.7× |

Not linear on the device: the first preview of the session (100 KB) cost more per KB than the second (500 KB, about 3.7 ms per KB), so part of the 929 ms is one-off warm-up (fonts and caches on first use), not the note. A note of ordinary size (under 20 KB) enters preview in well under 0.5 s. A 500 KB note takes about 2 s with no feedback; if that matters, the next step is to lay out the first screen first and the rest in the background. Not needed for this release.

Each preview entry logs `preview layout of <note> (<bytes> bytes): <n> ms` at Debug level. Run the desktop table again with:

    cargo test --release -p sigma-scribe --test preview_perf -- --ignored --nocapture

# Search across all notes — S22.1 (1.19.1, 2026-11-02)

Gate (`.ai/sprints/S22-fixes-and-search.md`): a three-letter query over the test-notes pack answers in under 2 s on the Forma, and typing stays responsive while it runs. The search runs on a background thread (`sigma_core::search::Searcher`); the Find bar sends a request on every keystroke and the newest one wins.

On the Forma (2026-11-02, build 1.19.1 at `9f5b0d3`, Settings → Logging at Debug, 303 notes in the library including the S10.1 test-notes pack, cap 500 matches). Each answered search logs `search: <n> chars, <a> of <a> notes read, <h> hits in <m> notes: <t> ms`, or, when the 500-hit cap stopped the scan early, `search: <n> chars, <a> notes read, <h> hits in <m> notes (capped): <t> ms` (no "of": the notes left are not counted); the query itself is never logged.

| query | notes read | hits | time |
|---|---|---|---|
| 1 letter | 2–3 (capped at 500 hits) | 500 | 7–43 ms |
| 2 letters | 303 (all) | 1–3 | 40–43 ms |
| **3 letters** | **303 (all)** | **2** | **37 ms** |
| 4 letters | 303 (all) | 1 | 40–43 ms |
| 5 letters | 303 (all) | 1 | 49–80 ms |

A whole-library read is about 40 ms, around 0.13 ms per note, so the result is on screen before the next keystroke; the 80 ms is the first search of the session (cold file cache). A one-letter query stops after two or three notes because it fills the cap. Keystroke-to-refresh while searching, from the session's `perf:` line: 19 keystrokes, p50 84 ms, p95 155 ms, max 176 ms (the list redraw is included). The gate is met by a factor of about 50; a stored index (decision S22-D4) is not needed at this library size. The numbers that would change it: a library ten times larger, or notes much larger than the pack's.

# Wiki-link autocomplete — S23 (1.20.0, 2026-11-02)

Gate (`.ai/sprints/S23-wiki-autocomplete.md`): typing with the suggestion bar up keeps keystroke-to-refresh near the 116 ms p50 measured on the Forma on 2026-11-02 (1.18.1).

The bar lists the library's notes by name with `Library::note_paths`, which walks the folders without reading any note (`all_notes` reads each one to count words). The list is read once when the bar opens and kept while it stays up; each keystroke then runs `suggest_notes` over it. A heading query reads the one note it names, once.

Desktop (2026-11-02, release build, 2000 notes of about 3 KB in 40 folders; `cargo test -p sigma-core --release -- --ignored --nocapture suggest_timing`):

| step | time |
|---|---|
| `all_notes` (for comparison) | 44 ms |
| `note_paths`, once when the bar opens | 7.5 ms |
| `suggest_notes`, per keystroke | 1.4 ms |

The first version folded every name through the accent table on each keystroke: 24.6 ms per call, roughly 200 ms on the Forma at its usual 8 to 10 times. ASCII letters now take a direct path, and the cached list is moved from bar to bar instead of copied.

Emulator, test-notes pack (16 notes), 300 keystrokes, release build: p50 19.7 ms without the bar, 20.2 ms with `[[` typed first (`perf-type`). The typed letters stop matching any note after a few keys, so most of those keystrokes ran the lookup and found nothing; the bar was up for the first few only.

On the Forma: not yet measured. To be filled in from the `perf:` line after the device pass, with the bar up.
