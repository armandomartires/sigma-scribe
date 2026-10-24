# UI reference — the Kobo's own interface

Photos and screenshots of the stock Kobo UI (Forma, firmware 4.38, Portuguese) and of Kobo's web reader. They are the reference for **how Sigma Scribe chrome should look and behave**: every new screen, control or adjustment starts here, and `docs/ui-rulebook.md` turns what they show into rules.

**The images are not in git.** They are the Kobo's and third parties' artwork (four are press and web captures), one photo had an account e-mail (blurred in the copy kept here). They live in `docs/ui-reference/images/` on the maintainer's machine, listed in `.git/info/exclude`. If you work on a fresh clone, ask for them or take your own photos of a Forma; the notes below describe what to look at.

Photos are 1000 px wide greyscale (`photo-*`); web captures keep their own size (`web-*`).

## Settings pages (the pattern Sigma Scribe's Settings follows)

| Image | What it shows | Take from it |
|---|---|---|
| `photo-settings-date-time.jpg` | Settings → Date and time. Top bar: back arrow left, small centred "Configurações", close × right. Big serif page title, hairline rules between rows, label left, value right in italics. An on/off row uses a **toggle switch** with an upper-case label (ATIVADO). A paragraph of help text under the rows. | Top bar, title size, hairline rows, italic values, help paragraph under rows. |
| `photo-settings-beta-features.jpg` | Settings → Beta features. Only a close × (no back arrow on this page). An intro paragraph, grey upper-case **band** headings, **checkbox + italic "Ativado"** rows, italic action words ("Começar") at the right edge, descriptions that wrap to three lines. | Bands, checkbox rows, text actions, wrapped descriptions. The model for Settings → Experimental features. |
| `photo-settings-accounts.jpg` | Settings → Accounts, "Página 1 de 2" in italics right of the title, and the **scroll bar**: a hairline track down the right margin with a **hollow, rounded thumb** at the top. Rows with small icons; an off toggle ("Desativado"); long grey help text. | The scroll bar (Sigma Scribe's `scrollbar.rs`), the "Page i of n" label, an off switch. |
| `photo-settings-reading-page1.jpg`, `…-b.jpg` | Reading settings, page 1 of 2. Bands, drop-down rows ("Páginas para acabar o capítulo ⌄"), a checkbox row, a note, an illustration, and a **segmented control** ("Defeito" selected = filled dark with white italic text, "Invertido" outlined). | Drop-down rows with a chevron, segmented control (our inline buttons), the thumb at the top of the scroll bar. |
| `photo-settings-reading-page2.jpg` | Reading settings, page 2 of 2: the scroll thumb now low in the track; drop-downs; checkboxes; four **picture choices** where the chosen one has a thick frame. | Thumb position per page, picture choices (a possible landscape-keys chooser). |
| `web-energy-saving.png` | Energy saving and privacy: values like "15 mins ⌄" right-aligned, checkboxes, a disabled row in grey ("Change PIN"). | Disabled rows in grey, drop-down values. |

## Library and store (colour and density of the stock app)

| Image | What it shows | Take from it |
|---|---|---|
| `photo-library-awards.jpg`, `photo-library-statistics.jpg` | More → Statistics / Awards. A status row (time left; light, Wi-Fi, battery, sync, search right), a tab strip with an underlined active tab, **bottom tab bar** (Início, Os meus livros, Descobrir, Mais in bold when active, NickelMenu), stat blocks with large serif numerals, a donut chart, a grey band. | Tab strip and bottom bar language, big numerals for counts. |
| `photo-library-google-drive-files.jpg` | A file list: italic names, sizes and a download arrow at the right, a "Classificar: Nome ⌄" sort row, a "…" menu. | Our Files sheet: sort row, right-aligned meta, row rules. |
| `photo-store-discover.jpg` | Store list: cover thumbnails, serif titles, sans author and upper-case price button, stars, "Leia mais" links. A **vertical scroll bar with ˄ ˅ chevrons** at the right edge. | List density; note that lists show a bar and chevrons. |

## Reading view and panels

| Image | What it shows | Take from it |
|---|---|---|
| `photo-reading-view.jpg` | The reading page with the toolbar: back + title left, icons right (navigate, rotate, light, settings, …), the chapter and progress at the bottom: "PÁGINA 6 DE 196" in capitals, a **slider with a hollow circular thumb** and |< >| ends, a list icon with the chapter name. | Slider look (hairline + hollow thumb), upper-case small captions, toolbar icon order. |
| `photo-reading-brightness-panel.jpg` | The light panel: a popover under the toolbar icon (the icon turns into a **filled dark disc with a white glyph**), "Brilho" title, a **slider with the value (26 %) above the thumb** and a sun at each end, "Luz natural" with an AUTO switch, a second slider (sun ↔ moon), "Hora da cama: 23:00 ⌄", a "Saiba mais" link. | The Backlight slider (nine stops), value over the thumb, popovers, the selected-icon state. |
| `web-font-settings.png` | Font panel: Font Face and Supplemental drop-downs, **Font size, Line spacing, Margins sliders with − and + ends**, a three-way Justification segmented control. | Slider with − / + ends, drop-down boxes, three-way control. |
| `web-beta-features-and-footnotes.png` | Beta features page, a word-lookup popover (selection actions: Highlight, Add Note, Search, Google it; a definition below), and footnote previews with Close / See more buttons. | Selection action menu, dialog buttons. |
| `web-reader-settings.png` | Kobo's web reader Reading Settings popover: Theme and Font rows with ›, an A–A size slider, Page Layout, Margins. | Compact popover rows with a chevron. |

## The Kobo on-screen keyboard (reference for a "Kobo layout" option)

| Image | What it shows |
|---|---|
| `photo-keyboard-lowercase.jpg` | Wi Fi login. **Suggestion strip** on top (`.com .ca .co.jp .co.uk .fr .ne.jp .edu`, an up arrow at the right), a **number row 1 2 3 4 5 6 7 8 9 0 always visible**, then `q…p`, `a…l @`, `⇧ z…m . ⌫`, and a bottom row: `#&+` (symbols), `ÉÀÇ` (accents), a **space bar drawn as a thin underline bracket**, `←` `→` (caret), and a dark filled **action key** (`Join`; Enter elsewhere). Keys are flat: no outlines, plain sans glyphs, a very light wash on a few keys (e-ink ghost of the last taps, not a design). |
| `photo-keyboard-uppercase.jpg` | Shift on: the same layout in capitals. |
| `photo-keyboard-symbols.jpg` | `#&+` layer: `# £ € $ % ^ & * ( )`, `! ¡ ? ¿ < > [ ] = +`, `| \ / « » : ; ' " @`, `~ ¢ ¥ • ° - _ , . ⌫`; the bottom keys become `ABC` and `ÉÀÇ`. |
| `photo-keyboard-accents-upper.jpg`, `…-lower.jpg` | `ÉÀÇ` layer, four rows of accented letters (`Ý Ÿ Š Ž Æ Œ Þ ß IJ Ŀ`, `À Á Â Ä Ã Å È É Ê Ë`, `Ò Ó Ô Ö Õ Ø Ì Í Î Ï`, `⇧ Ù Ú Û Ü Ñ Ç Đ Ł ⌫`); Shift switches upper and lower case; the bottom keys become `#&+` and `ABC`. |

What it means for Sigma Scribe: the Kobo keyboard is a **five-row phone-style board** with three layers (letters, symbols, accents), a context key (Join, Enter) and no modifier keys. Our **Compact** board is the nearest relative (letters, `123`, Shift, Back, Space, Return); our **Full** board is a PC board (Ctrl, Alt, Tab, Esc, arrows, PgUp/PgDn, Home/End). The "Kobo style" option (Settings → Enhanced keyboard off, v1.13) is a keyboard style of its own: number row on top, an accent layer instead of long-press, the underline-bracket space bar, a dark Return key, and nothing else on the board.

## First run, loading and the tour (the Kobo's dialogs and text buttons)

| Image | What it shows | Take from it |
|---|---|---|
| `photo-oobe-language-list.jpg` | Language list in two columns: each language in its own script and italic, hairline rows (Bokmål … 繁體中文 (倉頡)). | Our Interface language list could use two columns and native names; the Kobo ships Zhuyin and Cangjie variants as separate entries. |
| `photo-oobe-welcome.jpg`, `…-b.jpg` | "Welcome to Kobo!": big serif title, a paragraph in the sans, two **text rows** ("Set up over Wi Fi", "Don't have a Wi Fi network?") in italics between hairlines; only a back arrow. | Plain italic rows as actions between rules. |
| `photo-wifi-choose-network.jpg` | A list with a Wi-Fi icon and a lock before each name, hairline rows, a **scroll bar at the right** (hairline + hollow thumb), and two italic text actions at the bottom ("Re-scan", "Add network"). Network names are blurred in our copy. | List rows with icons, bottom actions, the scroll bar again. |
| `photo-oobe-date-time-12h.jpg`, `…-24h.jpg` | "Set date and time": rows with label left and italic value right, a **toggle switch** (OFF = hollow knob left, ON = dark pill, knob right, upper-case state word), a **grey wash button** with an italic label ("Continue"). | The text-button shape: grey wash rectangle, italic label, no outline. Toggle states. |
| `photo-oobe-loading.jpg` | A framed dialog (1 px rule) with "Loading…", a **progress bar** (dark filled part, light grey remainder, flat ends) and a "Cancel" wash button. | Dialog frame and progress bar for slow work (indexing, dictionary load). |
| `photo-home.jpg` | Home: covers with "1% Read / 3 HOURS TO GO" (serif for the number, upper-case sans caption), tiles ("My Books 21 BOOKS", "Take a Quick Tour KOBO FORMA"), rules between blocks, bottom tab bar with the active tab in bold and filled icon. | Two-level captions (title serif, caption upper-case sans), tab bar active state. |
| `photo-quick-tour-1.jpg` … `-9.jpg` | The Quick Tour: top bar (back, centred "Quick Tour", close ×), a line illustration (a darker disc behind it for emphasis on pages 6 and 8), a serif headline, a sans paragraph, **"Page i of 9"** in italics at the bottom centre with ‹ › arrows at the corners; the last page has a "Done" wash button. | A paged tour or help: where the arrows and "Page i of n" go, illustration + headline + paragraph rhythm. A possible first-run tour for Sigma Scribe (keys, panels, Settings) could copy this. |

## Help as a modal dialog (the Kobo's "Help" tour)

| Image | What it shows | Take from it |
|---|---|---|
| `photo-help-dialog-1.jpg` … `-5.jpg` | Settings → Help opens a **modal dialog over the dimmed-free page** (the page behind stays fully drawn, cut off by the frame): a 1 px frame, a close × inside the top right corner, a **dotted circle** holding a line illustration (a darker disc on page 4), a serif headline, a sans paragraph, "Page i of 6" in italics at the bottom centre, ‹ › at the bottom corners (only › on page 1). | The dialog frame and in-dialog close, the dotted circle behind illustrations, paging inside a dialog. Gesture help (tap the sides, press and hold, swipe the left edge) is shown with an illustrated hand. |
| `photo-help-dialog-6.jpg` | "Reading menu": an **icon legend**: a column of icons (Lucide-like outlines: list, skip, page, sun, Aa, move, gear, three dots) each with its name in Literata, a "Done" wash button, "Page 6 of 6". | A legend for our title-bar icons (Files, Preview, Find, Scratch, To-dos, Contents, Graph, Settings, Help) as a Help page: icon left, name right, one per row. |

## How these map to Sigma Scribe

| Kobo | Sigma Scribe |
|---|---|
| Top bar (back, centred "Settings", close) | `draw_options` top bar |
| Page title + "Page i of n" + scroll bar | Settings sub-pages and `crates/render/src/scrollbar.rs` |
| Band | `Item::Band` (upper-case, grey wash) |
| Checkbox + italic "On" | `Kind::Check`; also Experimental features |
| Segmented control | `Kind::Inline` (selected filled) |
| Slider, hollow thumb | `crates/render/src/slider.rs` (stops, thumb) — Backlight, Keyboard size, Size |
| Toggle switch (Date and time, Natural light, parental lock) | not used; checkboxes carry on/off everywhere. A switch could replace them later if the whole app moves. |
| Drop-down row ("value ⌄") | `Kind::Pick`, opens a choice page |
| Grey wash button with an italic label (Continue, Cancel, Done) | not yet; our dialogs use filled/outlined cells. A candidate for confirmations. |
| Progress bar (dark part, light remainder) | not yet |
| On-screen keyboard (5 rows, three layers) | `Style::Kobo` in `crates/core/src/keyboard.rs`, drawn by `draw_kobo_keys` (Settings → Enhanced keyboard off) |

New images go in `images/` with a `photo-` or `web-` prefix and a row in the matching table.
