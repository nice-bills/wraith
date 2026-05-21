#!/usr/bin/env bash
# Write proof + vk JSON for stellar-cli --proof-file-path / --vk-file-path.
#
# Usage (source, then call):
#   source scripts/lib/write-stellar-proof-tmp.sh
#   write_stellar_proof_tmp payload.json prefix
# Sets PROOF_TMP and VK_TMP in the caller's shell.
write_stellar_proof_tmp() {
  local payload="${1:?payload.json}"
  local prefix="${2:-wraith}"

  PROOF_TMP="$(mktemp "/tmp/${prefix}-proof.XXXXXX.json")"
  VK_TMP="$(mktemp "/tmp/${prefix}-vk.XXXXXX.json")"

  node -e "
const fs = require('fs');
const p = JSON.parse(fs.readFileSync('$payload', 'utf8'));
const s = (h) => h.replace(/^0x/, '');
fs.writeFileSync('$PROOF_TMP', JSON.stringify({
  a: s(p.proof.a), b: s(p.proof.b), c: s(p.proof.c),
}));
fs.writeFileSync('$VK_TMP', JSON.stringify({
  alpha: s(p.verification_key.alpha),
  beta: s(p.verification_key.beta),
  gamma: s(p.verification_key.gamma),
  delta: s(p.verification_key.delta),
  ic: p.verification_key.ic.map(s),
}));
"

  export PROOF_TMP VK_TMP
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  write_stellar_proof_tmp "$@"
  echo "PROOF_TMP=$PROOF_TMP"
  echo "VK_TMP=$VK_TMP"
fi
