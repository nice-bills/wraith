#!/usr/bin/env bash
# Dev/staging ZK setup: PTAU + record ceremony metadata (Track A — not production MPC).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZK="$ROOT_DIR/tools/zk-circuits"
DEPLOYMENTS="$ROOT_DIR/deployments"

cd "$ZK"
bash setup-production.sh
bash setup-trusted-setup.sh

PTAU="$ZK/build/ptau/powersOfTau28_hez_final_15.ptau"
PTAU_SHA=""
if [[ -f "$PTAU" ]]; then
  PTAU_SHA="$(sha256sum "$PTAU" | awk '{print $1}')"
fi

RARIMO_SHA=""
if [[ -d "$ZK/rarimo/.git" ]]; then
  RARIMO_SHA="$(git -C "$ZK/rarimo" rev-parse HEAD 2>/dev/null || true)"
fi

mkdir -p "$DEPLOYMENTS"
cat > "$DEPLOYMENTS/ceremony.json" <<EOF
{
  "track": "development",
  "network": "futurenet",
  "ptau": {
    "path": "tools/zk-circuits/build/ptau/powersOfTau28_hez_final_15.ptau",
    "source": "hermez-powersOfTau28_hez_final_15",
    "sha256": "${PTAU_SHA:-unknown}",
    "warning": "Pre-contributed PTAU — run MPC ceremony before production launch"
  },
  "rarimoSubmodule": {
    "path": "tools/zk-circuits/rarimo",
    "commit": "${RARIMO_SHA:-unknown}"
  },
  "circuits": {
    "e2e_claims": "tools/zk-circuits/circuits/e2e_claims.circom",
    "rarimo_layout_stub": "tools/zk-circuits/circuits/rarimo_layout_stub.circom",
    "rarimo_query": "tools/zk-circuits/rarimo/circuits/identityManagement/queryIdentity.circom"
  },
  "notes": "Track B production MPC not run. See docs/PRODUCTION_ZK_ROADMAP.md Phase 4."
}
EOF

cat > "$DEPLOYMENTS/circuits.json" <<EOF
{
  "production": {
    "passport": {
      "source": "rarimo/passport-zk-circuits",
      "commit": "${RARIMO_SHA:-unknown}",
      "register": "registerIdentityBuilder",
      "query": "queryIdentity"
    },
    "pipelineStub": {
      "circuit": "rarimo_layout_stub.circom",
      "claimLayout": "RarimoQuery",
      "publicSignalLayout": "nullifier,birthDate,expirationDate,pad3,pad4,nationality"
    }
  },
  "ciOnly": ["e2e_claims.circom"],
  "demoDeprecated": ["age_check.circom", "passport_verifier.circom", "simple_mult.circom"]
}
EOF

echo "Wrote $DEPLOYMENTS/ceremony.json"
echo "Wrote $DEPLOYMENTS/circuits.json"
echo "Run: pnpm run build:production  (in tools/zk-circuits, may take significant time)"
