# Design brief — Writing on a Kobo

**Status:** look-and-feel reference, 20 October 2026. Version 0.1.0 of the mock.  
**Companion file:** `kobo-writing-mock.html` — a self-contained HTML mock of the same design. Open it in a browser. It is the visual source of truth. This brief is the written source of truth.  
**Target:** a Cobalt (Rust) app for the Kobo Libra 2 and Libra Colour. Other Cobalt-supported Kobos are desirable. The stock Kobo reader must stay intact.

If the HTML and this brief ever disagree, follow the HTML for look and the brief for behaviour that the HTML cannot show (persistence, device constraints, what is out of scope).

---

## 1. What this is

A notebook for an e-reader. The page is the product. Chrome is quiet, large, and only there when the hand needs it.

The HTML mock draws a plastic device on a dark desk so the page can be judged at the right proportion. That frame is not part of the app. On the device the app *is* the screen.

The mock is a browser prototype. The real application is a Cobalt app. Do not port the React code. Port the layout, the tokens, the interactions, and the data rules.

### Hardware

- Libra 2: 7", 1264 × 1680 Carta 1200, ~1 GHz single core, 512 MB RAM, capacitive touch, page-turn buttons, no official stylus.
- Libra Colour: same page, plus an MPP stylus and limited colour. Stylus and handwriting are optional and model-specific. Not required for the core writing experience.
- Portrait is 1264 × 1680. Landscape is 1680 × 1264. The app must work in both.
- Design for e-ink: large touch targets, measured text, careful full and partial refreshes, no animation that ghosts, no glow.

### Platform

- Primary: Cobalt. Site https://bandarlabs.github.io/Cobalt/ · repo https://github.com/BandarLabs/Cobalt
- Scaffold with `kobo new`. Iterate in the Cobalt simulator (`kobo dev`, Libra screen profile when available).
- Use ScreenBuilder, lifecycle callbacks, `store()` / `shelf()` for persistence, and measured text primitives.
- Keep callbacks fast. Offload heavy work.
- Alternative prototype path: KOReader, only for early external-keyboard experiments. Cobalt is the final platform.

---

## 2. Visual language

The page should read at arm's length. Nothing should glow. Night mode is the same drawing, inverted — white ink on a black page, not a grey card.

### Colour

Day (paper):

| Role | Hex | Use |
|---|---|---|
| Paper | `#f3f0e8` | Page background |
| Ink | `#1c1c1c` | Text, icons, borders of active controls |
| Rule | `#c9c2b4` | Hairlines, inactive borders |
| Mute | `#5e584e` | Hints, file names under a to-do, placeholders |
| Wash | `#e7e1d4` | Pressed or selected icon fill |
| Gutter | `#e0dcd4` | Extra side margin in landscape when Line is on |

Night:

| Role | Hex |
|---|---|
| Paper | `#111111` |
| Ink | `#ece8df` |
| Rule | `#3c3c3c` |
| Mute | `#a8a294` |
| Wash | `#1c1c1c` |
| Gutter | `#242424` |

The mock's desk (`#2a2622`) and plastic bezel (`#1a1815`) exist only in the browser frame. Do not put them on the device.

### Type

- Reading face: Literata (serif). Fallback on device: the closest available serif, then a system serif.
- UI face: Atkinson Hyperlegible (sans). Fallback: a clear sans.
- Two faces only. Options offers **Literata** and **Atkinson**.
- Title of the mock (the word "Writing" on the desk) is Literata at about 28 px. That title is not on the device.
- Body defaults: 20 px, line height 1.5, margin 28 px.
- Size steps: 16, 18, 20, 22, 26.
- Leading steps: 1.35, 1.5, 1.65, 1.8.
- Margin steps: 16, 22, 28, 36.
- UI labels are small but never the only way to hit a control. Icons are 20 px at a 1.75 stroke, inside a 44 × 44 target.

### Touch and rhythm

- Minimum hit target: 44 × 44 CSS pixels on the page, which is the device pixel on the Libra.
- Hairlines are 1 px solid ink or rule. No drop shadows on the page. No rounded cards. A 4 px corner on an icon button is enough.
- Selected control: ink fill, paper text. Or a 1 px ink border plus wash fill for icon buttons.
- No slide animations. Drawers and sheets appear and disappear. E-ink cannot animate cleanly.
- Prefer partial refreshes for typing and caret movement. Full refresh on theme change, orientation change, and opening a sheet.

---

## 3. Page structure

The screen is a column, always, in both orientations.

```
┌─────────────────────────────────────────┐
│ Title bar          filename    [icons]  │  52 px min
├─────────────────────────────────────────┤
│ Find bar (only when open)               │
├─────────────────────────────────────────┤
│                                         │
│  Page (editor or preview)               │
│     ┌──────────┐                        │
│     │ Drawer   │  optional, one side    │
│     └──────────┘                        │
│                                         │
├─────────────────────────────────────────┤
│ Status: words · line     Paper · saved  │
├─────────────────────────────────────────┤
│ On-screen keyboard (only when Keys on)  │
└─────────────────────────────────────────┘
```

Title bar and status bar stay full width even when Line is on and the page column is locked. Only the writing column narrows.

### Title bar

Left: the current file name. Tap it to rename in place. Enter confirms, Escape cancels, blur confirms. Long names ellipsize.

Right, in this order, 44 px icon buttons:

1. **Files** — folder. Opens the files sheet.
2. **Preview** — eye in write mode, pencil in preview. Toggles raw ↔ rendered Markdown.
3. **Find and replace** — magnifying glass.
4. **Scratch pad** — sticky note.
5. **To-dos** — list with checks.
6. **Contents** — tree. Headings of the current note.
7. **Graph** — connected nodes. Wiki links.
8. **Options** — sliders.
9. **Help** — circle with a question mark.

An active icon has a 1 px ink border and a wash fill. Labels are in `aria-label`; on device, a short tooltip or the sheet title is enough. Do not rely on the icon alone if the meaning is ambiguous — the mock uses icons because the bar is crowded, and the sheets name themselves.

### Status bar

Left: word count, then `· line N` in write mode or `· preview` in preview mode.  
Right: `USB` if the USB keyboard setting is on, then `Paper` or `Night`, then `· saved`, then a **Keys** button in write mode.

**Keys** toggles the on-screen keyboard. It is hidden in preview. The keyboard itself is hidden whenever a full sheet is open.

**Prev** and **Next** on the mock's plastic grip are the device's page-turn buttons. They scroll the current page up and down. They do not change the file.

### Find and replace

Opens under the title bar, full width of the page column. Two fields (Find, Replace) and a close mark, then a tool row:

- Count: `1 of 5`, or `None`, or `Find` when the query is empty.
- **Prev** and **Next** select the match in the editor and scroll it into view.
- **Replace** replaces the current match. **All** replaces every match.
- **Aa** toggles match case. Off by default (case-insensitive).

Ctrl+F / ⌘F opens it. Escape closes it. Opening it while in preview switches to write so the match can be selected. If text is selected and it is a single line, that text seeds the Find field.

Search is the current note only.

---

## 4. The page

### Write mode

A plain textarea. No chrome inside the page. Padding is the margin setting. Font, size, and leading come from Options. Tab inserts either a tab character or N spaces (Markdown default is 4 spaces). Shift+Delete is also backspace.

Caret, selection, and scroll must survive re-renders. Autosave on every change. The status line says `saved` once the write has landed; there is no manual save.

### Preview mode

Rendered Markdown of the same file. The eye becomes a pencil so the way back is obvious. Supported syntax:

- Headings `#` `##` `###` — these are the Contents rows.
- Bold `**` and italic `*`.
- Lists `-` and `1.`, including task lists `- [ ]` and `- [x]`.
- Block quote `>`.
- Horizontal rule `---`.
- Inline code and fenced code blocks.
- Images `![alt](url)` — http(s) and relative paths only.
- Web links `[label](url)` — open outside the app.
- Tables: header, dash row, cells.
- Strike `~~text~~`.
- Wiki links `[[note]]` and `[[note|label]]`. A known note is a solid link. A missing note is dashed and creates the note on tap. The graph is drawn from these links.
- Backlinks at the bottom of the preview: other notes that point here.

Preview is read-only in the mock. Editing happens in write mode. (Open question for the device: whether a tap on preview should drop into write at that line. The mock does not do this.)

### Line (words per line)

Options has a **Line** switch, 8 to 16 words, default 12.

- **Off.** Size and Margin are independent. The user sets them.
- **On.** The app fits the vertical page (1264 px wide). It picks a size and a margin so a line holds the chosen number of words. Changing Size moves Margin to keep the count. Changing Margin moves Size to keep the count. Margin is the flexible one.
- **Rotate to landscape while On.** The page column stays locked at the 1264 px proportion (1264/1680 of the landscape width). The extra pixels are a gutter, slightly darker in day (`#e0dcd4`) and slightly lighter in night (`#242424`). Title bar, status bar, and keyboard stay full width. The type scale follows the locked column so the word count holds.

Measure with the real face, not a guess. Recalculate when the face, the word count, or the orientation changes.

---

## 5. Side drawer

One drawer, on the left or the right (Options → **Drawer**). It covers part of the page, about 48% and at most 320 px. It does not push the page. It does not animate.

Three panes can be open at once. They split the drawer vertically. Order from the top is fixed:

1. **Contents** — headings of the current note. Tap a heading to jump to that line. Empty state: "No headings in this note. Start a line with #."
2. **To-dos** — every Markdown checkbox in every file. Filter button cycles **All → Open → Done**. Checking a box rewrites that line in the file. Tapping the text opens the note and jumps to the line. Each row shows the file name under the text. Done rows are struck through and muted.
3. **Scratch** — a free text area. Placeholder: "A side note. It stays off the page."

If only one pane is open it fills the drawer. If two or three are open they share the height equally, with a 1 px ink rule between them.

### Scratch pad persistence

The scratch pad is a real file, `.scratchpad.md`, not a transient buffer. It stays when the user changes notes, restarts the app, or reboots the device. It is hidden from the file list so it is not opened or deleted by mistake. It cannot be deleted. It is excluded from the graph. Todos inside it still appear in the to-do pane.

---

## 6. Sheets (full-page overlays)

Files, Graph, Options, and Help cover the page. They are not permanent columns. Close with the X, or by toggling the same icon. Opening one closes the others. They sit above the drawer.

### Files

A browser of folders and notes. `.txt` and `.md` only.

- Tap a folder to enter it. A crumb says "Up to …".
- Tap a note to open it and close the sheet.
- **More** on a row reveals Rename, Move, Delete. Delete is armed: the first tap arms, the second confirms. The last real note cannot be deleted.
- **New note** and **New folder** at the bottom. A new folder starts in rename.
- Move shows a list of folders (and the root) and places the item there.
- The current note is marked. Word count is shown under each note.
- Nested folders are allowed.

### Graph

A simple map of notes and `[[wiki]]` links. Tap a node to open that note. The scratch file is not a node. Empty graph is fine.

The mock uses a circle layout. On device, a calm list of notes with their links is acceptable if a drawn graph is too heavy. The links are the point, not the picture.

### Options

Rows, each with a short label and a smaller hint:

| Row | Control | Notes |
|---|---|---|
| Paper | Day / Night | Night is the same page, inverted |
| Type | Literata / Atkinson | Serif for reading, sans for the bus |
| Line | On/Off + stepper 8–16 | See §4 |
| Size | Stepper | Tied to Margin when Line is on |
| Leading | Stepper | Space between lines |
| Margin | Stepper | Room for a thumb |
| Tab key | Tab / Spaces | Markdown default is spaces |
| Tab width | 2 / 4 / 8 | Spaces inserted, or columns a tab stands for |
| Keyboard | Theme / Light / Dark | The on-screen keyboard can differ from the page |
| Drawer | Left / Right | Contents, to-dos, and scratch open from this side |
| USB keyboard | On / Off | Hint: needs a powered hub. The reader does not power the port. |

### Help

A menu of two pages, with breadcrumbs.

- **App info** — version, license, author. Currently "Version 0.1.0. A look-and-feel reference for a notebook on a Kobo reader." License and author are not chosen yet.
- **Markdown reference** — the syntax in §4, each with a source sample and one sentence.

Sub-pages say "Back to Help", and the title is `Help / App info` or `Help / Markdown reference`.

---

## 7. On-screen keyboard

Shown only in write mode, only when Keys is on, and only when no sheet is open. A tap on a key must not steal focus from the editor or the scratch pad: route the key to whichever field is focused.

### Portrait, and landscape Centre

The compact board, same in both:

```
q w e r t y u i o p
a s d f g h j k l
Shift  z x c v b n m  Back
123  Tab  Del  Space  Return
```

`123` swaps to a symbol layer (`1–0`, then `# * - [ ] ( ) \` /`, then `. , ! ? : | ~ >`). `ABC` returns. Shift uppercases letters and maps the usual punctuation; it releases after one key. **Del** is Delete; with Shift held it is Backspace. **Tab** follows the Tab key setting.

### Landscape layouts

A centred row of three buttons, always in the same place: **Split · Full · Centre**. In Full, two more buttons sit at the right edge and do not shift the three: **Arrows** (or **123**) just inside **Pad left** / **Pad right**.

- **Centre.** The compact board, centred. Background still spans the full keyboard bar.
- **Split.** The compact board cut in two, with a clear gap between the halves. Key size stays the same. Left half has Shift and the left letters, plus 123, Tab, Space. Right half has the right letters, Back, Del, Space, Return.
- **Full.** The same compact letter board as Centre (no top number row), plus a pad on the left or the right.
  - Number pad: 7 8 9 / 4 5 6 / 1 2 3 / 0 . and a spacer.
  - Direction pad: ↑ on top, ← → in the middle, ↓ below. **Arrows** / **123** toggles the pad, not the letters.

Keyboard tone (Options): **Theme** follows the page, **Light** is always paper, **Dark** is always night. A user may want the keyboard a different colour from the page.

### USB keyboard

A setting, not a driver, in this version. When on, the status bar shows `USB`. Document the constraint in the app: a powered hub or Y-cable is required, because the reader does not supply VBUS. Reading `/dev/input/event*` is custom platform code; the Cobalt SDK does not expose it yet. Bluetooth is the simpler path on recent models. Do not block the on-screen keyboard on this.

---

## 8. Data

### Notes and folders

- A note has an id, a name, a body, and a folder id (or none).
- A folder has an id, a name, and a parent id (or none).
- Names are cleaned on rename. Opening a wiki link to a missing note creates it.
- Autosave. The whole library persists locally and survives restart and reboot.
- Export path so files can leave the device: USB mass storage, and Syncthing or a similar sync, later. Do not design a cloud account.

### Seed content (for the first run and for tests)

- `field-notes.md` at the root — a short page about the screen, with a wiki link, a task list, and a heading outline.
- `drafts/chapter-01.md` — a short scene with wiki links and open tasks.
- `scratch.txt` at the root — a plain list.
- `.scratchpad.md` — empty, hidden.

### What persists

Theme, face, size, leading, margin, tab kind and width, keyboard tone, keyboard layout, Line on/off and words per line, drawer side, USB flag, Keys on/off, the library, and the scratch file. Find query does not need to persist. Which drawer panes are open may reset on launch.

---

## 9. E-ink rules for the implementer

- Large targets. If a control is under 44 px, make it larger or give it a label.
- Measured text. Do not guess line length.
- Partial refresh for keystrokes and caret. Full refresh for theme, orientation, and opening a sheet.
- No animation. The drawer toggles. Do not slide it.
- No colour meaning. Day and night are the only themes. Colour on the Libra Colour is optional decoration, never the only signal.
- Offline first. The app must be fully usable with no network.
- Do not replace or break the stock Kobo reader. Cobalt keeps it intact and runs this app in an isolated process.
- Avoid heavy computation on the UI thread. Parsing a long note for the outline or the to-do list should not block input.

---

## 10. Build order

Match the mock, in this order. Stop and compare with the HTML at the end of each step.

1. Single-file editor, on-screen keyboard, autosave. Title bar with the file name. Status bar with words, line, theme, Keys.
2. File list that opens `.txt` and `.md`. Create, rename, move, delete, folders.
3. Markdown preview. Eye ↔ pencil. The syntax in §4.
4. Options: Paper, Type, Size, Leading, Margin, Tab, keyboard tone, USB flag.
5. Line (words per line) with the locked 1264 px column in landscape.
6. Side drawer: Contents, then To-dos, then Scratch, with the side setting.
7. Find and replace.
8. Wiki links, backlinks, graph.
9. Help: app info and the Markdown reference.
10. External keyboard (USB host + HID, or Bluetooth). Document the powered-hub requirement.
11. Package, sign, and test on a Libra 2 and a Libra Colour.

---

## 11. Out of scope

- Replacing the stock reader.
- A cloud account or an always-on sync service. Local files plus a practical export path are enough.
- Stylus handwriting as a requirement. Optional later, Libra Colour only.
- Animation, blur, shadows, or translucent panels on the page.
- A permanent narrow sidebar. Overlays and the drawer only.
- Tiny icon-only controls with no reachable name.

---

## 12. How to use the HTML mock

Open `kobo-writing-mock.html` in a browser. It needs no server and no network (the fonts are embedded).

- **Vertical / Horizontal** on the desk switches the device frame. That control is not in the app.
- **Prev / Next** on the plastic grip are the page-turn buttons.
- Everything inside the screen is the app.

Judge spacing, type size, and contrast against this file, not against a phone screenshot. The frame is proportioned to 1264 × 1680 and 1680 × 1264.

---

## 13. Open decisions

- License and author name. Help currently says both are unset.
- Whether preview can be edited in place, or only by switching back to write.
- Graph on device: drawn map, or a list of notes and their links.
- Exact export path on device (USB folder, Syncthing, both).
- Bluetooth keyboard versus USB host, and when.

Do not invent answers for these. Leave them as settings or as later work, and keep the mock's behaviour until a decision is written down.
