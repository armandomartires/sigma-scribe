<!-- DRAFT for MobileRead → E-Book Readers → Kobo Reader → Kobo Developer's Corner. NOT posted. Posting needs Armando's explicit approval and a passing device test. -->
**Title:** Sigma Scribe 1.0 — a Markdown notebook for the Kobo Forma (NickelMenu)

Hi all,

I've released **Sigma Scribe**, a distraction-free Markdown notebook for the **Kobo Forma**. It's a native program (Rust + FBInk) started from **NickelMenu**; your notes are ordinary `.md` files in `sigma-scribe/notes` on the USB drive.

What it does:
- On-screen keyboard (Compact / Full, landscape layouts), touch selection, undo, autosave
- Notes and folders, Markdown preview pages, `[[wiki links]]`, backlinks
- Contents / To-dos / Scratch drawer, Find & Replace
- Paper / Night, two fonts, words-per-line mode, landscape
- USB-OTG keyboard and mouse with a powered hub (experimental)

Tested: **Kobo Forma, firmware 4.38.23697** only. Reports from other models are welcome, but please keep a backup.

Install: install NickelMenu first, then unpack the release tarball onto the Kobo drive's root and eject. Update by unpacking over it (notes are kept). Uninstall: delete `.adds/sigma-scribe` and `.adds/nm/sigma-scribe`.

Download and source (AGPL-3.0): <GitHub release link>
Screenshots: <links to docs/screenshots/v1.0>

Thanks to NiLuJe for FBInk, pgaskin for NickelMenu, and the KOReader folks for documenting Kobo internals.

— Sigma Recon
