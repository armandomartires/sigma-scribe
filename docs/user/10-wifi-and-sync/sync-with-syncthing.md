---
title: Sync with Syncthing
summary: Pair the Kobo with a computer or phone and share the notes folder.
order: 2
---

# Sync with Syncthing

Settings → **Sync** runs [Syncthing](https://syncthing.net) on the Forma and
keeps the notes folder in step with the reader's other computers. Nothing
goes through a server of ours: the devices talk to each other, on the local
network when they can, through Syncthing's public relays when they cannot.

Sync is **off** until you turn it on. It runs only while Σscribe is open.

Packaging, internals, the settings the app writes and the notes for trying it on the Forma are in [../dev/sync-design.md](../../dev/sync-design.md).

## Pairing without typing 56 characters on the Kobo

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
4. Accept adds the device **and shares the notes folder with it** (see the [design page](../../dev/sync-design.md#6-folder-and-device-settings-the-app-writes)). If
   the computer ticked its own folder in step 2, its offer also shows as a
   **pending folder** (label, who offered it); **Accept** makes that the
   notes folder in place of the Kobo's own (see the [design page](../../dev/sync-design.md#6-folder-and-device-settings-the-app-writes)) and syncing starts.
   **Ignore** dismisses the offer for now; it can
   come back on the next connection. If the computer ticked nothing, it sees
   "<Kobo name> wants to share folder Σscribe notes" and accepts that.

**Pairing by scanning.** Under the ID, **Show QR code** opens a full page
with the ID as a QR code (the plain ID string with its dashes, which is what
Syncthing's own GUI encodes and what the Android app's scanner reads). On the
phone: Syncthing app → Devices → + → scan. The phone then adds the Kobo and
the Kobo lists the phone under *Waiting for you*. The whole phone flow:
scan the code, **Accept** the phone on the Kobo (which shares the notes
folder with it), then accept "Σscribe notes" on the phone (it shows the offer from "kobo", the Kobo's Syncthing device name, as a notification and in its Web GUI; until then the Kobo's status says *Waiting for <phone> to accept*, see Status below). A computer can
be added to the Kobo the usual way (above), or the phone can introduce it.

First sync with notes already on both sides is Syncthing's usual merge: the
newer copy of a file with the same name wins and the other is kept as a
conflict file (see Conflict files below). Back up both sides first.

## Syncthing follows the radio

With Settings → Sync on, **Wi-Fi off stops Syncthing** and **Wi-Fi on lets it start again**. The
Settings switch itself is not touched, and a radio turned on does not start
what Settings turned off.
