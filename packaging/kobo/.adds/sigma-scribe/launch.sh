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
# Same list KOReader stops (koreader.sh), less dhcpcd: Wi-Fi left on in
# Nickel stays up for Settings -> Plugins -> Check for plugins (the exit
# reboots anyway). sickel is Nickel's watchdog.
killall -q -TERM nickel hindenburg sickel fickel strickel fontickel adobehost foxitpdf iink dhcpcd-dbus fmon >/dev/null 2>&1
sleep 2
# Loopback. With Nickel stopped the Forma's `lo` can be left without
# 127.0.0.1, and Syncthing (Settings -> Sync) then cannot bind its local API
# ("cannot assign requested address"). Bring it up when it has no address;
# never take it down. busybox ifconfig, else ip.
{
  if ifconfig lo 2>/dev/null | grep -q '127\.0\.0\.1'; then
    echo "net: lo already has 127.0.0.1"
  else
    echo "net: lo has no 127.0.0.1, bringing it up"
    ifconfig lo 127.0.0.1 netmask 255.0.0.0 up 2>&1 ||
      { ip addr add 127.0.0.1/8 dev lo 2>&1; ip link set lo up 2>&1; }
  fi
  ifconfig lo 2>&1 | grep -E 'inet |UP' | sed 's/^ */net: lo: /'
} >>"$LOG" 2>&1
# The network as the app finds it (Nickel sets INTERFACE and WIFI_MODULE).
# The app's Wi-Fi button (docs/dev/wifi-design.md) uses both, through wifi.sh, to turn
# the radio on and off: keep them for the app, with the Forma's values when
# the launcher was not started from Nickel.
export INTERFACE="${INTERFACE:-eth0}" WIFI_MODULE="${WIFI_MODULE:-8192es}"
export SIGMA_WIFI_DIR="$DIR"
{
  echo "net: interface ${INTERFACE:-unset}, module ${WIFI_MODULE:-unset}"
  ifconfig "${INTERFACE:-wlan0}" 2>&1 | grep -E 'inet |Link|UP' | sed 's/^ */net: /'
  for p in wpa_supplicant dhcpcd udhcpc; do pidof "$p" >/dev/null && echo "net: $p running"; done
  grep -m1 nameserver /etc/resolv.conf | sed 's/^/net: /'
} >>"$LOG" 2>&1
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
# Settings -> Sync: the app stops Syncthing itself; this is for a crash that
# left it running (a reboot would end it anyway, but not before the sync).
killall -q syncthing >/dev/null 2>&1
sync
reboot
