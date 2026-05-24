#!/usr/bin/env bash
# Run the same checks as .github/workflows/ci.yml locally before push.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

run() {
  local name="$1"
  shift
  echo ""
  echo "=== $name ==="
  if "$@"; then
    echo -e "${GREEN}✓ $name${NC}"
  else
    echo -e "${RED}✗ $name failed${NC}" >&2
    exit 1
  fi
}

echo "Wraith local CI (mirrors GitHub Actions — no deploy, no Futurenet writes)"

run "cargo fmt --check" cargo fmt --all -- --check
run "cargo clippy" cargo clippy --all-targets -- -D warnings
run "cargo test --all" cargo test --all
run "pnpm install --frozen-lockfile" pnpm install --frozen-lockfile
run "SDK build (tsc)" pnpm --filter @wraith/stellar-identity-sdk run build
run "SDK tests" pnpm --filter @wraith/stellar-identity-sdk test
run "adapter smoke" make smoke
run "country code tests" node --test scripts/test/country-codes.test.mjs
run "document intake tests" node --test scripts/test/document-intake.test.mjs
run "prover unit tests" pnpm --filter @wraith/prover test

run "passport layout validate" node scripts/validate-passport-json.mjs --mode layout \
  tools/zk-circuits/fixtures/passport.layout.json

if [[ -f tools/zk-circuits/build/passport-layout/passport_layout_final.zkey ]]; then
  run "passport layout prove smoke" ./scripts/passport-ready-layout.sh \
    tools/zk-circuits/fixtures/passport.layout.json
else
  echo ""
  echo "=== passport layout prove smoke (skipped) ==="
  echo "  No layout zkey — run: make setup-passport"
fi

echo ""
echo -e "${GREEN}All CI checks passed.${NC}"
