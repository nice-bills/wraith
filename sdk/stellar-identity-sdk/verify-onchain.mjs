import pkg from "@stellar/stellar-sdk";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
const { Keypair, SorobanTransactionBuilder, Networks, xdr, Address, StrKey } = pkg;

const CONTRACT_ID = process.env.CONTRACT_ID || "CDCQKLVESDP3PUQBI2LKSTKPDPXOSUNEQFKNNZDEWQFOJ2LJN3DY65A6";
const FUTURENET_RPC = process.env.RPC_URL || "https://rpc-futurenet.stellar.org:443";
const NETWORK_PASSPHRASE = "Test SDF Future Network ; October 2022";

const SECRET = process.env.SECRET_KEY;
if (!SECRET) {
  console.error("Error: SECRET_KEY environment variable is required");
  process.exit(1);
}

function decimalToBeHex(decimalStr, byteLen) {
  const hex = BigInt(decimalStr).toString(16);
  return hex.padStart(byteLen * 2, '0');
}

function decimalToLeHex(decimalStr, byteLen) {
  let n = BigInt(decimalStr);
  const bytes = [];
  for (let i = 0; i < byteLen; i++) {
    bytes.push(Number(n & BigInt(0xff)).toString(16).padStart(2, '0'));
    n >>= BigInt(8);
  }
  return bytes.join('');
}

function parseDecimalG1(arr) {
  const x = decimalToBeHex(arr[0], 32);
  const y = decimalToBeHex(arr[1], 32);
  return Buffer.from(x + y, 'hex');
}

function parseDecimalG2(arr) {
  const x = parseDecimalFp2(arr[0]);
  const y = parseDecimalFp2(arr[1]);
  return Buffer.concat([x, y]);
}

function parseDecimalFp2(pair) {
  const c0 = decimalToBeHex(pair[0], 32);
  const c1 = decimalToBeHex(pair[1], 32);
  return Buffer.from(c1 + c0, 'hex');
}

function computeVkHash(alpha, beta, gamma, delta, ic) {
  const allBytes = Buffer.concat([alpha, beta, gamma, delta, ...ic]);
  return createHash('sha256').update(allBytes).digest('hex');
}

function computePubInputsHash(pubSignals) {
  const chunks = pubSignals.map(s => Buffer.from(decimalToBeHex(s, 32), 'hex'));
  return createHash('sha256').update(Buffer.concat(chunks)).digest('hex');
}

function scBytesN(buf) {
  return xdr.ScVal.scvBytes(buf);
}

async function getAccount(address) {
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

async function simulateTx(tx) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 2,
      method: "simulateTransaction",
      params: { transaction: tx.toXDR("base64") },
    }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function sendTx(tx) {
  const res = await fetch(FUTURENET_RPC, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 3,
      method: "sendTransaction",
      params: { transaction: tx.toXDR("base64") },
    }),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length < 5) {
    console.error("Usage: SECRET_KEY=<key> CONTRACT_ID=<id> node verify-onchain.mjs <vkJson> <proofJson> <pubSignalsJson> <appId> <subject> [claimsJson]");
    console.error("  vkJson:        path to age_verifier.json from circom build");
    console.error("  proofJson:     path to snark_proof.json");
    console.error("  pubSignalsJson: path to public.json");
    console.error("  appId:         e.g. age_check");
    console.error("  subject:       Stellar public key (G...)");
    console.error("  claimsJson:    optional, e.g. '{\"age\":30,\"country_code\":840,\"is_human\":true}'");
    process.exit(1);
  }

  const [vkPath, proofPath, pubSignalsPath, appId, subject, claimsJson] = args;

  const vk = JSON.parse(readFileSync(vkPath, 'utf8'));
  const proof = JSON.parse(readFileSync(proofPath, 'utf8'));
  const pubSignals = JSON.parse(readFileSync(pubSignalsPath, 'utf8'));
  const claims = claimsJson ? JSON.parse(claimsJson) : { age: 30, country_code: 840, is_human: true };

  console.log("=== verify_and_record with BN254 proof ===\n");
  console.log(`Contract: ${CONTRACT_ID}`);
  console.log(`App: ${appId}`);
  console.log(`Subject: ${subject}`);
  console.log(`PubSignals: ${pubSignals.join(', ')}`);

  const keypair = Keypair.fromSecret(SECRET);
  const sourceAddress = keypair.publicKey();
  console.log(`Source: ${sourceAddress}`);

  const sourceAccount = await getAccount(sourceAddress);
  const account = new pkg.SorobanAccount(sourceAddress, sourceAccount.sequence);
  const contract = new xdr.Contract(StrKey.decodeContractId(CONTRACT_ID));

  const vkAlpha = parseDecimalG1(vk.vk_alpha_1);
  const vkBeta = parseDecimalG2(vk.vk_beta_2);
  const vkGamma = parseDecimalG2(vk.vk_gamma_2);
  const vkDelta = parseDecimalG2(vk.vk_delta_2);
  const vkIc = vk.IC.map(ic => parseDecimalG1(ic));

  const vkHash = computeVkHash(vkAlpha, vkBeta, vkGamma, vkDelta, vkIc);
  console.log(`VK hash: ${vkHash}`);

  const pubInputsHash = computePubInputsHash(pubSignals);
  console.log(`Pub inputs hash: ${pubInputsHash}`);

  const verificationKeyScVal = xdr.ScVal.scvMap(new xdr.ScMap({
    keyVals: [
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("alpha"), val: scBytesN(vkAlpha) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("beta"), val: scBytesN(vkBeta) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("gamma"), val: scBytesN(vkGamma) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("delta"), val: scBytesN(vkDelta) }),
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("ic"),
        val: xdr.ScVal.scvVec(new xdr.ScVec({ vec: vkIc.map(scBytesN) }))
      }),
    ]
  }));

  const proofA = parseDecimalG1(proof.pi_a);
  const proofB = parseDecimalG2(proof.pi_b);
  const proofC = parseDecimalG1(proof.pi_c);

  const proofScVal = xdr.ScVal.scvMap(new xdr.ScMap({
    keyVals: [
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("a"), val: scBytesN(proofA) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("b"), val: scBytesN(proofB) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("c"), val: scBytesN(proofC) }),
    ]
  }));

  const pubSignalsScVal = xdr.ScVal.scvVec(
    new xdr.ScVec({
      vec: pubSignals.map(s => xdr.ScVal.scvU256(new xdr.UInt64({ high: BigInt(s), low: BigInt(0) })))
    })
  );

  const claimsScVal = xdr.ScVal.scvMap(new xdr.ScMap({
    keyVals: [
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("age"), val: xdr.ScVal.scvU32(claims.age) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("country_code"), val: xdr.ScVal.scvU32(claims.country_code) }),
      new xdr.ScMapEntry({ key: xdr.ScVal.scvSymbol("is_human"), val: xdr.ScVal.scvBool(claims.is_human) }),
    ]
  }));

  const nullifierBytes = Buffer.from(process.env.NULLIFIER_HEX || 'aa7ab731cfc7097867559684457e65a7e523e8fe3b568b212dc2a4a7062dc56f', 'hex');
  const nullifierScVal = scBytesN(nullifierBytes);
  const publicInputsHashScVal = scBytesN(Buffer.from(pubInputsHash, 'hex'));
  const subjectScVal = new Address(subject).toScVal();
  const appIdScVal = xdr.ScVal.scvSymbol(appId);

  const tx = new SorobanTransactionBuilder(account, {
    fee: 300000,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .setTimeout(60)
    .addOperation(contract.call(
      "verify_and_record",
      appIdScVal,
      subjectScVal,
      nullifierScVal,
      publicInputsHashScVal,
      verificationKeyScVal,
      proofScVal,
      pubSignalsScVal,
      claimsScVal
    ))
    .build();

  console.log("\nBuilt transaction, simulating...");
  const simResult = await simulateTx(tx);

  if (simResult.results && simResult.results[0]) {
    console.log("\nSimulation SUCCESS!");
    console.log("Cost:", JSON.stringify(simResult.results[0].cost, null, 2));

    const auths = simResult.results[0].auth;
    const needsAuth = auths && auths.length > 0;

    if (needsAuth) {
      console.log("\nTransaction requires authorization - using assembleTransaction approach");
      process.exit(1);
    } else {
      console.log("\nSubmitting transaction...");
      keypair.sign(tx);
      const submitted = await sendTx(tx);
      console.log("Submitted:", JSON.stringify(submitted, null, 2));

      if (submitted.status === "PENDING" || submitted.status === "CONFIRMED" || submitted.status === "SUCCESS") {
        console.log("\nSUCCESS! Transaction submitted.");
      } else {
        console.log("\nTransaction status:", submitted.status);
        if (submitted.error) {
          console.error("Error:", submitted.error);
        }
      }
    }
  } else if (simResult.error) {
    console.error("\nSimulation ERROR:", simResult.error);
    process.exit(1);
  } else {
    console.log("\nFull simulation result:", JSON.stringify(simResult, null, 2).slice(0, 1000));
    process.exit(1);
  }
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});