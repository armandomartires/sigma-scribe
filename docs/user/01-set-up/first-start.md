---
title: First start
summary: Open Σscribe for the first time and see what the first start sets up.
order: 4
---

# First start

On the Kobo, open the NickelMenu and tap *Σscribe*. The stock reader stops while Σscribe runs; leaving the app (the power button) saves your note and brings the reader back.

## Keeping the files out of the Kobo library
Since firmware 4.17 the Kobo library also indexes hidden folders, so Σscribe's `.txt` and `.svg` files would show up as books. The first time you start the app, `launch.sh` adds one line to the Kobo's own config (`.kobo/Kobo/Kobo eReader.conf`, section `[FeatureSettings]`, key `ExcludeSyncFolders`; the original is saved next to it as `Kobo eReader.conf.sigma-bak`). It does this only if you have not set that key yourself. The Kobo must be restarted for it to count (the app restarts it when you leave). Items the library already holds are not removed by the line: remove them in Nickel (Settings → Device information is not needed: long-press the item → Remove) or let a later cleanup delete them. A firmware update can reset the config; starting the app again re-adds the line. To undo: delete that line, or copy the `.sigma-bak` file back.

## Plugins
The package includes four plugins: the Graph, and the side drawer's Contents, To-dos and Scratch panes. The first start after a new install turns them all on. Turn any of them off in Settings → Plugins; an update keeps that choice.
