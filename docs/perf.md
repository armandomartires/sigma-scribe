# Performance — S7 (v0.7.0)

Target (S7 DoD): keystroke-to-ink < 150 ms at p95 on the Kobo Forma.

## Desktop measurement (2026-10-21)
`sigma-emu` release build, Forma metrics 1440×1920, `perf-type 300` (300 on-screen key taps; each sample = event handling + layout + draw + refresh-region computation, the same path `run()` times on the device):

| machine | p50 | p95 | max |
|---|---|---|---|
| i5-6300U (desktop) | 16.1–16.3 ms | 20.0–20.6 ms | 63.7–69.3 ms |

## On the Forma (not yet measured)
The Forma could not be used in S7 (no device access). The app now records every key event's handling time and, on exit, writes `perf: n=… p50=… p95=… max=…` to `.adds/sigma-scribe/sigma-scribe.log`. The next device session (v0.7 checklist) gives the real numbers. Caveats: the Forma's single Cortex-A9 class CPU is several times slower than the desktop, so the desktop p95 of ~20 ms could become ~100–200 ms; and the logged time ends when FBInk is asked to refresh, so the panel waveform (~fast-mode update) adds on top. If p95 ≥ 150 ms, the first optimisation is to redraw only the edited line band instead of the full canvas before diffing.
