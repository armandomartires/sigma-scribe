# sigma-scribe

A distraction-free Markdown notebook for the **Kobo Forma** — native Rust, drawn with [FBInk](https://github.com/NiLuJe/FBInk), launched from [NickelMenu](https://github.com/pgaskin/NickelMenu). Notes are plain `.md` files you can reach over USB.

Licence: **AGPL-3.0** (see `LICENSE`) · Author: **Sigma Recon** · Fonts: Literata (Regular, SemiBold, Italic), Atkinson Hyperlegible (SIL OFL 1.1, `assets/fonts/`)

Status: **v0.2.3 passed on the Forma (editor, keyboards, touch selection, library). v0.3.0 (Markdown preview) built; awaiting the on-device check.**

## Run it on your desktop
```sh
scripts/sim.sh            # live window, 1440x1920 scaled to fit
scripts/sim.sh --eink     # with e-ink flash/ghosting look
scripts/sim.sh --headless tests/emu/v0.1-editor.script docs/screenshots   # PNGs, no window
cargo test --workspace
```
Window controls: mouse = touch, PageUp/PageDown = page-turn buttons, keyboard = USB keyboard, Esc = power (save & quit).

## Build for the Kobo
`scripts/package-kobo.sh` → `dist/sigma-scribe-<ver>-kobo/` (`.adds/` tree). Install: `docs/install-plan-forma.md`.

## Docs
`plan.md` · `design-brief.md` · `mockup/` · `sprints/` · `tasks/` · `docs/testing.md` · `docs/deployment.md` · `docs/install-plan-forma.md` · `docs/publishing.md` · `docs/s0-findings.md`
