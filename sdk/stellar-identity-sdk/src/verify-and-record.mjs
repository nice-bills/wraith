#!/usr/bin/env node
/**
 * Wraith SDK - verify_and_record invocation tool
 *
 * Uses stellar-cli for contract invocation (since Futurenet RPC doesn't support Soroban).
 *
 * Usage:
 *   node verify-and-record.mjs <adapter-output.json> <appId> <subject> [nullifier] [publicInputsHash]
 */

import { spawnSync } from "child_process";
import fs from "fs";
import path from "path";

const CONTRACT_ID = process.env.CONTRACT_ID || "CD3CGXCZUUOGWRRKPAJKOZHHLOQY5QAEBHWAQUL54XTPHEULBFTDDOZQ";
const IDENTITY = process.env.IDENTITY || "bills-futurenet";
const RPC_URL = process.env.RPC_URL || "https://rpc-futurenet.stellar.org:443";
const NETWORK = "Test SDF Future Network ; October 2022";

function hexToBytes(hex) {
  if (typeof hex !== 'string') hex = String(hex);
  return hex.replace(/^0x/, "").toLowerCase();
}

function buildVkJson(adapter) {
  const vk = adapter.verification_key || adapter.vk;
  
  // Convert hex fields to raw bytes (no 0x prefix)
  const result = {
    alpha: hexToBytes(vk.alpha || vk.vk_alpha_1),
    beta: hexToBytes(vk.beta || vk.vk_beta_2),
    delta: hexToBytes(vk.delta || vk.vk_delta_2),
    gamma: hexToBytes(vk.gamma || vk.vk_gamma_2),
    ic: (vk.ic || vk.IC || []).map(ic => hexToBytes(Array.isArray(ic) ? ic[0] : ic)),
  };
  
  return JSON.stringify(result);
}

function buildProofJson(adapter) {
  const proof = adapter.proof || adapter;
  
  // Handle both array format (pi_a, pi_b, pi_c) and object format (a, b, c)
  let a, b, c;
  
  if (Array.isArray(proof.a)) {
    // Groth16 format: pi_a, pi_b, pi_c
    a = proof.a;
    b = proof.b;
    c = proof.c;
  } else {
    a = proof.a;
    b = proof.b;
    c = proof.c;
  }
  
  return JSON.stringify({
    a: a.map(x => hexToBytes(x)),
    b: b.map(x => x.map(y => hexToBytes(y))),
    c: c.map(x => hexToBytes(x)),
  });
}

function invokeVerifyAndRecord({ appId, subject, nullifier, publicInputsHash, vkJson, proofJson, pubSignals, claims }) {
  const tmpDir = "/tmp/zk-invoke";
  fs.mkdirSync(tmpDir, { recursive: true });

  const vkPath = `${tmpDir}/vk.json`;
  const proofPath = `${tmpDir}/proof.json`;
  const claimsPath = `${tmpDir}/claims.json`;
  const pubSignalsPath = `${tmpDir}/pub_signals.json`;
  const pubInputsHashPath = `${tmpDir}/pub_inputs_hash.json`;

  fs.writeFileSync(vkPath, vkJson);
  fs.writeFileSync(proofPath, proofJson);
  fs.writeFileSync(claimsPath, JSON.stringify(claims));
  fs.writeFileSync(pubSignalsPath, JSON.stringify(pubSignals));
  fs.writeFileSync(pubInputsHashPath, JSON.stringify(publicInputsHash));

  const cmd = [
    "stellar-cli", "contract", "invoke",
    "--source", IDENTITY,
    "--rpc-url", RPC_URL,
    "--network-passphrase", NETWORK,
    "--id", CONTRACT_ID,
    "--send=yes",
    "--",
    "verify_and_record",
    "--app-id", appId,
    "--subject", subject,
    "--nullifier", nullifier,
    "--public_inputs_hash", publicInputsHash,
    "--vk-file-path", vkPath,
    "--proof-file-path", proofPath,
    "--pub-signals-file-path", pubSignalsPath,
    "--claims-file-path", claimsPath,
  ];

  console.log("[Wraith] Invoking:", cmd.join(" "));
  console.log("[Wraith] VK:", vkJson.slice(0, 80) + "...");
  console.log("[Wraith] Proof:", proofJson.slice(0, 80) + "...");

  try {
    const result = spawnSync(cmd[0], cmd.slice(1), {
      encoding: "utf8",
      maxBuffer: 1024 * 1024,
    });
    console.log("[Wraith] Result:", result.stdout);
    if (result.status !== 0) {
      console.error("[Wraith] Exit code:", result.status);
      if (result.stderr) console.error("[Wraith] Stderr:", result.stderr.slice(0, 1000));
      return { success: false, error: result.stderr || `exit ${result.status}` };
    }
    return { success: true, output: result.stdout };
  } catch (err) {
    console.error("[Wraith] Error:", err.message);
    return { success: false, error: err.message };
  }
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 3) {
    console.error("Usage: node verify-and-record.mjs <adapter-output.json> <appId> <subject> [nullifier] [publicInputsHash]");
    process.exit(1);
  }

  const [adapterPath, appId, subject, nullifierHex, pubHashHex] = args;

  if (!fs.existsSync(adapterPath)) {
    console.error(`File not found: ${adapterPath}`);
    process.exit(1);
  }

  const adapter = JSON.parse(fs.readFileSync(adapterPath, "utf8"));

  // Use provided nullifier or generate deterministic one based on adapter
  const nullifier = nullifierHex || adapter.nullifier || "00".repeat(32);
  
  // Use provided publicInputsHash or compute from public signals
  let publicInputsHash = pubHashHex || adapter.public_inputs_hash || "00".repeat(32);
  if (publicInputsHash === "00".repeat(32) && adapter.public_signals_decimals) {
    // Simple hash of public signals for demo purposes
    const signalsStr = adapter.public_signals_decimals.join(",");
    const hashBuf = await crypto.subtle.digest('SHA-256', Buffer.from(signalsStr));
    publicInputsHash = Array.from(new Uint8Array(hashBuf)).map(b => b.toString(16).padStart(2, '0')).join('');
  }

  console.log(`[Wraith] Contract: ${CONTRACT_ID}`);
  console.log(`[Wraith] App: ${appId}`);
  console.log(`[Wraith] Subject: ${subject}`);
  console.log(`[Wraith] Nullifier: ${nullifier.slice(0, 16)}...`);
  console.log(`[Wraith] PublicInputsHash: ${publicInputsHash.slice(0, 16)}...`);

  const vkJson = buildVkJson(adapter);
  const proofJson = buildProofJson(adapter);
  const pubSignals = (adapter.public_signals_decimals || adapter.public_signals || []).map(String);
  
  // Claims from adapter or defaults matching existing record
  const claims = adapter.claims || { age: 30, country_code: 840, is_human: true };

  const result = invokeVerifyAndRecord({
    appId,
    subject,
    nullifier,
    publicInputsHash,
    vkJson,
    proofJson,
    pubSignals,
    claims,
  });

  if (result.success) {
    console.log("[Wraith] Invocation successful!");
    process.exit(0);
  } else {
    console.error("[Wraith] Invocation failed!");
    process.exit(1);
  }
}

main();