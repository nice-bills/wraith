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

echo "Wraith local CI (mirrors GitHub Actions)"

run "cargo fmt --check" cargo fmt --all -- --check
run "cargo clippy" cargo clippy --all-targets -- -D warnings
run "cargo test --all" cargo test --all
run "pnpm install --frozen-lockfile" pnpm install --frozen-lockfile
run "SDK tests" pnpm --filter @wraith/stellar-identity-sdk test
run "adapter smoke" make smoke

echo ""
echo -e "${GREEN}All CI checks passed.${NC}"
