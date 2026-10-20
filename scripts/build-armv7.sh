#!/bin/sh
# Static ARMv7 (Kobo) build of sigma-scribe with FBInk.
# Needs: rustup target armv7-unknown-linux-musleabihf; an ARM gcc (Debian
# gcc-arm-linux-gnueabihf, or the musl.cc toolchain in ~/.local/toolchains);
# FBInk built as a static lib:
#   cd ~/src/FBInk && make staticlib MINIMAL=true IMAGE=true KOBO=true CROSS_TC=armv7l-linux-musleabihf
set -eu
T="$HOME/.local/toolchains/armv7l-linux-musleabihf-cross/bin"
if [ -x "$T/armv7l-linux-musleabihf-gcc" ]; then CC="$T/armv7l-linux-musleabihf-gcc"; AR="$T/armv7l-linux-musleabihf-ar"
else CC=arm-linux-gnueabihf-gcc; AR=arm-linux-gnueabihf-ar; fi
export CC_armv7_unknown_linux_musleabihf="$CC" AR_armv7_unknown_linux_musleabihf="$AR"
export FBINK_DIR="${FBINK_DIR:-$HOME/src/FBInk}"
test -f "$FBINK_DIR/Release/libfbink.a" || { echo "missing $FBINK_DIR/Release/libfbink.a" >&2; exit 1; }
cargo build --release --target armv7-unknown-linux-musleabihf -p sigma-scribe --bin sigma-scribe
file target/armv7-unknown-linux-musleabihf/release/sigma-scribe
