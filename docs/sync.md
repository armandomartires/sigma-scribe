# Sync (Syncthing on the Kobo)

Settings → **Sync** runs [Syncthing](https://syncthing.net) on the Forma and
keeps the notes folder in step with the reader's other computers. Nothing
goes through a server of ours: the devices talk to each other, on the local
network when they can, through Syncthing's public relays when they cannot.

Sync is **off** until you turn it on. It runs only while Σscribe is open.

This page is the design (decided 2026-10-28: Syncthing runs *on* the Kobo,
not as a one-way export) and the notes for trying it on the Forma. Status:
built and tested on the desktop with a fake REST server and a fake
binary; **never run against a real Syncthing, and not run on the Forma**
(see §12).

## 1. Packaging

| | |
|---|---|
| Release | Syncthing **v2.1.7**, asset `syncthing-linux-arm-v2.1.7.tar.gz` (13.4 MB; the binary is 32 MB unpacked) |
| sha256 | `d8b59029e25992bc0d280fccc5626e5dca743b285fca1a36db1148fcf871b639` |
| Source | https://github.com/syncthing/syncthing/tree/v2.1.7 |
| Binary | static ELF, 32-bit ARM EABI5, compiled for ARMv6, so it runs on the Forma's Cortex-A9 (ARMv7). Pure Go, no libc, no shared libraries. |

How the pin was checked (2026-10-28): the tarball was downloaded and hashed;
the hash equals the one in the release's `sha256sum.txt.asc` and the
`digest` GitHub reports for the asset. The GPG signature on
`sha256sum.txt.asc` was **not** verified: it is made by key
`37C84554E7E0A261E4F76E1ED26E6ED000654A3E`, which is not in the release key
Syncthing publishes at `syncthing.net/release-key.txt` (primary
`FBA2E162F2F44657B38F0309E5665F9BD5970C47`) and which the keyserver returns
without a user ID. Anyone updating the pin should verify the signature once
they have the right key, and say so here.

`scripts/fetch-syncthing.sh` downloads the tarball (`curl`, else `wget`) into
`target/syncthing/`, checks the sha256 above, and refuses anything else.
`scripts/package-kobo.sh` calls it and unpacks into the package:

```
.adds/sigma-scribe/syncthing/syncthing        the binary
.adds/sigma-scribe/syncthing/LICENSE.txt      MPL-2.0, as shipped by Syncthing
.adds/sigma-scribe/syncthing/AUTHORS.txt
.adds/sigma-scribe/syncthing/NOTICE.txt       ours: what it is, where the source is
.adds/sigma-scribe/syncthing/ca-certificates.crt   Mozilla's root certificates (below)
```

**Root certificates.** Syncthing's relays and global discovery servers are
reached over HTTPS, and Go looks for root certificates in `/etc/ssl/certs`
and the like. The Kobo has none it finds: on the Forma (2026-10-28) the log
showed `x509: certificate signed by unknown authority` for
`relays.syncthing.net`. The package therefore carries Mozilla's CA bundle as
curl extracts it, and the app starts Syncthing with `SSL_CERT_FILE` pointing
at it (`Config::ca_file`, set when `ca-certificates.crt` is beside the
binary). The app's own HTTPS (plugins repository) is unaffected: it uses the
roots compiled into the program (`webpki-roots`), which hold only the keys,
not the certificates Go needs.

| | |
|---|---|
| File | `cacert-2026-10-20.pem` from https://curl.se/ca/ (200 KB, 118 certificates) |
| sha256 | `796f1389bfea13c0ed78f663b765baaff54a508e5483786bd6ece73c5e3e3cbe` (equal to curl's published `.sha256`) |
| Licence | MPL-2.0 (Mozilla's certificate data), named in `NOTICE.txt` and `THIRD-PARTY.txt` |

`scripts/fetch-cacert.sh` pins it the same way as the Syncthing release. The
bundle ages: certificates expire and roots are withdrawn, so refresh the pin
(a dated file from https://curl.se/docs/caextract.html) with each Syncthing
update, or at least once a year.

To change the pin: edit `SYNCTHING_VERSION` and `SYNCTHING_SHA256` in
`scripts/fetch-syncthing.sh`, rebuild, run the Forma checklist (§12), and
update the table above.

**Licence.** Syncthing is MPL-2.0 (file-level copyleft). It is shipped
**unmodified, as a separate executable the app starts as a process**, so it is
an aggregate with the AGPL-3.0-only app, not a combined work. MPL-2.0 §3.2
requires the recipient to be told how to get the source of the executable:
`NOTICE.txt` does that (URL above, exact tag) and the full licence text sits
beside the binary. `THIRD-PARTY.txt` lists it. `cargo deny` is unaffected: no
crate is added. The Go modules inside the binary carry their own licences;
Syncthing's own build lists them, and the source tag is the authority. Before
a release that ships it, check that list against `docs/release/checklist.md`.

**Size and updates.** The package grows by 13 MB (compressed); the Kobo has
about 6.7 GB free. Updating the app replaces `syncthing/` and never touches
the identity or the database in `syncthing-home/` (§2). Syncthing's own
auto-upgrade is off (`--no-upgrade`): the pinned build is the one tested.

## 2. Where things live

| Path on the Kobo | What |
|---|---|
| `.adds/sigma-scribe/syncthing/syncthing` | the binary (in the package) |
| `.adds/sigma-scribe/syncthing-home/` | Syncthing's `--home`: `config.xml`, the device key and certificate, the database. **Not** in the package, so an update never changes the device ID. |
| `sigma-scribe/notes/` | the synced folder |

Keep `syncthing-home/` in backups: losing it gives the Kobo a new device ID
(the other computers would have to add it again). Removing the folder is the
"reset sync" for now.

`/mnt/onboard` is vfat: no permissions, no symlinks, case-insensitive, 2 s
time stamps. The folder is configured for that (`ignorePerms`,
`modTimeWindowS: 2`). Syncthing's v2 database is SQLite, which is on the same
vfat volume; this is the first thing to watch on the Forma (§12).

## 3. Lifecycle

- The **app** owns the process; the launcher does not start it. `App::run`
  starts it when `sync=1` is in the settings, and Settings → Sync → On starts
  it at once. Off stops it.
- A **supervisor thread** (`sigma_core::sync`) runs it: spawn, wait until the
  REST API answers (up to 90 s: the first start generates a key), set up the
  config, then poll. The input loop never waits for it; the app reads the
  latest status on each tick.
- **Stop:** `POST /rest/system/shutdown`, wait up to 8 s for the process,
  then kill it (at once when the API does not answer, as during the first
  start; on the Kobo a kill is followed by `killall syncthing`, because the
  real Syncthing runs a child under a monitor process that a kill of the
  parent would leave behind). This happens when sync is turned off (the page waits for it:
  normally a second or two) and when the app exits (after the note and the
  scratch pad are saved). The launcher also runs `killall -q syncthing`
  before the reboot, in case the app died without cleaning up. A reboot ends
  it in any case.
- **Crash:** if it exits while it should run, the supervisor retries after
  15 s, at most 3 times in a row; then Settings shows the reason and sync
  stays off until the reader switches it again.
- **Suspend and USB.** Σscribe has neither: the power button saves and exits,
  and the launcher reboots into the stock reader, which is what exposes USB
  storage. So "stop on exit" covers both. If the app ever gains a suspend
  path, call `sync_pause()` there; the REST API supports `POST
  /rest/system/pause`.
- **Network.** Sync needs the Wi-Fi the launcher leaves up (PR #89). Without
  it, Syncthing runs and reports no connection; nothing is lost. The title-bar
  Wi-Fi button (`docs/wifi.md`) turns the radio on and off: **Wi-Fi off stops
  Syncthing and Wi-Fi on lets it start again** (with Settings → Sync on; the
  setting itself is not touched).
- **Loopback.** Syncthing's API listens on 127.0.0.1, which needs the `lo`
  interface. With Nickel stopped the Forma's `lo` has no address (found on the
  device, 2026-10-28: "listen tcp 127.0.0.1:8484: bind: cannot assign
  requested address", three starts in a row, then Failed). The launcher now
  gives `lo` 127.0.0.1 when it has none (`ifconfig lo 127.0.0.1 up`, else `ip`)
  and logs `net: lo …` lines; it never takes it down. The supervisor also
  tries to bind 127.0.0.1 before every start: if that fails it does not start
  Syncthing, the page says **Loopback network is down**, and it looks again
  every 15 s. When Syncthing does exit, the last `ERR` line it logged during
  that start is shown on a Reason row under Status (the exit status goes to the `sync:` log), so the next
  failure explains itself on screen.

## 4. The local API

- Listens on `127.0.0.1` only (`--gui-address`), port 8484
  (`SIGMA_SYNC_PORT` overrides).
- The **API key** is 32 random bytes from `/dev/urandom` (hex), made at each
  start and given to Syncthing in the environment (`STGUIAPIKEY`), so it is on
  no disk and in no `ps` line. No GUI password: the port is loopback on a
  single-user device.
- Syncthing is told: no browser, no restart loop (`--no-restart`), no
  upgrade, no port probing, no default folder, no usage reports, no crash
  reports. Its own log goes to `.adds/sigma-scribe/log/syncthing.log` (rotated
  at 256 KB, one old file). The app's side is logged under `sync:` in
  `sigma-scribe.log` (start, stop, status changes, Accept and Ignore, failed
  calls, a note taken in or kept as a conflict); never a note's text or the
  name of a device.
- The app talks plain HTTP/1.0 over a `TcpStream` (no new crate), with a
  5 s timeout and a 512 KB limit on any answer. The JSON is read by a small
  bounded parser (`sigma_core::sync::json`), because the app has no serde.

What the app calls: `GET /rest/system/ping`, `GET /rest/system/status`
(`myID`), `GET /rest/config/devices`, `GET /rest/config/folders`,
`PATCH /rest/config/options` (at every start),
`GET /rest/config/defaults/device` and `…/folder` (the templates Accept
fills in), `POST /rest/config/devices`, `POST /rest/config/folders`,
`PUT /rest/config/folders/<id>`,
`GET /rest/system/connections`, `GET /rest/db/status?folder=`,
`GET /rest/stats/folder`, `GET /rest/cluster/pending/devices`,
`GET /rest/cluster/pending/folders`,
`DELETE /rest/cluster/pending/devices?device=`, `POST /rest/system/shutdown`.

## 5. What is synced

One folder: the notes folder, as a **send & receive** folder with ID chosen by
whoever shares it first (§7).

Syncthing reads `.stignore` there. Σscribe writes it **once**, when the
folder is set up and no `.stignore` exists (an edited one is never touched):

```
// Σscribe: files that belong to this device, or that the app makes and removes.
.trash
.sigma-settings
.sigma-last-opened
.handwriting
.*.tmp
```

Every dotfile the app writes in the notes folder, and the decision:

| File | Written by | Synced? |
|---|---|---|
| `*.md`, `*.txt`, folders | notes | yes |
| `.ink/<note>/…` | ink panel (strokes, SVG), linked from the note | yes |
| `.scratchpad.md` | the scratch pad | yes (it is text of the reader's; see §9) |
| `.sigma-settings` | Settings | **no**: per device (rotation, UI scale, whether sync is on) |
| `.sigma-last-opened` | Library | **no**: per device |
| `.handwriting/` | handwriting samples | **no**: tied to this device's pen |
| `.trash/` | delete (Trash screen, 7.9) | **no**: a deleted note is deleted on the other computers too; use Syncthing's trash-can versioning there if you want a safety net |
| `.<name>.tmp` | atomic save (`write_atomic`) | **no**: exists for milliseconds |
| `.stfolder`, `.stignore`, `.stversions` | Syncthing | its own business; none is listed as a note (the library skips dotfiles) |

Deletes propagate. That is Syncthing's contract and the main thing to tell
the reader (§12).

## 6. Folder and device settings the app writes

At every start:

- Options: `urAccepted: -1`, `startBrowser: false`,
  `autoUpgradeIntervalH: 0`, `crashReportingEnabled: false`,
  `natEnabled: false` (no router probing; relays and local discovery still
  work), the bandwidth limits are left alone. Global discovery and relays stay on,
  the Syncthing defaults, so a computer on another network can still reach
  the Kobo.
- The notes folder, made in one of two ways (§7), always with the same
  settings: `path` = the notes folder, `type: sendreceive`,
  `fsWatcherEnabled: true`, `fsWatcherDelayS: 10`, `rescanIntervalS: 3600`,
  `ignorePerms: true`, `modTimeWindowS: 2`, `maxConflicts: 5`, plus the
  `.stignore` (§5).
  - *Accepting a device* shares the notes folder with it. If a folder is
    already at the notes path the device is added to it (a `PUT` of that
    folder). Otherwise the Kobo makes the folder: a new random ID in
    Syncthing's own style (`xxxxx-xxxxx`, lower-case base32) and the label
    "Σscribe notes". The remote device then sees "… wants to share folder
    Σscribe notes". This is what makes a phone that only scanned the QR code
    (§7) receive the notes: it offers no folder, so without this nothing
    would ever be shared.
  - *Accepting a folder offer* creates the folder with the offered `id` and
    `label`, shared with the offering device, or adds the device if that
    folder is already the notes folder.
  - The folder the Kobo makes itself is remembered (written before it is made, removed if Syncthing refuses it) in
    `syncthing-home/notes-folder-made-here` (its ID). While the notes folder
    is that one, an offer of **any** folder ID is listed, because a computer
    that ticked its own folder when it added the Kobo offers it under its own
    ID; accepting it **replaces** the Kobo's folder: the folder is removed
    from Syncthing's configuration and made again under the offered ID, at the
    same path, with the devices already on it plus the offerer (the marker is
    deleted). Removing a folder from the configuration does not delete its
    files (it only stops syncing it; the REST page says nothing either way,
    Syncthing's remove-folder dialog says files stay, and the app never
    deletes a note itself). If Syncthing refuses the new folder the old one is
    put back. A folder that came from an offer has no marker, and keeps the
    old rule: an offer for another folder ID is not listed, an offer for the
    notes folder's own ID (a second computer) is, and Accept adds that device.
  - *At start*, for devices accepted by a build that shared nothing: if no
    folder is at the notes path and a device other than this one is
    configured, the Kobo makes the folder as above, shared with all of them
    (log `sync: share notes with … (accepted earlier)`). With a folder at
    the notes path, whoever made it, nothing is done. A marker that names no
    folder (a crash, or the folder removed from the computer) is stale and is
    replaced. Removing the folder on the computer therefore brings it back at
    the next start while another device is configured.
  No other folder is ever created.
- A device, when the reader accepts it: `addresses: ["dynamic"]`, not an
  introducer, `autoAcceptFolders: false`.

## 7. Pairing without typing 56 characters on the Kobo

The ID is long (`P56IOI7-MZJNU2Y-IQGDREY-DM2MGTI-MGL3BXN-PQ6W5BM-TBBZ4TJ-XZWICQ2`),
and the Kobo has no copy and paste out. So the Kobo is the one that is
**added**, not the one that adds:

1. Settings → Sync → On. The page shows this device's ID, in two lines of
   four groups, large enough to read across a desk.
2. On the computer, in Syncthing: *Add Remote Device*, type or paste that ID
   (a phone's camera or screenshot-to-text can carry it; dashes optional),
   name it "Forma", tick the notes folder under *Sharing*, Save.
3. The computer connects to the Kobo (local discovery on the same Wi-Fi, or
   the global discovery servers). The Kobo has not heard of it, so
   Syncthing reports a **pending device**. Settings → Sync lists it under
   *Waiting for you* with its name and address and **Accept / Ignore**.
4. Accept adds the device **and shares the notes folder with it** (§6). If
   the computer ticked its own folder in step 2, its offer also shows as a
   **pending folder** (label, who offered it); **Accept** makes that the
   notes folder in place of the Kobo's own (§6) and syncing starts.
   **Ignore** dismisses the offer for now (`DELETE …/pending/…`); it can
   come back on the next connection. If the computer ticked nothing, it sees
   "<Kobo name> wants to share folder Σscribe notes" and accepts that.

**Pairing by scanning.** Under the ID, **Show QR code** opens a full page
with the ID as a QR code (the plain ID string with its dashes, which is what
Syncthing's own GUI encodes and what the Android app's scanner reads). On the
phone: Syncthing app → Devices → + → scan. The phone then adds the Kobo and
the Kobo lists the phone under *Waiting for you*. The whole phone flow:
scan the code, **Accept** the phone on the Kobo (which shares the notes
folder with it), then accept "Σscribe notes" on the phone (it shows the offer from "kobo", the Kobo's Syncthing device name, as a notification and in its Web GUI; until then the Kobo's status says *Waiting for <phone> to accept*, §10). A computer can
be added to the Kobo the usual way (above), or the phone can introduce it. The code is black on white with a 4-module quiet zone whatever
the Night setting, whole pixels per module, up to 14 px: about 39 mm across at
the Forma's 300 dpi (the ID is upper case, digits and dashes, so it fits QR
version 4, 33 modules, at error correction M). It is made by the `qrcodegen`
crate (MIT, no dependencies, one file; chosen over writing an encoder, which
is a few hundred lines to get right). The desktop's Add Device dialog does not
scan; the typed ID above still works. Only after the ID is known does the row
appear. Whether a phone reads it off e-ink at arm's length is for the Forma
checklist (§12).

First sync with notes already on both sides is Syncthing's usual merge: the
newer copy of a file with the same name wins and the other is kept as a
conflict file (§8). Back up both sides first.

## 8. Conflict files

When two computers change the same note before they meet, Syncthing keeps
the newer one under the name and saves the other next to it as
`note.sync-conflict-20261028-141500-ABCDEFG.md`. These are ordinary notes to
Σscribe: they show in the Files list (sorted after the original, as the
names sort), open, edit and delete like any other, and Syncthing syncs them
back to the other computers, so a conflict is visible everywhere. Syncthing
keeps at most 5 per note (`maxConflicts`) and prunes the oldest. The Sync
page counts them ("2 files") so the reader knows to look. No merging is
attempted. The app's own copies (§9) are named `note.conflict-…` and are
counted too; Syncthing does not prune those.

## 9. When notes change underneath the app

Today (before this change) the app read a note once, when it was opened, and
wrote the whole buffer on every edit. A file changed on disk meanwhile was
lost at the next keystroke. With sync that is not acceptable, so:

- The app remembers the **stamp** (modification time and length) of the open
  note and of the scratch pad, and a hash of the text, as of its last read or
  write.
- **Before every save**, if the stamp on disk differs, the file is read:
  - same text as the app last read or wrote (a touch, an identical rewrite):
    nothing to do, the stamp is updated;
  - different: the file was changed by someone else. The reader's buffer is
    **not** written over it. The reader's text goes to a sibling
    `note.conflict-<yyyymmdd-hhmmss>.md` (the same atomic write; `-2`, `-3`
    if the name is taken), the file on disk is loaded into the buffer, and a
    `sync:` line is logged. Nothing is lost either way, and the conflict file
    syncs like any note. There is no on-screen message yet (§13).
  - A weakness: if the file was rewritten with the same modification time
    **and** the same length as the app last saw (FAT keeps 2 s), the stamp
    does not change and the file is not read. A Syncthing delivery carries
    the sender's time, so this needs two equal edits to the second.
- The guard is always on; it costs one `stat` per save.
- While sync is on, **every second** (not every tick) the stamps are
  compared. A changed file whose buffer has no unsaved edit is reloaded in
  place (an open preview is laid out again), keeping the caret as near as it can (the same `take_text` the
  rename-and-relink path uses). Between a keystroke and its save there is no
  gap in which this can lose an edit: the save is synchronous.
- A note **deleted** on disk while open is left alone: the next edit saves it
  again, which is the safe reading (the reader's text survives; the other
  computers get it back).
- The Files list, the drawer's Contents and To-dos and the Graph read the
  disk when opened, so they are current at that moment. They are not live
  while open; a refresh on change is a follow-up.

## 10. Status the page shows

| Row | Source |
|---|---|
| Sync (On / Off) | the setting |
| Status | *Starting…*, *Not installed*, *Stopped: <reason>*, *No devices yet*, *No device connected*, *Not shared yet* (a device is connected but no folder is at the notes path: the folder is being made, could not be made, or the offer was ignored; never "Up to date"), *Waiting for Joe to accept* (a connected device on the folder has not accepted it: Syncthing's `remoteState` is `notSharing`; with several, "Joe +2"; a "Next step" line under Status says to accept "Σscribe notes" in Syncthing on that device), *Syncing 3 files (120 KB)*, *Scanning*, *Up to date*, or the first folder error |
| Devices | `connections`: "1 of 2 connected" |
| Last sync | `stats/folder` → `lastFile.at`, shown as `2026-10-28 14:05` |
| Conflicts | count of `*.sync-conflict-*` and `*.conflict-*` files in the notes folder (a walk of at most 20,000 entries, once a minute, dotted folders skipped) |
| Device ID | `system/status` → `myID` |
| Waiting for you | pending devices, then pending folders, each with Accept / Ignore (at most 4 of each; the rest wait) |

*Waiting for … to accept* comes from `GET /rest/db/completion?folder=<id>&device=<id>`
(Syncthing 1.20 and later; the bundled 2.1.7 has it). Its `remoteState` is
`unknown` (the device is not connected), `paused`, `notSharing` (the device
has not accepted the folder) or `valid` (it shares the folder back); only
`notSharing` is waited for. The call is made once per poll for each device
that is connected and on the notes folder, and only while the folder exists.
The order of the status: errors, no devices, not connected, not shared yet,
syncing, waiting, scanning, up to date.

While Settings → Sync is open the supervisor polls every second, otherwise
every 10 s.

## 11. Battery and memory (Forma: 512 MB, one Cortex-A9, e-ink)

These are estimates from Syncthing's behaviour; the real figures come from
§12.

- **Memory.** An idle Syncthing with a small folder holds roughly 40–70 MB
  resident (Go runtime plus SQLite). The Forma has 512 MB and Nickel is
  stopped, so there is room, but the font and dictionary reads share it. The
  process is started with `GOMEMLIMIT=96MiB` (a soft limit that makes the Go
  collector work harder before growing), `GOGC=50` and `GOMAXPROCS=1`.
  If the folder grows to thousands of files, expect the database to grow
  with it.
- **CPU and battery.** Idle cost is the Wi-Fi radio staying associated and
  small wake-ups: local discovery (UDP broadcast every 30 s), relay and
  discovery keep-alives, the file watcher (inotify, free until something
  changes) and the hourly rescan. The first sync, and any large change, is
  the expensive part: hashing on one slow core. Because sync only runs while
  the app is open, a Kobo left alone in Nickel pays nothing. Turning on the
  Wi-Fi is the larger cost, and it is the launcher's, not sync's.
- **Storage wear.** The database is SQLite on the vfat volume. Syncthing 2
  batches writes; the hourly rescan writes nothing if nothing changed. Watch
  `syncthing-home/` size in the pass.
- **Polling.** The status adds one small local REST call per connected
  device per poll (a few hundred bytes, no disk), so every 10 s with the page
  closed. Negligible next to the rest.
- **Ports.** Syncthing also listens on TCP and QUIC port 22000 on every
  interface (so a computer can connect to the Kobo) and sends UDP broadcasts
  on 21027. On a shared network, the Kobo is visible as a Syncthing device;
  only devices the reader has accepted may connect.

## 12. To try on the Forma

Not done; the session that built this could not run the Syncthing binary and
did not touch the device. For `docs/device-checklist-prerelease.md`:

1. The package contains `syncthing/syncthing` (32 MB) and the notices;
   `file` says ARM, statically linked; `syncthing --version` runs on the
   Forma and prints v2.1.7.
2. Settings → Sync → On: status goes *Starting…* → *No device connected*
   within a minute on the first start (key generation on one core: time it);
   the page shows a 63-character device ID.
3. Add the Kobo on a computer by that ID. The computer appears under
   *Waiting for you* within about a minute on the same Wi-Fi. Accept (this
   also shares the notes folder with it); accept the folder it offers, or the
   Kobo's "Σscribe notes" on the computer (§12.13). Both directions: edit a note on the computer, see it on the
   Kobo; edit on the Kobo, see it on the computer.
4. **The notes folder on vfat**: no permission errors, no endless rescans, no
   case or name errors in `.adds/sigma-scribe/log/syncthing.log`; a note named
   with accents and spaces syncs; `.trash`, `.sigma-settings` and
   `.handwriting` do not appear on the computer.
5. **Open note changes underneath:** with a note open on the Kobo, edit it on
   the computer; within a few seconds the Kobo shows the new text. Edit both
   at once: a `note.conflict-<date>-<time>.md` file appears and nothing is
   lost. Also edit the scratch pad on the computer.
6. Turn Wi-Fi off in Nickel before launching (or walk out of range): the page
   says *No device connected*; no hang, no error dialog.
7. Exit with the power button: the app saves and exits; `syncthing` is not
   running afterwards (`ps`); the launcher log shows no leftover.
8. Measure and write down: resident memory (`/proc/<pid>/status` `VmRSS`) idle
   and during the first sync; time to first status; battery drop over an hour
   idle with sync on and Wi-Fi on, against sync off and Wi-Fi on; the size of
   `syncthing-home/`.
9. TLS: Syncthing's global discovery and relays use HTTPS and need CA
   certificates. Found on the Forma (2026-10-28): `x509: certificate signed by
   unknown authority` for the relays, so the package now ships
   `syncthing/ca-certificates.crt` and the app passes it as `SSL_CERT_FILE`
   (§1). To try: `.adds/sigma-scribe/log/syncthing.log` has no `x509` lines,
   and has `Joined relay` (or a relay listener that stays up) once Wi-Fi is
   on. Local discovery on the same Wi-Fi never needed it.
10. Syncing a few hundred notes the first time: does the app stay responsive
    (typing latency in `perf:` lines) while Syncthing hashes?
12. QR code: Settings → Sync → Show QR code; the Syncthing app on a phone
    (Devices → + → scan) reads it from the Forma's screen at normal reading
    distance, in the room's light, and offers the right device ID. Try Night
    mode too (the code must stay black on white).
13. Sharing on Accept (not yet tried): (a) phone: scan the code, then on the
    Kobo Accept the phone under *Waiting for you*; within about a minute the
    phone shows "<Kobo name> wants to share folder Σscribe notes"; accept it
    there and the notes arrive. `syncthing.log`/`sigma-scribe.log` show
    `sync: share notes with <id>` and `sync: notes folder xxxxx-xxxxx created`;
    `syncthing-home/notes-folder-made-here` holds that ID. (b) computer that
    ticked the notes folder when it added the Kobo: Accept the device, then
    its folder offer appears; Accept it; the log shows
    `sync: notes folder now <id> (offered by …)`, the marker is gone, no note
    was lost. (c) computer that ticked nothing: it is offered "Σscribe notes"
    like the phone. (d) a second device after (a): Accept adds it to the same
    folder. While a device is connected and the folder does not exist yet the
    status is *Not shared yet*, not *Up to date* (found on the Forma
    2026-10-28: it said *Up to date* with no folder).
14. A device accepted before this existed (the phone accepted under 1.15.x)
    and no notes folder: restart sync (Off, On). The log shows
    `sync: share notes with <id> (accepted earlier)` then
    `notes folder … created`; the phone is offered "Σscribe notes".
15. Waiting: with the phone connected and not yet accepting, Settings → Sync
    says *Waiting for <phone> to accept* with the next-step line, not *Up to
    date*; within a poll (10 s, 1 s with the page open) of accepting
    "Σscribe notes" on the phone it changes to *Syncing* or *Up to date*.
16. Expected in `syncthing.log`, harmless: Syncthing warns that it "should not
    run as a privileged or system user"; the Kobo runs everything as root.
11. Loopback (found on the Forma, fix not yet tried there): the launcher log has
    `net: lo has no 127.0.0.1, bringing it up` then a `net: lo: inet …` line;
    `.adds/sigma-scribe/log/syncthing.log` has no "cannot assign requested
    address"; Settings → Sync leaves *Starting* for *No devices yet*. If `lo`
    stays down, the page must say "Loopback network is down" (not "Stopped"),
    and a Stopped status must show Syncthing's own last error under it.

## 13. Not done / later

- Refreshing the Files list and the drawer panes live when a sync lands.
- An on-screen message when a note is reloaded or kept as a conflict, and
  showing which note changed.
- A diff for conflict files.
- Pausing on suspend (no suspend path exists).
- Choosing the bandwidth, or syncing only on power.
- Syncing the handwriting samples between devices (deliberately not today).
