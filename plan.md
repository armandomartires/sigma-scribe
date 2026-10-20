# Plan — Sigma Scribe (alt): a Markdown notebook on the Kobo Forma

**Status:** ✅ **APPROVED** 2026-10-20 · target **Kobo Forma** · repo name `sigma-scribe` · development not started (next: Sprint 0).
**Inputs:** `design-brief.md` (behaviour source of truth), `mockup/kobo-writing-mock.html` (look source of truth, self-contained React mock, ~735 KB with embedded fonts).

---

## 1. What the app is

A distraction-free Markdown/plain-text notebook for an e-reader. "The page is the product."
Core features (from the brief):

- Single-page editor (write mode) with autosave, caret/scroll preservation, tab-as-spaces, on-screen keyboard (portrait compact board; landscape Centre / Split / Full with num/arrow pad; symbol layer; keyboard tone).
- Markdown preview (headings, emphasis, lists, task lists, quotes, rules, code, images, links, tables, strike, `[[wiki]]` links, backlinks).
- Library: folders + `.md`/`.txt` notes; create / rename / move / armed delete; seed content on first run.
- Options: Day/Night, Literata/Atkinson, Size/Leading/Margin steppers, **Line** mode (fit N words per line, locked 1264-px column + gutter in landscape), Tab, Keyboard tone, Drawer side, USB-keyboard flag.
- Side drawer (left/right, ≤48% / 320 px, no animation): Contents, To-dos (all files, All/Open/Done filter, toggles rewrite the file), Scratch (persistent hidden `.scratchpad.md`).
- Sheets: Files, Graph (circle map or link list), Options, Help (App info + Markdown reference).
- Find & replace in current note (count, prev/next, replace/all, match case).
- Page-turn buttons scroll the page. Brief specifies 1264×1680 / 1680×1264 (Libra); **on the Forma this becomes 1440×1920 portrait / 1920×1440 landscape** (same 3:4 ratio, so the mock scales 1.139×).

## 2. Current state

- Folder contains only the brief and the HTML mock. **No code, no repo (not a git repo), no build config, no tests.**
- The mock is a browser prototype; the brief says *do not port the React code* — port layout, tokens, interactions and data rules.

## 3. Target platform & constraints

### The actual device (read-only probe, 2026-10-20 21:12 CEST)

| Field | Value |
|---|---|
| Model | **Kobo Forma** (device code 377, serial prefix N782, codename *frost*, Mark 7 / i.MX6SLL) |
| Serial | <FORMA-SERIAL> |
| Firmware | **4.38.23697** (kernel 4.1.15 family) — 4.x, *not* 5.x |
| User storage | `/media/amartires/KOBOeReader`, vfat, 6.8 GB, 6.7 GB free (8 GB model) |
| Existing mods | `.adds/nickeldbus` (NickelDBus 0.2.0) only. **No NickelMenu, no KOReader, no Cobalt**, no pending `KoboRoot.tgz`. `.kobo/ssh-disabled` present |

### Hardware

- 8" 1440×1920 Carta 1200 (300 ppi), portrait and landscape (gyro rotation).
- i.MX6 SLL ~1 GHz single core, 512 MB RAM. Capacitive touch, **two physical page-turn buttons**.
- Wi-Fi only. **No Bluetooth** → no BT keyboard; external keyboard only via USB OTG with a powered hub / Y-cable (the Forma does not supply VBUS) and custom `/dev/input/event*` reading.
- No stylus. Greyscale only → **Libra Colour / Kaleido work dropped**.

### Constraints
E-ink (partial refresh for typing, full for theme/orientation/sheets, no animation), offline first, stock Nickel must stay intact, fast UI callbacks with parsing off the UI thread. Do **not** upgrade the Forma to firmware 5.x (Cobalt and NickelMenu-hook generation are 4.x).

### Platform verdict: Cobalt on the Forma

- Cobalt's device support matrix (docs/DEVICES.md, https://github.com/BandarLabs/Cobalt/blob/main/docs/DEVICES.md) lists Clara BW/Colour/HD, Elipsa 2E, Libra 2, Libra Colour, Libra H2O. **The Forma is not listed, and no Forma issue exists** in the repo.
- It is not blocked either: unlisted devices "still start, after the owner has been told what is untested and has agreed" (provisional profile in `crates/kobo-profile/src/provisional.rs`). Firmware 4.38.23697 is a branch already tested on the Clara HD, Libra 2 and Libra H2O, which share the Forma's Mark 7 / kernel 4.1.15 driver generation. That makes it plausible but **unproven**: touch transform, page-button codes, rotation and refresh have never been measured on a Forma. Install docs: https://github.com/BandarLabs/Cobalt/blob/main/docs/INSTALL.md (USB `.kobo/KoboRoot.tgz` install, NickelMenu as launcher).
- **Decision: keep Cobalt as Plan A, gated by a Sprint 0 go/no-go spike** (read-only doctor + attended display/touch/page-key/exit/recovery tests). If it fails or is too flaky, use Plan B.
- **Plan B (fallback): a native Rust app on FBInk** (FBInk has full Forma support, incl. rotation and waveform control), launched from **NickelMenu**, which stops/hands back Nickel the way KOReader does. Same `core` crate; only the platform layer changes (framebuffer drawing + font rasterising via FBInk/`fontdue`, raw evdev touch + keys). More work (own widget/refresh layer), but no dependency on an untested profile.
- Plan C (prototype only): a KOReader plugin in Lua. KOReader is mature on the Forma, but it would mean rewriting `core` in Lua. Not recommended except for quick keyboard experiments.

## 4. Architecture / approach

**Platform: Cobalt** (BandarLabs, https://bandarlabs.github.io/Cobalt/) — Rust SDK + launcher + sandboxed per-app process, opened from Kobo's menu, reboot returns to Nickel. The Forma is **not** on Cobalt's tested list (see §3 verdict). Cobalt is Plan A behind a Sprint 0 gate; Plan B is native Rust + FBInk + NickelMenu, using the same `core` crate.

Proposed crate layout (one Cargo workspace):

```
sigma-scribe/
  crates/
    core/      # pure Rust, no Cobalt deps: library model, markdown parser→AST,
               # wiki-link index/backlinks, todo extraction, find/replace,
               # Line-mode fitting math (takes a text-measure trait), settings
    ui/        # Cobalt ScreenBuilder screens: title bar, page, status, keyboard,
               # drawer, sheets; refresh policy (partial vs full)
    app/       # Cobalt entry point, lifecycle callbacks, store()/shelf() persistence,
               # background worker for parsing, input (touch, page keys)
  assets/fonts # Literata, Atkinson Hyperlegible (OFL) — check licences/bundling
  tests/       # golden screenshots, fixtures (seed content)
```

Key design decisions:
- **Core is platform-agnostic** → fully unit-testable on desktop with `cargo test`; the Cobalt layer is thin.
- **Storage:** notes as real `.md`/`.txt` files in an app data folder (enables USB export), settings via Cobalt `store()`. Atomic write (tmp + rename) for autosave; "saved" shown after fsync.
- **Text measurement** via Cobalt measured-text primitives behind a trait so Line mode is tested with a fake measurer on desktop.
- **Refresh policy module**: centralised rules (keystroke = partial/fast waveform of dirty rect; sheet/theme/orientation = full).
- Design tokens (colours, type steps) as constants mirroring §2 of the brief.

Platform layer is kept behind a small trait (`Display`, `Input`, `Store`, `TextMeasure`) so the Plan B (FBInk) backend can replace Cobalt without touching `core`/screen logic.

Resolution: all layout tokens (44 px targets, 20 px body, margins, 320 px drawer, Line-mode locked column) are defined for 1264-wide and scaled by 1440/1264 ≈ 1.139 on the Forma; Line mode locks the column to 1440/1920 of the landscape width.

## 5. Toolchain

- Rust stable + Cobalt CLI (`kobo new`, `kobo dev` simulator — needs a custom 1440×1920 Forma screen profile, `kobo doctor`/probe, `kobo package` → `KoboRoot.tgz`). Exact install steps to be confirmed from Cobalt docs/INSTALL.md in Sprint 0.
- Plan B extras: FBInk (static lib), NickelMenu config entry, `armv7` cross toolchain (e.g. `cross` or the koxtoolchain `kobo` target).
- Cross-target: ARMv7 hard-float (`armv7-unknown-linux-gnueabihf` or musl, whichever Cobalt templates use).
- Cobalt on device: one-time USB install (`install.sh` / browser installer) → restart → open Cobalt from Kobo menu.
- Dev loop extras: `cargo fmt`, `clippy`, `cargo test`, git repo + CI (GitHub Actions: test + cross build) — to be set up in Sprint 0.
- Optional: SSH/Terminal on device (Cobalt Terminal app) for logs.

## 6. Testing strategy

1. **Unit (desktop, `core`)**: markdown parser cases (each syntax in §4), wiki-links/backlinks, todo toggle rewrites exact line, find/replace (case, count, all), rename cleaning, move/delete rules (last note, scratch undeletable), Line-mode fitting, settings round-trip.
2. **Property/fuzz** on parser and editor ops (no panic, round-trip stability).
3. **Simulator (`kobo dev`)**: each build-order step compared side by side with the HTML mock at 1264×1680 and 1680×1264; golden screenshot tests where the simulator allows headless capture.
4. **Performance budget** on device: keystroke-to-ink < ~150 ms, open 100 KB note < 1 s, outline/todo parse never blocks input; memory ceiling well under 512 MB.
5. **On-device (Kobo Forma)**: manual checklist per sprint (touch targets, ghosting, refresh, page-turn keys, rotation, reboot persistence, USB export, return to Nickel).
6. **Persistence/robustness**: kill app mid-save, power off, low storage → no data loss.

## 7. Deployment & rollback

- **Dev:** install Cobalt once over USB; sideload dev builds per Cobalt's developer flow (USB copy or local network) — confirm method in Sprint 0.
- **Release:** package + sign Cobalt app; optionally submit to Cobalt Store (needs licence decision).
- **Backup first:** before any install, copy the Kobo's whole user partition (`/mnt/onboard`, incl. `.kobo/`) to the PC.
- **Rollback:** remove the app from Cobalt (Store/launcher); Cobalt itself is removable per its INSTALL.md recovery steps; a restart always returns to stock Nickel; Cobalt does not touch the boot chain. Notes live as plain files and are exported over USB before uninstall.

## 8. Risks & open questions

Risks
- Cobalt is young (2026); SDK APIs (ScreenBuilder, measured text, refresh control, raw input) may lack things we need → spike in Sprint 0.
- Text-editing widget with caret/selection on e-ink may not exist in the SDK → we may build our own editor buffer + caret rendering (largest effort).
- Partial-refresh latency/ghosting while typing.
- Forma is untested on Cobalt: touch mapping, page-button keycodes, rotation and refresh need to be measured (S0 gate). Night mode ghosting.
- No Bluetooth on the Forma; a USB-OTG keyboard needs a powered hub plus `/dev/input` access the Cobalt SDK does not expose (easy under Plan B).
- An accidental Kobo firmware update to 5.x would break NickelMenu/Cobalt: keep auto-update off / stay offline when possible.
- Font licences/bundling size; Markdown images (remote http fetch while offline-first).

### Decisions (recorded 2026-10-20)
1. Device: Kobo Forma, firmware 4.38.23697 (stay on 4.x).
2. Platform: **Cobalt (Plan A)** with a Sprint 0 go/no-go; fallback **native Rust + FBInk + NickelMenu (Plan B)**.
3. Installing Cobalt + NickelMenu on the Forma is approved **in S0, only after a verified full backup**.
4. Keyboard: on-screen **and** USB-OTG (powered hub/Y-cable). No Bluetooth (Forma has none).
5. Toolchain (Rust, Cobalt CLI) installed at S0 start. Repo: `sigma-scribe`.
6. Scope: v1.0 = all features in the brief, released incrementally (roadmap below).
7. Distribution: publish to the **Cobalt Store** (see §11, docs/publishing.md).

### Still open
- Licence (must combine with AGPL-3.0 Cobalt monorepo) and author name.
- Edit-in-preview; graph as map vs list; export path (USB folder / Syncthing).
- Whether Cobalt Store allows a raw-input (USB-OTG) capability (ask maintainers by S8).

## 9. Sprints (detail in `sprints/` and `tasks/`)

Each sprint ends with a simulator comparison against the mock and (from S1) an on-device check.

| # | Sprint | Goal | Rough tasks |
|---|---|---|---|
| S0 | Foundations & Forma gate | Toolchain proven; Cobalt go/no-go on Forma | Install Rust/Cobalt CLI; `kobo new`; git+CI; full backup of `/mnt/onboard`; Forma simulator profile 1440×1920; install Cobalt via USB `KoboRoot.tgz`; run read-only doctor; attended tests (display, touch transform, page buttons, rotation, partial refresh, exit to Nickel, recovery); spike text input/caret + measured text; **go/no-go → Cobalt or FBInk/NickelMenu**; optionally report results upstream as a Forma device thread |
| S1 | Editor core | Brief step 1 | Text buffer + caret/selection; page render with tokens; title bar + rename; status bar; on-screen keyboard (portrait + symbol layer); autosave (atomic) via store; page-turn scroll; seed content |
| S2 | Library | Step 2 | Folder/note model; Files sheet (enter, up, open, more → rename/move/armed delete, new note/folder, word counts); name cleaning; persistence tests |
| S3 | Preview | Step 3 | Markdown parser → AST (core) + renderer; eye/pencil toggle; links/images policy; tables, code, tasks |
| S4 | Options & Line mode | Steps 4–5 | Options sheet + all settings persisted; Day/Night with full refresh; fonts; Line fitting with real measure; landscape locked column + gutter; landscape keyboard Centre/Split/Full + pads |
| S5 | Drawer | Step 6 | Drawer left/right; Contents (jump); To-dos across files with filter + line rewrite; Scratch pad file; background parsing worker |
| S6 | Find & wiki | Steps 7–8 | Find/replace bar; wiki-link index, missing-note creation, backlinks, Graph (map or list per decision) |
| S7 | Help & hardening | Step 9 + quality | Help pages; perf profiling on the Forma; robustness tests (power loss); accessibility of targets; landscape/rotation pass |
| S8 | Keyboard & export (optional) | Step 10 | USB-OTG HID keyboard via powered hub (no Bluetooth on Forma); USB export folder / Syncthing doc |
| S9 | Release | Step 11 | Package, sign, install/rollback docs, final test on the Forma, optional Store submission (Cobalt path) or NickelMenu `KoboRoot.tgz` (FBInk path) |


## 10. Version roadmap

| Version | Sprint | Contents |
|---|---|---|
| — | S0 | Toolchain, backup, Cobalt on Forma, go/no-go (no release) |
| **v0.1.0** | S1 | Text editor + on-screen keyboard (portrait), autosave, title/status bars, page keys |
| v0.2.0 | S2 | Files: folders, create/rename/move/delete |
| v0.3.0 | S3 | Markdown preview |
| v0.4.0 | S4 | Options, Day/Night, typography, Line mode, landscape keyboards |
| v0.5.0 | S5 | Drawer: Contents, To-dos, Scratch |
| v0.6.0 | S6 | Find & replace, wiki links, backlinks, graph |
| v0.7.0 | S7 | Help pages, performance & robustness hardening |
| v0.8.0 | S8 | USB-OTG keyboard, export path |
| **v1.0.0** | S9 | Feature-complete, Store Stable release |

Each version goes to the Cobalt Store Beta channel once the Store submission is accepted (first submission targeted at v0.1.0 or v0.2.0, depending on reviewer feedback); v1.0.0 is promoted to Stable.

## 11. Cobalt Store publishing (summary; full: docs/publishing.md)
- Apps live in the BandarLabs/Cobalt monorepo under `apps/sigma-scribe/` (crate `kobo-sigma-scribe`) with a single `cobalt-app.json` manifest.
- `node tools/app-contribute.mjs --dry-run` validates, tests, cross-builds ARMv7 and test-signs locally.
- PR against `beta`; human review of purpose, licensing, capabilities, setup, screenshots. CI signs (Ed25519 project key) and publishes Beta; maintainers promote to Stable.
- Every release: strictly newer version + meaningful release notes.
- Needs from you: licence, author, display name/summary, screenshots OK.

