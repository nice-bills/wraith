#!/usr/bin/env node
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  resolveCountryCode,
  normalizeAttestedClaims,
  countryCodeFromNationality,
} from "../lib/country-codes.mjs";

describe("resolveCountryCode", () => {
  it("accepts ISO numeric", () => {
    assert.equal(resolveCountryCode(566), 566);
    assert.equal(resolveCountryCode("840"), 840);
  });
  it("accepts alpha-2 and alpha-3", () => {
    assert.equal(resolveCountryCode("NG"), 566);
    assert.equal(resolveCountryCode("nga"), 566);
    assert.equal(resolveCountryCode("GB"), 826);
    assert.equal(resolveCountryCode("USA"), 840);
  });
  it("rejects unknown codes", () => {
    assert.throws(() => resolveCountryCode("XX"), /unknown country/);
    assert.throws(() => resolveCountryCode(99999), /unknown ISO/);
  });
});

describe("normalizeAttestedClaims", () => {
  it("maps country field aliases", () => {
    const c = normalizeAttestedClaims({
      age: 30,
      country: "DE",
      is_human: true,
    });
    assert.equal(c.country_code, 276);
  });
  it("maps country_code string alpha", () => {
    const c = normalizeAttestedClaims({
      age: 25,
      country_code: "NGA",
      is_human: false,
    });
    assert.equal(c.country_code, 566);
    assert.equal(c.is_human, false);
  });
});

describe("countryCodeFromNationality", () => {
  it("still resolves MRZ-style alpha-3", () => {
    assert.equal(countryCodeFromNationality("USA"), 840);
  });
});
