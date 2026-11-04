---
title: Right-to-left text
summary: Type Arabic, Persian, Urdu or Hebrew and see how the editor handles it.
order: 5
---

# Right-to-left text

## Turn it on

The right-to-left layouts are hidden until **Experimental features** is on. See [Experimental features](../12-troubleshooting/experimental-features.md).

1. Open Settings → **Experimental features** and turn it on.
2. Open Settings → **Keyboard & input** → **Layout** and pick **Arabic**, **Persian (Farsi)**, **Urdu (phonetic)** or **Hebrew**.

## What the editor does

- Letters join and marks sit on their letters.
- A line that starts with a right-to-left letter is drawn flush right. Latin words and numbers inside it keep their own order.
- The caret, taps and selections follow what you see on screen.
- **Left** and **Right** swap in a right-to-left line, on the on-screen keyboard and on a USB keyboard.
- Preview draws such a line flush right too.

## Not yet

- The interface itself is not mirrored, and there is no Arabic, Hebrew, Persian or Urdu interface language.
- A line's direction comes from its own first strong letter, not from the whole paragraph.
- Urdu is drawn with the Naskh face, not Nastaliq.
- Backspace deletes one character at a time. The arrows and Delete move over a letter together with its marks.
