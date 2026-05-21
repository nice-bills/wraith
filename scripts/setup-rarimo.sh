#!/usr/bin/env bash
# Clone Rarimo passport circuits (not a git submodule — see deployments/README).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
REPO="${RARIMO_REPO:-https://github.com/rarimo/passport-zk-circuits.git}"
REF="${RARIMO_REF:-f899082185ef84751e26cc9c6e9382a27ea3ca18}"

if [[ -d "$RARIMO/.git" ]]; then
  CURRENT="$(git -C "$RARIMO" rev-parse HEAD 2>/dev/null || true)"
  if [[ "$CURRENT" != "$REF" ]]; then
    echo "Updating rarimo to pinned ref $REF (was ${CURRENT:0:7})"
    git -C "$RARIMO" fetch --depth 1 origin "$REF" 2>/dev/null || git -C "$RARIMO" fetch origin
    git -C "$RARIMO" checkout "$REF"
  fi
  echo "✓ rarimo at $RARIMO ($REF)"
else
  echo "Cloning $REPO @ $REF → $RARIMO"
  git clone --depth 1 "$REPO" "$RARIMO"
  git -C "$RARIMO" checkout "$REF"
fi

if [[ -d "$RARIMO/node_modules" ]]; then
  echo "✓ rarimo node_modules present"
else
  echo "Installing rarimo dependencies..."
  (cd "$RARIMO" && CI=1 pnpm install --config.confirmModulesPurge=false)
fi
echo "✓ rarimo ready"
