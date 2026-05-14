#!/usr/bin/env bash
# Regenerate src-tauri/icons/* from the inline brand SVG.
#
# Pastel-shifted in v0.3.0 — the bundled icons were rendered from the
# pre-v0.3.0 saturated logo and don't match the in-app palette. Run
# this whenever the logo gradient or sound-wave bars change.
#
# Steps:
#   1. Write the brand SVG to a temp file (mirrors src/components/Logo.tsx).
#   2. Render to a 1024x1024 PNG using @resvg/resvg-js (vendored to
#      /tmp by the script the first time it runs — no deps in repo).
#   3. Run `npx @tauri-apps/cli icon` to generate every size + .icns.
#
# Usage: ./scripts/regen-icons.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v node >/dev/null 2>&1; then
  echo "node is required" >&2
  exit 1
fi

# Vendor @resvg/resvg-js into /tmp so we don't pollute the repo or the
# user's global npm. Cheap on rerun thanks to npm's install cache. We
# need a package.json in the dir for `npm install` to land
# node_modules locally instead of walking up to a parent.
RESVG_DIR="/tmp/dicto-icon-deps"
if [[ ! -d "$RESVG_DIR/node_modules/@resvg/resvg-js" ]]; then
  mkdir -p "$RESVG_DIR"
  pushd "$RESVG_DIR" >/dev/null
  [[ -f package.json ]] || npm init -y >/dev/null
  npm install --silent --no-save @resvg/resvg-js
  popd >/dev/null
fi

SVG_FILE="$(mktemp -t dicto-logo).svg"
PNG_FILE="$(mktemp -t dicto-logo).png"
trap 'rm -f "$SVG_FILE" "$PNG_FILE"' EXIT

# Keep this SVG in sync with src/components/Logo.tsx.
cat > "$SVG_FILE" <<'SVG'
<svg width="1024" height="1024" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <rect x="0" y="0" width="100" height="100" rx="22" ry="22" fill="#1F1B18"/>
  <rect x="32" y="35" width="8" height="30" rx="4" fill="#FAF6EF"/>
  <rect x="46" y="22" width="8" height="56" rx="4" fill="#D4894A"/>
  <rect x="60" y="30" width="8" height="40" rx="4" fill="#FAF6EF"/>
</svg>
SVG

node -e "
  const fs = require('fs');
  const { Resvg } = require('$RESVG_DIR/node_modules/@resvg/resvg-js');
  const svg = fs.readFileSync('$SVG_FILE');
  const resvg = new Resvg(svg, { fitTo: { mode: 'width', value: 1024 } });
  fs.writeFileSync('$PNG_FILE', resvg.render().asPng());
"

echo "Rendered logo to $PNG_FILE"
echo "Running tauri icon..."
npx --yes @tauri-apps/cli@latest icon --output src-tauri/icons "$PNG_FILE"

# Also overwrite the 1024 source PNG kept in the repo for posterity.
cp "$PNG_FILE" src-tauri/icons/icon-1024.png

echo "Icons regenerated under src-tauri/icons/."
