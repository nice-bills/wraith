#!/usr/bin/env node
/**
 * Normalize NFC dump JSON → Rarimo-shaped passport.json.
 * Handles common alias keys from Android dumpers / manual exports.
 *
 * Usage: node scripts/normalize-passport-json.mjs <raw.json> [out.json]
 */
import { readFileSync, writeFileSync } from "node:fs";

const inPath = process.argv[2];
const outPath = process.argv[3] ?? inPath;
if (!inPath) {
  console.error("Usage: normalize-passport-json.mjs <raw.json> [out.json]");
  process.exit(1);
}

const raw = JSON.parse(readFileSync(inPath, "utf8"));

function pick(...keys) {
  for (const k of keys) {
    if (raw[k] != null && String(raw[k]).trim() !== "") return raw[k];
  }
  return undefined;
}

const sod = pick("sod", "SOD", "securityObject", "SecurityObject");
const dg1 = pick("dg1", "DG1", "dg1Base64", "mrz");
const dg15 = pick("dg15", "DG15");

const out = {
  sod,
  dg1,
  dg15,
  dateOfBirth: pick("dateOfBirth", "date_of_birth", "birthDate", "dob"),
  documentExpiryDate: pick("documentExpiryDate", "expiryDate", "dateOfExpiry"),
  documentNumber: pick("documentNumber", "passportNumber"),
  nationality: pick("nationality", "issuerCountry"),
  country_code: raw.country_code ?? raw.countryCode ?? null,
  gender: pick("gender", "sex"),
  lastName: pick("lastName", "surname"),
  firstName: pick("firstName", "givenNames", "givenName"),
  documentType: raw.documentType,
  signature: raw.signature,
  passportImageRaw: pick("passportImageRaw", "faceImage"),
  issuingAuthority: raw.issuingAuthority,
};

for (const [k, v] of Object.entries(out)) {
  if (v == null || (typeof v === "string" && v.trim() === "")) delete out[k];
}

writeFileSync(outPath, JSON.stringify(out, null, 2) + "\n");
console.log("Wrote", outPath);
console.log("Fields:", Object.keys(out).join(", "));
if (!out.sod || !out.dg1) {
  console.warn("Missing sod/dg1 — re-scan or check dumper format (docs/PASSPORT_SCAN.md)");
  process.exit(1);
}
