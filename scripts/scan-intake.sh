#!/usr/bin/env bash
# Unified document intake — NFC scan, MRZ, photo KYC staging, attested claims.
#
# Usage:
#   ./scripts/scan-intake.sh detect <file.json>
#   ./scripts/scan-intake.sh nfc <raw-dump.json> [--submit]
#   ./scripts/scan-intake.sh auto <file.json> [--submit]
#   ./scripts/scan-intake.sh claims <claims.json> [--submit]
#   ./scripts/scan-intake.sh mrz "<MRZ line(s)>"
#   ./scripts/scan-intake.sh kyc --front photo.jpg [--back id-back.jpg] [--selfie selfie.jpg]
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CMD="${1:-}"
shift || true

usage() {
  cat <<EOF
Document intake (see docs/DOCUMENT_INTAKE.md)

  $0 detect <file.json>              Show recommended path (full/layout/attested/kyc)
  $0 nfc <raw-dump.json> [--submit]  Normalize NFC dump → Rarimo full path
  $0 auto <file.json> [--submit]     Detect path and run (nfc/claims/layout)
  $0 claims <claims.json> [--submit] KYC attested path (no NFC)
  $0 mrz "<MRZ lines>"               Parse MRZ → attested claims → Futurenet
  $0 kyc --front <photo> [...]       Stage photos for KYC vendor

Scan apps: docs/PASSPORT_SCAN.md
No NFC:    docs/KYC_ATTESTED.md
EOF
}

SUBMIT=0
ARGS=()
for arg in "$@"; do
  case "$arg" in
    --submit) SUBMIT=1 ;;
    *) ARGS+=("$arg") ;;
  esac
done

case "$CMD" in
  detect)
    [[ ${#ARGS[@]} -ge 1 ]] || { usage; exit 1; }
    node "$ROOT_DIR/scripts/detect-document-path.mjs" "${ARGS[0]}"
    ;;
  nfc)
    [[ ${#ARGS[@]} -ge 1 ]] || { usage; exit 1; }
    RAW="${ARGS[0]}"
    OUT="$ROOT_DIR/passport-data/normalized-$(basename "$RAW" .json).json"
    node "$ROOT_DIR/scripts/normalize-passport-json.mjs" "$RAW" "$OUT"
    SUBMIT_ARGS=()
    [[ "$SUBMIT" == "1" ]] && SUBMIT_ARGS=(--submit)
    exec "$ROOT_DIR/scripts/passport-ready.sh" --full "$OUT" "${SUBMIT_ARGS[@]}"
    ;;
  auto)
    [[ ${#ARGS[@]} -ge 1 ]] || { usage; exit 1; }
    FILE="${ARGS[0]}"
    PATH_JSON="$(node "$ROOT_DIR/scripts/detect-document-path.mjs" "$FILE")"
    PATH_NAME="$(echo "$PATH_JSON" | jq -r .path)"
    echo "Detected: $PATH_NAME"
    case "$PATH_NAME" in
      full)
        SUBMIT_ARGS=()
        [[ "$SUBMIT" == "1" ]] && SUBMIT_ARGS=(--submit)
        exec "$ROOT_DIR/scripts/passport-ready.sh" --full "$FILE" "${SUBMIT_ARGS[@]}"
        ;;
      layout)
        SUBMIT_ARGS=()
        [[ "$SUBMIT" == "1" ]] && SUBMIT_ARGS=(--submit)
        exec "$ROOT_DIR/scripts/passport-ready.sh" --layout "$FILE" "${SUBMIT_ARGS[@]}"
        ;;
      attested)
        exec "$ROOT_DIR/scripts/attested-ready.sh" "$FILE"
        ;;
      kyc-intake)
        echo "Photo intake manifest — complete KYC then run claims path"
        cat "$FILE"
        exit 0
        ;;
      *)
        echo "Cannot auto-route. Try: normalize-passport-json.mjs or docs/DOCUMENT_INTAKE.md"
        exit 1
        ;;
    esac
    ;;
  claims)
    [[ ${#ARGS[@]} -ge 1 ]] || { usage; exit 1; }
    exec "$ROOT_DIR/scripts/attested-ready.sh" "${ARGS[0]}"
    ;;
  mrz)
    [[ ${#ARGS[@]} -ge 1 ]] || { usage; exit 1; }
    CLAIMS="$ROOT_DIR/passport-data/mrz-claims-$(date +%s).json"
    node "$ROOT_DIR/scripts/build-attested-claims.mjs" --mrz --out "$CLAIMS" "${ARGS[@]}"
    exec "$ROOT_DIR/scripts/attested-ready.sh" "$CLAIMS"
    ;;
  kyc)
    node "$ROOT_DIR/scripts/prepare-kyc-intake.mjs" "$@"
    ;;
  ""|-h|--help|help)
    usage
    ;;
  *)
    echo "Unknown command: $CMD"
    usage
    exit 1
    ;;
esac
