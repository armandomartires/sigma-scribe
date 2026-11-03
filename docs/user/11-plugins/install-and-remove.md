---
title: Install and remove plugins
summary: Install a plugin from the repository, or remove one you no longer want.
order: 3
---

# Install and remove plugins

Plugins come from one place, the plugins repository, and are installed in the
`plugins` folder of Σscribe's folder on the Kobo (`.adds/sigma-scribe/plugins/`).
The app remembers there which are installed and which are on.

## Installing from the repository

Settings → Plugins → **Repository** → **Check for plugins** fetches the list of
plugins from the repository and shows what it offers, each with **Install**,
**Update** or "Up to date" (or "Needs a newer app"). It needs Wi-Fi: turn it on with
the Wi-Fi button in the title bar (see [wifi.md](../10-wifi-and-sync/join-a-network.md)). The screen waits while a download runs.

Packages are verified before anything is installed, and a failed install leaves
the version you had. A new plugin starts on; an update keeps its on/off choice
and its settings. The repository's list and packages are not signed: plugins
are trusted because they come from the plugins repository over a secure
connection, so install only plugins you want.

Each installed plugin also has a **Remove** row. It is greyed while the plugin
is on, so nothing that is running is taken away; switch the plugin off first.

How the repository, the package format and the checks work, and how to use an
offline mirror, are in [../dev/plugin-api.md](../../dev/plugin-api.md#plugin-repository-packages-and-installation).

Plugin kinds, the plugin API, how the host runs plugins and their tests (for plugin authors and developers) are in [../dev/plugin-api.md](../../dev/plugin-api.md).
