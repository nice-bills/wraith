#!/usr/bin/env node
/**
 * Build attested claims JSON from passport JSON or MRZ.
 *
 * Usage:
 *   node scripts/build-attested-claims.mjs passport-data/my-id.json
 *   node scripts/build-attested-claims.mjs --mrz "P<USA..."
 *   node scripts/build-attested-claims.mjs --out claims.json passport.json
 */
import { readFileSync, writeFileSync } from "node:fs";
import {
  ageFromBirthYymmdd,
  countryCodeFromNationality,
  currentYymmddUtc,
  normalizeAttestedClaims,
  parseBirthYymmdd,
  resolveCountryCode,
} from "./lib/country-codes.mjs";
import { analyzeDocumentFile } from "./lib/document-path.mjs";
import { mrzToAttestedClaims } from "./lib/parse-mrz.mjs";

const args = process.argv.slice(2);
let outPath = null;
let mrzMode = false;
const paths = [];

for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a === "--out" || a === "-o") {
    outPath = args[++i];
    continue;
  }
  if (a === "--mrz") {
    mrzMode = true;
    continue;
  }
  paths.push(a);
}

function fromPassportJson(path) {
  const j = JSON.parse(readFileSync(path, "utf8"));
  const analysis = analyzeDocumentFile(path);

  if (analysis.path === "attested") {
    return normalizeAttestedClaims(j);
  }

  const birthRaw =
    j.dateOfBirth ?? j.date_of_birth ?? j.birthDate ?? j.dob ?? null;
  if (!birthRaw) {
    throw new Error("missing dateOfBirth — scan NFC, parse MRZ, or set age manually");
  }
  const birthYymmdd = parseBirthYymmdd(birthRaw);
  const countrySrc =
    j.country_code ??
    j.countryCode ??
    j.country ??
    j.nationality ??
    j.issuerCountry;
  const countryCode = countrySrc != null
    ? resolveCountryCode(countrySrc)
    : countryCodeFromNationality(j.nationality ?? j.issuerCountry);
  const age = ageFromBirthYymmdd(birthYymmdd, currentYymmddUtc());
  return { age, country_code: countryCode, is_human: true };
}

let claims;
if (mrzMode) {
  claims = mrzToAttestedClaims(paths.join("\n"));
  delete claims._source;
} else {
  const path = paths[0];
  if (!path) {
    console.error(
      "Usage: build-attested-claims.mjs <passport.json> | build-attested-claims.mjs --mrz <lines>"
    );
    process.exit(1);
  }
  claims = fromPassportJson(path);
}

const json = JSON.stringify(claims, null, 2) + "\n";
if (outPath) {
  writeFileSync(outPath, json);
  console.log("Wrote", outPath);
} else {
  process.stdout.write(json);
}
