#!/bin/sh
# Run the web-preview wasm module in Wasmtime and require preview_smoke() == 0.
# Usage: ci-smoke.sh path/to/sigma_web_preview.wasm
set -eu
wasm="${1:?usage: ci-smoke.sh <module.wasm>}"
if [ ! -f "$wasm" ]; then
  echo "error: no such wasm module: $wasm" >&2
  exit 1
fi
# preview_smoke has no arguments and returns i32. Wasmtime prints the result.
out=$(wasmtime --invoke preview_smoke "$wasm")
echo "preview_smoke => $out"
# Accept a bare 0, or a typed "(i32.const 0)"-style printout.
case "$out" in
  0|'(i32.const 0)') ;;
  *)
    echo "error: preview_smoke failed (expected 0)" >&2
    exit 1
    ;;
esac
echo "preview:smoke ok"
