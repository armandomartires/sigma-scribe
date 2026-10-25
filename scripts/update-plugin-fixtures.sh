#!/usr/bin/env bash
# Rebuilds the plugins from a sigma-scribe-plugins checkout and copies each
# into app/tests/fixtures/<id>/, the modules the plugin tests and the
# equivalence tests (app/tests/*_equivalence.rs) load.
# Usage: scripts/update-plugin-fixtures.sh [path/to/sigma-scribe-plugins]
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
repo="${1:-$here/../sigma-scribe-plugins}"
[ -x "$repo/scripts/build-plugin.sh" ] || { echo "not a sigma-scribe-plugins checkout: $repo" >&2; exit 1; }
if [ -n "$(git -C "$repo" status --porcelain)" ]; then
  echo "warning: $repo has uncommitted changes; the fixtures will not match its commit" >&2
fi
for id in graph-map contents-pane todo-pane scratch-pane; do
  "$repo/scripts/build-plugin.sh" "$id"
  entry="$(sed -n 's/^entry=//p' "$repo/plugins/$id/plugin.conf")"
  dest="$here/app/tests/fixtures/$id"
  mkdir -p "$dest"
  cp "$repo/dist/$id/plugin.conf" "$repo/dist/$id/$entry" "$dest/"
  {
    echo "Built from sigma-scribe-plugins commit $(git -C "$repo" rev-parse HEAD)"
    echo "with scripts/build-plugin.sh $id (rustc $(rustc --version | cut -d' ' -f2))."
    echo "Refresh with scripts/update-plugin-fixtures.sh."
    ( cd "$dest" && sha256sum "$entry" )
  } > "$dest/SOURCE.txt"
  cat "$dest/SOURCE.txt"
done
