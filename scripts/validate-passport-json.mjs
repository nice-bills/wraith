#!/usr/bin/env node
/**
 * Validate Rarimo-shaped passport JSON before process_passport.js.
 * Usage: node scripts/validate-passport-json.mjs path/to/passport.json
 */
import { readFileSync } from "node:fs";

const path = process.argv[2];
if (!path) {
  console.error("Usage: validate-passport-json.mjs <passport.json>");
  process.exit(1);
}

const j = JSON.parse(readFileSync(path, "utf8"));
const missing = [];
for (const key of ["sod", "dg1"]) {
  if (j[key] == null || String(j[key]).trim() === "") missing.push(key);
}
if (missing.length) {
  console.error("Missing required fields:", missing.join(", "));
  console.error("See tools/zk-circuits/fixtures/passport.template.json");
  process.exit(1);
}

const hints = [];
if (!j.dateOfBirth && !j.documentExpiryDate) {
  hints.push("dateOfBirth (needed for on-chain age via layout path)");
}
if (!j.nationality && j.country_code == null) {
  hints.push("nationality or country_code (needed for country claim)");
}
if (hints.length) {
  console.warn("Warnings (layout prove path may fail):", hints.join("; "));
}

console.log("OK: passport JSON has sod + dg1");
if (j.dateOfBirth) console.log("  dateOfBirth:", j.dateOfBirth);
if (j.nationality) console.log("  nationality:", j.nationality);
if (j.country_code != null) console.log("  country_code:", j.country_code);
