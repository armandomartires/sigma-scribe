---
title: Common problems
summary: Fixes for the problems people meet most often.
order: 1
---

# Common problems

## Korean, Japanese or Chinese shows as empty boxes

The font pack is not installed. Install it as described in [Font packs](../08-keyboards-and-languages/font-packs.md).

## Paste does nothing

Nothing has been copied yet. Σscribe keeps copied text inside the app, so copy or cut something in a note first. See [Select, copy and paste](../03-write-and-edit/select-copy-paste.md).

## The USB keyboard does not work

- The Kobo does not power its USB port. Connect the keyboard through a powered hub or a powered OTG cable.
- Turn on the USB keyboard in the settings, as in [Connect](../09-usb-keyboard-and-mouse/connect.md).

## A computer cannot see the Kobo

While the USB keyboard is on, a computer cannot see the Kobo. Leave Σscribe first. See [Connect](../09-usb-keyboard-and-mouse/connect.md) and [Export](../09-usb-keyboard-and-mouse/export.md).

## Wi-Fi: "Could not join: wrong password? Forget it and try again"

The password was wrong, and it was not kept. Tap the **×** beside the network to forget it, then join it again with the right password. See [Join a network](../10-wifi-and-sync/join-a-network.md).

## Wi-Fi: "Joined, but the network gave no address"

Σscribe waits 40 seconds for an address. If none comes, the network stays saved and Σscribe keeps looking: when an address appears, the message goes and the menu shows the address.

## Sync says "Not shared yet"

A device is connected but the notes folder is not shared with it. Accept "Σscribe notes" in Syncthing on that device. See [Sync status](../10-wifi-and-sync/sync-status.md).

## Sync says "Loopback network is down"

Σscribe could not start Syncthing because the Kobo's local network is down. It looks again every 15 seconds. When Syncthing stops, a **Reason** row under Status shows the last error it logged.

## Σscribe's files show up as books in the Kobo library

See [First start](../01-set-up/first-start.md) for the line Σscribe adds to the Kobo's configuration, why the Kobo must restart for it to count, and how to remove the items.

## Still stuck

Turn on [the log file](the-log-file.md), repeat the problem, and then [report a bug](report-a-bug.md).
