# Sigma Scribe test notes pack (v1.0)

Copy the `_test` folder into `sigma-scribe/notes/` on the Kobo (USB, from the PC) so it lands at `sigma-scribe/notes/_test/`. Your own notes are not touched. Remove the folder to uninstall. Installing it on the device needs separate approval.

- `test-00-start-here.md`: index; every feature with the note that exercises it
- `test-markdown-tour.md`: all Markdown the preview renders
- `test-find-50-lamps.md`: exactly 50 "lamp" (48 lower case) for Find, Aa, Replace All and Undo
- `test-todos.md`, `test-todos-other.md`: open/done boxes across two notes (drawer To-dos, filter, cross-file toggle)
- `test-headings.md`: deep heading tree (drawer Contents, jump)
- `test-long-note.md`: ~12 preview pages (page buttons, wheel, scrolling)
- `wiki/`: links between notes, aliases `[[a|label]]`, a link to a missing note, backlinks, an orphan
- `folders/a/b/`: nested folders (Files, move, rename)
- `test-unicode.md`: accents (FR/PT/DE/ES) and RTL/CJK sample lines for future layouts
- `graph-300/` (optional): 300 linked notes for Graph map performance; every tenth is an orphan

All note file names start with `test-` so wiki links never resolve to your own notes.
