# Deployment & rollback — Kobo Forma

Device: Kobo Forma, serial <FORMA-SERIAL>, firmware 4.38.23697. **Never update to firmware 5.x** (Cobalt and NickelMenu hooks are 4.x only).

## 1. Backup (always first)
```sh
# device connected over USB, mounted (read-only is fine)
M=/media/amartires/KOBOeReader
D=~/backups/kobo-forma-<FORMA-SERIAL>/$(date +%F)
mkdir -p "$D" && rsync -a "$M"/ "$D"/
( cd "$M" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "$D.device.sha256"
( cd "$D" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "$D.copy.sha256"
diff "$D.device.sha256" "$D.copy.sha256" && echo BACKUP OK
```
Keep at least the pre-Cobalt backup forever.

## 2. Install Cobalt + NickelMenu (S0, once)
1. Charge the Forma (the Kobo installer silently does nothing on low battery).
2. Browser installer https://bandarlabs.github.io/Cobalt/install/ or `curl -fsSL https://bandarlabs.github.io/Cobalt/install.sh | sh` — writes `.kobo/KoboRoot.tgz` + NickelMenu.
3. Eject, let the reader restart and install. Open Cobalt from the Kobo menu; accept the untested-device notice.

## 3. Dev builds
Sideload per Cobalt `docs/DEVELOPING.md` (over Wi-Fi/SSH with `kobo devices`, `kobo session --keep-awake on`) or USB. Plan B: package our own `KoboRoot.tgz` + NickelMenu entry in `.adds/nm/`.

## 4. Releases
Via Cobalt Store (see publishing.md); users update over Wi-Fi.

## 5. Rollback
- App: remove it in Cobalt Store/launcher; reinstall previous version from the signed archive.
- Notes: plain `.md`/`.txt` files — copy them off over USB before any uninstall.
- Cobalt: remove per Cobalt `docs/INSTALL.md` "remove" section; a restart always returns to the stock reader (Cobalt does not touch the boot chain).
- Last resort: restore files from the §1 backup; Kobo factory reset (keeps firmware) as absolute last resort.
