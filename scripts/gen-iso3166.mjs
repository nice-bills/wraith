#!/usr/bin/env node
/**
 * Regenerate scripts/lib/iso3166.json from lukes/ISO-3166 public dataset.
 * Run: node scripts/gen-iso3166.mjs
 */
import { writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const out = join(dirname(fileURLToPath(import.meta.url)), "lib/iso3166.json");
const res = await fetch(
  "https://raw.githubusercontent.com/lukes/ISO-3166-Countries-with-Regional-Codes/master/all/all.json"
);
if (!res.ok) throw new Error(`fetch failed: ${res.status}`);
const all = await res.json();
const entries = [];
for (const row of all) {
  const alpha2 = row["alpha-2"];
  const alpha3 = row["alpha-3"];
  const num = parseInt(row["country-code"], 10);
  if (!alpha2 || !alpha3 || !num) continue;
  entries.push({ numeric: num, alpha2, alpha3, name: row.name });
}
entries.sort((a, b) => a.numeric - b.numeric);
const byNumeric = Object.fromEntries(entries.map((e) => [String(e.numeric), e]));
const byAlpha2 = Object.fromEntries(entries.map((e) => [e.alpha2, e.numeric]));
const byAlpha3 = Object.fromEntries(entries.map((e) => [e.alpha3, e.numeric]));
writeFileSync(out, JSON.stringify({ entries, byNumeric, byAlpha2, byAlpha3 }));
console.log(`Wrote ${entries.length} countries to ${out}`);
