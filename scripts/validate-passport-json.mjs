#!/usr/bin/env node
/**
 * Validate passport JSON for layout or full (Rarimo) paths.
 *
 * Usage:
 *   node scripts/validate-passport-json.mjs [--mode layout|full|attested] <passport.json>
 *
 * layout — dateOfBirth + nationality/country_code (Futurenet RarimoQuery demo)
 * full   — sod + dg1 from chip (required for process_passport / Phase 2)
 */
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
let mode = "full";
const paths = [];
for (let i = 0; i < args.length; i++) {
  const a = args[i];
  if (a === "--mode" || a === "-m") {
    mode = args[++i] ?? mode;
    continue;
  }
  if (a.startsWith("--mode=")) {
    mode = a.slice(7);
    continue;
  }
  paths.push(a);
}

const path = paths[0];
if (!path) {
  console.error("Usage: validate-passport-json.mjs [--mode layout|full|attested] <passport.json>");
  process.exit(1);
}

if (!["layout", "full", "attested"].includes(mode)) {
  console.error("mode must be layout, full, or attested");
  process.exit(1);
}

const j = JSON.parse(readFileSync(path, "utf8"));
const fileMode = j._wraithMode;
if (fileMode && fileMode !== mode && mode !== "attested") {
  console.warn(`Note: file _wraithMode=${fileMode}, validating as --mode ${mode}`);
}

function isPlaceholder(v) {
  if (v == null) return true;
  const s = String(v).trim();
  if (!s) return true;
  return /<(base64|hex)|placeholder|example>/i.test(s);
}

function hasChipField(v) {
  return !isPlaceholder(v);
}

if (mode === "attested") {
  const missing = [];
  if (j.age == null) missing.push("age");
  if (j.country_code == null && j.countryCode == null) missing.push("country_code");
  if (j.is_human == null && j.isHuman == null) missing.push("is_human");
  if (missing.length) {
    console.error("Attested claims missing:", missing.join(", "));
    process.exit(1);
  }
  console.log("OK: attested claims");
  console.log("  age:", j.age);
  console.log("  country_code:", j.country_code ?? j.countryCode);
  console.log("  is_human:", j.is_human ?? j.isHuman);
  process.exit(0);
}

if (mode === "layout") {
  const missing = [];
  if (!j.dateOfBirth) missing.push("dateOfBirth");
  if (!j.nationality && j.country_code == null) missing.push("nationality or country_code");
  if (missing.length) {
    console.error("Layout path missing:", missing.join(", "));
    console.error("Template: tools/zk-circuits/fixtures/passport.layout.json");
    process.exit(1);
  }
  if (j.sod != null || j.dg1 != null) {
    if (hasChipField(j.sod) && hasChipField(j.dg1)) {
      console.log("OK: layout path (+ chip fields present; use --mode full for Rarimo register)");
    } else if (hasChipField(j.sod) || hasChipField(j.dg1)) {
      console.warn("Warning: only one of sod/dg1 set — full Rarimo path needs both from scan");
    } else {
      console.log("OK: layout path (claims only — demo RarimoQuery on Futurenet)");
    }
  } else {
    console.log("OK: layout path (claims only — demo RarimoQuery on Futurenet)");
  }
  console.log("  dateOfBirth:", j.dateOfBirth);
  if (j.nationality) console.log("  nationality:", j.nationality);
  if (j.country_code != null) console.log("  country_code:", j.country_code);
  process.exit(0);
}

// full mode
const missing = [];
for (const key of ["sod", "dg1"]) {
  if (!hasChipField(j[key])) missing.push(key);
}
if (missing.length) {
  console.error("Full path missing real chip fields:", missing.join(", "));
  console.error("Scan passport first — see docs/PASSPORT_SCAN.md");
  console.error("Template: tools/zk-circuits/fixtures/passport.template.json");
  process.exit(1);
}

const hints = [];
if (!j.dateOfBirth) hints.push("dateOfBirth (helps layout + query witnesses)");
if (!j.nationality && j.country_code == null) {
  hints.push("nationality or country_code");
}
if (!j.dg15 || isPlaceholder(j.dg15)) {
  hints.push("dg15 (optional but needed for some passport types)");
}
if (hints.length) console.warn("Warnings:", hints.join("; "));

console.log("OK: full passport JSON (sod + dg1 from chip)");
if (j.dateOfBirth) console.log("  dateOfBirth:", j.dateOfBirth);
if (j.nationality) console.log("  nationality:", j.nationality);
if (j.country_code != null) console.log("  country_code:", j.country_code);
