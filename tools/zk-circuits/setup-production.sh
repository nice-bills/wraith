#!/bin/bash
# Setup script for rarimo passport-zk-circuits integration
# Run this script from tools/zk-circuits directory

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RARIMO_DIR="$SCRIPT_DIR/rarimo"

echo "=== Wraith ZK Circuit Setup ==="
echo ""

# Check if circom is installed
if ! command -v circom &> /dev/null; then
    echo "ERROR: circom is not installed."
    echo "Install from: https://docs.circom.io/getting-started/installation/"
    exit 1
fi

echo "✓ circom found"

# Check if snarkjs is installed
if ! command -v snarkjs &> /dev/null; then
    echo "ERROR: snarkjs is not installed."
    echo "Install: pnpm add -g snarkjs"
    exit 1
fi

echo "✓ snarkjs found"

# Clone rarimo if not present
if [ ! -d "$RARIMO_DIR" ]; then
    echo ""
    echo "Cloning rarimo/passport-zk-circuits..."
    git clone --depth 1 https://github.com/rarimo/passport-zk-circuits.git "$RARIMO_DIR"
    echo "✓ rarimo cloned"
else
    echo "✓ rarimo already present"
fi

# Setup circomlib submodule if needed
if [ ! -d "$RARIMO_DIR/circomlib" ]; then
    echo ""
    echo "Initializing circomlib submodule..."
    cd "$RARIMO_DIR"
    git submodule update --init --recursive circomlib
    cd "$SCRIPT_DIR"
    echo "✓ circomlib initialized"
else
    echo "✓ circomlib present"
fi

# Install pnpm dependencies (rarimo submodule)
echo ""
echo "Installing pnpm dependencies in rarimo..."
cd "$RARIMO_DIR"
pnpm install
cd "$SCRIPT_DIR"
echo "✓ pnpm dependencies installed"

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Run trusted setup: pnpm run trusted-setup"
echo "2. Build circuits: pnpm run build:production"
echo "3. See CIRCUITS_REQUIREMENTS.md for production integration details"