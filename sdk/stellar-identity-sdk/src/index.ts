import { createHash } from "node:crypto";

export type Hex = `0x${string}`;

export type VerificationSource =
  | { kind: "OnchainGroth16" }
  | { kind: "AttestedProver"; attestationHash: Hex };

export type ClaimLayout = "Standard" | "RarimoQuery";

export interface AppPolicy {
  owner: string;
  minAge: number;
  requireHumanity: boolean;
  sanctionsRoot: Hex;
  excludedCountries: number[];
  expirationWindow: number;
  sanctionsEnabled: boolean;
  claimLayout: ClaimLayout;
}

export interface AttestedClaims {
  age: number;
  countryCode: number;
  isHuman: boolean;
}

export interface VerificationRecord {
  appId: string;
  subject: string;
  nullifier: Hex;
  publicInputsHash: Hex;
  verifiedLedger: number;
  source: VerificationSource;
  age: number;
  countryCode: number;
  isHuman: boolean;
}

export interface ProofInput {
  a: string;
  b: string;
  c: string;
}

export interface VerificationKeyInput {
  alpha: string;
  beta: string;
  gamma: string;
  delta: string;
  ic: string[];
}

export interface VerificationPayload {
  nullifier: Hex;
  publicInputsHash: Hex;
  vk: VerificationKeyInput;
  proof: ProofInput;
  /** Hex-encoded Bn254Fr scalars (Soroban field encoding). */
  publicSignals: string[];
  /** YYMMDD for RarimoQuery claim layout; use 0 for Standard. */
  currentDateYmd?: number;
  claims: AttestedClaims;
}

export interface AttestedPayload {
  prover: string;
  nullifier: Hex;
  publicInputsHash: Hex;
  attestationHash: Hex;
  claims: AttestedClaims;
}

/** Output shape from `proof-adapter`. */
export interface AdapterOutput {
  proof: ProofInput;
  verification_key: VerificationKeyInput;
  public_signals_decimals: string[];
  public_signals_hex?: string[];
  claims?: {
    age: number;
    country_code: number;
    is_human: boolean;
  };
}

export type InvokeContract = <T = unknown>(
  method: string,
  args: unknown[],
) => Promise<T>;

export class StellarIdentityClient {
  constructor(private readonly invokeContract: InvokeContract) {}

  async init(admin: string, prover: string): Promise<void> {
    assertNonEmpty("admin", admin);
    assertNonEmpty("prover", prover);
    await this.invokeContract("init", [admin, prover]);
  }

  async getAdmin(): Promise<string> {
    return this.invokeContract<string>("get_admin", []);
  }

  async getProver(): Promise<string> {
    return this.invokeContract<string>("get_prover", []);
  }

  async setProver(newProver: string): Promise<void> {
    assertNonEmpty("newProver", newProver);
    await this.invokeContract("set_prover", [newProver]);
  }

  async registerApp(
    appId: string,
    policy: AppPolicy,
    vkHash?: Hex,
  ): Promise<AppPolicy> {
    assertNonEmpty("appId", appId);
    assertClaimsPolicy(policy);
    return this.invokeContract<AppPolicy>("register_app", [
      appId,
      policy,
      vkHash ?? null,
    ]);
  }

  async updateAppPolicy(
    appId: string,
    policy: AppPolicy,
    vkHash?: Hex,
  ): Promise<AppPolicy> {
    assertNonEmpty("appId", appId);
    assertClaimsPolicy(policy);
    return this.invokeContract<AppPolicy>("update_app_policy", [
      appId,
      policy,
      vkHash ?? null,
    ]);
  }

  async revokeApp(appId: string): Promise<AppPolicy> {
    assertNonEmpty("appId", appId);
    return this.invokeContract<AppPolicy>("revoke_app", [appId]);
  }

  async getPolicy(appId: string): Promise<AppPolicy | null> {
    return this.invokeContract<AppPolicy | null>("get_policy", [appId]);
  }

  async isAppRegistered(appId: string): Promise<boolean> {
    return this.invokeContract<boolean>("is_app_registered", [appId]);
  }

  async getVkHash(appId: string): Promise<Hex | null> {
    return this.invokeContract<Hex | null>("get_vk_hash", [appId]);
  }

  async verifyAndRecord(
    appId: string,
    subject: string,
    payload: VerificationPayload,
  ): Promise<VerificationRecord> {
    assertNonEmpty("appId", appId);
    assertNonEmpty("subject", subject);
    assertVerificationPayload(payload);
    return this.invokeContract<VerificationRecord>("verify_and_record", [
      appId,
      subject,
      payload.nullifier,
      payload.publicInputsHash,
      payload.vk,
      payload.proof,
      payload.publicSignals,
      payload.currentDateYmd ?? 0,
      payload.claims,
    ]);
  }

  async recordAttestedResult(
    appId: string,
    subject: string,
    payload: AttestedPayload,
  ): Promise<VerificationRecord> {
    assertNonEmpty("appId", appId);
    assertNonEmpty("subject", subject);
    assertNonEmpty("prover", payload.prover);
    assertHex32("nullifier", payload.nullifier);
    assertHex32("publicInputsHash", payload.publicInputsHash);
    assertHex32("attestationHash", payload.attestationHash);
    assertClaims(payload.claims);
    return this.invokeContract<VerificationRecord>("record_attested_result", [
      payload.prover,
      appId,
      subject,
      payload.nullifier,
      payload.publicInputsHash,
      payload.attestationHash,
      payload.claims,
    ]);
  }

  async isVerified(appId: string, subject: string): Promise<boolean> {
    return this.invokeContract<boolean>("is_verified", [appId, subject]);
  }

  async getRecord(
    appId: string,
    subject: string,
  ): Promise<VerificationRecord | null> {
    return this.invokeContract<VerificationRecord | null>("get_record", [
      appId,
      subject,
    ]);
  }

  async hasNullifier(appId: string, nullifier: Hex): Promise<boolean> {
    return this.invokeContract<boolean>("has_nullifier", [appId, nullifier]);
  }

  async setAppApproval(appId: string, approved: boolean): Promise<void> {
    assertNonEmpty("appId", appId);
    await this.invokeContract("set_app_approval", [appId, approved]);
  }

  async isApprovedApp(appId: string): Promise<boolean> {
    return this.invokeContract<boolean>("is_approved_app", [appId]);
  }

  async setApprovalMode(required: boolean): Promise<void> {
    await this.invokeContract("set_approval_mode", [required]);
  }
}

/** Encode a u32 public signal as a Bn254Fr hex string (matches Soroban `Fr::to_bytes()` BE layout). */
export function decimalToBn254FrHex(decimal: string): Hex {
  const n = BigInt(decimal);
  if (n < 0n || n > 0xffff_ffffn) {
    throw new Error(`public signal must fit u32, got ${decimal}`);
  }
  const buf = Buffer.alloc(32);
  buf.writeUInt32BE(Number(n), 28);
  return `0x${buf.toString("hex")}` as Hex;
}

/** SHA-256 over concatenated Bn254Fr field bytes (matches on-chain `compute_pub_signals_hash`). */
export function computePublicInputsHash(publicSignalsHex: string[]): Hex {
  if (!publicSignalsHex.length) {
    throw new Error("publicSignalsHex cannot be empty.");
  }
  const parts = publicSignalsHex.map((signal, index) => {
    if (!/^0x[0-9a-fA-F]+$/.test(signal)) {
      throw new Error(`publicSignalsHex[${index}] must be hex.`);
    }
    return Buffer.from(signal.slice(2), "hex");
  });
  if (parts.some((part) => part.length !== 32)) {
    throw new Error("each public signal must be 32 bytes.");
  }
  const digest = createHash("sha256").update(Buffer.concat(parts)).digest("hex");
  return `0x${digest}` as Hex;
}

/** Canonical claims hash for the attested path. */
export function computeAttestedClaimsHash(claims: AttestedClaims): Hex {
  const buf = Buffer.alloc(12);
  buf.writeUInt32LE(claims.age, 0);
  buf.writeUInt32LE(claims.countryCode, 4);
  buf.writeUInt32LE(claims.isHuman ? 1 : 0, 8);
  const digest = createHash("sha256").update(buf).digest("hex");
  return `0x${digest}` as Hex;
}

/**
 * Build attestation hashes for `record_attested_result`.
 * Pass Soroban XDR bytes for addresses/symbol (from stellar-cli or SDK encoding).
 */
export function computeAttestationHash(params: {
  proverXdr: Buffer;
  appIdXdr: Buffer;
  subjectXdr: Buffer;
  nullifier: Hex;
  publicInputsHash: Hex;
}): Hex {
  const nullifierBytes = Buffer.from(params.nullifier.slice(2), "hex");
  const pubHashBytes = Buffer.from(params.publicInputsHash.slice(2), "hex");
  if (nullifierBytes.length !== 32 || pubHashBytes.length !== 32) {
    throw new Error("nullifier and publicInputsHash must be 32 bytes.");
  }
  const digest = createHash("sha256")
    .update(params.proverXdr)
    .update(params.appIdXdr)
    .update(params.subjectXdr)
    .update(nullifierBytes)
    .update(pubHashBytes)
    .digest("hex");
  return `0x${digest}` as Hex;
}

export function buildAttestedPayload(params: {
  prover: string;
  nullifier: Hex;
  claims: AttestedClaims;
  proverXdr: Buffer;
  appIdXdr: Buffer;
  subjectXdr: Buffer;
}): AttestedPayload {
  const publicInputsHash = computeAttestedClaimsHash(params.claims);
  const attestationHash = computeAttestationHash({
    proverXdr: params.proverXdr,
    appIdXdr: params.appIdXdr,
    subjectXdr: params.subjectXdr,
    nullifier: params.nullifier,
    publicInputsHash,
  });
  return {
    prover: params.prover,
    nullifier: params.nullifier,
    publicInputsHash,
    attestationHash,
    claims: params.claims,
  };
}

/** Convert proof-adapter JSON into a contract-ready verification payload. */
export function fromAdapterPayload(
  adapter: AdapterOutput,
  nullifier: Hex,
  options?: { publicInputsHash?: Hex },
): VerificationPayload {
  const decimals =
    adapter.public_signals_hex?.length === adapter.public_signals_decimals.length
      ? null
      : adapter.public_signals_decimals;
  const publicSignals =
    adapter.public_signals_hex ??
    (decimals ?? []).map((value) => decimalToBn254FrHex(value));
  const publicInputsHash =
    options?.publicInputsHash ?? computePublicInputsHash(publicSignals);
  const claims = adapter.claims
    ? {
        age: adapter.claims.age,
        countryCode: adapter.claims.country_code,
        isHuman: adapter.claims.is_human,
      }
    : { age: 0, countryCode: 0, isHuman: false };
  if (!adapter.claims) {
    throw new Error("adapter output is missing claims.");
  }
  return {
    nullifier,
    publicInputsHash,
    vk: adapter.verification_key,
    proof: adapter.proof,
    publicSignals,
    claims,
  };
}

export function normalizeHex32(input: string): Hex {
  const sanitized = input.startsWith("0x") ? input.slice(2) : input;
  if (!/^[0-9a-fA-F]*$/.test(sanitized)) {
    throw new Error("Input is not valid hex.");
  }
  if (sanitized.length > 64) {
    throw new Error("Hex input exceeds 32 bytes.");
  }
  return `0x${sanitized.padStart(64, "0").toLowerCase()}` as Hex;
}

/**
 * @deprecated Use `InvokeContract` from your transaction builder (e.g. stellar-cli or @stellar/stellar-sdk Contract + rpc.Server.simulateTransaction).
 * The previous `soroban.invoke` JSON-RPC method is not part of the standard Stellar API.
 */
export function createSorobanRpcInvoke(
  _contractId: string,
  _transport: unknown,
): InvokeContract {
  return async () => {
    throw new Error(
      "createSorobanRpcInvoke is deprecated. Build transactions with @stellar/stellar-sdk Contract.call and rpc.Server, or invoke via stellar-cli.",
    );
  };
}

function assertNonEmpty(name: string, value: string): void {
  if (!value.trim()) {
    throw new Error(`${name} cannot be empty.`);
  }
}

function assertClaimsPolicy(policy: AppPolicy): void {
  if (!Array.isArray(policy.excludedCountries)) {
    throw new Error("excludedCountries must be an array.");
  }
  if (policy.minAge < 0) {
    throw new Error("minAge cannot be negative.");
  }
  if (policy.excludedCountries.length > 32) {
    throw new Error("excludedCountries cannot exceed 32 entries.");
  }
  if (policy.sanctionsEnabled) {
    throw new Error(
      "sanctionsEnabled is not supported until sanctions circuit integration is complete.",
    );
  }
}

function assertClaims(claims: AttestedClaims): void {
  if (claims.age < 0) {
    throw new Error("claims.age cannot be negative.");
  }
  if (claims.countryCode < 0) {
    throw new Error("claims.countryCode cannot be negative.");
  }
}

export function assertVerificationPayload(payload: VerificationPayload): void {
  assertClaims(payload.claims);
  assertHex32("nullifier", payload.nullifier);
  assertHex32("publicInputsHash", payload.publicInputsHash);
  if (!payload.publicSignals.length) {
    throw new Error("publicSignals cannot be empty.");
  }
  if (payload.publicSignals.length + 1 !== payload.vk.ic.length) {
    throw new Error("vk.ic length must equal publicSignals.length + 1.");
  }
  for (let i = 0; i < payload.publicSignals.length; i++) {
    assertHex32(`publicSignals[${i}]`, payload.publicSignals[i] as Hex);
  }
  assertProofInput("proof", payload.proof);
  assertVkInput("vk", payload.vk);
}

function assertHex32(name: string, value: string): void {
  assertNonEmpty(name, value);
  if (!/^0x[0-9a-fA-F]{64}$/.test(value)) {
    throw new Error(`${name} must be a 32-byte hex string (66 chars with 0x prefix).`);
  }
}

function assertProofInput(name: string, proof: ProofInput): void {
  if (!/^0x[0-9a-fA-F]{64}$/.test(proof.a)) {
    throw new Error(`${name}.a must be a G1 point (64 hex chars with 0x).`);
  }
  if (!/^0x[0-9a-fA-F]{256}$/.test(proof.b)) {
    throw new Error(`${name}.b must be a G2 point (256 hex chars with 0x).`);
  }
  if (!/^0x[0-9a-fA-F]{64}$/.test(proof.c)) {
    throw new Error(`${name}.c must be a G1 point (64 hex chars with 0x).`);
  }
}

function assertVkInput(name: string, vk: VerificationKeyInput): void {
  if (!/^0x[0-9a-fA-F]{64}$/.test(vk.alpha)) {
    throw new Error(`${name}.alpha must be a G1 point.`);
  }
  if (!/^0x[0-9a-fA-F]{256}$/.test(vk.beta)) {
    throw new Error(`${name}.beta must be a G2 point.`);
  }
  if (!/^0x[0-9a-fA-F]{256}$/.test(vk.gamma)) {
    throw new Error(`${name}.gamma must be a G2 point.`);
  }
  if (!/^0x[0-9a-fA-F]{256}$/.test(vk.delta)) {
    throw new Error(`${name}.delta must be a G2 point.`);
  }
  if (!Array.isArray(vk.ic) || vk.ic.length === 0) {
    throw new Error(`${name}.ic must be a non-empty array.`);
  }
  const g1Pattern = /^0x[0-9a-fA-F]{64}$/;
  for (let i = 0; i < vk.ic.length; i++) {
    if (!g1Pattern.test(vk.ic[i])) {
      throw new Error(`${name}.ic[${i}] must be a G1 point.`);
    }
  }
}
