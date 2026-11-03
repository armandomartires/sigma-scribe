---
title: Plugin settings
summary: Change a plugin's own settings.
order: 2
---

# Plugin settings

A plugin can declare settings in `plugin.conf`, one line each:

    setting=<key>|<Label>|<Option A>,<Option B>[,...]|<Default>

(two to four options, at most eight settings; a bad line is skipped). While
the plugin is **on**, Settings → Plugins shows a row "<Name> · Settings" that
opens the plugin's own page, one choice per setting. While it is **off** the
row is greyed and says to turn the plugin on first, so the page cannot be
reached. A change applies the next time the plugin's screen opens. The Graph declares
**Start on** (Map or List), **Labels** (Auto or All) and **Layout** (Quick or
Best).
