# S0 findings (desktop part) — 2026-10-20

Device work (Cobalt install, doctor, attended tests) is **not done yet**: it needs Armando holding the Forma.

## Backup (S0-T01) — done
- `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-20/` (46 MB, 62 files), copied from the read-only mount.
- sha256 manifests `2026-10-20.device.sha256` / `2026-10-20.copy.sha256` identical → **BACKUP OK**.
- `KoboReader.sqlite` integrity_check = ok (Python sqlite3, read-only).

## Toolchain (S0-T02) — done (no sudo used)
- Rust 1.99.1 (existing rustup) + targets `armv7-unknown-linux-musleabihf`, `armv7-unknown-linux-gnueabihf`; clippy, rustfmt.
- Node v20.20.1 (existing fnm).
- Cobalt cloned to `~/src/Cobalt` @ `94f6920` (v0.3.27); `kobo` CLI 0.3.27 installed with `cargo install --path crates/kobo-cli` into `~/.cargo/bin` (it replaced a `kobo` 0.3.27 previously installed from `~/work/sigmarecon/Cobalt`).
- ARM C cross-compiler: `apt install gcc-arm-linux-gnueabihf` needs sudo (no password available) → used the user-space musl.cc toolchain at `~/.local/toolchains/armv7l-linux-musleabihf-cross/`. Build with `scripts/build-armv7.sh`.
- `gh` 2.102.0 in `~/.local/bin`, **not logged in**.
- Static ARMv7 build of v0.1: OK (ELF 32-bit ARM EABI5, statically linked, ~1.9 MB).

## What the Cobalt SDK can and cannot do for this app (key finding)
From `SDK.md` and the sources at v0.3.27:
- "There is **no free-form drawing, no colour, no font choice and no pixel positioning**." Layout comes from physical dimensions.
- No multi-line text field. The only grid with a caret is `terminal(rows, Caret)` (monospace, `FontSize::Terminal` 1.8 mm em, block caret), sized by `terminal_grid_for`.
- Keyboards are app-built from `grid` cells, addressed positionally — works well.
- Page-turn keys arrive as `on_page_turn(forward)` — good.
- Store: keys `[a-z0-9._-]` only, ≤256 keys per app, ≤256 KiB per value; app never sees a path. Notes are therefore **not** plain files on the USB volume; export needs `kobo_sdk::exports::Export` / `kobo export`.
- No raw `/dev/input` for apps (USB-OTG keyboard needs a runtime capability).
- No Forma simulator profile; profiles live in `crates/kobo-profile` (adding one = Cobalt porting work, docs/PORTING.md). Screenshots use `KOBO_SIM_PROFILE=libra-2-388` (same 3:4, Mk7); layout tests use our own `FORMA_METRICS` (1440×1920 @ 300 ppi).

### Impact on the design brief
| Brief requirement | Cobalt v0.3.27 |
|---|---|
| Literata / Atkinson, size/leading/margin, words-per-line | ✗ (one system face; monospace grid for the page) |
| Night mode (inverted page) | ✗ (no colour/inversion control) |
| Drawer overlay at 48 % / 320 px, gutter | ✗ (popover/modal only, no pixel layout) |
| Tap-to-place caret in text | ✗ (terminal grid has no hit test; v0.1 uses ← → ↑ ↓ keys) |
| OSK incl. layers, Split/Full landscape | ✓ (grid cells) |
| Markdown preview | ~ (`rich_text`, headings, tables, checklist rows) |
| Notes as .md files over USB | ✗ direct; ~ via Export |
| USB-OTG keyboard | ✗ without new capability |

**Recommendation for the S0 go/no-go:** Cobalt can ship a functional v0.1–v0.3 (editor, files, preview) but cannot meet the brief's typography, theming, drawer and file rules. Plan B (native Rust + FBInk + NickelMenu) can. The `sigma-core` crate is already platform-free, so switching costs only the screens layer. Decision needed from Armando before S2.

## v0.1 in the simulator — milestone reached
- Editor (buffer, caret, soft wrap, page-key scroll, autosave to store) + on-screen keyboard (letters, Shift one-shot, 123 symbol layer, Tab/Del/Shift+Del/Space/Return), status bar (words · line · Paper · saved), Keys toggle.
- Autosave survives a simulator restart (text typed in one run present in the next).
- Tests: 23 core + 5 ui unit tests; drive script `tests/drive/v0.1-editor.kobo` passes; screenshots in `docs/screenshots/`.

## 2026-10-20 22:14 CEST — Cobalt installer blocks the Forma → Plan B
`kobo setup --dry-run --volume /media/amartires/KOBOeReader` → `unsupported Kobo hardware: model code N782 has no reviewed Cobalt profile` (hard check, no override; PORTING.md requires maintainer review and attended evidence). Together with the SDK limits above, Armando chose **Plan B: native Rust + FBInk + NickelMenu**. Cobalt layers removed from the repo; `sigma-core` kept unchanged.

## Plan B bring-up (desktop)
- FBInk `886f25f1` (v1.25.0-543) built as `Release/libfbink.a` with `make staticlib MINIMAL=true IMAGE=true KOBO=true CROSS_TC=armv7l-linux-musleabihf`; linked through a small C shim (`crates/device/csrc/sg_fbink.c`).
- `sigma-scribe` ARMv7: static, stripped, FBInk symbols present.
- NickelMenu v0.6.0 `KoboRoot.tgz` downloaded to `~/src/nickelmenu-v0.6.0/` (contents: `usr/local/Kobo/imageformats/libnm.so`, `mnt/onboard/.adds/nm/doc`).
- Backup note: the earlier read-only SQLite check had created `KoboReader.sqlite-shm/-wal` in the backup; removed, backup re-verified identical to the device manifest.
