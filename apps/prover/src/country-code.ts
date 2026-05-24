import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

type IsoEntry = { numeric: number; alpha2: string; alpha3: string; name: string };
type IsoDb = {
  byNumeric: Record<string, IsoEntry>;
  byAlpha2: Record<string, number>;
  byAlpha3: Record<string, number>;
};

const isoPath = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../../scripts/lib/iso3166.json"
);
const { byNumeric, byAlpha2, byAlpha3 } = JSON.parse(
  readFileSync(isoPath, "utf8")
) as IsoDb;

export function resolveCountryCode(input: string | number): number {
  if (input == null || (typeof input === "string" && input.trim() === "")) {
    throw new Error("country is required (ISO numeric, alpha-2, or alpha-3)");
  }
  if (typeof input === "number") {
    if (!Number.isInteger(input) || input <= 0) {
      throw new Error(`invalid country numeric code: ${input}`);
    }
    if (!byNumeric[String(input)]) {
      throw new Error(`unknown ISO 3166-1 numeric code: ${input}`);
    }
    return input;
  }
  const raw = input.trim();
  if (/^\d+$/.test(raw)) {
    const n = Number(raw);
    if (!byNumeric[String(n)]) {
      throw new Error(`unknown ISO 3166-1 numeric code: ${raw}`);
    }
    return n;
  }
  const code = raw.toUpperCase();
  if (code.length === 2 && byAlpha2[code] != null) return byAlpha2[code];
  if (code.length === 3 && byAlpha3[code] != null) return byAlpha3[code];
  throw new Error(
    `unknown country '${input}' — use ISO 3166-1 numeric, alpha-2, or alpha-3`
  );
}

function fieldValue(obj: Record<string, unknown>, key: string): unknown {
  const fields = obj.fields;
  if (!fields || typeof fields !== "object" || Array.isArray(fields)) return null;
  const f = (fields as Record<string, unknown>)[key];
  if (f && typeof f === "object" && f !== null && "value" in f) {
    return (f as { value?: unknown }).value;
  }
  return null;
}

export function pickCountryCode(obj: Record<string, unknown>): number | null {
  const direct =
    obj.country_code ??
    obj.countryCode ??
    obj.country ??
    obj.nationality ??
    fieldValue(obj, "country") ??
    fieldValue(obj, "nationality") ??
    fieldValue(obj, "country-code");
  if (direct == null) return null;
  if (typeof direct === "number") {
    try {
      return resolveCountryCode(direct);
    } catch {
      return null;
    }
  }
  if (typeof direct === "string" && direct.trim() !== "") {
    try {
      return resolveCountryCode(direct);
    } catch {
      return null;
    }
  }
  return null;
}

export function normalizeAttestedClaims(raw: Record<string, unknown>): {
  age: number;
  country_code: number;
  is_human: boolean;
} {
  const ageVal = raw.age;
  if (ageVal == null) throw new Error("claims.age is required");
  const age = typeof ageVal === "number" ? ageVal : Number(ageVal);
  if (!Number.isInteger(age) || age < 0 || age > 150) {
    throw new Error(`invalid claims.age: ${String(ageVal)}`);
  }
  const country = pickCountryCode(raw);
  if (country == null) {
    throw new Error(
      "claims need country_code, countryCode, country, or nationality (ISO numeric or alpha)"
    );
  }
  const humanSrc = raw.is_human ?? raw.isHuman;
  const is_human =
    humanSrc === undefined || humanSrc === null ? true : Boolean(humanSrc);
  return { age, country_code: country, is_human };
}
