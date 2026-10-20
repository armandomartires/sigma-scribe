#!/bin/sh
# Run v0.x in the Cobalt browser simulator (no Forma profile yet: Libra 2, same 3:4).
cd "$(dirname "$0")/../app" && KOBO_SIM_PROFILE="${KOBO_SIM_PROFILE:-libra-2-388}" exec kobo dev "${1:-127.0.0.1:8787}"
