#!/usr/bin/env bash
# Rebuilds the graph-map plugin from a sigma-scribe-plugins checkout and
# copies it into app/tests/fixtures/graph-map/, the module the plugin tests
# and the equivalence test (app/tests/graph_plugin_equivalence.rs) load.
# Usage: scripts/update-graph-plugin-fixture.sh [path/to/sigma-scribe-plugins]
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
repo="${1:-$here/../sigma-scribe-plugins}"
[ -x "$repo/scripts/build-plugin.sh" ] || { echo "not a sigma-scribe-plugins checkout: $repo" >&2; exit 1; }
if [ -n "$(git -C "$repo" status --porcelain)" ]; then
  echo "warning: $repo has uncommitted changes; the fixture will not match its commit" >&2
fi
"$repo/scripts/build-plugin.sh" graph-map
dest="$here/app/tests/fixtures/graph-map"
mkdir -p "$dest"
cp "$repo/dist/graph-map/plugin.conf" "$repo/dist/graph-map/graph.wasm" "$dest/"
{
  echo "Built from sigma-scribe-plugins commit $(git -C "$repo" rev-parse HEAD)"
  echo "with scripts/build-plugin.sh graph-map (rustc $(rustc --version | cut -d' ' -f2))."
  echo "Refresh with scripts/update-graph-plugin-fixture.sh."
  ( cd "$dest" && sha256sum graph.wasm )
} > "$dest/SOURCE.txt"
cat "$dest/SOURCE.txt"
