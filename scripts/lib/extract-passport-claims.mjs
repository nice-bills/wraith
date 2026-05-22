#!/usr/bin/env node
/**
 * Extract YYMMDD birth date + numeric country_code from passport JSON.
 * Prints JSON: { birthYymmdd, countryCode }
 */
import { readFileSync } from "node:fs";
import {
  countryCodeFromNationality,
  parseBirthYymmdd,
} from "./country-codes.mjs";

const path = process.argv[2];
const j = JSON.parse(readFileSync(path, "utf8"));

const birthYymmdd = parseBirthYymmdd(j.dateOfBirth ?? j.date_of_birth);
const countryCode =
  j.country_code != null && j.country_code !== ""
    ? Number(j.country_code)
    : countryCodeFromNationality(j.nationality ?? j.issuerCountry);

process.stdout.write(JSON.stringify({ birthYymmdd, countryCode }));
