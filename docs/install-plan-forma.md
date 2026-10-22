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
| 4 | Display: compare with `docs/screenshots/v0.1/v0.1-start.png` | Same layout, crisp Literata text, no rotated or mirrored image | |
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


## Upgrade 0.4.0 → 1.0.0 (APPROVED and RUN 2026-10-21 20:35 CEST — on-device checklist pending)
Run log: Forma found as `/dev/sdc` (USB "Linux File-Stor Gadget", serial <FORMA-SERIAL>, label KOBOeReader, `.kobo/version` fw 4.38.23697) and mounted alone with udisksctl at `/media/amartires/KOBOeReader`; sdb (Kingston DataTraveler 64 GB) and every other disk untouched. Device log read read-only (115 lines): sessions since 0.4.0 (08:49–08:53 CEST) ended exit 0; the only error is the old v0.2.1 `fbink refresh failed (-1)` / exit 1 at 00:25 (fixed in 0.2.3); **no `perf:` lines** (0.4.0 predates perf logging, added in 0.7). Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-21-pre-v1.0.0` (`.adds/sigma-scribe`, `.adds/nm/sigma-scribe`, `sigma-scribe/notes` — 11 files) verified against the device checksums (`SHA256SUMS`, also `../2026-10-21-pre-v1.0.0.device.sha256`). Current binary confirmed 0.4.0 (`689b84c4…2043`). CI binary from `dist/ci` (main run 39610899875) verified `bb38a083…dad1`, copied as `.new`, synced, renamed; **device copy sha256 `bb38a083b0925ced9adc61690430d0f768509a1cf284a0c533cf5839e570dad1` ✅**. `launch.sh`, `touch.conf` and `.adds/nm/sigma-scribe` match the package (unchanged). Synced, unmounted, powered off with udisksctl; `/dev/sdc` gone from lsblk. Fresh-install and uninstall tests NOT run (not approved).

The device runs **0.4.0**. **0.5.0, 0.6.0, 0.7.0, 0.8.0 and 0.8.1 were never installed or tested on the device.** This single section replaces all earlier pending sections: the upgrade goes straight from 0.4.0 to 1.0.0, so the checklists below cover v0.5 to v1.0. Passing it is the S9 "final regression" (S9-T01) and the precondition for any tag, release or post.

**Which package:** the **CI-built** release candidate (artifact `sigma-scribe-1.0.0-kobo` of the release workflow for the S9.2 title-bar PR (PR #10, run 39609247695); CI builds are reproducible, so the main-branch run after the merge gives the same hashes; this supersedes the S9 candidate `f2424114…f093d4` and the S9.1 candidate `22d8faac…7e52`), because that is byte-for-byte what a release would ship. Binary sha256 `bb38a083b0925ced9adc61690430d0f768509a1cf284a0c533cf5839e570dad1`, tarball sha256 `edba7f6fb63d0147f0464addb8249ecb1946da4514e0823ac9cf6575e07b8c58`. (The locally built `dist/sigma-scribe-1.0.0-kobo` has binary `6d4316335944ae6b96726b16187d76cf7a6dad86fc0dc99442acb1a44bd24f9f` — same source, different toolchain build of FBInk/gcc, so a different hash; it is not the one to test.) Download it into `dist/ci/` with `gh run download` and check `scripts/check-package.sh` before copying.

Procedure (same as 0.4.0): confirm it is the same Forma (serial <FORMA-SERIAL>) and touch no other disk (in particular not sdb, the 64 GB stick); read `.adds/sigma-scribe/sigma-scribe.log` read-only; make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.0.0` (`.adds/sigma-scribe`, `.adds/nm/sigma-scribe` and `sigma-scribe/notes`); replace **only** `.adds/sigma-scribe/sigma-scribe` with the CI package's binary (sha256 `bb38a083b0925ced9adc61690430d0f768509a1cf284a0c533cf5839e570dad1`) via `.new` + sync + rename; verify the sha256 of the device copy; sync, unmount and power off with udisksctl. `launch.sh`, `touch.conf`, the NickelMenu entry and the licences are unchanged since 0.3.0 (same checksums); `THIRD-PARTY.txt` is new and is added only in the fresh-install test below. Settings from 0.4.0 are read as-is.

Rollback: copy the 0.4.0 binary back (sha256 `689b84c4e763119cbdfa19eb07ade52d5709cdf034bf971fc2e5116e97c02043`, also in the pre-v1.0.0 backup). `.scratchpad.md` is hidden and ignored by 0.4.0; notes created from links are ordinary `.md` files and stay. `launch.sh` reboots on exit, which also returns the USB port to normal (gadget) mode.

## 1.4.6-probe g-sensor log (not run; needs approval; diagnostic only)
Purpose: S12 device check item 1 (plan decision 39) — which input node reports `MSC_RAW` and which code each way of holding the Forma gives. The build is **1.4.6 plus a log-only probe** (branch `v1.4.6-probe`, draft PR #22, never merged or released): it changes no behaviour and keeps `gsensor::ENABLED = false`, so Auto orientation still says "no sensor". Sprint doc: `sprints/S12.1-gsensor-probe.md` (on that branch).

Package: CI artifact `sigma-scribe-1.4.6-probe-kobo` from release.yml run 39764138168 (commit 0e57a36). Binary sha256 `06f070af2efd06a97623dbeec2bbadf06ca59904795e0411e4c6ea386f0c716f`; tarball `beba257847feca73aa87d648990970d355013e06c3c762c2b26403cf211b6701`.

**Install** (same safety steps as 1.4.0; can follow the 1.4.6 upgrade or replace it for one session):
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup; make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.6-probe` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
3. Copy **only** `.adds/sigma-scribe/sigma-scribe` from the probe package (via `.new` + sync + hash check + rename) and check the binary sha256 above on the device. Nothing else changes (add `THIRD-PARTY.txt` as in the 1.4.6 plan if it is not there yet). Sync, unmount, eject.

**Collect** (the user, on the Forma):
1. Open Sigma Scribe from NickelMenu as usual; the probe starts with it.
2. Hold the Forma upright (page buttons on the right) for ~5 s.
3. Turn it a quarter **clockwise** and hold ~5 s; then **upside down** ~5 s; then a quarter **counter-clockwise** from upright ~5 s; then back upright ~5 s.
4. Lay it **face up** on a table ~5 s, then hold it up again.
5. Note the order and anything odd (e.g. one turn that produced no change), then leave the app with the power button as usual.

**Fetch back** (PC, Kobo connected, read-only): copy `/mnt/onboard/.adds/sigma-scribe/gsensor-probe.log` (the probe log; capped at 2 MB) and `sigma-scribe.log` into the backup folder, then sync and eject. Nothing on the device is changed by fetching.

**Afterwards:** either keep 1.4.6-probe (behaves like 1.4.6) or put the 1.4.6 binary back (same procedure, 1.4.6 hash). The log decides the 0x19 / 0x1a mapping; turning `ENABLED` on is a separate change and approval.

## Upgrade 1.5.4 → 1.6.2 (run 2026-10-22, user-approved; supersedes the 1.6.1, 1.6.0 and 1.5.5 plans below)
Same steps as 1.5.4 → 1.5.5 below, backup folder `<date>-pre-v1.6.2`, CI 1.6.2 package: binary sha256 `46787595ce50d17c9b9b7b0218032f59368244c49ba1193eaca45cfab8f54011`, tarball `dbbfeece6eb5f83d0e2f10a51382e4758ba93f64c43743f9ec4619f8f4aa42f6` (PR #36 release run 39831043595, head 6d463c8). Nothing else changes; `.sigma-settings` is kept. Rollback: the 1.5.4 binary from the backup. Checks: `docs/device-checklist-prerelease.md` → "Language override (v1.5.5)", "Latin layouts (v1.6.0)", "Cyrillic and Greek layouts (v1.6.1)" and "UI languages (v1.6.2)".

Run log (2026-10-22 ~20:40 CEST): Forma found as `/dev/sdc` (serial N7828B0052845 matching `.kobo/version` and udev, label KOBOeReader, fw 4.38.23697) and mounted alone with udisksctl; sdb (64 GB) and every other disk untouched. Current binary confirmed 1.5.4 (`2d609ca1…b420`). `CurrentLocale=pt-PT`. Device log read read-only and copied (last session exit 0). Backup `~/backups/kobo-forma-N7828B0052845/2026-10-22-pre-v1.6.2` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe` notes, log, Kobo config; 335 files) compared with the device (`diff -r`) and `2026-10-22-pre-v1.6.2.SHA256SUMS` written. CI package downloaded with `gh run download`: tarball and per-file sha256 ✅; `launch.sh`, `touch.conf` and `.adds/nm/sigma-scribe` identical to the device. Binary copied as `.new`, synced, hash-checked, renamed. **Device sha256: binary `46787595ce50d17c9b9b7b0218032f59368244c49ba1193eaca45cfab8f54011` ✅**; `THIRD-PARTY.txt` unchanged (`f61a6486…2ee4`). Synced, unmounted, powered off with udisksctl; `/dev/sdc` gone from lsblk. App not yet started on the device; checklist pass pending (user).

## Upgrade 1.5.4 → 1.6.1 (also valid from 1.5.5, 1.6.0 or 1.4.0; not run; needs approval)
Same steps as 1.5.4 → 1.5.5 below, with backup folder `<date>-pre-v1.6.1` and the CI 1.6.1 package: binary sha256 `7e9d8cbf4a299bab91a81e0abae6735392fca9307098db489caf2b858635d59e`, tarball `02f12be1b47731577ddfd0ea2b768c1eb6fc28e333c093253946a359fbf3a5a4` (CI release.yml run 39821036458). Nothing else changes; `.sigma-settings` is kept (`layout=ru`, `ua`, `gr` are read as US by older binaries). Rollback: the previous binary from the backup. Checks: `docs/device-checklist-prerelease.md` → "Language override (v1.5.5)", "Latin layouts (v1.6.0)" and "Cyrillic and Greek layouts (v1.6.1)".

## Upgrade 1.5.4 → 1.6.0 (also valid from 1.5.5 or 1.4.0; not run; needs approval)
Same steps as 1.5.4 → 1.5.5 below, with backup folder `<date>-pre-v1.6.0` and the CI 1.6.0 package: binary sha256 `f0241692a62f5c57ac295182a469862bd878514867d4bdb1c8a6d258f14133f9`, tarball `f6ed0f851ab1aa75f9363732ac89b19698831574d27febd1daeed3786ae28266` (CI release.yml run 39816994477). Nothing else changes; `.sigma-settings` is kept (a new layout value such as `layout=pl` is read as US by older binaries). Rollback: the previous binary from the backup. Checks: `docs/device-checklist-prerelease.md` → "Language override (v1.5.5)" and "Latin layouts (v1.6.0)".

## Upgrade 1.5.4 → 1.5.5 (also valid from 1.4.0; not run; needs approval)
The Forma runs 1.5.4 since 2026-10-22 (run log below), so `THIRD-PARTY.txt` is already there; from 1.4.0 add it as in the 1.5.4 plan.
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.5.5` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.5.5 package, replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `bcf02bdd55892e1cdf3416c687e7cdf30c8169573af2759cde62bb92320b8909`, tarball `ab5930a75a35af670e0b4304bf1d7c012d0b9f048bbd953e7721ef4245e83bfa` (CI release.yml run 39812591834); `THIRD-PARTY.txt` sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4` (add only if absent).
5. Verify the sha256 on the device, then sync, unmount and power off.

Nothing else changes. `.sigma-settings` is kept; 1.5.5 adds `ui_lang_override=off` and `ui_lang=en` on the next save. `Kobo eReader.conf` is still only read. Rollback: the 1.5.4 binary from the backup (it ignores the new keys). Checks: `docs/device-checklist-prerelease.md` → "Language override (v1.5.5)".

## Upgrade 1.4.0 → 1.5.4 (run 2026-10-22, user-approved; replaces the 1.5.3, 1.5.2, 1.5.1, 1.5.0 and 1.4.6 plans below)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.5.4` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.5.4 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `2d609ca1c95e81feea8a86c215ad6b21c10fea51c594de4e13edfc674b2db420`, tarball `184f5cdfc1139a2532879d5a8dad3ea2e62ea7b8393c86f5e8b475709c8f9da2` (CI release.yml run 39797533111);
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` if it is not there yet, sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify the sha256 on the device, then sync, unmount and power off.

Nothing else changes: the letter model is inside the binary (no new files to copy). `.sigma-settings` is kept; 1.5.4 adds `keyboard_size=auto`, `ink_pen=finger` and `ink_a2=off`, `ink_boxes=off` and `ink_words=off` on the next save. New data written by the app: `sigma-scribe/notes/**/.ink/` (drawings) and, only if the user uses Train, `sigma-scribe/notes/.handwriting/samples.txt`; both are inside `notes`, so the backup covers them. Note that renaming a note now also edits `[[links]]` in other notes (decision 52). Rollback: the 1.4.0 binary from the backup (it ignores the new keys and files; `![ink](…)` lines stay as text). Checks: `docs/device-checklist-prerelease.md` (all sections up to "Gestures").

Run log (2026-10-22 ~17:45 CEST): Forma found as `/dev/sdc` (USB "File-Stor Gadget", serial <FORMA-SERIAL> matching `.kobo/version`, label KOBOeReader, fw 4.38.23697) and mounted alone with udisksctl at `/media/amartires/KOBOeReader`; sdb (DataTraveler 64 GB) and every other disk untouched. Current binary confirmed 1.4.0 (`2f89c5fd…dd1d`). Device log read read-only and copied into the backup (`sigma-scribe.log.copy`; last session 08:44 CEST exit 0, no error/panic lines). Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-22-pre-v1.5.4` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes` — 329 files) verified against the device checksums (`SHA256SUMS`). CI package from main release run 39804507625 (same bytes as run 39797533111): tarball `184f5cdf…9da2` ✅, binary `2d609ca1…b2db420` ✅. Binary copied as `.new`, synced, hash-checked, renamed; `THIRD-PARTY.txt` added (was absent). **Device sha256: binary `2d609ca1c95e81feea8a86c215ad6b21c10fea51c594de4e13edfc674b2db420` ✅, `THIRD-PARTY.txt` `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4` ✅.** `.adds/nm/sigma-scribe`, `launch.sh`, `touch.conf`, settings and notes unchanged. Synced, unmounted, powered off with udisksctl; `/dev/sdc` gone from lsblk. Device checklist pass pending (user).

## Upgrade 1.4.0 → 1.5.3 (not run; superseded by 1.5.4)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.5.3` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.5.3 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `448c830c2f98f87f64ecb77860247c731ca1cc3e5b5b617a6d07d9f30e26f169`, tarball `1f71d3b4d4799bc83599f3991f596f8a4f45dd7b309154944b8b1b03809214b2` (CI release.yml run 39792338074);
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` if it is not there yet, sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify the sha256 on the device, then sync, unmount and power off.

Nothing else changes: the letter model is inside the binary (no new files to copy). `.sigma-settings` is kept; 1.5.3 adds `keyboard_size=auto`, `ink_pen=finger` and `ink_a2=off`, `ink_boxes=off` and `ink_words=off` on the next save. New data written by the app: `sigma-scribe/notes/**/.ink/` (drawings) and, only if the user uses Train, `sigma-scribe/notes/.handwriting/samples.txt`; both are inside `notes`, so the backup covers them. Note that renaming a note now also edits `[[links]]` in other notes (decision 52). Rollback: the 1.4.0 binary from the backup (it ignores the new keys and files; `![ink](…)` lines stay as text). Checks: `docs/device-checklist-prerelease.md` (all sections up to "Free writing and word mode").

## Upgrade 1.4.0 → 1.5.2 (not run; superseded by 1.5.3)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.5.2` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.5.2 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `8fe47dda4f9caec2c04420c5f34f7ec5944f9443ec4c5266ecf6891cd3e98040`, tarball `b215d8394923232d3176390b860ac4dc2b5cfabb3aab42518065729f5c2d1423` (CI release.yml run 39785500768);
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` if it is not there yet, sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify the sha256 on the device, then sync, unmount and power off.

Nothing else changes: the letter model is inside the binary (no new files to copy). `.sigma-settings` is kept; 1.5.2 adds `keyboard_size=auto`, `ink_pen=finger` and `ink_a2=off` on the next save. New data written by the app: `sigma-scribe/notes/**/.ink/` (drawings) and, only if the user uses Train, `sigma-scribe/notes/.handwriting/samples.txt`; both are inside `notes`, so the backup covers them. Note that renaming a note now also edits `[[links]]` in other notes (decision 52). Rollback: the 1.4.0 binary from the backup (it ignores the new keys and files; `![ink](…)` lines stay as text). Checks: `docs/device-checklist-prerelease.md` (all sections up to "Letters and wiki relinks").

## Upgrade 1.4.0 → 1.5.1 (not run; superseded by 1.5.2)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.5.1` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.5.1 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `4f55e94be77d5fd6af7e8fbe471789cf4d89ae56ab930635759ac90e13c323fd`, tarball `c4dd63aeb4bc1e4954f8b44df35f3686cb04c39f1c995091610f80a922608093` (CI release.yml run 39778177555);
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` if it is not there yet, sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify the sha256 on the device, then sync, unmount and power off.

Nothing else changes. `.sigma-settings` is kept; 1.5.1 adds `keyboard_size=auto`, `ink_pen=finger` and `ink_a2=off` on the next save. Ink blocks are written under `sigma-scribe/notes/**/.ink/` (include it in backups from now on). Rollback: the 1.4.0 binary from the backup (it ignores the new keys; `![ink](…)` lines stay as text). Checks: `docs/device-checklist-prerelease.md` (all sections up to "Ink preview, rename and contact size").

## Upgrade 1.4.0 → 1.5.0 (not run; superseded by 1.5.1)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.5.0` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.5.0 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `b4c9957410bdb7fe093aaa95dcf2def73167f57f720e69fe9954c6def78108fa`, tarball `f23eeccf6024e369772d97b57c76a204511b71deb883177edf15532bb06cbaae` (CI release.yml run 39774205878);
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` if it is not there yet, sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify the sha256 on the device, then sync, unmount and power off.

Nothing else changes. `.sigma-settings` is kept; 1.5.0 adds `keyboard_size=auto`, `ink_pen=finger` and `ink_a2=off` on the next save. Ink blocks are written under `sigma-scribe/notes/**/.ink/` (include it in backups from now on). Rollback: the 1.4.0 binary from the backup (it ignores the new keys; `![ink](…)` lines stay as text). Checks: `docs/device-checklist-prerelease.md` (all sections up to "Write panel").

## Upgrade 1.4.0 → 1.4.6 (not run; superseded by 1.5.0)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.6` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.4.6 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `e884f1e71ab1661184171b76270665c19b839ce115f9673e34592b2d8daedacd`, tarball `acf2551db89ecf20d02dd5361e3dab79c87d8ac5e0ab5937d3e9e095cd39e48a`;
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` (Lucide ISC notice, missing on the device since 1.1), sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify both sha256 on the device, then sync, unmount and power off.

Nothing else changes (`launch.sh`, `touch.conf`, licences and the NickelMenu entry are identical). `.sigma-settings` is kept as is; 1.4.6 reads it unchanged and adds `keyboard_size=auto` on the next save. Rollback: the 1.4.0 binary from the backup (it ignores the new key). Checklist: `sprints/S14.1-pointer-black.md`, `sprints/S14.2-settings-redesign.md`, `sprints/S14.3-keyboard-size.md`, `sprints/S14.4-keyboard-slider.md`, `sprints/S14.5-slider-grey.md` and `sprints/S14.6-arrow-glyph.md`.

## Upgrade 1.4.0 → 1.4.5 (not run; superseded by 1.4.6)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.5` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.4.5 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `9fcbbed743bbe3d64b886ce4c3206f483b77dd0bd5c91d5d02892a3aa46e3a3e`, tarball `7e77ede427f317bc1a9a3e9b6fcf5b038584c7c8b2eba123fb80a48debdf7744`;
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` (Lucide ISC notice, missing on the device since 1.1), sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify both sha256 on the device, then sync, unmount and power off.

Nothing else changes (`launch.sh`, `touch.conf`, licences and the NickelMenu entry are identical). `.sigma-settings` is kept as is; 1.4.5 reads it unchanged and adds `keyboard_size=auto` on the next save. Rollback: the 1.4.0 binary from the backup (it ignores the new key). Checklist: `sprints/S14.1-pointer-black.md`, `sprints/S14.2-settings-redesign.md`, `sprints/S14.3-keyboard-size.md`, `sprints/S14.4-keyboard-slider.md` and `sprints/S14.5-slider-grey.md`.

## Upgrade 1.4.0 → 1.4.4 (not run; superseded by 1.4.6)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.4` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.4.4 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `085e9b1b6128379a8d0032e860edd2c4d6baa85f8cc66dc38b7e282593448548`, tarball `f024c73f07d64591eded70f544e10868e348f01a73679d6675ca463143163181`;
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` (Lucide ISC notice, missing on the device since 1.1), sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify both sha256 on the device, then sync, unmount and power off.

Nothing else changes (`launch.sh`, `touch.conf`, licences and the NickelMenu entry are identical). `.sigma-settings` is kept as is; 1.4.4 reads it unchanged and adds `keyboard_size=auto` on the next save. Rollback: the 1.4.0 binary from the backup (it ignores the new key). Checklist: `sprints/S14.1-pointer-black.md`, `sprints/S14.2-settings-redesign.md`, `sprints/S14.3-keyboard-size.md` and `sprints/S14.4-keyboard-slider.md`.

## Upgrade 1.4.0 → 1.4.3 (not run; superseded by 1.4.6)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.3` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.4.3 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `a3bef0bcd147c10c4b4e810773ad38ee93262dbd93056994f1502675b7f17b2c`, tarball `1dd829d9fa17014e04c6751934befcb8cdcda9cefab200204a251158974b2b88`;
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` (Lucide ISC notice, missing on the device since 1.1), sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify both sha256 on the device, then sync, unmount and power off.

Nothing else changes (`launch.sh`, `touch.conf`, licences and the NickelMenu entry are identical). `.sigma-settings` is kept as is; 1.4.3 reads it unchanged and adds `keyboard_size=auto` on the next save. Rollback: the 1.4.0 binary from the backup (it ignores the new key). Checklist: `sprints/S14.1-pointer-black.md`, `sprints/S14.2-settings-redesign.md` and `sprints/S14.3-keyboard-size.md`.

## Upgrade 1.4.0 → 1.4.2 (not run; superseded by 1.4.6)
Same procedure as 1.4.0:
1. Verify the serial (<FORMA-SERIAL>) and touch no other disk (never sdb).
2. Read the log and copy it into the backup.
3. Make a checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.2` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, log).
4. From the CI 1.4.2 package:
   - replace `.adds/sigma-scribe/sigma-scribe` (via `.new` + sync + hash check + rename), binary sha256 `1e2ca1c93790ca2cec935ddaeaff4547055958068aaf04552608f615d5d41cb1`, tarball `4cf42621f9c414e531443ccd26f860f591c966cff8a4d1c691bc24c4458685a2`;
   - add `.adds/sigma-scribe/THIRD-PARTY.txt` (Lucide ISC notice, missing on the device since 1.1), sha256 `f61a648623ee545238601212f3d29eb056bddca7ed738f1427a2ca1bfc712ee4`.
5. Verify both sha256 on the device, then sync, unmount and power off.

Nothing else changes (`launch.sh`, `touch.conf`, licences and the NickelMenu entry are identical). `.sigma-settings` is kept as is and 1.4.2 reads it unchanged. Rollback: the 1.4.0 binary from the backup (it reads the same settings). Checklist: `sprints/S14.1-pointer-black.md` and `sprints/S14.2-settings-redesign.md`.

## Upgrade 1.4.0 → 1.4.1 (not run; superseded by 1.4.2)
Same procedure as 1.4.0: verify the serial (<FORMA-SERIAL>), touch no other disk (never sdb), read and copy the log, checksum-verified backup into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.4.1`, then replace `.adds/sigma-scribe/sigma-scribe` with the CI 1.4.1 binary (via `.new` + sync + hash check + rename) **and add `.adds/sigma-scribe/THIRD-PARTY.txt` from the same CI package** (Lucide ISC notice; missing on the device since 1.1). Verify both sha256 on the device against the package, sync, unmount, power off. Nothing else changes (`launch.sh`, `touch.conf`, licences, NickelMenu entry identical). Checklist: `sprints/S14.1-pointer-black.md`.

## Upgrade 1.0.3 → 1.4.0 (APPROVED and RUN 2026-10-22 ~08:30 CEST — on-device checklist pending)
Run log: Forma found as `/dev/sdc` (serial <FORMA-SERIAL> verified, label KOBOeReader, model N782) and mounted alone; sdb and every other disk untouched. The device was still on **1.0.3** (binary `339b6261…9be0`); 1.0.4–1.3.0 were never installed. Log read read-only (418 lines) and copied into the backup as `device-log-1.0.3.log`: no sessions after 01:00 CEST, so the newest numbers are the 1.0.3 ones already recorded (22-key session: typing p95 95.6 ms, pointer move-to-submit p50 169 / p95 344 / max 489 ms over 52 DU updates; 4-key session: typing p95 248 ms, pointer p50 170 / p95 333 ms over 243 updates); no 1.0.4 data. `Kobo eReader.conf` read read-only: **`CurrentLocale=en`** → 1.4.0 starts in English. Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-22-pre-v1.4.0` (`.adds/sigma-scribe`, `.adds/nm`, `sigma-scribe/notes`, the log and a read-only copy of the Kobo config; 331 files) compared file by file with the device and `SHA256SUMS` verified. Only `.adds/sigma-scribe/sigma-scribe` replaced (copy to `.new`, sync, hash check, rename, sync) by the CI 1.4.0 binary, sha256 on the device `2f89c5fd559ffbb1c8bd070213992e38bb4e7e6b8bc5f97bbd10eabefc85dd1d` ✓ (tarball `6b63d580…8cf3`). `launch.sh`, `touch.conf` and licences are identical to the package; notes unchanged (checksums). Synced, unmounted, powered off; gone from `lsblk`. Not copied (only the binary was in scope): `THIRD-PARTY.txt` with the Lucide ISC notice — add it with the next package update. Rollback: the 1.0.3 binary from the backup.

On-device checklist — 1.4.0 (covers 1.0.4–1.4): see `sprints/S14-ui-languages.md` and the 1.0.4–1.3 sprint docs — pointer smoothness and Graph pill (1.0.4); 200 % legibility and fit (1.1); Orientation rows, inverted touch and page buttons, Auto shows "Auto (no sensor)" (1.2; G-sensor probe still to do); Layouts, AltGr and dead keys on the K400 and a 60 % board (1.3); App info → Version 1.4.0 and Language English (from `CurrentLocale=en`), then Kobo → Français / Português and relaunch (1.4).

## Upgrade 1.0.2 → 1.0.3 (APPROVED and RUN 2026-10-22 00:50 CEST — on-device checklist pending)
Run log: Forma found as `/dev/sdc` (serial <FORMA-SERIAL>, label KOBOeReader, fw 4.38.23697) and mounted alone; sdb and every other disk untouched. Log read read-only (223 lines) and copied into the backup as `device-log-1.0.2.log`: two 1.0.2 sessions (00:17–00:23 and 00:34–00:47 CEST, both exit 0). Host mode switched on and off cleanly in both, but **no USB keyboard or pointer was detected** in either session, so 1.0.2 logged **no `perf pointer` lines and no typing `perf:` summary**. The last pointer/typing numbers are still the 1.0.1 ones (GL16 p50 606 / p95 1091 ms; typing p95 186.7 ms). Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-22-pre-v1.0.3` (328 files, `SHA256SUMS` verified). Device binary was `5e14620b…3376` (1.0.2) ✓; replaced via `sigma-scribe.new` + sync + rename by CI 1.0.3, sha256 on the device `339b62613cf13b51c7e140ac9c85887b5780677028b759799aa6d04178b49be0` ✓. Synced, unmounted, powered off; gone from `lsblk`.

**Package:** CI-built `sigma-scribe-1.0.3-kobo` (release workflow on PR #7). Tarball sha256 `b6d4dd52bd56aa07ebfcb005f7c651b111a5ad3e1f261f72ae6f209f7d07a6d4`; binary sha256 `339b62613cf13b51c7e140ac9c85887b5780677028b759799aa6d04178b49be0`. Rollback: the 1.0.2 binary from the backup.

On-device checklist — v1.0.3 (K400 Plus on the charging splitter, Options → USB keyboard & mouse = On):
- [ ] Move the pointer for ~30 s: responsive; only light trails while moving
- [ ] Rest 3 s: pointer hides, one clean pass, no marks left, no flash
- [ ] Type while moving the pointer: no typing lag
- [ ] Log: `perf pointer: du …`, `perf pointer: gl16-cleanup …`, pointer p50/p95 summary; typing `perf:` p95 back near 66–94 ms
- [ ] Contents open → preview: drawer stays, text beside it; tap a heading on a later page → jumps; Back returns
- [ ] To-dos in preview: tick a box → saved, preview updates
- [ ] Scratch in preview: USB typing goes to the scratch only; the note is unchanged; Esc leaves the scratch
- [ ] Graph / Files / Options from preview: no drawer; it returns after closing
- [ ] Landscape + Night

## Upgrade 1.0.1 → 1.0.2 (APPROVED and RUN 2026-10-22 00:08 CEST — on-device checklist pending)
Run log: Forma found as `/dev/sdc` (serial <FORMA-SERIAL>, label KOBOeReader, fw 4.38.23697) and mounted alone; sdb and every other disk untouched. Log read read-only (187 lines): one 1.0.1 session, 23:30–23:47 CEST, exit 0, no errors. USB host on → Logitech receiver keyboard (event3) + pointer (event4) → both gone → host off (gadget). **`perf pointer` (GL16, 9 samples, regions 18×32 … 49×32): min 393, p50 606, p95 1091, max 1091 ms — well over the 300 ms threshold** (reported; no code changed in this run). Typing: 101 keystrokes, p50 72.5, p95 186.7, max 302.5 ms. The user also created `_test/test-chapter-01.md`. Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-22-pre-v1.0.2` (327 files incl. 316 in `notes/_test/`, `SHA256SUMS` verified). Device binary was `ffc2e119…6567` (1.0.1) ✓; replaced via `sigma-scribe.new` + sync + rename by CI 1.0.2, sha256 on the device `5e14620bfeb40ca937f7ff06f9b9bf08b2df4af0c556f2a49c41e9c668703376` ✓. Synced, unmounted, powered off; gone from `lsblk`.

**Package:** CI-built `sigma-scribe-1.0.2-kobo` (release workflow on PR #5, feat/v1.0.2). Tarball sha256 `1b9c2192c6f0e6d858a8ba5803cb93080dc7e663e91091b26a302da2f098e22f`; binary sha256 `5e14620bfeb40ca937f7ff06f9b9bf08b2df4af0c556f2a49c41e9c668703376`.

Procedure (same as 1.0.0 → 1.0.1): identify the Forma by serial <FORMA-SERIAL>, mount only that disk (never sdb or any other disk); read the log read-only (record the `perf pointer` lines from 1.0.1); back up `.adds/sigma-scribe`, `.adds/nm/sigma-scribe` and `sigma-scribe/notes` to `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.0.2` and verify against checksums; confirm the device binary is 1.0.1 (`ffc2e119e569e4364e565b7779d4e650183bb6107bc46e4bc982356650306567`), else stop; replace **only** the binary via `sigma-scribe.new` + sync + rename; verify its sha256 on the device; sync, unmount, eject, confirm gone from `lsblk`. Notes (incl. `notes/_test/`) untouched. Rollback: the 1.0.1 binary from the backup.

On-device checklist — v1.0.2:
- [ ] Graph (from `_test/test-00-start-here`): notes are round dots, links plain lines, the current note filled; labels beside dots only where they fit
- [ ] Tap a dot: it grows into a pill with the full name in one update, **no flash**; neighbours stand out; small dots are easy to hit with a finger
- [ ] Tap the pill again: preview opens and a **Back** arrow appears left of Files; Back returns to the start note
- [ ] In preview follow a wiki link and a backlink, then Back twice: same notes, same mode and page
- [ ] Files → another note, To-dos → open, Contents → heading: each can be undone with Back; Back hides when the history is empty
- [ ] Esc (USB keyboard) and Alt+Left go back; title bar fits in portrait and landscape
- [ ] Night + landscape map reads well

## Upgrade 1.0.0 → 1.0.1 (APPROVED and RUN 2026-10-21 23:20 CEST, with the test notes — on-device checklist pending)
Run log: Forma found as `/dev/sdc` (serial <FORMA-SERIAL>, label KOBOeReader, fw 4.38.23697) and mounted alone at `/media/amartires/KOBOeReader`; sdb (64 GB stick) and every other disk untouched. Log read read-only (155 lines, all sessions exit 0 except the old v0.2.1 `exit 1` at 00:25): 1.0.0 sessions 20:46 (`perf: 12 keystrokes, p50 53.2 ms, p95 65.8 ms, max 81.3 ms`) and 20:48–21:04 (`usb: host mode on -> Ok(()) (role now Some("host"))`; "Logitech USB Receiver" event3 keyboard + event4 pointer; both gone on unplug; `host mode off -> Ok(()) (role now Some("gadget"))`; `perf: 19 keystrokes, p50 74.7 ms, p95 93.7 ms, max 118.0 ms`). Backup `~/backups/kobo-forma-<FORMA-SERIAL>/2026-10-21-pre-v1.0.1` (11 files, `SHA256SUMS` verified). Device binary was `bb38a083…dad1` (1.0.0) ✓; replaced via `sigma-scribe.new` + sync + rename by the CI 1.0.1 binary, sha256 on the device `ffc2e119e569e4364e565b7779d4e650183bb6107bc46e4bc982356650306567` ✓. Test notes: `notes/_test` did not exist; copied `_test/` (315 files incl. graph-300, pack tarball `460a8611…5c9b`) without overwrite; 315/315 checksums verified on the device. Synced, unmounted, powered off; gone from `lsblk`.

**Package:** CI-built `sigma-scribe-1.0.1-kobo` (release workflow on PR #3, feat/v1.0.1). Tarball sha256 `3aa18e6b2d0514f9b446a3e520d05307d7632b0ac40ba0b6376db6b8137ff03c`; binary `.adds/sigma-scribe/sigma-scribe` sha256 `ffc2e119e569e4364e565b7779d4e650183bb6107bc46e4bc982356650306567`.

Procedure (same as 0.4.0 → 1.0.0): identify the Forma by serial <FORMA-SERIAL> and mount only that disk (never sdb, the 64 GB stick, or any other disk); read `.adds/sigma-scribe/sigma-scribe.log` read-only; back up `.adds/sigma-scribe`, `.adds/nm/sigma-scribe` and `sigma-scribe/notes` into `~/backups/kobo-forma-<FORMA-SERIAL>/<date>-pre-v1.0.1` and verify it against checksums; confirm the device binary is 1.0.0 (`bb38a083b0925ced9adc61690430d0f768509a1cf284a0c533cf5839e570dad1`), else stop; replace **only** the binary; verify its sha256 on the device; sync, unmount, eject, confirm gone from `lsblk`. Settings carry over (`scroll=` is new and defaults to Traditional).

**Optional, separate approval — test notes:** after the binary step and before eject, copy `dist/test-notes/_test/` (from `scripts/make-test-notes.sh`, tarball sha256 `460a8611e3fbae74aa166751c3db93bfe14b41e2895638b99f3949efd7f45c9b`) to `sigma-scribe/notes/_test/` on the Kobo. Nothing else in `notes/` is touched; `graph-300/` may be left out (`--no-graph`). Uninstall = delete `notes/_test/`.

Rollback: copy the 1.0.0 binary back from the pre-v1.0.1 backup (sha256 above).

On-device checklist — v1.0.1 (K400 Plus on the charging splitter, Options → USB keyboard & mouse = On):
- [ ] Move the pointer in circles for 30 s over text and the title bar: **no ghost trails**; the arrow still feels responsive
- [ ] Let it hide (3 s) and type a key: any faint residue along the path is cleaned, no flash
- [ ] Log has `perf pointer: gl16 … ms` lines; record the values (> 300 ms → raise the throttle to 250 ms)
- [ ] Options → Scroll = Natural: two-finger scroll reverses in note, preview, Files and Graph (zoom); back to Traditional restores 1.0 behaviour; survives a restart
- [ ] Options row reads **USB keyboard & mouse**; Help page **USB keyboard, mouse & export**
- [ ] (if the test notes were copied) `_test/test-00-start-here` opens; its links, Find (50 lamps) and Graph map work

### Extra steps and risks for USB keyboard and mouse (v0.8 / v0.8.1)
Nothing extra is installed: no kernel modules, no KoboRoot, no udev rules. What 1.0.0 does on the device, only while **Options → USB keyboard = On** (this also covers the mouse/touchpad):
- Switches the USB port to **host mode** by writing `host` to `/sys/kernel/debug/ci_hdrc.0/role` (mounting debugfs first if needed — the same method as KOReader's ExternalKeyboard plugin), and writes `gadget` back when the setting is turned off or the app exits. While it is on, the Kobo cannot be seen by a computer — leave the app first. A reboot always restores gadget mode.
- Polls `/proc/bus/input/devices` once a second for keyboards (EV_KEY with A, Z, Space, Enter) and relative pointers (EV_REL with REL_X + REL_Y) and reads their `/dev/input/eventN`; a combined device (K400 Plus) is read once and feeds both. The touch panel (absolute) and the gpio buttons are never treated as either.
- Logs every step to `.adds/sigma-scribe/sigma-scribe.log` (`usb: host mode on -> …`, `usb: "…" on eventN (keyboard …, pointer …)`, `usb: eventN gone`).

Hardware: first the **regular wired 60% USB keyboard** that already worked on this Forma (most likely with KOReader), then the **Logitech K400 Plus** (Unifying receiver, keyboard + touchpad, never tested on the device); a micro-USB OTG adapter and power (powered hub or powered OTG/Y-cable) — the Forma does not supply VBUS. See `docs/usb-keyboard.md`.

Risks (only the device can confirm) and the fallback if it fails:
1. *Lowered:* host mode with a wired keyboard is known to work on this Forma (most likely via KOReader). Our switch writes the same debugfs file (`ci_hdrc.0/role`) the same way KOReader does (differences in `docs/usb-keyboard.md`: no `/proc/mounts` check; we restore gadget only if we switched). If it fails, the log shows it, the status bar says `no USB`, and everything else works. Kernel modules are **not** part of this plan.
2. *Unverified:* the K400 Plus receiver and touchpad were never used on this Forma. The receiver may expose the K400 as one combined device (hid-logitech-dj) or as separate keyboard and mouse nodes; both are handled, but which one appears is unverified — record the `usb:` log lines.
3. Nickel is stopped by `launch.sh` before the app starts (same as KOReader) and the device reboots on exit, so Nickel does not see the keys; the earlier note about Nickel reading them was wrong.
4. Pointer motion is raw relative counts (no acceleration curve); speed on the 1440×1920 panel may feel slow or fast — note it.
5. Battery: host mode and the receiver may drain faster; the setting is Off by default.

**Device results (user, 2026-10-21 ~21:00 CEST, 1.0.0):** Logitech K400 Plus on a USB splitter that also charges the Forma: **keyboard works, pointer works, two-finger scroll works (up and down)** ✅. Bug found: the **pointer leaves ghost trails** (planned fix in v1.0.1, `sprints/S10-pointer-scroll-usb.md`). The other checklist items below were not reported yet.

On-device checklist — v0.5 (drawer, never tested on the device):
- The Contents, To-dos and Scratch icons open/close their panes in any combination; panes share the height; Options → Drawer Left/Right moves the drawer; no drawer in preview; long lists end in "N more…".
- Contents lists the headings; tapping one jumps the caret there.
- To-dos lists boxes from all notes; filter All/Open/Done; checking a box in another note changes only that line; tapping the text opens that note at the line.
- Scratch: type into it, switch notes, leave and relaunch from NickelMenu — the text is still there; it is not in Files.
- Landscape and Night with the drawer open; no ghosting after closing panes.

On-device checklist — v0.6 (find & wiki, never tested on the device):
- Find: the magnifier (or Ctrl+F) opens the bar; count `i of n`; Prev/Next; Aa; Replace; **Replace All across 50 matches** and one Undo brings them back; a one-line selection seeds the query; Esc / X closes; from preview it switches to write.
- Wiki links in preview: a known `[[note]]` opens it in preview; **a missing `[[note]]` (dashed) is created next to the linking note** and opens in write mode; it shows in Files.
- Backlinks at the end of the preview list the right notes; each opens.
- Graph → **List** (since S9.1 the Graph opens on the map): one row per note with → / ← links; page-turn scrolls; tap opens; the scratch pad is not listed.

On-device checklist — v0.7 (help & hardening):
- Help (`?`, rightmost title-bar icon since S9.2, in write and preview): menu → App info (version 1.0.0, licence, notes path, buttons, USB hint, credits) and Markdown reference (pages turn with the page buttons, `page i of n`); "← Back to Help" and Esc go back; X closes.
- **Pull power (hold power / force off) during a save ×10** (type, then cut power right after the status shows "saving") → after each restart the note opens and is a complete old or new version; no `.tmp` debris shown in Files.
- All controls hittable with a finger: title bar, drawer rows and Filter, Find bar, Files rows and More, Graph, Options, Help — portrait and landscape.
- Both orientations: switch to Landscape and back with Help, Files, Options and the drawer; text and caret survive; no ghosting.
- Typing feel: after a few minutes of typing, exit and read the log — the last `perf:` line shows key events with **p95 < 150 ms** (report the numbers; see `docs/perf.md`).
On-device checklist — v0.8 (USB keyboard) — **first with the wired 60% keyboard** that already worked on this Forma, **then with the Logitech K400 Plus** (never tested on the device):
- With USB keyboard Off: the port behaves as in 0.4.0 (after leaving the app and the reboot, a computer sees the Kobo).
- Options → USB keyboard On: status shows `no USB`; the log has `usb: host mode on -> Ok(())` (record the actual line and `role now`).
- **Wired 60% keyboard**, hot-plugged through the powered hub / powered OTG cable: within ~1 s the status shows `USB`; the log names it (`keyboard true`); typing goes into the note; Shift gives capitals/symbols (US layout); if the 60% board uses Fn layers, check arrows, Home/End, PgUp/PgDn, Delete and Esc through Fn.
- **Shortcuts**: Ctrl+Z / Ctrl+Y, Ctrl+A, Ctrl+C / X / V, Ctrl+F (opens Find, typing goes into the field, Esc closes), Shift+arrows select, Ctrl+Left/Right by word, Home/End, Ctrl+Home/End, PgUp/PgDn.
- Scratch pane takes USB typing; the on-screen keyboard keeps working alongside (one-shot Shift unaffected).
- **Unplug** while typing (and while holding Ctrl) → no crash, status back to `no USB`; re-plug works again.
- ✅ 2026-10-21 (user): K400 Plus keyboard verified on the device. **Then the K400 Plus receiver**: record the `usb:` lines (one combined device or separate keyboard/mouse), repeat typing, shortcuts and unplug.
- Help → USB keyboard & export reads correctly; landscape typing works.
- Export: copy `sigma-scribe/notes` to a computer over USB; files open as plain Markdown.

On-device checklist — v0.8.1 (mouse / touchpad — K400 Plus touchpad, never tested on the device; a plain USB mouse too if available):
- With USB keyboard On and the receiver plugged in: the log shows a pointer; status `USB`.
- ✅ 2026-10-21 (user): K400 pointer and two-finger scroll (up/down) work. ❌ The arrow leaves ghost trails (v1.0.1 fix). Touchpad moves a small arrow; it redraws in small steps (no full-screen flashes), with no trail left behind; it hides after ~3 s still and as soon as you type (USB or on-screen).
- Left click places the caret, presses title-bar icons, keyboard keys, drawer rows, Files rows, Options cells, Help rows and Close.
- Click-and-drag on the note selects text and the Cut/Copy/Paste bar appears; double-click selects a word; right click does nothing.
- Two-finger scroll (if the receiver sends wheel events; the K400's may not — note it) and a mouse wheel: note moves 3 rows per notch; preview, Files, Graph and Help turn a page per scroll.
- Landscape and Night: the arrow is visible and follows the rotated screen.
- Unplug the receiver while holding the button → no crash; arrow gone; keyboard and touch keep working.

On-device checklist — v1.0 title bar (S9.2):
- Title bar icons are large (about twice the 0.4.0 size) and in this order after the file name: Files, eye/pencil, Find, Scratch, To-dos, Contents, Graph, Aa, ? — portrait and landscape, Paper and Night.
- Every icon opens the right thing with one finger tap, in write mode and (Files, pencil, Find, Graph, Aa, ?) in preview; the ? icon is visible in preview.
- Files, Graph, Help and Options sheets: the X is at the far right of the bar and closes the sheet.
- The file name still shows (ellipsized if long) and tap-to-rename still works; the bar height feels acceptable in landscape.

On-device checklist — v1.0 Graph map (S9.1):
- Graph (title bar) opens on **Map**: every note visible, the open note is the filled box in the middle, links are lines with arrowheads; opening takes well under a second with the real library (record the time, and the note/link counts from the footer).
- Readable at 16 greys in **Paper** and **Night**, portrait and landscape; no ghosting left after the full refresh on open.
- Tap a note: it gets a thick border, its neighbours and their links stay dark, the rest go grey (partial refresh, no flash); tap it again (or **Open**) → it opens in preview; tapping empty map clears the selection.
- Drag on the map pans without a full flash; page-turn buttons pan half a screen up/down; **−** / **+** zoom through 4 levels (full refresh), **−** at the smallest level recentres.
- **List** shows the old list, **Map** comes back; Esc / X close the sheet.
- The scratch pad never appears as a node. Add a `[[link]]` in a note and reopen Graph → the map includes the new link (layout recomputed once).
- Mouse (if the v0.8.1 checks passed): wheel zooms, click selects/opens, drag pans.

On-device checklist — v1.0 (release; S9 definition of done):
- Help → App info shows **Version 1.0.0**.
- **Update keeps notes**: after the upgrade above, every note, folder, the scratch pad and the settings from 0.4.0 are still there.
- **Fresh install from the release tarball** *(needs its own approval: it deletes the app folder on the device after a verified backup)*: with notes backed up, remove `.adds/sigma-scribe` and `.adds/nm/sigma-scribe`, unpack the CI tarball (sha256 `edba7f6fb63d0147f0464addb8249ecb1946da4514e0823ac9cf6575e07b8c58`) onto the drive root, eject; NickelMenu shows *Sigma Scribe*; first launch creates `sigma-scribe/notes` if missing; `THIRD-PARTY.txt` is present.
- **Uninstall clean** *(needs its own approval)*: delete `.adds/sigma-scribe` and `.adds/nm/sigma-scribe`; NickelMenu no longer lists it; the stock reader works; `sigma-scribe/notes` is untouched. Then re-install for daily use.
- No regressions anywhere; the log shows exit 0, no refresh failures, and the `perf:` line with p95 < 150 ms (record it).
