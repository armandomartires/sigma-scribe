---
title: Switch plugins on and off
summary: Switch a plugin on or off and see what changes.
order: 1
---

# Switch plugins on and off

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

The release package bundles four plugins (since 1.15.0): `graph-map` (the
Graph) and the drawer's `contents-pane`, `todo-pane` and `scratch-pane`. On a
new install `launch.sh` turns them on (it writes `plugins.state` only when
there is none). The package has no `plugins.state`, so an update keeps your
on/off choices; it does replace the bundled plugins' files with the versions
it ships.
