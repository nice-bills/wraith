/** ISO 3166-1 numeric codes keyed by ISO 3166-1 alpha-3. Extend as needed. */
export const ISO3_NUM = {
  USA: 840,
  GBR: 826,
  DEU: 276,
  FRA: 250,
  CAN: 124,
  AUS: 36,
  IND: 356,
  BRA: 76,
  MEX: 484,
  NLD: 528,
  ESP: 724,
  ITA: 380,
  POL: 616,
  SWE: 752,
  NOR: 578,
  CHE: 756,
  JPN: 392,
  KOR: 410,
  CHN: 156,
  ZAF: 710,
  NGA: 566,
  KEN: 404,
  GHA: 288,
  EGY: 818,
  ARE: 784,
  SAU: 682,
  ISR: 376,
  TUR: 792,
  RUS: 643,
  UKR: 804,
  ARG: 32,
  COL: 170,
  CHL: 152,
  PER: 604,
  PHL: 608,
  SGP: 702,
  MYS: 458,
  IDN: 360,
  THA: 764,
  VNM: 704,
  PAK: 586,
  BGD: 50,
  IRL: 372,
  PRT: 620,
  BEL: 56,
  AUT: 40,
  CZE: 203,
  HUN: 348,
  ROU: 642,
  GRC: 300,
  NZL: 554,
};

export function countryCodeFromNationality(nationality) {
  if (nationality == null || nationality === "") {
    throw new Error("nationality is empty");
  }
  const nat = String(nationality).trim().toUpperCase();
  if (/^\d+$/.test(nat)) return Number(nat);
  if (ISO3_NUM[nat]) return ISO3_NUM[nat];
  throw new Error(
    `cannot map nationality '${nationality}' — set country_code (ISO numeric) in JSON`
  );
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
