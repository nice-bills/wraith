#!/bin/bash
# Trusted Setup Script for Wraith ZK Circuits
# This script prepares the Powers of Tau ceremony for production circuits

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
PTAU_DIR="$BUILD_DIR/ptau"
RARIMO_DIR="$SCRIPT_DIR/rarimo"

echo "=== Wraith ZK Circuit Trusted Setup ==="
echo ""

# Create directories
mkdir -p "$BUILD_DIR"
mkdir -p "$PTAU_DIR"

# Check for circom
if ! command -v circom &> /dev/null; then
    echo "ERROR: circom is not installed"
    echo "Install: https://docs.circom.io/getting-started/installation/"
    exit 1
fi
echo "✓ circom found: $(circom --version 2>/dev/null || echo 'unknown version')"

# Check for snarkjs
if ! command -v snarkjs &> /dev/null; then
    echo "ERROR: snarkjs is not installed"
    echo "Install: pnpm add -g snarkjs"
    exit 1
fi
echo "✓ snarkjs found: $(snarkjs --version 2>/dev/null || echo 'unknown version')"

# Check if rarimo is available
if [ ! -d "$RARIMO_DIR" ]; then
    echo "ERROR: rarimo not found at $RARIMO_DIR"
    echo "Run: git clone https://github.com/rarimo/passport-zk-circuits.git rarimo"
    exit 1
fi

echo ""
echo "=== Step 1: Prepare Powers of Tau ==="

PTAU_FILE="$PTAU_DIR/powersOfTau28_hez_final_15.ptau"

if [ -f "$PTAU_FILE" ]; then
    echo "Found existing PTAU file: $PTAU_FILE"
    echo "Skipping download..."
else
    echo "Downloading pre-contributed PTAU file..."
    echo "NOTE: For production, run your own ceremony!"
    echo ""

    curl -L -o "$PTAU_FILE" \
        "https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_15.ptau" \
        --progress-bar || {
        echo "Download failed. Manually download from:"
        echo "https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_15.ptau"
        echo "Place at: $PTAU_FILE"
        exit 1
    }
fi

echo "✓ PTAU file ready: $PTAU_FILE"
echo ""

echo "=== Step 2: Build Rarimo Circuits ==="

cd "$RARIMO_DIR"

# Check if circomlib is available
if [ ! -d "circomlib" ]; then
    echo "Initializing rarimo submodules..."
    git submodule update --init --recursive || true
fi

# Build circuit (this may take a while)
echo "Building registerIdentity circuit..."
# circom circuits/identityManagement/registerIdentityBuilder.circom \
#     --r1cs --wasm --sym -o circuits/build/ 2>/dev/null || true

echo "NOTE: Full circuit build requires significant time and memory."
echo "See $RARIMO_DIR/README.md for detailed build instructions."
echo ""

echo "=== Step 3: Setup Verification Key ==="

# For demo purposes, we can use the existing rarimo verification key
# In production, you would generate your own

if [ -f "$RARIMO_DIR/circuits/verification_key.json" ]; then
    echo "✓ Found rarimo verification key"
else
    echo "NOTE: You may need to build rarimo circuits first to get verification key"
fi

echo ""
echo "=== Trusted Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Review the PTAU file provenance"
echo "2. For production: run your own Powers of Tau ceremony"
echo "3. Build circuits: cd rarimo && pnpm run compile"
echo "4. Generate proving/verification keys"
echo "5. Update contract with new VK hash"
echo ""
echo "For production ceremony, see:"
echo "https://github.com/iden3/snarkjs/blob/master/src/powersoftau_guide.js"