#!/usr/bin/env bash
# Clone Rarimo passport circuits (not a git submodule — see deployments/README).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RARIMO="$ROOT_DIR/tools/zk-circuits/rarimo"
REPO="${RARIMO_REPO:-https://github.com/rarimo/passport-zk-circuits.git}"
REF="${RARIMO_REF:-main}"

if [[ -d "$RARIMO/.git" ]]; then
  echo "✓ rarimo already at $RARIMO"
else
  echo "Cloning $REPO → $RARIMO"
  git clone --depth 1 --branch "$REF" "$REPO" "$RARIMO"
fi

if [[ -d "$RARIMO/node_modules" ]]; then
  echo "✓ rarimo node_modules present"
else
  echo "Installing rarimo dependencies..."
  (cd "$RARIMO" && CI=1 pnpm install --config.confirmModulesPurge=false)
fi
echo "✓ rarimo ready"
