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
APP_ID="${APP_ID:-wraith$(date +%s)}"
NULLIFIER="${NULLIFIER:-$(openssl rand -hex 32)}"
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"

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

echo "=== KYC attested path (Model B) ==="
echo "Contract: $CONTRACT_ID"
echo "App:      $APP_ID"
echo "Prover:   $PROVER"
echo "Subject:  $SUBJECT"
echo "Claims:   $CLAIMS"
echo ""

CONFIGURED_PROVER="$(invoke --send=no -- get_prover | tr -d '"')"
if [[ "$CONFIGURED_PROVER" != "$PROVER" ]]; then
  echo "warning: SOROBAN_SOURCE_ACCOUNT ($PROVER) is not contract prover ($CONFIGURED_PROVER)"
  echo "  Tx may fail with Unauthorized. Use the configured prover key or admin set_prover."
fi

echo "1. register_app (if needed)..."
if invoke --send=no -- is_app_registered --app_id "$APP_ID" 2>/dev/null | grep -q false; then
  invoke --send=yes -- register_app --app_id "$APP_ID" --policy "$POLICY" --vk_hash null
else
  echo "   already registered"
fi

echo "2. Hashes..."
PUB_HASH=$(invoke --send=no -- hash_attested_claims --claims "$CLAIMS" | tr -d '"')
ATT_HASH=$(invoke --send=no -- hash_attestation \
  --prover "$PROVER" --app_id "$APP_ID" --subject "$SUBJECT" \
  --nullifier "$NULLIFIER" --claims "$CLAIMS" | tr -d '"')

echo "3. record_attested_result..."
invoke --send=yes -- record_attested_result \
  --prover "$PROVER" \
  --app_id "$APP_ID" \
  --subject "$SUBJECT" \
  --nullifier "$NULLIFIER" \
  --public_inputs_hash "$PUB_HASH" \
  --attestation_hash "$ATT_HASH" \
  --claims "$CLAIMS"

echo "4. is_verified..."
invoke --send=no -- is_verified --app_id "$APP_ID" --subject "$SUBJECT"
echo ""
echo "=== attested path complete (app $APP_ID) ==="
