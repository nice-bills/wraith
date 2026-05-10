import { normalizeHex32, assertVerificationPayload, type Hex } from "./index.js";

import { describe, it } from "node:test";
import assert from "node:assert";

const paddedHex = (zeros: number, suffix: string): string =>
  `0x${"0".repeat(zeros)}${suffix}`;

const fullHex = (chars: string, len: number): Hex =>
  `0x${chars.repeat(len).padStart(64, "0")}` as Hex;

describe("normalizeHex32", () => {
  it("pads short hex to 32 bytes", () => {
    const result = normalizeHex32("0x1");
    assert.strictEqual(result, "0x0000000000000000000000000000000000000000000000000000000000000001");
  });

  it("lowercases output", () => {
    const result = normalizeHex32("0xABCDEF");
    assert.strictEqual(result, paddedHex(58, "abcdef"));
  });

  it("throws on invalid hex", () => {
    assert.throws(() => normalizeHex32("xyz"), /not valid hex/);
  });

  it("throws on >64 char hex", () => {
    assert.throws(() => normalizeHex32("0x" + "a".repeat(65)), /exceeds 32 bytes/);
  });

  it("handles 0x prefix stripped input", () => {
    const result = normalizeHex32("abc");
    assert.strictEqual(result, paddedHex(61, "abc"));
  });

  it("accepts full 32-byte hex", () => {
    const result = normalizeHex32(fullHex("f", 32));
    assert.strictEqual(result, fullHex("f", 32));
  });
});

describe("assertVerificationPayload", () => {
  it("accepts valid payload", () => {
    assert.doesNotThrow(() => {
      assertVerificationPayload({
        nullifier: fullHex("a", 32),
        publicInputsHash: fullHex("b", 32),
        vk: {
          alpha: fullHex("c", 32),
          beta: fullHex("d", 128),
          gamma: fullHex("e", 128),
          delta: fullHex("f", 128),
          ic: [fullHex("1", 32)],
        },
        proof: {
          a: fullHex("2", 32),
          b: fullHex("3", 128),
          c: fullHex("4", 32),
        },
        publicSignals: [fullHex("5", 32)],
        claims: {
          age: 25,
          countryCode: 840,
          isHuman: true,
        },
      });
    });
  });

  it("rejects negative age", () => {
    assert.throws(() => {
      assertVerificationPayload({
        nullifier: fullHex("a", 32),
        publicInputsHash: fullHex("b", 32),
        vk: {
          alpha: fullHex("c", 32),
          beta: fullHex("d", 128),
          gamma: fullHex("e", 128),
          delta: fullHex("f", 128),
          ic: [fullHex("1", 32)],
        },
        proof: {
          a: fullHex("2", 32),
          b: fullHex("3", 128),
          c: fullHex("4", 32),
        },
        publicSignals: [fullHex("5", 32)],
        claims: {
          age: -1,
          countryCode: 840,
          isHuman: true,
        },
      });
    }, /age cannot be negative/);
  });

  it("rejects empty publicSignals", () => {
    assert.throws(() => {
      assertVerificationPayload({
        nullifier: fullHex("a", 32),
        publicInputsHash: fullHex("b", 32),
        vk: {
          alpha: fullHex("c", 32),
          beta: fullHex("d", 128),
          gamma: fullHex("e", 128),
          delta: fullHex("f", 128),
          ic: [fullHex("1", 32)],
        },
        proof: {
          a: fullHex("2", 32),
          b: fullHex("3", 128),
          c: fullHex("4", 32),
        },
        publicSignals: [],
        claims: {
          age: 25,
          countryCode: 840,
          isHuman: true,
        },
      });
    }, /publicSignals cannot be empty/);
  });
});