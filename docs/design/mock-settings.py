#!/usr/bin/env python3
"""Static mocks for docs/design/settings-redesign.md (proposal, not the app).
1440x1920, UI scale 200 %, pure greys only. Run: python3 docs/design/mock-settings.py"""
from PIL import Image, ImageDraw, ImageFont
W, H = 1440, 1920
PAPER, INK, RULE, MUTE = 240, 28, 195, 89
F = "assets/fonts/AtkinsonHyperlegible-"
big = ImageFont.truetype(F + "Bold.ttf", 56)
lab = ImageFont.truetype(F + "Regular.ttf", 50)
labb = ImageFont.truetype(F + "Bold.ttf", 50)
small = ImageFont.truetype(F + "Regular.ttf", 38)
sec = ImageFont.truetype(F + "Bold.ttf", 38)
OUT = "docs/screenshots/proposal-settings/"
TOP, ROW, PAD = 150, 150, 48

def page(title, back):
    im = Image.new("L", (W, H), PAPER); d = ImageDraw.Draw(im)
    x = PAD
    if back:  # back arrow, 44 px target at 200 %
        d.line([(PAD + 10, 75), (PAD + 60, 75)], fill=INK, width=7)
        d.line([(PAD + 10, 75), (PAD + 34, 51)], fill=INK, width=7)
        d.line([(PAD + 10, 75), (PAD + 34, 99)], fill=INK, width=7)
        x = PAD + 110
    d.text((x, 75), title, font=big, fill=INK, anchor="lm")
    d.line([(W - PAD - 50, 50), (W - PAD, 100)], fill=INK, width=7)  # close X
    d.line([(W - PAD - 50, 100), (W - PAD, 50)], fill=INK, width=7)
    d.line([(0, TOP - 1), (W, TOP - 1)], fill=RULE, width=2)
    return im, d

def chevron(d, y):
    cx = W - PAD - 20
    d.line([(cx - 18, y - 26), (cx + 8, y), (cx - 18, y + 26)], fill=INK, width=6, joint="curve")

def switch(d, y, on):
    x1, x0 = W - PAD, W - PAD - 170
    d.rounded_rectangle([x0, y - 42, x1, y + 42], 42, fill=INK if on else PAPER, outline=INK, width=4)
    kx = x1 - 42 if on else x0 + 42
    d.ellipse([kx - 32, y - 32, kx + 32, y + 32], fill=PAPER, outline=INK, width=4)

def stepper(d, y, val):
    x1 = W - PAD
    for i, s in enumerate(["+", "\u2212"]):
        bx = x1 - 110 - i * 330
        d.rectangle([bx, y - 50, bx + 110, y + 50], outline=RULE, width=3)
        d.text((bx + 55, y), s, font=labb, fill=INK, anchor="mm")
    d.text((x1 - 275, y), val, font=lab, fill=INK, anchor="mm")

def row(d, y, label, hint=None, value=None, kind=None, on=False):
    cy = y + ROW // 2
    if hint:
        d.text((PAD, cy - 22), label, font=lab, fill=INK, anchor="lm")
        d.text((PAD, cy + 36), hint, font=small, fill=MUTE, anchor="lm")
    else:
        d.text((PAD, cy), label, font=lab, fill=INK, anchor="lm")
    if kind == "chevron":
        chevron(d, cy)
        if value:
            d.text((W - PAD - 70, cy), value, font=lab, fill=MUTE, anchor="rm")
    elif kind == "switch":
        switch(d, cy, on)
    elif kind == "stepper":
        stepper(d, cy, value)
    elif kind == "radio":
        cx = W - PAD - 40
        d.ellipse([cx - 34, cy - 34, cx + 34, cy + 34], outline=INK, width=5)
        if on:
            d.ellipse([cx - 18, cy - 18, cx + 18, cy + 18], fill=INK)
    elif value:
        d.text((W - PAD, cy), value, font=lab, fill=MUTE, anchor="rm")
    d.line([(PAD, y + ROW - 1), (W - PAD, y + ROW - 1)], fill=RULE, width=2)
    return y + ROW

def section(d, y, name):
    d.text((PAD, y + 60), name.upper(), font=sec, fill=MUTE, anchor="lm")
    return y + 96

# 1. Root
im, d = page("Settings", False)
y = TOP
for name, val in [("Writing", "Literata, 20"), ("Display", "Paper, 200 %"), ("Keyboard & input", "Full, US"),
                  ("Language", "English"), ("USB & devices", "Off"), ("Help & about", "1.4.1")]:
    y = row(d, y, name, value=val, kind="chevron")
im.save(OUT + "1-settings-root.png")

# 2. Writing
im, d = page("Writing", True)
y = section(d, TOP, "Text")
y = row(d, y, "Typeface", value="Literata", kind="chevron")
y = row(d, y, "Size", value="20", kind="stepper")
y = row(d, y, "Leading", value="1.5", kind="stepper")
y = row(d, y, "Margin", value="28", kind="stepper")
y = section(d, y, "Line length")
y = row(d, y, "Fixed words per line", "Size follows the margin", kind="switch", on=False)
y = row(d, y, "Words per line", value="12", kind="stepper")
y = section(d, y, "Tab key")
y = row(d, y, "Tab key inserts", value="Spaces", kind="chevron")
y = row(d, y, "Tab width", value="4", kind="stepper")
im.save(OUT + "2-writing.png")

# 3. Keyboard & input
im, d = page("Keyboard & input", True)
y = section(d, TOP, "On-screen keyboard")
y = row(d, y, "Keyboard", value="Full", kind="chevron")
y = row(d, y, "Layout", "Also used by USB keyboards", value="US", kind="chevron")
y = row(d, y, "Keyboard tone", value="Follow page", kind="chevron")
im.save(OUT + "3-keyboard-input.png")

# 3b. USB & devices (switch + chevron)
im, d = page("USB & devices", True)
y = row(d, TOP, "USB keyboard & mouse", "Needs a powered hub: the reader does not power the port", kind="switch", on=True)
y = row(d, y, "Scroll direction", value="Traditional", kind="chevron")
y = row(d, y, "Connected", value="K400 Plus", kind=None)
im.save(OUT + "5-usb-devices.png")

# 4. Radio list: Layout
im, d = page("Layout", True)
y = TOP
for i, n in enumerate(["US", "UK", "French (AZERTY)", "German (QWERTZ)", "Spanish", "Portuguese", "Italian"]):
    y = row(d, y, n, kind="radio", on=(i == 0))
d.text((PAD, y + 70), "Applies to the on-screen Full keyboard and USB keyboards.", font=small, fill=MUTE, anchor="lm")
im.save(OUT + "4-layout-radio.png")
print("ok")
