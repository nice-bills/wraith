import {
  Keypair,
  TransactionBuilder,
  xdr,
  Address,
  Contract,
  Account,
} from "@stellar/stellar-sdk";
import crypto from "crypto";
import fs from "fs";

const CONTRACT_ID = process.env.CONTRACT_ID || "CDCQKLVESDP3PUQBI2LKSTKPDPXOSUNEQFKNNZDEWQFOJ2LJN3DY65A6";
const FUTURENET_RPC = process.env.RPC_URL || "https://rpc-futurenet.stellar.org:443";
const NETWORK_PASSPHRASE = "Test SDF Future Network ; October 2022";

function decimalToBeHex(decimalStr, byteLen) {
  const hex = BigInt(decimalStr).toString(16);
  return hex.padStart(byteLen * 2, '0');
}

function parseDecimalG1(arr) {
  const x = decimalToBeHex(arr[0], 32);
  const y = decimalToBeHex(arr[1], 32);
  return Buffer.from(x + y, 'hex');
}

function parseDecimalG2(arr) {
  const c0 = decimalToBeHex(arr[0], 32);
  const c1 = decimalToBeHex(arr[1], 32);
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
  const allBytes = Buffer.concat([vk.alpha, vk.beta, vk.gamma, vk.delta, ...vk.ic]);
  return crypto.createHash('sha256').update(allBytes).digest('hex');
}

function computePubInputsHash(pubSignals) {
  const chunks = pubSignals.map(s =>
    Buffer.from(decimalToBeHex(s, 32), 'hex')
  );
  return crypto.createHash('sha256').update(Buffer.concat(chunks)).digest('hex');
}

function deriveNullifier(privateInputs) {
  return crypto.createHash('sha256').update(JSON.stringify(privateInputs)).digest('hex');
}

async function fetchAccount(address) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "getAccount",
      params: { account_id: address },
    }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function simulateTx(txXdr) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 2,
      method: "simulateTransaction",
      params: { transaction: txXdr },
    }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 5) {
    console.error("Usage: SECRET_KEY=<key> node verify-call.mjs <vkJson> <proofJson> <pubSignalsJson> <appId> <subject> [claimsJson]");
    process.exit(1);
  }

  const secret = process.env.SECRET_KEY;
  if (!secret) {
    console.error("Error: SECRET_KEY environment variable is required");
    process.exit(1);
  }

  const [vkPath, proofPath, pubSignalsPath, appId, subject, claimsJson] = args;

  const vkRaw = parseVkFromSnarkjs(JSON.parse(fs.readFileSync(vkPath, 'utf8')));
  const proofRaw = parseProofFromSnarkjs(JSON.parse(fs.readFileSync(proofPath, 'utf8')));
  const pubSignals = JSON.parse(fs.readFileSync(pubSignalsPath, 'utf8'));
  const claims = claimsJson ? JSON.parse(claimsJson) : { age: 30, country_code: 840, is_human: true };

  console.log("=== verify_and_record via Contract.call ===\n");
  console.log(`Contract: ${CONTRACT_ID}`);
  console.log(`App: ${appId}`);
  console.log(`Subject: ${subject}`);
  console.log(`PubSignals: ${pubSignals.join(', ')}`);

  const keypair = Keypair.fromSecret(secret);
  const sourceAddress = keypair.publicKey();
  console.log(`Source: ${sourceAddress}`);

  const sourceAccount = await fetchAccount(sourceAddress).catch(() => null);
  const seqNum = sourceAccount ? parseInt(sourceAccount.sequence) : 0;
  const account = new Account(sourceAddress, String(seqNum));

  const contract = new Contract(CONTRACT_ID);

  const vkHash = computeVkHash(vkRaw);
  const pubInputsHash = computePubInputsHash(pubSignals);
  const nullifierHex = process.env.NULLIFIER_HEX || deriveNullifier(claims);

  console.log(`VK hash: ${vkHash}`);
  console.log(`Pub inputs hash: ${pubInputsHash}`);

  const vkScVal = xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("alpha"), val: xdr.ScVal.scvBytes(vkRaw.alpha) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("beta"), val: xdr.ScVal.scvBytes(vkRaw.beta) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("gamma"), val: xdr.ScVal.scvBytes(vkRaw.gamma) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("delta"), val: xdr.ScVal.scvBytes(vkRaw.delta) }),
    new xdr.ScMapEntry({
      key: xdr.ScVal.scvSymbol("ic"),
      val: xdr.ScVal.scvVec(vkRaw.ic.map(g1 => xdr.ScVal.scvBytes(g1)))
    }),
  ]);

  const proofScVal = xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: xdr.ScVal.scvBytes(proofRaw.a) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: xdr.ScVal.scvBytes(proofRaw.b) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: xdr.ScVal.scvBytes(proofRaw.c) }),
  ]);

  const pubSignalsScVal = xdr.ScVal.scvVec(
    pubSignals.map(s => xdr.ScVal.scvU256(new xdr.UInt64({ high: BigInt(s), low: BigInt(0) })))
  );

  const claimsScVal = xdr.ScVal.scvMap([
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("age"), val: xdr.ScVal.scvU32(claims.age) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("country_code"), val: xdr.ScVal.scvU32(claims.country_code) }),
    new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("is_human"), val: xdr.ScVal.scvBool(claims.is_human) }),
  ]);

  const op = contract.call(
    "verify_and_record",
    xdr.ScVal.scvSymbol(appId),
    new Address(subject).toScVal(),
    xdr.ScVal.scvBytes(Buffer.from(nullifierHex.replace(/^0x/, ''), 'hex')),
    xdr.ScVal.scvBytes(Buffer.from(pubInputsHash, 'hex')),
    vkScVal,
    proofScVal,
    pubSignalsScVal,
    claimsScVal
  );

  const tx = new TransactionBuilder(account, {
    fee: "300000",
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .setTimeout(60)
    .addOperation(op)
    .build();

  console.log("\nBuilt transaction, simulating...");
  const simResult = await simulateTx(tx.toXDR("base64"));

  if (simResult.results && simResult.results[0]) {
    console.log("\nSimulation SUCCESS!");
    console.log("Cost:", JSON.stringify(simResult.results[0].cost, null, 2));
    console.log("NOTE: Full submission requires assembleTransaction approach for Soroban auth");
  } else if (simResult.error) {
    console.error("\nSimulation ERROR:", simResult.error);
    process.exit(1);
  } else {
    console.log("\nFull simulation result:", JSON.stringify(simResult, null, 2).slice(0, 1000));
  }
}

main().catch(console.error);