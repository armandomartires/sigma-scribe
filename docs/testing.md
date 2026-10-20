# Testing strategy

1. **Unit (desktop):** `cargo test` on `core` — buffer ops, keyboard mapping, Markdown parser (every syntax in brief §4), wiki links/backlinks, todo line rewrite, find/replace, file rules (last note, undeletable scratch), Line-mode fitting with a fake `TextMeasure`, settings round-trip.
2. **Property/fuzz:** parser and buffer ops never panic; edit sequences round-trip.
3. **Simulator:** `kobo dev` at Forma metrics (1440×1920, 1920×1440). Compare with `mockup/kobo-writing-mock.html` scaled ×1.139. Layout tests check tappability (≥44 px design ≈ 50 px Forma) and stable layout after state changes.
4. **CI:** fmt, clippy -D warnings, tests, ARMv7 static build on every push; Store dry-run (`node tools/app-contribute.mjs --dry-run`) from S9.
5. **On-device (Kobo Forma, fw 4.38.23697):** per-sprint checklist in `sprints/Sx-*.md`. Always also: exit to Nickel, reboot to Nickel, notes visible over USB.
6. **Performance budget:** keystroke-to-ink < 150 ms p95; open 100 KB note < 1 s; parsing never blocks input; RSS well under 512 MB.
7. **Robustness:** kill app mid-save, force power-off during save (×10), low storage → no corruption (atomic tmp+rename+fsync).
