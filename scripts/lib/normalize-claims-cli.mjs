#!/usr/bin/env node
/**
 * Normalize attested claims to JSON on stdout.
 * Usage:
 *   node scripts/lib/normalize-claims-cli.mjs <claims.json>
 *   node scripts/lib/normalize-claims-cli.mjs --age 30 --country NG [--human|--no-human]
 */
import { readFileSync } from "node:fs";
import { normalizeAttestedClaims } from "./country-codes.mjs";

const args = process.argv.slice(2);
let raw = null;

if (args[0] && !args[0].startsWith("-")) {
  raw = JSON.parse(readFileSync(args[0], "utf8"));
} else {
  let age;
  let country;
  let is_human = true;
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === "--age") age = Number(args[++i]);
    else if (a === "--country") country = args[++i];
    else if (a === "--human") is_human = true;
    else if (a === "--no-human") is_human = false;
    else {
      console.error(`Unknown arg: ${a}`);
      process.exit(1);
    }
  }
  raw = { age, country_code: country, is_human };
}

process.stdout.write(JSON.stringify(normalizeAttestedClaims(raw)));
