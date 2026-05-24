/**
 * ISO 3166-1 country resolution (numeric, alpha-2, alpha-3).
 * Data: scripts/lib/iso3166.json (249 territories).
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const { byNumeric, byAlpha2, byAlpha3 } = JSON.parse(
  readFileSync(join(__dirname, "iso3166.json"), "utf8")
);

/** @deprecated use resolveCountryCode — kept for imports */
export const ISO3_NUM = Object.fromEntries(
  Object.entries(byAlpha3).map(([a3, n]) => [a3, n])
);

/**
 * Resolve ISO 3166-1 numeric code from number or alpha-2/alpha-3 string.
 * @param {string|number|null|undefined} input
 * @returns {number}
 */
export function resolveCountryCode(input) {
  if (input == null || input === "") {
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
  const raw = String(input).trim();
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
    `unknown country '${input}' — use ISO 3166-1 numeric, alpha-2 (e.g. NG), or alpha-3 (e.g. NGA)`
  );
}

export function countryCodeFromNationality(nationality) {
  return resolveCountryCode(nationality);
}

/**
 * Normalize attested claims object (age, country_code, is_human).
 * Accepts country / countryCode / nationality as alpha or numeric.
 * @param {Record<string, unknown>} raw
 * @returns {{ age: number, country_code: number, is_human: boolean }}
 */
export function normalizeAttestedClaims(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("claims object required");
  }
  const ageVal = raw.age;
  if (ageVal == null || ageVal === "") {
    throw new Error("claims.age is required");
  }
  const age = Number(ageVal);
  if (!Number.isInteger(age) || age < 0 || age > 150) {
    throw new Error(`invalid claims.age: ${ageVal}`);
  }

  const countrySrc =
    raw.country_code ??
    raw.countryCode ??
    raw.country ??
    raw.nationality ??
    raw.issuerCountry;
  const country_code = resolveCountryCode(
    typeof countrySrc === "string" || typeof countrySrc === "number"
      ? countrySrc
      : null
  );

  const humanSrc = raw.is_human ?? raw.isHuman;
  const is_human =
    humanSrc === undefined || humanSrc === null ? true : Boolean(humanSrc);

  return { age, country_code, is_human };
}

export function parseBirthYymmdd(raw) {
  const digits = String(raw ?? "").replace(/\D/g, "");
  if (digits.length === 6) return digits;
  if (digits.length === 8) return digits.slice(2);
  throw new Error(`cannot parse dateOfBirth '${raw}' — use YYMMDD or YYYYMMDD`);
}

export function ageFromBirthYymmdd(birthYymmdd, currentYymmdd) {
  const birthYy = Math.floor(Number(birthYymmdd) / 10_000);
  const birthMm = Math.floor((Number(birthYymmdd) / 100) % 100);
  const birthDd = Number(birthYymmdd) % 100;
  const curYy = Math.floor(Number(currentYymmdd) / 10_000);
  const curMm = Math.floor((Number(currentYymmdd) / 100) % 100);
  const curDd = Number(currentYymmdd) % 100;

  const birthFullYear = birthYy <= 50 ? 2000 + birthYy : 1900 + birthYy;
  const currentFullYear = curYy <= 50 ? 2000 + curYy : 1900 + curYy;

  let age = currentFullYear - birthFullYear;
  if (curMm < birthMm || (curMm === birthMm && curDd < birthDd)) {
    age -= 1;
  }
  return age;
}

export function currentYymmddUtc() {
  const d = new Date();
  const yy = String(d.getUTCFullYear() % 100).padStart(2, "0");
  const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
  const dd = String(d.getUTCDate()).padStart(2, "0");
  return `${yy}${mm}${dd}`;
}
