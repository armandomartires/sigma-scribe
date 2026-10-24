# Install, update and uninstall (users)

Tested on: **Kobo Forma, firmware 4.38.23697**. Other Kobo models are untested.

## What you need
- NickelMenu installed (from its own release: https://github.com/pgaskin/NickelMenu/releases).
- `sigma-scribe-<ver>-kobo.tar.gz` from the GitHub release, and its `.tar.gz.sha256`.

## Install
1. Check the download: `sha256sum -c sigma-scribe-<ver>-kobo.tar.gz.sha256`.
2. Connect the Kobo over USB and tap *Connect*.
3. Unpack onto the root of the KOBOeReader drive: `tar -xzf sigma-scribe-<ver>-kobo.tar.gz -C /path/to/KOBOeReader` (this only adds `.adds/sigma-scribe/` and `.adds/nm/sigma-scribe`).
4. Eject safely. The Kobo restarts its library; open NickelMenu → *Σscribe*.

Leaving the app (power button) saves and restarts the Kobo back into the normal reader.

## Keeping the files out of the Kobo library
Since firmware 4.17 the Kobo library also indexes hidden folders, so Σscribe's `.txt` and `.svg` files would show up as books. The first time you start the app, `launch.sh` adds one line to the Kobo's own config (`.kobo/Kobo/Kobo eReader.conf`, section `[FeatureSettings]`, key `ExcludeSyncFolders`; the original is saved next to it as `Kobo eReader.conf.sigma-bak`). It does this only if you have not set that key yourself. The Kobo must be restarted for it to count (the app restarts it when you leave). Items the library already holds are not removed by the line: remove them in Nickel (Settings → Device information is not needed: long-press the item → Remove) or let a later cleanup delete them. A firmware update can reset the config; starting the app again re-adds the line. To undo: delete that line, or copy the `.sigma-bak` file back.

## Update
Same as install: unpacking over an older version replaces the program files only. Your notes in `sigma-scribe/notes/` (and the hidden settings and scratch files there) are not in the package and are kept. If you edited `.adds/sigma-scribe/touch.conf`, back it up first — the package contains the default.

## Uninstall
Delete `.adds/sigma-scribe/` and `.adds/nm/sigma-scribe` from the drive, eject. Your notes stay in `sigma-scribe/notes/` until you delete that folder yourself. Nothing else on the Kobo is changed.

## Getting your notes off (export)
Notes are plain Markdown files: connect over USB and copy `sigma-scribe/notes`.

## USB keyboard and mouse
Options → USB keyboard On, then a keyboard / mouse through a **powered** hub or a powered OTG cable (the Kobo does not power its port). While it is on, a computer cannot see the Kobo; leave the app first. US layout only. See Help → USB keyboard & export.
