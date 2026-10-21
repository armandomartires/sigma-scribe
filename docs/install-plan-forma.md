# Install plan — NickelMenu + Sigma Scribe v0.1 on the Kobo Forma

**Status:** PLAN ONLY — nothing has been written to the Kobo. Run it with Armando holding the device.
Device: Kobo Forma N782 (code 377), serial <FORMA-SERIAL>, firmware **4.38.23697**. Backup: `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-20/` (verified against `2026-10-20.device.sha256`).

## What gets written
| Path on the Kobo | From | Purpose |
|---|---|---|
| `.kobo/KoboRoot.tgz` (consumed and deleted by the Kobo at boot) | NickelMenu v0.6.0 release, sha256 `322ff9aa863860e8f5f7e0b55cae561c54bf95983b9bce1d19819d1225d064af` | installs `/usr/local/Kobo/imageformats/libnm.so` + `.adds/nm/doc` |
| `.adds/nm/sigma-scribe` | `packaging/kobo` | menu entry: `menu_item:main:Sigma Scribe:cmd_spawn:quiet:exec /mnt/onboard/.adds/sigma-scribe/launch.sh` |
| `.adds/sigma-scribe/` (`sigma-scribe`, `launch.sh`, `touch.conf`, `LICENSE`, `OFL-*.txt`) | `scripts/package-kobo.sh` → `dist/sigma-scribe-0.1.0-kobo/` | static ARMv7 binary (FBInk inside) + launcher |
| `sigma-scribe/notes/field-notes.md` (created by the app) | app | your note, plain Markdown |

Nothing else changes: no firmware, no books, no `KoboReader.sqlite` edits. `launch.sh` stops Nickel while the app runs and **reboots** to the stock reader when you leave (power button) or if the app crashes.

## 1. Before you start
1. Charge the Forma above 50 % (the Kobo's installer silently skips `KoboRoot.tgz` on low battery).
2. Keep firmware at 4.38: don't accept a Kobo software update (NickelMenu does not support 5.x).
3. On the PC: `./scripts/package-kobo.sh` and check `dist/sigma-scribe-0.1.0-kobo.sha256`.
4. `sha256sum ~/src/nickelmenu-v0.6.0/KoboRoot.tgz` = `322ff9aa…d064af`.
5. Re-verify the backup: `cd ~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-20 && find . -type f -print0 | sort -z | xargs -0 sha256sum | diff - ../2026-10-20.device.sha256 && echo OK`.

## 2. Copy (PC, Kobo connected and "Connect" tapped)
```sh
udisksctl unmount -b /dev/sdc                 # drop the read-only mount
udisksctl mount -b /dev/sdc                   # read-write at /media/amartires/KOBOeReader
K=/media/amartires/KOBOeReader
grep -q '^N782' "$K/.kobo/version"            # still the Forma
test ! -e "$K/.kobo/KoboRoot.tgz"             # nothing else pending
cp ~/src/nickelmenu-v0.6.0/KoboRoot.tgz "$K/.kobo/KoboRoot.tgz"
cp -r dist/sigma-scribe-0.1.0-kobo/.adds/. "$K/.adds/"
( cd "$K" && sha256sum .kobo/KoboRoot.tgz .adds/nm/sigma-scribe .adds/sigma-scribe/* )   # compare with dist/*.sha256
sync
udisksctl unmount -b /dev/sdc && udisksctl power-off -b /dev/sdc
```
Unplug the cable.

## 3. First boot
The Forma shows "Updating…" (installs NickelMenu) and restarts. **Leave it on the home screen for a minute**: NickelMenu disables itself if the reader restarts again too soon (boot-loop guard).

## 4. Rollback / uninstall
| Want | Do |
|---|---|
| Leave the app | Power button (saves, then the Kobo reboots to the stock reader). A hard power-off (hold power ~10 s) also returns to stock. |
| Remove the app | USB: delete `.adds/sigma-scribe/` and `.adds/nm/sigma-scribe`. Notes stay in `sigma-scribe/notes/`. |
| Remove NickelMenu | Create an empty file `.adds/nm/uninstall` and reboot (or power off right after boot starts: NickelMenu failsafe). |
| Menu entry gone | NickelMenu failsafe tripped (`libnm.so.failsafe`): reinstall NickelMenu's `KoboRoot.tgz`, boot, wait a minute. |
| Anything else | Restore files from the backup; as a last resort Kobo's own factory reset (keeps firmware). |
| Logs | `.adds/sigma-scribe/sigma-scribe.log` over USB. |

## 5. On-device checklist (v0.1 / S0 gate)
| # | Do | Expect | Result |
|---|---|---|---|
| 1 | After the update reboot, look at the home screen | Normal Kobo home; books intact | |
| 2 | Open the main menu (☰, top left on 4.38) | New entry **Sigma Scribe** (NickelMenu) | |
| 3 | Tap **Sigma Scribe** | Within ~5 s a full flash, then the page: title `field-notes.md`, seeded note, status `… words · line N   Paper · saved  [Keys]`, keyboard at the bottom | |
| 4 | Display: compare with `docs/screenshots/v0.1-start.png` | Same layout, crisp Literata text, no rotated or mirrored image | |
| 5 | Touch accuracy: tap `q`, `p`, `Shift`, `Return`, `Space`, and the middle of the page | Each key types what it shows; page tap moves the caret there. If taps land mirrored/rotated, note where, edit `.adds/sigma-scribe/touch.conf` (try `swap,mirror_y`, `swap`, `mirror_x`) and retest | |
| 6 | Type a sentence quickly (~20 keys) | No missed or doubled keys; status returns to `saved` | |
| 7 | Ghosting/refresh while typing | Only the edited line and status update; light ghosting acceptable; note latency feel (target < 150 ms) | |
| 8 | Tap **Keys** twice | Keyboard hides (page grows), then returns | |
| 9 | `123`, then a symbol, then `ABC` | Symbol layer shows `1–0 # * - [ ] ( ) \` / . , ! ? : \| ~ >`; symbol typed | |
| 10 | Page-turn buttons (both) on a long note | Upper = back, lower = forward; page scrolls by a screen | |
| 11 | Rotate the device to landscape | v0.1 stays portrait (rotation is S4); no crash | |
| 12 | Press the power button | Kobo reboots into the stock reader | |
| 13 | Connect USB, open `sigma-scribe/notes/field-notes.md` on the PC | Your typed text is there | |
| 14 | Launch again | The note opens with your text | |
| 15 | Recovery: while the app runs, hold power ~10 s, then boot | Stock reader starts normally | |
| 16 | Copy `.adds/sigma-scribe/sigma-scribe.log` to the PC | Start/exit lines, no errors | |

### S0 go/no-go (Plan B)
- **GO** if 1–4, 5 (with a working `touch.conf`), 6, 10, 12–15 pass.
- **Fix then GO** if only latency/ghosting (7) or rotation (11) need work — tune waveform/regions in S1/S4.
- **NO-GO / rethink** if the panel stays blank or garbled, input cannot be read at all, the device fails to return to the stock reader, or any data on the Kobo is lost.

## Upgrade 0.1.0 → 0.2.0 (not run; needs approval)
NickelMenu is already installed, so only the binary changes. Host side: mount the Forma read-write, replace `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.2.0-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `38e6955f03f1f1a100dde8dfbbea11f31433cf831afa7d5469f8156c6ec74b31`), verify the checksum, sync, unmount, eject. Nothing under `sigma-scribe/notes` is touched; on first launch 0.2.0 keeps the existing `field-notes.md` and does not re-seed. Rollback: copy the 0.1.0 binary back (`dist/sigma-scribe-0.1.0-kobo`). On-device checklist: `tasks/S2-tasks.md` S2-T09.

## Upgrade 0.2.0 → 0.2.1 (not run; needs approval)
Same procedure as 0.1.0 → 0.2.0: back up `sigma-scribe/notes` and `.adds/sigma-scribe`, replace only `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.2.1-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `a429e7cc63adcb58f136c0e5da16454fd83b1680aea73e03c54330f9ddec29a2`); every other package file is identical to 0.2.0. Checklist: `sprints/S2.5-full-keyboard.md` (On-device).

## Upgrade 0.2.1 → 0.2.2 (not run; needs approval)
Same procedure: back up `sigma-scribe/notes` and `.adds/sigma-scribe`, replace only `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.2.2-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `33dc4c5a1fb6b12703533ce9498c3dc568ddeca36e111f686e86e452ba3f385e`); all other files are identical to 0.2.1. Checklist: `sprints/S2.6-touch-selection.md`.

## v0.2.2 skipped → upgrade 0.2.1 → 0.2.3
0.2.2 was never installed: the v0.2.1 log showed FBInk refusing a 1×34 refresh region and the app exiting with code 1 (2026-10-21 00:25 CEST), and 0.2.2 shares that code. 0.2.3 = 0.2.2 + the fix. Same procedure; replace only `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.2.3-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `c6631dc3a13233132d729dd9db3676458c4afcb7b3105c2751b8e00d7afeefe6`). Checklist: `sprints/S2.6-touch-selection.md`.

## Upgrade 0.2.3 → 0.3.0 (APPROVED 2026-10-21 01:35 CEST)
Same procedure as 0.2.3: read the log first, back up, then replace only `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.3.0-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `d86b3d5a9abefb95d55b6ac124ffbb307539744dcadce848615376168ba18716`). The new fonts are embedded in the binary. Checklist: `sprints/S3-markdown-preview.md`.

## Upgrade 0.3.0 → 0.4.0 (APPROVED and RUN 2026-10-21 08:35 CEST)
Run log: Forma found as `/dev/sdc` (serial <FORMA-SERIAL>, fw 4.38.23697), mounted at `/media/amartires/KOBOeReader`; no other disk touched. Device log read first (read-only): the v0.3.0 session (02:55–02:57 CEST) ended with exit 0; the only error in the log is the old v0.2.1 refresh failure (00:25, fixed in 0.2.3). Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-21-pre-v0.4.0` (`.adds/sigma-scribe` + `notes`, 10 files) matches the device checksums (`SHA256SUMS`). Only `.adds/sigma-scribe/sigma-scribe` replaced (copy to `.new`, sync, rename); device copy sha256 `689b84c4…2043` ✅ (old `d86b3d5a…8716`). Synced, unmounted and powered off with udisksctl. On-device checklist below: ✅ passed 2026-10-21 (landscape direction confirmed); tagged `v0.4.0`.

Same procedure as 0.3.0: confirm it is the same Forma (serial <FORMA-SERIAL>), read `.adds/sigma-scribe/sigma-scribe.log` read-only, make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-0.4.0`, then replace **only** `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.4.0-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `689b84c4e763119cbdfa19eb07ade52d5709cdf034bf971fc2e5116e97c02043`), verify the sha256 on the device copy, sync and eject cleanly. `launch.sh`, `touch.conf`, the NickelMenu entry and the licences are unchanged from 0.3.0 (same checksums). The settings file `notes/.sigma-settings` from 0.2.1+ is read as-is (Keyboard choice kept); new settings start at the brief's defaults.

Rollback: copy the 0.3.0 binary (sha256 `d86b3d5a9abefb95d55b6ac124ffbb307539744dcadce848615376168ba18716`) back; an 0.4 settings file is harmless to 0.3.0 (it only reads `keyboard=`).

On-device checklist (`sprints/S4-options-line-mode.md`):
- `Aa` opens Options; every row changes and stays after leaving the app and relaunching from NickelMenu.
- Night: white ink on a black page, no grey card, one full flash on switch; Files, preview and Options follow it; Keyboard tone Light/Dark works.
- Line on: about N words per line (8–16); Size moves Margin and Margin moves Size; the count holds after Orientation → Landscape (column locked, gutters).
- Landscape: picture upright when held sideways (note which way round), taps land where touched, Split / Full / Centre and the number/arrow pads type correctly, Pad left/right moves the pad; back to Portrait.
- Tab key: Spaces 2/4/8 and Tab.
- No regressions: typing, selection, Files, preview; the log shows exit 0 and no refresh failures.


## Upgrade 0.4.0 → 0.5.0 (PENDING APPROVAL — not run)
Same procedure as 0.4.0: confirm it is the same Forma (serial <FORMA-SERIAL>) and touch no other disk; read `.adds/sigma-scribe/sigma-scribe.log` read-only; make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v0.5.0`; replace **only** `.adds/sigma-scribe/sigma-scribe` with `dist/sigma-scribe-0.5.0-kobo/.adds/sigma-scribe/sigma-scribe` (sha256 `1ada5ae93504d287bb685e15e9cb3a15288cb65073dc233b220cd4293ec01f8f`) via `.new` + sync + rename; verify the sha256 of the device copy; sync, unmount and power off with udisksctl. `launch.sh`, `touch.conf`, the NickelMenu entry and the licences are unchanged (same checksums). 0.5 creates `notes/.scratchpad.md` on first use of Scratch.

Rollback: copy the 0.4.0 binary back (sha256 `689b84c4e763119cbdfa19eb07ade52d5709cdf034bf971fc2e5116e97c02043`); `.scratchpad.md` is ignored by 0.4.0 (hidden file) and can stay.

On-device checklist (`sprints/S5-side-drawer.md`):
- The three drawer icons open/close their panes in any combination; panes share the height; Options → Drawer Left/Right moves the drawer.
- Contents lists the headings; tapping one jumps the caret there.
- To-dos lists boxes from all notes; filter All/Open/Done; checking a box in another note changes only that line (check the file in Files/preview); tapping the text opens that note at the line.
- Scratch: type into it, switch notes, leave and relaunch from NickelMenu — the text is still there; it is not in Files.
- Landscape and Night with the drawer open; no ghosting after closing panes.
- No regressions; the log shows exit 0 and no refresh failures.
