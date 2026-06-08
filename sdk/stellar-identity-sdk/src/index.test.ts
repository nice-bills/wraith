import {
  normalizeHex32,
  assertVerificationPayload,
  decimalToBn254FrHex,
  computePublicInputsHash,
  computeAttestationHash,
  computeAttestedClaimsHash,
  buildAttestedPayload,
  fromAdapterPayload,
  type Hex,
  type AdapterOutput,
} from "./index.js";

import { describe, it } from "node:test";
import assert from "node:assert";

const paddedHex = (zeros: number, suffix: string): string =>
  `0x${"0".repeat(zeros)}${suffix}`;

const fullHex = (chars: string, len: number): Hex =>
  `0x${chars.repeat(len).padStart(64, "0")}` as Hex;

const sampleVk = {
  alpha: fullHex("c", 32),
  beta: fullHex("d", 256),
  gamma: fullHex("e", 256),
  delta: fullHex("f", 256),
  ic: [fullHex("1", 32), fullHex("2", 32)],
};

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
});

describe("decimalToBn254FrHex", () => {
  it("encodes u32 as BE in the low limb (matches Soroban Fr::to_bytes)", () => {
    const encoded = decimalToBn254FrHex("25");
    assert.strictEqual(
      encoded,
      "0x0000000000000000000000000000000000000000000000000000000000000019",
    );
  });
});

describe("computePublicInputsHash", () => {
  it("hashes field encodings", () => {
    const signals = [
      decimalToBn254FrHex("25"),
      decimalToBn254FrHex("840"),
      decimalToBn254FrHex("1"),
    ];
    const hash = computePublicInputsHash(signals);
    assert.match(hash, /^0x[0-9a-f]{64}$/);
  });

  it("appends currentDateYmd for Rarimo binding", () => {
    const signals = [
      decimalToBn254FrHex("25"),
      decimalToBn254FrHex("840"),
      decimalToBn254FrHex("1"),
    ];
    const h0 = computePublicInputsHash(signals, 0);
    const h1 = computePublicInputsHash(signals, 260515);
    assert.notEqual(h0, h1);
  });
});

describe("computeAttestedClaimsHash", () => {
  it("matches contract LE u32 packing (golden vectors)", () => {
    assert.strictEqual(
      computeAttestedClaimsHash({ age: 25, countryCode: 840, isHuman: true }),
      "0x4696fd2021cf7b5093dcd463c913b9c54fea0dc4ba123dac9b2678da21ea1d35",
    );
    assert.strictEqual(
      computeAttestedClaimsHash({ age: 28, countryCode: 840, isHuman: true }),
      "0xb4dd459e53a3bfea927d4f5b2d51552f636bea49c582bca880514a3949663fbd",
    );
  });
});

describe("buildAttestedPayload", () => {
  it("derives matching attestation inputs", () => {
    const claims = { age: 25, countryCode: 840, isHuman: true };
    const expectedPublicInputsHash = computeAttestedClaimsHash(claims);
    const expectedAttestationHash = computeAttestationHash({
      proverXdr: Buffer.from("prover-xdr"),
      appIdXdr: Buffer.from("app-xdr"),
      subjectXdr: Buffer.from("subject-xdr"),
      nullifier: fullHex("a", 32),
      publicInputsHash: expectedPublicInputsHash,
    });

    const payload = buildAttestedPayload({
      prover: "prover",
      nullifier: fullHex("a", 32),
      claims,
      proverXdr: Buffer.from("prover-xdr"),
      appIdXdr: Buffer.from("app-xdr"),
      subjectXdr: Buffer.from("subject-xdr"),
    });

    assert.deepStrictEqual(payload, {
      prover: "prover",
      nullifier: fullHex("a", 32),
      publicInputsHash: expectedPublicInputsHash,
      attestationHash: expectedAttestationHash,
      claims,
    });
  });
});

describe("fromAdapterPayload", () => {
  it("converts adapter output with decimal signals", () => {
    const adapter: AdapterOutput = {
      proof: {
        a: fullHex("2", 32),
        b: fullHex("3", 256),
        c: fullHex("4", 32),
      },
      verification_key: {
        ...sampleVk,
        ic: [
          fullHex("1", 32),
          fullHex("2", 32),
          fullHex("3", 32),
          fullHex("4", 32),
        ],
      },
      public_signals_decimals: ["25", "840", "1"],
      public_signals_hex: [
        decimalToBn254FrHex("25"),
        decimalToBn254FrHex("840"),
        decimalToBn254FrHex("1"),
      ],
      claims: { age: 25, country_code: 840, is_human: true },
    };
    const payload = fromAdapterPayload(adapter, fullHex("a", 32));
    assert.strictEqual(payload.claims.age, 25);
    assert.strictEqual(payload.publicSignals.length, 3);
    assert.strictEqual(payload.vk.ic.length, 4);
    assert.doesNotThrow(() => assertVerificationPayload(payload));
  });

  it("preserves currentDateYmd for Rarimo adapter output", () => {
    const publicSignals = Array.from({ length: 23 }, (_, index) =>
      decimalToBn254FrHex(String(index + 1)),
    );
    const adapter: AdapterOutput = {
      proof: {
        a: fullHex("2", 32),
        b: fullHex("3", 256),
        c: fullHex("4", 32),
      },
      verification_key: {
        ...sampleVk,
        ic: Array.from({ length: publicSignals.length + 1 }, (_, index) =>
          fullHex(String((index % 10) + 1), 32),
        ),
      },
      public_signals_decimals: publicSignals.map((_, index) => String(index + 1)),
      public_signals_hex: publicSignals,
      current_date_ymd: 260515,
      claims: { age: 25, country_code: 840, is_human: true },
    };

    const payload = fromAdapterPayload(adapter, fullHex("a", 32));

    assert.strictEqual(payload.currentDateYmd, 260515);
    assert.strictEqual(
      payload.publicInputsHash,
      computePublicInputsHash(publicSignals, 260515),
    );
  });
});

describe("assertVerificationPayload", () => {
  it("accepts valid payload", () => {
    assert.doesNotThrow(() => {
      assertVerificationPayload({
        nullifier: fullHex("a", 32),
        publicInputsHash: fullHex("b", 32),
        vk: sampleVk,
        proof: {
          a: fullHex("2", 32),
          b: fullHex("3", 256),
          c: fullHex("4", 32),
        },
        publicSignals: [decimalToBn254FrHex("25")],
        claims: {
          age: 25,
          countryCode: 840,
          isHuman: true,
        },
      });
    });
  });

  it("rejects ic length mismatch", () => {
    assert.throws(() => {
      assertVerificationPayload({
        nullifier: fullHex("a", 32),
        publicInputsHash: fullHex("b", 32),
        vk: {
          ...sampleVk,
          ic: [fullHex("1", 32)],
        },
        proof: {
          a: fullHex("2", 32),
          b: fullHex("3", 256),
          c: fullHex("4", 32),
        },
        publicSignals: [decimalToBn254FrHex("25"), decimalToBn254FrHex("840")],
        claims: {
          age: 25,
          countryCode: 840,
          isHuman: true,
        },
      });
    }, /vk.ic length/);
  });
});
