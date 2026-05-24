#!/usr/bin/env bash
# Model B — record KYC-style attested verification on Futurenet (no NFC / no ZK).
#
# Usage:
#   ./scripts/attested-ready.sh                    # defaults: age 25, US, human
#   ./scripts/attested-ready.sh claims.json
#   ./scripts/attested-ready.sh --age 30 --country 826 --human
#
# Requires: SOROBAN_SOURCE_ACCOUNT (signs as prover AND subject for local testing)
# For production: prover = backend key, subject = user key (both must sign the tx).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
PROVER="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
SUBJECT="${SUBJECT:-$PROVER}"
APP_ID="${APP_ID:-${STABLE_APP_ID:-wraith$(date +%s)}}"
NULLIFIER="${NULLIFIER:-$(openssl rand -hex 32)}"
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"
SKIP_REGISTER=0
PREPARE_ONLY=0
JSON_OUT=0

AGE=25
COUNTRY=840
HUMAN=true

if [[ $# -ge 1 && -f "$1" && "$1" != --* ]]; then
  AGE=$(jq -r .age "$1")
  COUNTRY=$(jq -r .country_code "$1")
  HUMAN=$(jq -r .is_human "$1")
  shift
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --age) AGE="$2"; shift 2 ;;
    --country) COUNTRY="$2"; shift 2 ;;
    --human) HUMAN=true; shift ;;
    --no-human) HUMAN=false; shift ;;
    --subject) SUBJECT="$2"; shift 2 ;;
    --app) APP_ID="$2"; shift 2 ;;
    --nullifier) NULLIFIER="${2#0x}"; shift 2 ;;
    --no-register) SKIP_REGISTER=1; shift ;;
    --prepare-only) PREPARE_ONLY=1; shift ;;
    --json) JSON_OUT=1; shift ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

CLAIMS=$(jq -nc --argjson age "$AGE" --argjson cc "$COUNTRY" --argjson human "$HUMAN" \
  '{age:$age, country_code:$cc, is_human:$human}')

POLICY=$(jq -nc \
  --arg owner "$PROVER" \
  --arg zero "$ZERO32" \
  '{owner:$owner, min_age:18, require_humanity:false, sanctions_root:$zero,
    excluded_countries:[], expiration_window:0, sanctions_enabled:false, claim_layout:"Standard"}')

invoke() {
  stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    "$@"
}

log() {
  if [[ "$JSON_OUT" == "1" ]]; then
    echo "$@" >&2
  else
    echo "$@"
  fi
}

log "=== KYC attested path (Model B) ==="
log "Contract: $CONTRACT_ID"
log "App:      $APP_ID"
log "Prover:   $PROVER"
log "Subject:  $SUBJECT"
log "Claims:   $CLAIMS"
log ""

CONFIGURED_PROVER="$(invoke --send=no -- get_prover | tr -d '"')"
if [[ "$CONFIGURED_PROVER" != "$PROVER" ]]; then
  log "warning: SOROBAN_SOURCE_ACCOUNT ($PROVER) is not contract prover ($CONFIGURED_PROVER)"
  log "  Tx may fail with Unauthorized. Use the configured prover key or admin set_prover."
fi

log "1. register_app (if needed)..."
if [[ "$SKIP_REGISTER" == "1" ]]; then
  log "   skipped (--no-register)"
elif invoke --send=no -- is_app_registered --app_id "$APP_ID" 2>/dev/null | grep -q false; then
  invoke --send=yes -- register_app --app_id "$APP_ID" --policy "$POLICY" --vk_hash null
else
  log "   already registered"
fi

log "2. Hashes..."
PUB_HASH=$(invoke --send=no -- hash_attested_claims --claims "$CLAIMS" | tr -d '"')
ATT_HASH=$(invoke --send=no -- hash_attestation \
  --prover "$PROVER" --app_id "$APP_ID" --subject "$SUBJECT" \
  --nullifier "$NULLIFIER" --claims "$CLAIMS" | tr -d '"')

if [[ "$PREPARE_ONLY" == "1" ]]; then
  RESULT=$(jq -nc \
    --arg app_id "$APP_ID" \
    --arg prover "$PROVER" \
    --arg subject "$SUBJECT" \
    --arg nullifier "$NULLIFIER" \
    --arg public_inputs_hash "$PUB_HASH" \
    --arg attestation_hash "$ATT_HASH" \
    --argjson claims "$CLAIMS" \
    '{app_id:$app_id, prover:$prover, subject:$subject, nullifier:$nullifier,
      public_inputs_hash:$public_inputs_hash, attestation_hash:$attestation_hash, claims:$claims}')
  if [[ "$JSON_OUT" == "1" ]]; then
    echo "$RESULT"
  else
    echo "$RESULT" | jq .
  fi
  exit 0
fi

log "3. record_attested_result..."
invoke --send=yes -- record_attested_result \
  --prover "$PROVER" \
  --app_id "$APP_ID" \
  --subject "$SUBJECT" \
  --nullifier "$NULLIFIER" \
  --public_inputs_hash "$PUB_HASH" \
  --attestation_hash "$ATT_HASH" \
  --claims "$CLAIMS"

log "4. is_verified..."
VERIFIED=$(invoke --send=no -- is_verified --app_id "$APP_ID" --subject "$SUBJECT" | tr -d '"')
if [[ "$JSON_OUT" == "1" ]]; then
  jq -nc \
    --arg app_id "$APP_ID" \
    --arg subject "$SUBJECT" \
    --arg nullifier "$NULLIFIER" \
    --argjson verified "$VERIFIED" \
    '{app_id:$app_id, subject:$subject, nullifier:$nullifier, verified:$verified}'
else
  echo "$VERIFIED"
  echo ""
  echo "=== attested path complete (app $APP_ID) ==="
fi
