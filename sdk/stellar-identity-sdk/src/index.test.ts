import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  StellarIdentityClient,
  assertVerificationPayload,
  type InvokeContract,
  normalizeHex32,
} from "./index.js";

describe("normalizeHex32", () => {
  it("pads to 32 bytes", () => {
    assert.equal(
      normalizeHex32("1"),
      "0x0000000000000000000000000000000000000000000000000000000000000001",
    );
  });
});

describe("assertVerificationPayload", () => {
  it("rejects empty public signals", () => {
    assert.throws(() => {
      assertVerificationPayload({
        nullifier: normalizeHex32("1"),
        publicInputsHash: normalizeHex32("2"),
        vk: { alpha: "0x1", beta: "0x2", gamma: "0x3", delta: "0x4", ic: ["0x5"] },
        proof: { a: "0x1", b: "0x2", c: "0x3" },
        publicSignals: [],
        claims: { age: 18, countryCode: 840, isHuman: true },
      });
    });
  });
});

describe("StellarIdentityClient", () => {
  it("validates contract method inputs before invoking", async () => {
    const calls: Array<{ method: string; args: unknown[] }> = [];
    const invoke: InvokeContract = async (method, args) => {
      calls.push({ method, args });
      return undefined as never;
    };
    const client = new StellarIdentityClient(invoke);

    await client.init("admin", "prover");
    assert.equal(calls[0]?.method, "init");
  });
});
