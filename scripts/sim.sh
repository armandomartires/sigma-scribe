#!/bin/sh
# Desktop simulator for the Kobo Forma (1440x1920), live window.
#   mouse = touch · PageUp/PageDown = page-turn buttons · keyboard = USB keyboard
#   Esc = power button (save and quit)
# Options:  --eink     e-ink look (black flash on full refresh, light ghosting)
#           --height N window height in px (panel is scaled to fit; default 960)
#           --headless SCRIPT [SHOTS_DIR]   no window: run an emulator script, write PNGs
# Notes are plain .md files in ./target/emu-notes (override with SIGMA_NOTES).
set -eu
cd "$(dirname "$0")/.."
while [ $# -gt 0 ]; do
  case "$1" in
    --eink) export SIGMA_EINK=1 ;;
    --height) shift; export SIGMA_SIM_HEIGHT="$1" ;;
    --headless) shift; exec cargo run -q --bin sigma-emu -- --script "$1" --shots "${2:-target/shots}" ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
  shift
done
exec cargo run -q --bin sigma-scribe
