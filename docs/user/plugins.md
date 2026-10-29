# Plugins

Plugins come from one place, the plugins repository, and are installed in the
`plugins` folder of Σscribe's folder on the Kobo (`.adds/sigma-scribe/plugins/`).
The app remembers there which are installed and which are on.

## Switching plugins on and off

Settings → Plugins lists every plugin folder under `.adds/sigma-scribe/plugins/`
that has a valid `plugin.conf` (its id is the folder's name) and gives it an
On/Off switch. The choice is written to `plugins.state` straight away.

- A **screen plugin** (the Graph) takes effect at once for its title-bar
  icon: **off removes the Graph icon from the top menu** (the other icons close
  up, Settings stays flush right) and **on puts it back**. The Graph built
  into the app stays as the fallback if the plugin is on but fails.
- A **data pack** is read when the app starts, so its row says "restart to
  apply".
- A plugin that needs a newer app than this one is listed, greyed and without
  a switch.
- A plugin that is on disk but not yet in `plugins.state` shows as Off; its
  first switch-on adds it to the file.

### A plugin's own settings

A plugin can declare settings in `plugin.conf`, one line each:

    setting=<key>|<Label>|<Option A>,<Option B>[,...]|<Default>

(two to four options, at most eight settings; a bad line is skipped). While
the plugin is **on**, Settings → Plugins shows a row "<Name> · Settings" that
opens the plugin's own page, one choice per setting. While it is **off** the
row is greyed and says to turn the plugin on first, so the page cannot be
reached. A change applies the next time the plugin's screen opens. The Graph declares
**Start on** (Map or List), **Labels** (Auto or All) and **Layout** (Quick or
Best).

The release package bundles four plugins (since 1.15.0): `graph-map` (the
Graph) and the drawer's `contents-pane`, `todo-pane` and `scratch-pane`. On a
new install `launch.sh` turns them on (it writes `plugins.state` only when
there is none). The package has no `plugins.state`, so an update keeps your
on/off choices; it does replace the bundled plugins' files with the versions
it ships.

## Installing from the repository

Settings → Plugins → **Repository** → **Check for plugins** fetches the list of
plugins from the repository and shows what it offers, each with **Install**,
**Update** or "Up to date" (or "Needs a newer app"). It needs Wi-Fi: turn the
Kobo's Wi-Fi on in the Kobo's own reader before Σscribe starts (see
[wifi.md](wifi.md)). The screen waits while a download runs.

Packages are verified before anything is installed, and a failed install leaves
the version you had. A new plugin starts on; an update keeps its on/off choice
and its settings. The repository's list and packages are not signed: plugins
are trusted because they come from the plugins repository over a secure
connection, so install only plugins you want.

Each installed plugin also has a **Remove** row. It is greyed while the plugin
is on, so nothing that is running is taken away; switch the plugin off first.

How the repository, the package format and the checks work, and how to use an
offline mirror, are in [../dev/plugin-api.md](../dev/plugin-api.md#plugin-repository-packages-and-installation).

Plugin kinds, the plugin API, how the host runs plugins and their tests (for plugin authors and developers) are in [../dev/plugin-api.md](../dev/plugin-api.md).
