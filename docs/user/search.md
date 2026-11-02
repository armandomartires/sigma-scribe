# Search

The **Find** bar (the magnifier in the title bar) searches the open note, or all your notes at once. A switch at the left end of the bar picks which: **This note** or **All notes**. Find always opens on This note.

## This note

As before: the matches are marked on the page, **‹** and **›** step through them, and **Replace** and **Replace all** change the text ([getting-started.md](getting-started.md)).

## All notes

Tap **All notes** and type. Under the bar, a list of matching lines takes the place of the page.

<!-- screenshot: docs/screenshots/v1.19/search-list.png -->

- Each row shows the note's path and the line number, and below it the line with the hit in bold.
- Rows are grouped by note, in the order of the Files list.
- The line above the list says how many matches there are, in how many notes, and the page you are on (`12 matches in 4 notes · page 1 of 2`). Turn pages with the page buttons, as in the Graph list.
- With nothing typed, the list is empty and says how many notes there are to search.
- Search runs in the background, so you can keep typing. A new letter replaces the search in progress.

### What is searched

Every note under your notes folder, in all its subfolders, including the [journal](journal.md). The **Trash** (`.trash/`), the handwriting folder (`.ink/`) and hidden files are skipped. The open note is searched as it is on screen, not as last saved.

### Match cap

The list holds at most **500 matches**. If there are more, the line above it says "first 500 shown"; type more of the word to narrow it.

### Case

The **Aa** option is the same one Find uses: off ignores upper and lower case, on matches them exactly. It applies to both scopes.

### Replace

Replace is off in All notes. Changing text in many notes at once cannot be undone across files, so Replace works in This note only.

## Opening a result

- **Tap a row** to open that note in Write mode, with the caret on the match and the page scrolled to it.
- **Back** (the arrow in the title bar) returns to the results, with the same query and page.
- **Enter** opens the first result.
- **Esc** closes the Find bar.

### USB keyboard

With a USB keyboard ([usb-keyboard.md](usb-keyboard.md)): **Up** and **Down** move the selection in the list, **Enter** opens the selected row (the first one until you move), **Esc** closes the bar.

## Handwriting

Search reads text. Ink you have written with the Write panel ([handwriting.md](handwriting.md)) is found once it has been inserted as text; ink that was never recognised is not searched.

## Limits

- No regular expressions and no fuzzy matching: the query is plain text.
- No search inside the Trash.
- No replace across notes.
