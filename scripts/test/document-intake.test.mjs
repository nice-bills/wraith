#!/usr/bin/env node
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { analyzeDocument } from "../lib/document-path.mjs";
import { parseMrz, mrzToAttestedClaims } from "../lib/parse-mrz.mjs";
import { ageFromBirthYymmdd } from "../lib/country-codes.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "../..");

describe("document-path", () => {
  it("detects full path from nested dump template", () => {
    const raw = JSON.parse(
      readFileSync(
        join(ROOT, "tools/zk-circuits/fixtures/nfc-dump.nested.template.json"),
        "utf8"
      )
    );
    const r = analyzeDocument(raw);
    assert.equal(r.path, "full");
  });

  it("detects layout from passport.layout.json", () => {
    const raw = JSON.parse(
      readFileSync(join(ROOT, "tools/zk-circuits/fixtures/passport.layout.json"), "utf8")
    );
    assert.equal(analyzeDocument(raw).path, "layout");
  });

  it("detects attested claims", () => {
    const r = analyzeDocument({ age: 25, country_code: 840, is_human: true });
    assert.equal(r.path, "attested");
  });
});

describe("parse-mrz", () => {
  it("parses TD3 sample and derives US country", () => {
    const mrz = [
      "P<USASMITH<<JOHN<<<<<<<<<<<<<<<<<<<<<<<<<<<",
      "1234567897USA9501011M3001015<<<<<<<<<<<<<<04",
    ].join("\n");
    const fields = parseMrz(mrz);
    assert.equal(fields.nationality, "USA");
    assert.equal(fields.dateOfBirth, "950101");
    const claims = mrzToAttestedClaims(mrz);
    assert.equal(claims.country_code, 840);
    assert.ok(claims.age >= 18);
  });
});

describe("ageFromBirthYymmdd", () => {
  it("matches passport layout fixture age band", () => {
    const age = ageFromBirthYymmdd("950101", "250101");
    assert.ok(age >= 29 && age <= 31);
  });
});

describe("claim-layout-spec golden vectors", () => {
  it("matches deployments/claim-layout-spec.json", async () => {
    const { loadClaimLayoutSpec, deriveClaimsFromSpec } = await import(
      "../lib/claim-layout-spec.mjs"
    );
    const spec = loadClaimLayoutSpec();
    for (const v of spec.goldenVectors) {
      if (v.name === "age_from_birth_yymmdd") {
        const age = ageFromBirthYymmdd(
          String(v.birthYymmdd),
          String(v.currentYymmdd)
        );
        assert.equal(age, v.expectedAge, v.name);
        continue;
      }
      if (!v.layout || !v.signals) continue;
      const claims = deriveClaimsFromSpec(
        v.layout,
        v.signals,
        String(v.currentYymmdd)
      );
      assert.equal(claims.age, v.expected.age, `${v.name} age`);
      assert.equal(
        claims.country_code,
        v.expected.countryCode,
        `${v.name} country`
      );
      assert.equal(claims.is_human, v.expected.isHuman, `${v.name} human`);
    }
  });
});
