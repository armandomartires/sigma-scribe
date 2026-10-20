#!/bin/sh
# Static ARMv7 device build. Uses gcc-arm-linux-gnueabihf if installed,
# else the user-space musl.cc toolchain in ~/.local/toolchains.
set -eu
T="$HOME/.local/toolchains/armv7l-linux-musleabihf-cross/bin"
if command -v arm-linux-gnueabihf-gcc >/dev/null; then CC=arm-linux-gnueabihf-gcc; AR=arm-linux-gnueabihf-ar
else CC="$T/armv7l-linux-musleabihf-gcc"; AR="$T/armv7l-linux-musleabihf-ar"; fi
CC_armv7_unknown_linux_musleabihf="$CC" AR_armv7_unknown_linux_musleabihf="$AR" \
  cargo build --release --target armv7-unknown-linux-musleabihf -p kobo-sigma-scribe
file target/armv7-unknown-linux-musleabihf/release/sigma-scribe
