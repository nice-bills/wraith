/**
 * Detect recommended Wraith path from a JSON document file.
 * Returns: full | layout | attested | kyc-intake | unknown
 */
import { readFileSync } from "node:fs";

function isPlaceholder(v) {
  if (v == null) return true;
  const s = String(v).trim();
  if (!s) return true;
  return /<(base64|hex)|placeholder|example>/i.test(s);
}

function hasChipField(v) {
  return !isPlaceholder(v);
}

function pick(obj, ...keys) {
  for (const k of keys) {
    if (obj[k] != null && String(obj[k]).trim() !== "") return obj[k];
  }
  return undefined;
}

function flattenDocument(raw, depth = 0) {
  if (depth > 4 || raw == null || typeof raw !== "object") return raw;
  const merged = { ...raw };

  const nestedKeys = ["data", "passport", "lds", "document", "eMRTD", "result"];
  for (const key of nestedKeys) {
    if (raw[key] && typeof raw[key] === "object" && !Array.isArray(raw[key])) {
      Object.assign(merged, raw[key]);
    }
  }

  if (raw.files && typeof raw.files === "object") {
    Object.assign(merged, raw.files);
  }

  for (const [k, v] of Object.entries(raw)) {
    if (/^dg?\d+$/i.test(k) && typeof v === "object" && v?.value) {
      merged[k] = v.value;
    }
    if (/^SOD$/i.test(k) && typeof v === "object" && v?.value) {
      merged.sod = v.value;
    }
  }

  return merged;
}

export function analyzeDocument(raw) {
  const j = flattenDocument(raw);
  const sod = pick(j, "sod", "SOD", "securityObject", "SecurityObject");
  const dg1 = pick(j, "dg1", "DG1", "dg1Base64", "mrz");
  const hasChip = hasChipField(sod) && hasChipField(dg1);

  const hasLayout =
    pick(j, "dateOfBirth", "date_of_birth", "birthDate", "dob") &&
    (pick(j, "nationality", "issuerCountry") || j.country_code != null || j.countryCode != null);

  const hasAttested =
    j.age != null &&
    (j.country_code != null || j.countryCode != null) &&
    j.is_human != null;

  const hasKycManifest =
    j.intakeType === "kyc-photo" ||
    (Array.isArray(j.photos) && j.photos.length > 0) ||
    (j.idFront && j.selfie);

  if (hasChip) {
    return {
      path: "full",
      reason: "NFC chip fields sod + dg1 present",
      fields: Object.keys(j),
    };
  }
  if (hasAttested) {
    return {
      path: "attested",
      reason: "attested claims (age, country_code, is_human)",
      fields: Object.keys(j),
    };
  }
  if (hasKycManifest) {
    return {
      path: "kyc-intake",
      reason: "photo intake manifest — send to KYC vendor, then attested path",
      fields: Object.keys(j),
    };
  }
  if (hasLayout) {
    return {
      path: "layout",
      reason: "claims only (dateOfBirth + nationality) — no chip crypto",
      fields: Object.keys(j),
    };
  }
  if (hasChipField(sod) || hasChipField(dg1)) {
    return {
      path: "unknown",
      reason: "partial chip dump — re-scan or run normalize-passport-json.mjs",
      fields: Object.keys(j),
    };
  }
  return {
    path: "unknown",
    reason: "unrecognized format — see docs/DOCUMENT_INTAKE.md",
    fields: Object.keys(j),
  };
}

export function analyzeDocumentFile(path) {
  const raw = JSON.parse(readFileSync(path, "utf8"));
  return analyzeDocument(raw);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const path = process.argv[2];
  if (!path) {
    console.error("Usage: detect-document-path.mjs <document.json>");
    process.exit(1);
  }
  const result = analyzeDocumentFile(path);
  console.log(JSON.stringify(result, null, 2));
}
