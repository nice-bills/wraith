export type Hex = `0x${string}`;

export type VerificationSource =
  | { kind: "OnchainGroth16" }
  | { kind: "AttestedProver"; attestationHash: Hex };

export interface AppPolicy {
  owner: string;
  minAge: number;
  requireHumanity: boolean;
  sanctionsRoot: Hex;
  excludedCountries: number[];
  expirationWindow: number;
  sanctionsEnabled: boolean;
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
  publicSignals: string[];
  claims: AttestedClaims;
}

export interface AttestedPayload {
  prover: string;
  nullifier: Hex;
  publicInputsHash: Hex;
  attestationHash: Hex;
  claims: AttestedClaims;
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

export type JsonRpcTransport = (payload: unknown) => Promise<unknown>;

export function createSorobanRpcInvoke(
  contractId: string,
  transport: JsonRpcTransport,
): InvokeContract {
  assertNonEmpty("contractId", contractId);
  return async <T = unknown>(method: string, args: unknown[]): Promise<T> => {
    assertNonEmpty("method", method);
    const payload = {
      jsonrpc: "2.0",
      id: Date.now(),
      method: "soroban.invoke",
      params: {
        contractId,
        function: method,
        args,
      },
    };
    const raw = (await transport(payload)) as {
      result?: T;
      error?: { message?: string };
    };
    if (raw.error) {
      throw new Error(raw.error.message ?? "Soroban RPC call failed.");
    }
    if (!("result" in raw)) {
      throw new Error("Soroban RPC response is missing a result.");
    }
    return raw.result as T;
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
  for (let i = 0; i < payload.publicSignals.length; i++) {
    assertHex("publicSignals", i, payload.publicSignals[i]);
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

function assertHex(name: string, index: number, value: string): void {
  if (!/^0x[0-9a-fA-F]+$/.test(value)) {
    throw new Error(`${name}[${index}] must be a valid hex string.`);
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
  for (let i = 0; i < vk.ic.length; i++) {
    if (!/^0x[0-9a-fA-F]{64}$/.test(vk.ic[i])) {
      throw new Error(`${name}.ic[${i}] must be a G1 point.`);
    }
  }
}
