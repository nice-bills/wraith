import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  mapGenericWebhook,
  mapPersonaWebhook,
  mapSumsubWebhook,
} from "./kyc.js";
import { resolveCountryCode } from "./country-code.js";

const WALLET = "GDYLBKGXBZ4QWVKZGGEWEZ2CY4BNVULZ62JNI6JYGOENLNH6I5RC5SA5";

describe("resolveCountryCode", () => {
  it("resolves alpha-2", () => {
    assert.equal(resolveCountryCode("NG"), 566);
  });
});

describe("mapGenericWebhook", () => {
  it("maps flat payload with numeric country", () => {
    const r = mapGenericWebhook({
      wallet: WALLET,
      age: 30,
      country_code: 826,
      is_human: true,
    });
    assert.equal("wallet" in r && r.wallet, WALLET);
    if ("claims" in r) {
      assert.equal(r.claims.age, 30);
      assert.equal(r.claims.country_code, 826);
    }
  });

  it("maps country alpha-2 string", () => {
    const r = mapGenericWebhook({
      wallet: WALLET,
      age: 28,
      country: "NG",
    });
    assert.equal("claims" in r && r.claims.country_code, 566);
  });

  it("maps nationality alpha-3", () => {
    const r = mapGenericWebhook({
      wallet: WALLET,
      age: 22,
      nationality: "KEN",
    });
    assert.equal("claims" in r && r.claims.country_code, 404);
  });

  it("ignores missing wallet", () => {
    const r = mapGenericWebhook({ age: 25, country_code: 840 });
    assert.equal("status" in r && r.status, "ignored");
  });

  it("ignores missing country", () => {
    const r = mapGenericWebhook({ wallet: WALLET, age: 25 });
    assert.equal("status" in r && r.status, "ignored");
  });
});

describe("mapPersonaWebhook", () => {
  it("maps approved inquiry with metadata wallet and country", () => {
    const r = mapPersonaWebhook({
      data: {
        attributes: {
          status: "approved",
          metadata: { stellar_address: WALLET, country: "GB" },
          age: 30,
        },
      },
    });
    assert.ok("wallet" in r);
    if ("wallet" in r) {
      assert.equal(r.wallet, WALLET);
      assert.equal(r.claims.country_code, 826);
      assert.equal(r.vendor, "persona");
    }
  });

  it("ignores non-approved", () => {
    const r = mapPersonaWebhook({
      data: { attributes: { status: "pending", metadata: { wallet: WALLET } } },
    });
    assert.equal("status" in r && r.status, "ignored");
  });

  it("ignores approved without country when no env default", () => {
    const prev = process.env.KYC_DEFAULT_COUNTRY;
    delete process.env.KYC_DEFAULT_COUNTRY;
    const r = mapPersonaWebhook({
      data: {
        attributes: {
          status: "approved",
          metadata: { stellar_address: WALLET },
        },
      },
    });
    if (prev) process.env.KYC_DEFAULT_COUNTRY = prev;
    assert.equal("status" in r && r.status, "ignored");
  });
});

describe("mapSumsubWebhook", () => {
  it("maps GREEN review with country in info", () => {
    const r = mapSumsubWebhook({
      type: "applicantReviewed",
      externalUserId: WALLET,
      reviewResult: { reviewAnswer: "GREEN" },
      info: { age: 29, country: "DE" },
    });
    assert.equal("wallet" in r && r.wallet, WALLET);
    if ("claims" in r) {
      assert.equal(r.claims.country_code, 276);
    }
    assert.equal("vendor" in r && r.vendor, "sumsub");
  });
});
