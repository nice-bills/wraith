#!/usr/bin/env bash
# Rarimo-layout Groth16 E2E: 6 public signals (birthDate@[1], nationality@[5]) on Futurenet.
# Uses rarimo_layout_stub until a full Rarimo query zkey is built and pinned in deployments/circuits.json.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZK="$ROOT_DIR/tools/zk-circuits"
BUILD="$ZK/build/rarimo-layout"
: "${SOROBAN_SOURCE_ACCOUNT:?Set SOROBAN_SOURCE_ACCOUNT}"
: "${STELLAR_NETWORK:=futurenet}"

CONTRACT_ID="${CONTRACT_ID:-$(jq -r .contractId "$ROOT_DIR/deployments/futurenet.json")}"
SUBJECT="$(stellar-cli keys address "$SOROBAN_SOURCE_ACCOUNT")"
APP_ID="e2erarm$(date +%s)"
NULLIFIER="$(openssl rand -hex 32)"
ZERO32="0000000000000000000000000000000000000000000000000000000000000000"
PTAU="${ZK}/build/circuits_ptau10_final.ptau"
CURRENT_DATE="${CURRENT_DATE_YMD:-$(date -u +%y%m%d)}"

mkdir -p "$BUILD"

invoke() {
  stellar-cli contract invoke \
    --id "$CONTRACT_ID" \
    --source-account "$SOROBAN_SOURCE_ACCOUNT" \
    --network "$STELLAR_NETWORK" \
    "$@"
}

echo "=== Rarimo-layout Groth16 E2E ==="
echo "Contract:     $CONTRACT_ID"
echo "Current date: $CURRENT_DATE (YYMMDD)"

[[ -f "$PTAU" ]] || { echo "Missing $PTAU"; exit 1; }

echo "1. Compile + prove (rarimo_layout_stub)..."
circom "$ZK/circuits/rarimo_layout_stub.circom" --r1cs --wasm --sym -o "$BUILD" 2>&1 | tail -3
if [[ ! -f "$BUILD/rarimo_layout_stub_final.zkey" ]]; then
  snarkjs groth16 setup "$BUILD/rarimo_layout_stub.r1cs" "$PTAU" "$BUILD/rarimo_layout_stub_0000.zkey"
  snarkjs zkey contribute "$BUILD/rarimo_layout_stub_0000.zkey" "$BUILD/rarimo_layout_stub_final.zkey" \
    --name=wraith-rarimo-layout -v -e=wraith
  snarkjs zkey export verificationkey "$BUILD/rarimo_layout_stub_final.zkey" "$BUILD/verification_key.json"
fi

cat > "$BUILD/input.json" <<EOF
{"nullifier":"42","birth_date":"950101","expiration_date":"300101","pad3":"0","pad4":"0","nationality":"840"}
EOF
snarkjs groth16 fullprove "$BUILD/input.json" "$BUILD/rarimo_layout_stub_js/rarimo_layout_stub.wasm" \
  "$BUILD/rarimo_layout_stub_final.zkey" "$BUILD/proof.json" "$BUILD/public.json"

echo "2. proof-adapter (--rarimo-mode)..."
PAYLOAD="$BUILD/stellar-payload.json"
cargo run -q -p proof-adapter -- \
  --rarimo-mode --current-date "$CURRENT_DATE" \
  --proof "$BUILD/proof.json" --vk "$BUILD/verification_key.json" --public "$BUILD/public.json" \
  --out "$PAYLOAD"

VK_HASH=$(node "$ROOT_DIR/scripts/compute-vk-hash.mjs" "$PAYLOAD")
PUB_SIGNALS=$(jq -c '.public_signals_decimals' "$PAYLOAD")
CLAIMS=$(jq -c '.claims | {age, country_code, is_human}' "$PAYLOAD")
POLICY="{\"owner\":\"$SUBJECT\",\"min_age\":18,\"require_humanity\":true,\"sanctions_root\":\"$ZERO32\",\"excluded_countries\":[],\"expiration_window\":0,\"sanctions_enabled\":false,\"claim_layout\":\"RarimoQuery\"}"

node -e "
const fs=require('fs'); const p=JSON.parse(fs.readFileSync('$PAYLOAD'));
const s=h=>h.replace(/^0x/,'');
fs.writeFileSync('/tmp/wraith-rarimo-proof.json', JSON.stringify({a:s(p.proof.a),b:s(p.proof.b),c:s(p.proof.c)}));
fs.writeFileSync('/tmp/wraith-rarimo-vk.json', JSON.stringify({alpha:s(p.verification_key.alpha),beta:s(p.verification_key.beta),gamma:s(p.verification_key.gamma),delta:s(p.verification_key.delta),ic:p.verification_key.ic.map(s)}));
"

PUBHASH=$(invoke --send=no -- compute_pub_signals_hash --pub_signals "$PUB_SIGNALS" | tr -d '"')
echo "   vk_hash: $VK_HASH"
echo "   pub_hash: $PUBHASH"
echo "   claims:  $CLAIMS"

echo "3. register_app (claim_layout=RarimoQuery)..."
invoke --send=yes -- register_app --app_id "$APP_ID" --policy "$POLICY" --vk_hash "\"$VK_HASH\""

echo "4. verify_and_record..."
invoke --send=yes -- verify_and_record \
  --app_id "$APP_ID" --subject "$SUBJECT" --nullifier "$NULLIFIER" \
  --public_inputs_hash "$PUBHASH" \
  --proof-file-path /tmp/wraith-rarimo-proof.json \
  --vk-file-path /tmp/wraith-rarimo-vk.json \
  --pub_signals "$PUB_SIGNALS" \
  --current_date_ymd "$CURRENT_DATE" \
  --claims "$CLAIMS"

echo "5. is_verified..."
invoke --send=no -- is_verified --app_id "$APP_ID" --subject "$SUBJECT"
echo "=== Rarimo-layout E2E OK (app $APP_ID) ==="
