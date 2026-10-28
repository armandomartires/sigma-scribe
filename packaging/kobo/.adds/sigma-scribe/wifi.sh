#!/bin/sh
# Σscribe Wi-Fi helper, run by the app (never by hand needed):
#   wifi.sh on      load the driver, bring the interface up, start wpa_supplicant
#   wifi.sh off     stop DHCP and wpa_supplicant, bring the interface down, unload
#   wifi.sh dhcp    ask for an address on the network just joined
# After KOReader's platform/kobo/enable-wifi.sh, disable-wifi.sh and
# obtain-ip.sh, which do the same on the same hardware. Output is
# `wifi: ...` lines, which the app puts in its log; nothing here prints a
# network name or a key.
#
# The interface and the driver come from the stock reader's environment
# (INTERFACE, WIFI_MODULE, PLATFORM), which launch.sh keeps (the Forma:
# eth0, 8192es). The saved networks are the stock reader's own file,
# /etc/wpa_supplicant/wpa_supplicant.conf, so what it knew is known here and
# what is added here is known to it. SIGMA_WPA_DRIVER (default wext) and
# SIGMA_WPA_CONF override the driver flag and the file.
exec 2>&1
IF="${INTERFACE:-eth0}"
MOD="${WIFI_MODULE:-8192es}"
DRV="${SIGMA_WPA_DRIVER:-wext}"
CONF="${SIGMA_WPA_CONF:-/etc/wpa_supplicant/wpa_supplicant.conf}"
CTRL=/var/run/wpa_supplicant
say() { echo "wifi: $*"; }

# The .ko of a module: under /drivers/$PLATFORM/wifi on the Kobo.
find_ko() {
  for d in "/drivers/${PLATFORM:-none}/wifi" /drivers/*/wifi; do
    [ -f "$d/$1.ko" ] && { echo "$d/$1.ko"; return 0; }
  done
  return 1
}
loaded() { grep -q "^$1 " /proc/modules 2>/dev/null; }

dhcp() {
  if pidof dhcpcd >/dev/null 2>&1; then
    # The stock reader's dhcpcd is still running: have it ask again.
    dhcpcd -n "$IF" >/dev/null 2>&1 && say "dhcpcd rebound" || say "dhcpcd rebind failed"
  elif command -v dhcpcd >/dev/null 2>&1; then
    dhcpcd -d -t 30 -w "$IF" >/dev/null 2>&1 && say "dhcpcd started" || say "dhcpcd failed"
  elif command -v udhcpc >/dev/null 2>&1; then
    udhcpc -S -i "$IF" -s /etc/udhcpc.d/default.script -t 5 -T 3 -n -q >/dev/null 2>&1 \
      && say "udhcpc answered" || say "udhcpc failed"
  else
    say "no DHCP client found"
    return 1
  fi
}

on() {
  if [ ! -d "/sys/class/net/$IF" ]; then
    # The power switch module first, where there is one.
    if k=$(find_ko sdio_wifi_pwr) && ! loaded sdio_wifi_pwr; then
      insmod "$k" || say "insmod sdio_wifi_pwr failed"
    fi
    if k=$(find_ko "$MOD"); then
      loaded "$MOD" || insmod "$k" || say "insmod $MOD failed"
    else
      say "no $MOD.ko under /drivers"
      return 1
    fi
    n=0
    while [ ! -d "/sys/class/net/$IF" ] && [ "$n" -lt 20 ]; do sleep 0.5; n=$((n + 1)); done
    [ -d "/sys/class/net/$IF" ] || { say "$IF did not appear"; return 1; }
  fi
  ifconfig "$IF" up || { say "ifconfig up failed"; return 1; }
  if ! pidof wpa_supplicant >/dev/null 2>&1; then
    grep -q '^update_config=1' "$CONF" 2>/dev/null || say "warning: $CONF has no update_config=1, new networks will not be saved"
    wpa_supplicant -D "$DRV" -s -i "$IF" -O "$CTRL" -c "$CONF" -B || { say "wpa_supplicant failed to start"; return 1; }
  fi
  n=0
  while [ ! -S "$CTRL/$IF" ] && [ "$n" -lt 20 ]; do sleep 0.5; n=$((n + 1)); done
  [ -S "$CTRL/$IF" ] || { say "no control socket"; return 1; }
  say "on ($IF, $MOD, driver $DRV)"
}

off() {
  killall -q udhcpc 2>/dev/null
  if pidof dhcpcd >/dev/null 2>&1; then
    dhcpcd -k "$IF" >/dev/null 2>&1
    killall -q dhcpcd 2>/dev/null
  fi
  killall -q wpa_supplicant 2>/dev/null
  n=0
  while pidof wpa_supplicant >/dev/null 2>&1 && [ "$n" -lt 10 ]; do sleep 0.5; n=$((n + 1)); done
  ifconfig "$IF" down 2>/dev/null
  loaded "$MOD" && { rmmod "$MOD" || say "rmmod $MOD failed"; }
  loaded sdio_wifi_pwr && rmmod sdio_wifi_pwr 2>/dev/null
  say "off"
}

case "${1:-}" in
  on) on ;;
  off) off ;;
  dhcp) dhcp ;;
  *) echo "usage: wifi.sh on|off|dhcp" >&2; exit 2 ;;
esac
