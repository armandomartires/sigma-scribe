# Deployment & rollback — Kobo Forma (Plan B: FBInk + NickelMenu)

Device: Kobo Forma, serial <FORMA-SERIAL>, firmware 4.38.23697. **Never update to firmware 5.x** (NickelMenu is 4.x only).

## 1. Backup (always first)
```sh
M=/media/amartires/KOBOeReader
D=~/backups/kobo-forma-<FORMA-SERIAL>/$(date +%F)
mkdir -p "$D" && rsync -a "$M"/ "$D"/
( cd "$M" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "$D.device.sha256"
( cd "$D" && find . -type f -print0 | sort -z | xargs -0 sha256sum ) > "$D.copy.sha256"
diff "$D.device.sha256" "$D.copy.sha256" && echo BACKUP OK
```
Do not open the backup's `KoboReader.sqlite` in place (SQLite creates `-shm`/`-wal` files); copy it first.

## 2. First install
NickelMenu v0.6.0 `KoboRoot.tgz` + the `.adds` tree from `scripts/package-kobo.sh`. Exact steps: `.ai/install-plan-forma.md`.

## 3. Updating the app
Build `scripts/package-kobo.sh`, connect USB, replace `.adds/sigma-scribe/sigma-scribe` (and `launch.sh` if changed), `sync`, eject. No reboot or `KoboRoot.tgz` needed; notes are untouched.

## 4. Rollback
See the table in `.ai/install-plan-forma.md` §4. Notes are plain files under `sigma-scribe/notes/`.
