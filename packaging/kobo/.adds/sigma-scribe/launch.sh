#!/bin/sh
# Sigma Scribe launcher (started by NickelMenu).
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
"$DIR/sigma-scribe" >>"$LOG" 2>&1
echo "=== $(date) exit $?" >>"$LOG"
sync
reboot
