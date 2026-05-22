import {
  ageFromBirthYymmdd,
  countryCodeFromNationality,
  currentYymmddUtc,
} from "./country-codes.mjs";

function cleanMrz(input) {
  return input
    .replace(/\r/g, "")
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
}

function parseTd3(lines) {
  const l1 = lines[0].padEnd(44, "<");
  const l2 = lines[1].padEnd(44, "<");
  const nationality = l2.slice(10, 13).replace(/</g, "");
  const birth = l2.slice(13, 19);
  const sex = l2.slice(20, 21);
  const expiry = l2.slice(21, 27);
  return {
    documentType: l1.slice(0, 2).replace(/</g, ""),
    issuingCountry: l1.slice(2, 5).replace(/</g, ""),
    name: l1.slice(5).replace(/</g, " ").trim(),
    documentNumber: l2.slice(0, 9).replace(/</g, ""),
    nationality,
    dateOfBirth: birth,
    documentExpiryDate: expiry,
    gender: sex,
  };
}

function parseTd1(lines) {
  const l1 = lines[0].padEnd(30, "<");
  const l2 = lines[1].padEnd(30, "<");
  const l3 = lines[2].padEnd(30, "<");
  const birth = l2.slice(0, 6);
  const sex = l2.slice(7, 8);
  const expiry = l2.slice(8, 14);
  const nationality = l2.slice(15, 18).replace(/</g, "");
  return {
    documentType: l1.slice(0, 2).replace(/</g, ""),
    issuingCountry: l1.slice(2, 5).replace(/</g, ""),
    documentNumber: l1.slice(5, 14).replace(/</g, ""),
    dateOfBirth: birth,
    gender: sex,
    documentExpiryDate: expiry,
    nationality,
    name: l3.replace(/</g, " ").trim(),
  };
}

export function parseMrz(input) {
  const lines = cleanMrz(input);
  if (lines.length < 2) {
    throw new Error("MRZ needs at least 2 lines");
  }
  if (lines[0].length >= 40) return parseTd3(lines);
  if (lines.length >= 3) return parseTd1(lines);
  throw new Error("unsupported MRZ line length");
}

export function mrzToAttestedClaims(mrzInput, { isHuman = true } = {}) {
  const fields = parseMrz(mrzInput);
  const countryCode = countryCodeFromNationality(fields.nationality);
  const age = ageFromBirthYymmdd(fields.dateOfBirth, currentYymmddUtc());
  return {
    age,
    country_code: countryCode,
    is_human: isHuman,
    _source: { type: "mrz", fields },
  };
}
