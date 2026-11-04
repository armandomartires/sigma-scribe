---
title: Sync status
summary: Read the status rows on the Sync page.
order: 4
---

# Sync status

## Status the page shows

| Row | What it shows |
|---|---|
| Sync (On / Off) | the setting |
| Status | *Starting…*, *Not installed*, *Stopped: <reason>*, *No devices yet*, *No device connected*, *Not shared yet* (a device is connected but the notes folder is not shared with it yet: it is being set up, could not be set up, or the offer was ignored; never "Up to date"), *Waiting for Joe to accept* (a connected device has not yet accepted the folder; with several, "Joe +2"; a "Next step" line under Status says to accept "Σscribe notes" in Syncthing on that device), *Syncing 3 files (120 KB)*, *Scanning*, *Up to date*, or the first folder error |
| Devices | how many paired devices are connected, for example "1 of 2 connected" |
| Last sync | the last time a file arrived, or the last time a connected device had everything this one has, whichever is later, for example `2026-10-28 14:05`, in UTC. If nothing is waiting to travel and a device is connected and up to date, it is effectively "now". See [Last sync](../../dev/sync-design.md#10-status-the-page-shows) in the design page for how it is worked out |
| Conflicts | how many conflict files are in the notes folder |
| Device ID | this Kobo's Syncthing ID, to enter on the other device |
| Waiting for you | devices and folders offered to this Kobo, each with Accept / Ignore (at most 4 of each are shown; the rest wait) |
