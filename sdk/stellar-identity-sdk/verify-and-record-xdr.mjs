#!/usr/bin/env node
/**
 * Wraith SDK - verify_and_record with proper XDR encoding
 *
 * Handles snarkjs decimal array format from adapter output files.
 * Computes VK hash and public_inputs_hash correctly for the contract.
 *
 * Usage:
 *   SECRET_KEY=<secret> NULLIFIER_HEX=<hex> CONTRACT_ID=<id> \
 *     node verify-and-record-xdr.mjs <age_verifier.json> <snark_proof.json> <public.json> <appId> <subject> [claims]
 *
 * Example:
 *   SECRET_KEY=S... NULLIFIER_HEX=aa7ab... \
 *     node verify-and-record-xdr.mjs \
 *       ../tools/zk-circuits/build/age_verifier.json \
 *       ../tools/zk-circuits/proof/snark_proof.json \
 *       ../tools/zk-circuits/proof/public.json \
 *       age_check \
 *       GDYLBKGXBZ4QWVKZGGEWEZ2CY4BNVULZ62JNI6JYGOENLNH6I5RC5SA5 \
 *       '{"age":30,"country_code":840,"is_human":true}'
 */

import pkg from "@stellar/stellar-sdk";
const { Keypair, TransactionBuilder, xdr, Address, Contract, Account } = pkg;
import crypto from "crypto";
import fs from "fs";

const CONTRACT_ID = process.env.CONTRACT_ID || "CDCQKLVESDP3PUQBI2LKSTKPDPXOSUNEQFKNNZDEWQFOJ2LJN3DY65A6";
const FUTURENET_RPC = process.env.RPC_URL || "https://rpc-futurenet.stellar.org:443";
const NETWORK_PASSPHRASE = "Test SDF Future Network ; October 2022";

function isDecimalString(val) {
  return typeof val === "string" && /^\d+$/.test(val);
}

function decimalToBeHex(decimalStr, byteLen) {
  const hex = BigInt(decimalStr).toString(16);
  return hex.padStart(byteLen * 2, '0');
}

function parseDecimalG1(arr) {
  if (arr.length < 2) throw new Error(`G1 needs at least 2 coords, got ${arr.length}`);
  const x = decimalToBeHex(arr[0], 32);
  const y = decimalToBeHex(arr[1], 32);
  return Buffer.from(x + y, 'hex');
}

function parseDecimalG2(arr) {
  if (arr.length < 2) throw new Error(`G2 needs at least 2 coords, got ${arr.length}`);
  const x = parseDecimalFp2(arr[0]);
  const y = parseDecimalFp2(arr[1]);
  return Buffer.concat([x, y]);
}

function parseDecimalFp2(pair) {
  if (!Array.isArray(pair) || pair.length < 2) {
    throw new Error(`Fp2 needs [c0, c1], got ${JSON.stringify(pair)}`);
  }
  const c0 = decimalToBeHex(pair[0], 32);
  const c1 = decimalToBeHex(pair[1], 32);
  return Buffer.from(c1 + c0, 'hex');
}

function parseVkFromSnarkjs(vk) {
  return {
    alpha: parseDecimalG1(vk.vk_alpha_1),
    beta: parseDecimalG2(vk.vk_beta_2),
    gamma: parseDecimalG2(vk.vk_gamma_2),
    delta: parseDecimalG2(vk.vk_delta_2),
    ic: vk.IC.map(ic => parseDecimalG1(ic)),
  };
}

function parseProofFromSnarkjs(proof) {
  return {
    a: parseDecimalG1(proof.pi_a),
    b: parseDecimalG2(proof.pi_b),
    c: parseDecimalG1(proof.pi_c),
  };
}

function computeVkHash(vk) {
  const allBytes = Buffer.concat([
    vk.alpha, vk.beta, vk.gamma, vk.delta,
    ...vk.ic
  ]);
  return crypto.createHash('sha256').update(allBytes).digest('hex');
}

function computePubInputsHash(pubSignals) {
  const chunks = pubSignals.map(s => {
    const hex = decimalToBeHex(s, 32);
    return Buffer.from(hex, 'hex');
  });
  return crypto.createHash('sha256').update(Buffer.concat(chunks)).digest('hex');
}

function deriveNullifier(privateInputs) {
  return crypto.createHash('sha256').update(JSON.stringify(privateInputs)).digest('hex');
}

function encodeVkScVal(vk) {
  return xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("alpha"), val: xdr.ScVal.scvBytes(vk.alpha) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("beta"), val: xdr.ScVal.scvBytes(vk.beta) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("gamma"), val: xdr.ScVal.scvBytes(vk.gamma) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("delta"), val: xdr.ScVal.scvBytes(vk.delta) }),
    new xdr.ScMapEntry({
      key: xdr.ScVal.scvSymbol("ic"),
      val: xdr.ScVal.scvVec(vk.ic.map(g1 => xdr.ScVal.scvBytes(g1)))
    }),
  ]);
}

function encodeProofScVal(proof) {
  return xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: xdr.ScVal.scvBytes(proof.a) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: xdr.ScVal.scvBytes(proof.b) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: xdr.ScVal.scvBytes(proof.c) }),
  ]);
}

function encodePubSignals(signals) {
  return xdr.ScVal.scvVec(signals.map(s => {
    const n = BigInt(s);
    return xdr.ScVal.scvU256(new xdr.UInt64({ high: n, low: BigInt(0) }));
  }));
}

function encodeClaims(claims) {
  return xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("age"), val: xdr.ScVal.scvU32(claims.age) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("country_code"), val: xdr.ScVal.scvU32(claims.country_code) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("is_human"), val: xdr.ScVal.scvBool(claims.is_human) }),
  ]);
}

async function getAccount(address) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getAccount", params: { account_id: address } }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function simulateTx(txXdr) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 2, method: "simulateTransaction", params: { transaction: txXdr } }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function sendTx(txXdr) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 3, method: "sendTransaction", params: { transaction: txXdr } }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 5) {
    console.error("Usage: SECRET_KEY=<key> NULLIFIER_HEX=<hex> CONTRACT_ID=<id> node verify-and-record-xdr.mjs <vkJson> <proofJson> <pubSignalsJson> <appId> <subject> [claimsJson]");
    console.error("  vkJson:         path to age_verifier.json (snarkjs format)");
    console.error("  proofJson:       path to snark_proof.json");
    console.error("  pubSignalsJson:  path to public.json");
    console.error("  appId:          e.g. age_check");
    console.error("  subject:        Stellar public key (G...)");
    console.error("  claimsJson:     optional, e.g. '{\"age\":30,\"country_code\":840,\"is_human\":true}'");
    process.exit(1);
  }

  const secret = process.env.SECRET_KEY;
  if (!secret) {
    console.error("Error: SECRET_KEY environment variable is required");
    process.exit(1);
  }

  const [vkPath, proofPath, pubSignalsPath, appId, subject, claimsJson] = args;

  let vkRaw, proofRaw, pubSignals, claims;
  try {
    vkRaw = parseVkFromSnarkjs(JSON.parse(fs.readFileSync(vkPath, 'utf8')));
    proofRaw = parseProofFromSnarkjs(JSON.parse(fs.readFileSync(proofPath, 'utf8')));
    pubSignals = JSON.parse(fs.readFileSync(pubSignalsPath, 'utf8'));
  } catch (e) {
    console.error(`Failed to read input files: ${e.message}`);
    process.exit(1);
  }

  claims = claimsJson ? JSON.parse(claimsJson) : { age: 30, country_code: 840, is_human: true };

  const vkHash = computeVkHash(vkRaw);
  const pubInputsHash = computePubInputsHash(pubSignals);
  const nullifierHex = process.env.NULLIFIER_HEX || deriveNullifier(claims);

  console.log(`[Wraith] Contract: ${CONTRACT_ID}`);
  console.log(`[Wraith] App: ${appId}`);
  console.log(`[Wraith] Subject: ${subject}`);
  console.log(`[Wraith] VK hash: ${vkHash}`);
  console.log(`[Wraith] Pub inputs hash: ${pubInputsHash}`);
  console.log(`[Wraith] Nullifier: ${nullifierHex.slice(0, 16)}...`);
  console.log(`[Wraith] PubSignals: ${pubSignals.join(', ')}`);

  const keypair = Keypair.fromSecret(secret);
  const sourceAddress = keypair.publicKey();
  console.log(`[Wraith] Source: ${sourceAddress}`);

  let sourceAccount = await getAccount(sourceAddress).catch(() => null);
  const seqNum = sourceAccount ? parseInt(sourceAccount.sequence) : 0;
  const account = new Account(sourceAddress, String(seqNum));

  const contract = new Contract(CONTRACT_ID);

  const op = contract.call(
    "verify_and_record",
    xdr.ScVal.scvSymbol(appId),
    new Address(subject).toScVal(),
    xdr.ScVal.scvBytes(Buffer.from(nullifierHex.replace(/^0x/, ''), 'hex')),
    xdr.ScVal.scvBytes(Buffer.from(pubInputsHash, 'hex')),
    encodeVkScVal(vkRaw),
    encodeProofScVal(proofRaw),
    encodePubSignals(pubSignals),
    encodeClaims(claims)
  );

  const tx = new TransactionBuilder(account, {
    fee: "300000",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .setTimeout(60)
    .addOperation(op)
    .build();

  console.log("[Wraith] Built transaction, simulating...");
  const simResult = await simulateTx(tx.toXDR("base64"));

  if (simResult.results && simResult.results[0]) {
    console.log("[Wraith] Simulation SUCCESS!");
    console.log("[Wraith] Cost:", JSON.stringify(simResult.results[0].cost, null, 2));

    const auths = simResult.results[0].auth;
    const needsAuth = auths && auths.length > 0;
    console.log(`[Wraith] Needs auth: ${needsAuth}`);

    if (needsAuth) {
      console.log("[Wraith] This transaction requires authorization");
      process.exit(1);
    } else {
      console.log("[Wraith] Submitting transaction...");
      keypair.sign(tx);
      const submitted = await sendTx(tx.toXDR("base64"));
      console.log("[Wraith] Submitted:", JSON.stringify(submitted, null, 2));

      if (submitted.status === "PENDING" || submitted.status === "CONFIRMED" || submitted.status === "SUCCESS") {
        console.log("[Wraith] SUCCESS! Transaction submitted.");
      } else {
        console.log("[Wraith] Transaction status:", submitted.status);
        if (submitted.error) {
          console.error("[Wraith] Error:", submitted.error);
        }
      }
    }
  } else if (simResult.error) {
    console.error("[Wraith] Simulation ERROR:", simResult.error);
    process.exit(1);
  } else if (simResult.results?.[0]?.error) {
    console.error("[Wraith] Result ERROR:", simResult.results[0].error);
    process.exit(1);
  } else {
    console.log("[Wraith] Full simulation result:", JSON.stringify(simResult, null, 2).slice(0, 2000));
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("[Wraith] Fatal error:", err);
  process.exit(1);
});