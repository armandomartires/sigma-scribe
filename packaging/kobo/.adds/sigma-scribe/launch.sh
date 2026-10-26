#!/bin/sh
# Σscribe launcher (started by NickelMenu).
# Stops the stock reader (Nickel) so it does not draw over us or eat input,
# runs the app, then reboots back into the stock reader. Nothing outside
# /mnt/onboard is written. A crash also ends in the reboot.
DIR=/mnt/onboard/.adds/sigma-scribe
LOG="$DIR/sigma-scribe.log"
[ -f "$DIR/touch.conf" ] && . "$DIR/touch.conf"
export SIGMA_TOUCH="${SIGMA_TOUCH:-swap,mirror_x}"
export SIGMA_NOTES=/mnt/onboard/sigma-scribe/notes
mkdir -p "$SIGMA_NOTES"
{
  echo "=== $(date) start, touch=$SIGMA_TOUCH"
  cat /mnt/onboard/.kobo/version 2>/dev/null; echo
} >>"$LOG"
sync
# Same list KOReader stops (koreader.sh); sickel is Nickel's watchdog.
killall -q -TERM nickel hindenburg sickel fickel strickel fontickel adobehost foxitpdf iink dhcpcd-dbus dhcpcd fmon >/dev/null 2>&1
sleep 2
# Keep Σscribe's files out of the Kobo library (v1.11.1). Nickel indexes
# hidden folders too (firmware 4.17+) unless ExcludeSyncFolders says not to,
# and it rewrites its config when it is running, so the line is added here,
# with Nickel stopped. Done only when the key is not set; the original is
# kept once as "Kobo eReader.conf.sigma-bak".
CONF="/mnt/onboard/.kobo/Kobo/Kobo eReader.conf"
EXCLUDE='ExcludeSyncFolders=(\\.(?!kobo|adobe).+|([^.][^/]*/)+\\..+|sigma-scribe(/.*)?)'
if [ -f "$CONF" ] && ! grep -q '^ExcludeSyncFolders=' "$CONF"; then
  [ -f "$CONF.sigma-bak" ] || cp "$CONF" "$CONF.sigma-bak"
  if grep -q '^\[FeatureSettings\]' "$CONF"; then
    L="$EXCLUDE" awk '{print} /^\[FeatureSettings\]/{print ENVIRON["L"]}' "$CONF" >"$CONF.new"
  else
    { cat "$CONF"; printf '\n[FeatureSettings]\n%s\n' "$EXCLUDE"; } >"$CONF.new"
  fi
  mv "$CONF.new" "$CONF"
  echo "added ExcludeSyncFolders to the Kobo config" >>"$LOG"
fi
# A new install: turn the bundled plugins on (the package has no
# plugins.state, so an update never resets the user's choices). One
# "id=version,enabled" line per plugin folder with a plugin.conf.
STATE="$DIR/plugins/plugins.state"
if [ -d "$DIR/plugins" ] && [ ! -f "$STATE" ]; then
  for c in "$DIR"/plugins/*/plugin.conf; do
    [ -f "$c" ] || continue
    id=$(basename "$(dirname "$c")")
    v=$(sed -n 's/^version=//p' "$c" | head -1)
    [ -n "$v" ] && echo "$id=$v,enabled"
  done >"$STATE.new" && mv "$STATE.new" "$STATE"
  echo "turned on the bundled plugins" >>"$LOG"
fi
"$DIR/sigma-scribe" >>"$LOG" 2>&1
echo "=== $(date) exit $?" >>"$LOG"
sync
reboot
