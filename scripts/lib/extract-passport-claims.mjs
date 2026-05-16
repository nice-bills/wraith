#!/usr/bin/env node
/**
 * Extract YYMMDD birth date + numeric country_code from passport JSON.
 * Prints JSON: { birthYymmdd, countryCode }
 */
import { readFileSync } from "node:fs";

const ISO3_NUM = {
  USA: 840,
  GBR: 826,
  DEU: 276,
  FRA: 250,
  CAN: 124,
  AUS: 36,
  IND: 356,
  BRA: 76,
  MEX: 484,
};

const path = process.argv[2];
const j = JSON.parse(readFileSync(path, "utf8"));

function parseBirthYymmdd(raw) {
  const digits = String(raw ?? "").replace(/\D/g, "");
  if (digits.length === 6) return digits;
  if (digits.length === 8) return digits.slice(2); // YYYYMMDD → YYMMDD
  throw new Error(`cannot parse dateOfBirth '${raw}' → use YYMMDD or YYYYMMDD`);
}

function parseCountry(j) {
  if (j.country_code != null && j.country_code !== "") {
    return Number(j.country_code);
  }
  const nat = String(j.nationality ?? "").trim().toUpperCase();
  if (/^\d+$/.test(nat)) return Number(nat);
  if (ISO3_NUM[nat]) return ISO3_NUM[nat];
  throw new Error(
    `cannot map nationality '${j.nationality}' — set country_code (e.g. 840) in passport JSON`
  );
}

const birthYymmdd = parseBirthYymmdd(j.dateOfBirth ?? j.date_of_birth);
const countryCode = parseCountry(j);

process.stdout.write(JSON.stringify({ birthYymmdd, countryCode }));
