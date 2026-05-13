#!/usr/bin/env bash
# Download the default whisper.cpp model into src-tauri/resources/models so the
# Tauri bundler can include it. Run this before `pnpm tauri build`.
set -euo pipefail

MODEL="${1:-ggml-small.en}"
DEST_DIR="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/resources/models"
mkdir -p "$DEST_DIR"

BIN_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL}.bin"
COREML_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${MODEL}-encoder.mlmodelc.zip"

echo "→ Downloading ${MODEL}.bin ..."
curl -fL --progress-bar -o "${DEST_DIR}/${MODEL}.bin" "${BIN_URL}"

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
