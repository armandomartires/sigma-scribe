#!/bin/sh
# Fetches the pinned musl.cc ARMv7 cross gcc and builds the pinned FBInk
# static library. Used by GitLab CI (P1-03) before scripts/package-kobo.sh.
#
# Pins (recorded 2026-11-17, Europe/Brussels):
#   * musl.cc armv7l-linux-musleabihf-cross.tgz, SHA-512 from
#     https://musl.cc/SHA512SUMS (tarball last-modified 2021-11-23).
#   * FBInk commit 886f25f1, expanded to the full SHA below (the short
#     SHA is what .github/workflows/ci.yml already uses).
#
# Override MUSL_CC_URL, MUSL_CC_SHA512, FBINK_REPO, FBINK_SHA, or
# FBINK_DIR if needed. The gcc is installed where scripts/build-armv7.sh
# looks: $HOME/.local/toolchains/armv7l-linux-musleabihf-cross.
set -eu

MUSL_CC_URL="${MUSL_CC_URL:-https://musl.cc/armv7l-linux-musleabihf-cross.tgz}"
MUSL_CC_SHA512="${MUSL_CC_SHA512:-1bb399a61da425faac521df9b8d303e60ad101f6c7827469e0b4bc685ce1f3dedc606ac7b1e8e34d79f762a3bfe3e8ab479a97e97d9f36fbd9fc5dc9d7ed6fd1}"
FBINK_REPO="${FBINK_REPO:-https://github.com/NiLuJe/FBInk}"
FBINK_SHA="${FBINK_SHA:-886f25f13368859ad8a899b88d04c26e19cda32e}"

ROOT="${CI_PROJECT_DIR:-$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)}"
CACHE="${CI_KOBO_CACHE:-$ROOT/.ci-tools}"
TOOLCHAINS="${HOME}/.local/toolchains"
FBINK_DIR="${FBINK_DIR:-$HOME/src/FBInk}"
TAR="$CACHE/armv7l-linux-musleabihf-cross.tgz"

fail() { echo "ci-setup-kobo-toolchain: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "needs $1"; }

need curl
need tar
need git
need make
need sha512sum

mkdir -p "$CACHE" "$TOOLCHAINS" "$(dirname "$FBINK_DIR")"

if [ -f "$TAR" ]; then
  GOT=$(sha512sum "$TAR" | cut -d' ' -f1)
  if [ "$GOT" != "$MUSL_CC_SHA512" ]; then
    echo "ci-setup-kobo-toolchain: cached tarball does not match the pin; fetching again" >&2
    rm -f "$TAR"
  fi
fi
if [ ! -f "$TAR" ]; then
  echo "ci-setup-kobo-toolchain: fetching $MUSL_CC_URL"
  curl -fsSL --retry 3 --retry-delay 2 -o "$TAR.part" "$MUSL_CC_URL" || fail "download failed: $MUSL_CC_URL (musl.cc sometimes refuses CI IPs)"
  mv "$TAR.part" "$TAR"
fi
GOT=$(sha512sum "$TAR" | cut -d' ' -f1)
[ "$GOT" = "$MUSL_CC_SHA512" ] || { rm -f "$TAR"; fail "sha512 mismatch for musl.cc tarball: got $GOT"; }

rm -rf "$TOOLCHAINS/armv7l-linux-musleabihf-cross"
tar -xzf "$TAR" -C "$TOOLCHAINS"
GCC="$TOOLCHAINS/armv7l-linux-musleabihf-cross/bin/armv7l-linux-musleabihf-gcc"
[ -x "$GCC" ] || fail "tarball did not contain $GCC"
echo "ci-setup-kobo-toolchain: gcc is $GCC"

git config --global --add safe.directory '*' || true
if [ -d "$FBINK_DIR/.git" ]; then
  git -C "$FBINK_DIR" fetch --recurse-submodules origin
else
  rm -rf "$FBINK_DIR"
  git clone --recursive "$FBINK_REPO" "$FBINK_DIR"
fi
git -C "$FBINK_DIR" checkout --detach "$FBINK_SHA"
GOT=$(git -C "$FBINK_DIR" rev-parse HEAD)
[ "$GOT" = "$FBINK_SHA" ] || fail "FBInk HEAD is $GOT, want $FBINK_SHA"
git -C "$FBINK_DIR" submodule update --init --recursive

# CROSS_TC is the prefix of the musl.cc binaries (armv7l-linux-musleabihf-gcc).
PATH="$TOOLCHAINS/armv7l-linux-musleabihf-cross/bin:$PATH"
export PATH
make -C "$FBINK_DIR" staticlib MINIMAL=true IMAGE=true KOBO=true CROSS_TC=armv7l-linux-musleabihf
[ -f "$FBINK_DIR/Release/libfbink.a" ] || fail "missing $FBINK_DIR/Release/libfbink.a"

export FBINK_DIR
echo "ci-setup-kobo-toolchain: FBInk $FBINK_SHA at $FBINK_DIR"
echo "FBINK_DIR=$FBINK_DIR"
