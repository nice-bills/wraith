#!/usr/bin/env bash
# Groth16 E2E: e2e_claims (3 public signals) → prove → adapter → verify_and_record on Futurenet.
# simple_mult has only 1 public signal; this contract requires >= 3.
#
# stellar-cli notes (https://developers.stellar.org/docs/tools/cli/cookbook/contract-invoke-arguments):
#   - Option<BytesN<32>> vk_hash: pass JSON-quoted hex, e.g. --vk_hash "\"abc...\""
#   - pub_signals: JSON array of u256 decimals matching snarkjs public.json
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZK="$ROOT_DIR/tools/zk-circuits"
BUILD="$ZK/build/e2e"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
SUBJECT="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
APP_ID="e2egrth$(date +%s)"
NULLIFIER="$(openssl rand -hex 32)"
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"
PTAU="${ZK}/build/circuits_ptau10_final.ptau"

mkdir -p "$BUILD"

invoke() {
  stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    "$@"
}

echo "=== Groth16 E2E (e2e_claims) ==="
echo "Contract: $CONTRACT_ID"

[[ -f "$PTAU" ]] || { echo "Missing $PTAU"; exit 1; }

echo "1. Compile + prove..."
circom "$ZK/circuits/e2e_claims.circom" --r1cs --wasm --sym -o "$BUILD" 2>&1 | tail -3
if [[ ! -f "$BUILD/e2e_final.zkey" ]]; then
  snarkjs groth16 setup "$BUILD/e2e_claims.r1cs" "$PTAU" "$BUILD/e2e_0000.zkey"
  snarkjs zkey contribute "$BUILD/e2e_0000.zkey" "$BUILD/e2e_final.zkey" --name=wraith-e2e -v -e=wraith
  snarkjs zkey export verificationkey "$BUILD/e2e_final.zkey" "$BUILD/verification_key.json"
fi
cat > "$BUILD/input.json" <<'EOF'
{"age":"30","country_code":"840","is_human":"1"}
EOF
snarkjs groth16 fullprove "$BUILD/input.json" "$BUILD/e2e_claims_js/e2e_claims.wasm" \
  "$BUILD/e2e_final.zkey" "$BUILD/proof.json" "$BUILD/public.json"

echo "2. proof-adapter..."
PAYLOAD="$BUILD/stellar-payload.json"
cargo run -q -p proof-adapter -- \
  --proof "$BUILD/proof.json" --vk "$BUILD/verification_key.json" --public "$BUILD/public.json" \
  --age-index 0 --country-index 1 --humanity-index 2 --out "$PAYLOAD"

VK_HASH=$(node "$ROOT_DIR/scripts/compute-vk-hash.mjs" "$PAYLOAD")
PUB_SIGNALS=$(jq -c '.public_signals_decimals' "$PAYLOAD")
CLAIMS=$(jq -c '.claims | {age, country_code, is_human}' "$PAYLOAD")
POLICY="{\"owner\":\"$SUBJECT\",\"min_age\":18,\"require_humanity\":false,\"sanctions_root\":\"$ZERO32\",\"excluded_countries\":[],\"expiration_window\":0,\"sanctions_enabled\":false,\"claim_layout\":\"Standard\"}"

# shellcheck source=/dev/null
source "$ROOT_DIR/scripts/lib/write-stellar-proof-tmp.sh"
write_stellar_proof_tmp "$PAYLOAD" wraith-groth16
trap 'rm -f "$PROOF_TMP" "$VK_TMP"' EXIT

PUBHASH=$(invoke --send=no -- compute_pub_signals_hash --pub_signals "$PUB_SIGNALS" | tr -d '"')
echo "   vk_hash: $VK_HASH"
echo "   pub_hash: $PUBHASH"

echo "3. register_app..."
invoke --send=yes -- register_app --app_id "$APP_ID" --policy "$POLICY" --vk_hash "\"$VK_HASH\""

echo "4. verify_and_record..."
invoke --send=yes -- verify_and_record \
  --app_id "$APP_ID" --subject "$SUBJECT" --nullifier "$NULLIFIER" \
  --public_inputs_hash "$PUBHASH" \
  --proof-file-path "$PROOF_TMP" \
  --vk-file-path "$VK_TMP" \
  --pub_signals "$PUB_SIGNALS" \
  --current_date_ymd 0 \
  --claims "$CLAIMS"

echo "5. is_verified..."
invoke --send=no -- is_verified --app_id "$APP_ID" --subject "$SUBJECT"
echo "=== Groth16 E2E OK (app $APP_ID) ==="
