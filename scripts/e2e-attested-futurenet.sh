#!/usr/bin/env bash
# Attested-path E2E on Futurenet (no passport / no ZK).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$ROOT_DIR/scripts/attested-ready.sh" --age 30 --country 840 --human
