#!/usr/bin/env bash
# Print the SHA-256 of each model .bin in the resources dir. Use the output to
# populate src-tauri/src/model/manifest.rs.
set -euo pipefail

DIR="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources/models"
if [[ ! -d "$DIR" ]]; then
  echo "No models directory found at $DIR"
  exit 1
fi

for f in "$DIR"/*.bin; do
  [[ -e "$f" ]] || continue
  echo -n "$(basename "$f")  "
  shasum -a 256 "$f" | awk '{print $1}'
done
