<!-- DRAFT (S9-T06, optional). NOT posted anywhere. Posting needs Armando's approval. Device facts marked "confirmed" come from on-device sessions up to v0.4.0. -->
# Kobo Forma notes for developers (from Sigma Scribe)

- **Touch transform** (confirmed v0.1–v0.4): `swap,mirror_x` (KOReader's Kobo default) maps the cyttsp5 panel to the 1440×1920 portrait framebuffer. Lift is reported by any of `ABS_MT_TRACKING_ID=-1`, `BTN_TOUCH=0`, pressure/major 0.
- **Page buttons** (confirmed): `KEY_PAGE_BACK` 193 / `KEY_PAGE_FWD` 194 (with the buttons on the right, 193 is top), power `KEY_POWER` 116.
- **Refresh** (confirmed): FBInk regions must be at least 16×16 px — smaller regions crashed the EPDC path in our v0.2.2 test build; Fast for typing, Partial for paging, Full for sheet/layout changes.
- **Landscape** (confirmed v0.4): rotate in the app and keep the framebuffer portrait; 90° clockwise is the right direction for the buttons-right grip.
- **Running over Nickel**: stop the same process list as `koreader.sh`; reboot on exit.
- **USB host** (method from KOReader, not yet confirmed with our app): `echo host > /sys/kernel/debug/ci_hdrc.0/role` after mounting debugfs; a wired keyboard has worked on this Forma with KOReader. Power must come from a hub/cable.
