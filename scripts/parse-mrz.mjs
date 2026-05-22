#!/usr/bin/env node
/**
 * Parse ICAO MRZ (TD1 card or TD3 passport) → attested claims JSON.
 *
 * Usage:
 *   node scripts/parse-mrz.mjs "P<USASMITH<<JOHN<<<<<<<<<<<<<<<<<<<<<<<"
 *   node scripts/parse-mrz.mjs --file mrz.txt
 */
import { readFileSync } from "node:fs";
import { mrzToAttestedClaims } from "./lib/parse-mrz.mjs";

const args = process.argv.slice(2);
let mrzText = "";
if (args[0] === "--file") {
  mrzText = readFileSync(args[1], "utf8");
} else {
  mrzText = args.join("\n");
}

if (!mrzText.trim()) {
  console.error("Usage: parse-mrz.mjs <MRZ lines> | parse-mrz.mjs --file mrz.txt");
  process.exit(1);
}

try {
  const claims = mrzToAttestedClaims(mrzText);
  console.log(JSON.stringify(claims, null, 2));
} catch (err) {
  console.error(err.message);
  process.exit(1);
}
