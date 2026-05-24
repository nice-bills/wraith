import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  mapGenericWebhook,
  mapPersonaWebhook,
  mapSumsubWebhook,
} from "./kyc.js";

const WALLET = "GDYLBKGXBZ4QWVKZGGEWEZ2CY4BNVULZ62JNI6JYGOENLNH6I5RC5SA5";

describe("mapGenericWebhook", () => {
  it("maps flat payload", () => {
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

  it("ignores missing wallet", () => {
    const r = mapGenericWebhook({ age: 25, country_code: 840 });
    assert.equal("status" in r && r.status, "ignored");
  });
});

describe("mapPersonaWebhook", () => {
  it("maps approved inquiry with metadata wallet", () => {
    const r = mapPersonaWebhook({
      data: {
        attributes: {
          status: "approved",
          metadata: { stellar_address: WALLET },
        },
      },
    });
    assert.equal("wallet" in r && r.wallet, WALLET);
    assert.equal("vendor" in r && r.vendor, "persona");
  });

  it("ignores non-approved", () => {
    const r = mapPersonaWebhook({
      data: { attributes: { status: "pending", metadata: { wallet: WALLET } } },
    });
    assert.equal("status" in r && r.status, "ignored");
  });
});

describe("mapSumsubWebhook", () => {
  it("maps GREEN review", () => {
    const r = mapSumsubWebhook({
      type: "applicantReviewed",
      externalUserId: WALLET,
      reviewResult: { reviewAnswer: "GREEN" },
    });
    assert.equal("wallet" in r && r.wallet, WALLET);
    assert.equal("vendor" in r && r.vendor, "sumsub");
  });
});
