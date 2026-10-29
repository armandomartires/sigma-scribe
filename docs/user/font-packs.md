# Script font packs

Literata and Atkinson cover Latin, Greek, Cyrillic and Vietnamese. Other scripts (Korean, Japanese, Chinese) need an optional **font pack**: a single font file that Σscribe loads when it starts and uses for any character the built-in fonts lack.

## Where to put it

Copy the file to `.adds/sigma-scribe/fonts/` on the Kobo (the folder next to Σscribe's other files). One file serves Korean and Japanese: name it `cjk.ttf` (the app also accepts `korean.ttf`). Restart Σscribe to load it.

Without a font pack, characters in those scripts show as empty boxes.

How the pack is built, where the app looks for it and how to make your own are in [../dev/font-pack-build.md](../dev/font-pack-build.md).
