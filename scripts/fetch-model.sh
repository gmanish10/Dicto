#!/usr/bin/env bash
# Pre-seed the default whisper.cpp model into Dicto's app-data models dir.
#
# The model is NO LONGER bundled in the `.app` — the app auto-downloads it
# on first launch. This script is a dev convenience only: run it to put the
# model on disk ahead of time so `npx tauri dev` skips the first-run
# download. It is NOT required before `npx tauri build`.
#
# After download, the file is verified against the SHA-256 in
# `scripts/models.sha256`. A missing hash entry, a mismatch, or a network
# error fails the script and removes the partial download.
set -euo pipefail

MODEL="${1:-ggml-small.en}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# Mirror the runtime path: `model::user_models_dir` resolves the model from
# `~/Library/Application Support/com.dicto.app/models/`.
DEST_DIR="$HOME/Library/Application Support/com.dicto.app/models"
HASHES_FILE="$SCRIPT_DIR/models.sha256"
mkdir -p "$DEST_DIR"

BIN_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL}.bin"
COREML_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL}-encoder.mlmodelc.zip"
BIN_NAME="${MODEL}.bin"
BIN_PATH="${DEST_DIR}/${BIN_NAME}"
BIN_TMP="${BIN_PATH}.partial"

# Look up the expected SHA-256 from the manifest. We want this to fail
# loudly rather than silently skip verification.
if [[ ! -f "$HASHES_FILE" ]]; then
  echo "ERROR: missing $HASHES_FILE — can't verify ${MODEL}" >&2
  exit 1
fi
EXPECTED_SHA="$(awk -v file="$BIN_NAME" '$1 !~ /^#/ && $2 == file { print $1 }' "$HASHES_FILE" | head -n 1 || true)"
if [[ -z "$EXPECTED_SHA" ]]; then
  echo "ERROR: no SHA-256 entry for $BIN_NAME in $HASHES_FILE" >&2
  echo "       Add the expected hash before fetching this model." >&2
  exit 1
fi

cleanup_partial() {
  rm -f "$BIN_TMP"
}
trap cleanup_partial EXIT

echo "→ Downloading ${BIN_NAME} ..."
curl -fL --progress-bar -o "$BIN_TMP" "$BIN_URL"

echo "→ Verifying SHA-256 ..."
ACTUAL_SHA="$(shasum -a 256 "$BIN_TMP" | awk '{print $1}')"
if [[ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]]; then
  echo "ERROR: SHA-256 mismatch for ${BIN_NAME}" >&2
  echo "  expected: $EXPECTED_SHA" >&2
  echo "  actual:   $ACTUAL_SHA" >&2
  echo "  source:   $BIN_URL" >&2
  exit 1
fi
mv "$BIN_TMP" "$BIN_PATH"
trap - EXIT
echo "✓ ${BIN_NAME} verified."

echo "→ Downloading ${MODEL} CoreML encoder ..."
TMP_ZIP="$(mktemp -t coreml-encoder).zip"
if curl -fL --progress-bar -o "${TMP_ZIP}" "${COREML_URL}"; then
  unzip -oq "${TMP_ZIP}" -d "${DEST_DIR}"
  rm -f "${TMP_ZIP}"
  echo "✓ CoreML encoder installed."
else
  echo "⚠ CoreML encoder not available for ${MODEL}; whisper.cpp will fall back to Metal/CPU."
fi

echo "✓ Done. Model files in ${DEST_DIR}"
