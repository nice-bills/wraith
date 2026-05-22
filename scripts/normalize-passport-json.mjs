#!/usr/bin/env node
/**
 * Normalize NFC dump JSON → Rarimo-shaped passport.json.
 * Handles nested dumps (zkcreds, NFCPassportReader) and common alias keys.
 *
 * Usage:
 *   node scripts/normalize-passport-json.mjs <raw.json> [out.json]
 *   node scripts/normalize-passport-json.mjs --soft <raw.json> [out.json]   # keep claims-only output
 */
import { readFileSync, writeFileSync } from "node:fs";
import { analyzeDocument } from "./lib/document-path.mjs";

const args = process.argv.slice(2);
let soft = false;
const paths = [];
for (const a of args) {
  if (a === "--soft") soft = true;
  else paths.push(a);
}

const inPath = paths[0];
const outPath = paths[1] ?? inPath;
if (!inPath) {
  console.error("Usage: normalize-passport-json.mjs [--soft] <raw.json> [out.json]");
  process.exit(1);
}

const raw = JSON.parse(readFileSync(inPath, "utf8"));

function flatten(raw, depth = 0) {
  if (depth > 4 || raw == null || typeof raw !== "object" || Array.isArray(raw)) {
    return raw ?? {};
  }
  const merged = { ...raw };
  for (const key of ["data", "passport", "lds", "document", "eMRTD", "result"]) {
    if (raw[key] && typeof raw[key] === "object" && !Array.isArray(raw[key])) {
      Object.assign(merged, raw[key]);
    }
  }
  if (raw.files && typeof raw.files === "object") {
    Object.assign(merged, raw.files);
  }
  return merged;
}

const flat = flatten(raw);

function pick(...keys) {
  for (const k of keys) {
    if (flat[k] != null && String(flat[k]).trim() !== "") return flat[k];
  }
  return undefined;
}

const sod = pick("sod", "SOD", "securityObject", "SecurityObject");
const dg1 = pick("dg1", "DG1", "dg1Base64", "mrz", "MRZ");
const dg15 = pick("dg15", "DG15");

const out = {
  sod,
  dg1,
  dg15,
  dateOfBirth: pick("dateOfBirth", "date_of_birth", "birthDate", "dob"),
  documentExpiryDate: pick("documentExpiryDate", "expiryDate", "dateOfExpiry"),
  documentNumber: pick("documentNumber", "passportNumber"),
  nationality: pick("nationality", "issuerCountry", "issuingState"),
  country_code: flat.country_code ?? flat.countryCode ?? null,
  gender: pick("gender", "sex"),
  lastName: pick("lastName", "surname"),
  firstName: pick("firstName", "givenNames", "givenName"),
  documentType: flat.documentType,
  signature: flat.signature,
  passportImageRaw: pick("passportImageRaw", "faceImage", "photo"),
  issuingAuthority: flat.issuingAuthority,
};

for (const [k, v] of Object.entries(out)) {
  if (v == null || (typeof v === "string" && v.trim() === "")) delete out[k];
}

const analysis = analyzeDocument(raw);
if (analysis.path === "full") out._wraithMode = "full";
else if (analysis.path === "layout") out._wraithMode = "layout";

writeFileSync(outPath, JSON.stringify(out, null, 2) + "\n");
console.log("Wrote", outPath);
console.log("Detected path:", analysis.path, "—", analysis.reason);
console.log("Fields:", Object.keys(out).join(", "));

if (!out.sod || !out.dg1) {
  if (soft || analysis.path === "layout") {
    console.warn("No sod/dg1 — saved claims-only JSON (attested or layout path)");
    process.exit(0);
  }
  console.warn("Missing sod/dg1 — re-scan or use --soft for claims-only output");
  process.exit(1);
}
