#!/usr/bin/env bash
# Bump version, commit, tag, push. Triggers .github/workflows/release.yml.
#
# Usage: ./scripts/release.sh 0.1.1
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version>   e.g. $0 0.1.1" >&2
  exit 1
fi

NEW="$1"
if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "Error: '$NEW' is not a SemVer string (e.g. 0.1.1, 0.2.0-beta.1)" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Guard: must be on main, clean tree, up to date with origin.
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$BRANCH" != "main" && "$BRANCH" != claude/* ]]; then
  echo "Error: refusing to release from branch '$BRANCH'. Switch to main." >&2
  exit 1
fi
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree is dirty. Commit or stash first." >&2
  exit 1
fi
if git rev-parse "v$NEW" >/dev/null 2>&1; then
  echo "Error: tag v$NEW already exists. Pick a different version." >&2
  exit 1
fi

echo "→ Bumping to $NEW in package.json, Cargo.toml, tauri.conf.json"
# package.json
node -e "const p=require('./package.json');p.version='$NEW';require('fs').writeFileSync('./package.json',JSON.stringify(p,null,2)+'\n')"
# Cargo.toml — only the [package] version, not dependency versions
sed -i.bak -E '1,/^\[/{ s/^(version *= *)"[^"]+"/\1"'"$NEW"'"/; }' src-tauri/Cargo.toml && rm src-tauri/Cargo.toml.bak
# tauri.conf.json — replace top-level version key
node -e "const fs=require('fs');const p='src-tauri/tauri.conf.json';const c=JSON.parse(fs.readFileSync(p));c.version='$NEW';fs.writeFileSync(p,JSON.stringify(c,null,2)+'\n')"

# Cargo.lock — refresh just the dicto entry without rebuilding the world.
(cd src-tauri && cargo update -p dicto --precise "$NEW" 2>/dev/null || cargo check >/dev/null 2>&1 || true)

echo "→ Verifying versions match"
PKG="$(node -p "require('./package.json').version")"
CARGO="$(grep -E '^version' src-tauri/Cargo.toml | head -1 | sed -E 's/.*"(.+)".*/\1/')"
CONF="$(node -p "require('./src-tauri/tauri.conf.json').version")"
if [[ "$PKG" != "$NEW" || "$CARGO" != "$NEW" || "$CONF" != "$NEW" ]]; then
  echo "Error: version mismatch after bump (package.json=$PKG cargo=$CARGO conf=$CONF)" >&2
  exit 1
fi
echo "  package.json   = $PKG"
echo "  Cargo.toml     = $CARGO"
echo "  tauri.conf.json= $CONF"

echo
echo "→ Add a CHANGELOG.md entry now, then re-run this script with --resume to commit + tag."
echo "  Or press Enter to skip and commit/tag without changelog edits."
read -r -p "  Continue? [enter to commit / Ctrl-C to abort] " _

git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
[[ -f src-tauri/Cargo.lock ]] && git add src-tauri/Cargo.lock
[[ -f CHANGELOG.md ]] && git add CHANGELOG.md

git commit -m "Release v$NEW"
git tag -a "v$NEW" -m "Dicto v$NEW"

echo
echo "✓ Committed and tagged v$NEW locally."
echo "→ Push when ready:"
echo "    git push origin HEAD:main && git push origin v$NEW"
