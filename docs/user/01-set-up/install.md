---
title: Install
summary: Install Σscribe on a Kobo Forma from the GitHub release, with NickelMenu.
order: 1
---

# Install

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
