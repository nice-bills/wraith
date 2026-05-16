#!/usr/bin/env node
/**
 * Build queryIdentity witness input from register circuit input + identity SMT.
 * Mirrors rarimo/helpers/generateRegisterIdentityTest.js (single-leaf dev tree).
 *
 * Usage: node build-rarimo-query-input.mjs <register-input.json> [register-public.json] [out.json]
 */
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "../../..");
const RARIMO = join(ROOT, "tools/zk-circuits/rarimo");

const require = createRequire(join(RARIMO, "package.json"));
const { Poseidon } = require("@iden3/js-crypto");
const { babyJub } = require("@iden3/js-crypto");

const registerPath = process.argv[2];
const registerPublicPath = process.argv[3];
const outPath = process.argv[4] ?? join(dirname(registerPath), "query-input.json");

if (!registerPath) {
  console.error("Usage: build-rarimo-query-input.mjs <register-input.json> [register-public.json] [out.json]");
  process.exit(1);
}

const reg = JSON.parse(readFileSync(registerPath, "utf8"));
const EMPTY_DATE = "52983525027888"; // 0x303030303030 — unused date bounds

function docTypeFromName(name) {
  const m = name?.match(/registerIdentity_\d+_\d+_(\d+)_/);
  return m ? parseInt(m[1], 10) : 3;
}

const baseName = registerPath.replace(/.*\//, "").replace(/\.json$/, "");
const docType = docTypeFromName(baseName);
const dg1Len = docType === 3 ? 744 : 760;

if (!registerPublicPath) {
  console.error("register-public.json required (from passport-register-witness.sh)");
  process.exit(1);
}
const pub = JSON.parse(readFileSync(registerPublicPath, "utf8"));
const dg15Hash = Array.isArray(pub) ? pub[0] : pub.dg15PubKeyHash;
const passportHash = Array.isArray(pub) ? pub[1] : pub.passportHash;
const pkPassportHash =
  dg15Hash && dg15Hash !== "0" && dg15Hash !== 0 ? String(dg15Hash) : String(passportHash);

// TD1: country often in citizenship (public [5]); TD3: nationality at [5]. Witness uses dg1 layout only.

const skIdentity = BigInt(reg.skIdentity);
const dg1Bits = reg.dg1.slice(0, dg1Len);

const chunking = ["", "", "", ""];
for (let i = 0; i < 4; i++) {
  for (let j = 0; j < dg1Len / 4; j++) {
    chunking[i] += dg1Bits[i * (dg1Len / 4) + (dg1Len / 4 - 1 - j)].toString();
  }
}

const skHash = Poseidon.hash([skIdentity]);
const dgCommit = Poseidon.hash([
  BigInt(`0b${chunking[0]}`),
  BigInt(`0b${chunking[1]}`),
  BigInt(`0b${chunking[2]}`),
  BigInt(`0b${chunking[3]}`),
  skHash,
]);

const timestampSeconds = Math.floor(Date.now() / 1000).toString();
const value = Poseidon.hash([dgCommit, 1n, BigInt(timestampSeconds)]);

const pubkey = babyJub.mulPointEscalar(babyJub.Base8, skIdentity);
const pk_hash = Poseidon.hash(pubkey);
const index = Poseidon.hash([BigInt(pkPassportHash), pk_hash]);
const idStateRoot = Poseidon.hash([index, value, 1n]).toString();
const idStateSiblings = new Array(80).fill("0");

const now = new Date();
const yy = String(now.getUTCFullYear()).slice(2);
const mm = String(now.getUTCMonth() + 1).padStart(2, "0");
const dd = String(now.getUTCDate()).padStart(2, "0");
const currentDate = `0x3${yy[0]}3${yy[1]}3${mm[0]}3${mm[1]}3${dd[0]}3${dd[1]}`;

const queryInput = {
  dg1: dg1Bits,
  eventID: process.env.RARIMO_EVENT_ID ?? "304358862882731539112827930982999386691702727710421481944329166126417129570",
  eventData: process.env.RARIMO_EVENT_DATA ?? "1217571210886365587192326979343136122389414675532",
  idStateRoot,
  idStateSiblings,
  pkPassportHash: String(pkPassportHash),
  skIdentity: String(skIdentity),
  selector: "0",
  timestamp: timestampSeconds,
  currentDate,
  identityCounter: "1",
  timestampLowerbound: "0",
  timestampUpperbound: "19000000000",
  identityCounterLowerbound: "0",
  identityCounterUpperbound: "1000",
  birthDateLowerbound: EMPTY_DATE,
  birthDateUpperbound: EMPTY_DATE,
  expirationDateLowerbound: EMPTY_DATE,
  expirationDateUpperbound: "0x333030303030",
  citizenshipMask: "0",
};

writeFileSync(outPath, JSON.stringify(queryInput, null, 2));
console.log("Wrote", outPath);
console.log("idStateRoot:", idStateRoot);
console.log("pkPassportHash:", pkPassportHash);
