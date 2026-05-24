import { execFile } from "node:child_process";
import { mkdtempSync, writeFileSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { ROOT, stableAppId } from "./config.js";
import type { AttestedClaimsJson } from "./kyc.js";

const execFileAsync = promisify(execFile);

export type PreparedAttested = {
  app_id: string;
  prover: string;
  subject: string;
  nullifier: string;
  public_inputs_hash: string;
  attestation_hash: string;
  claims: AttestedClaimsJson;
};

export type SubmittedAttested = {
  app_id: string;
  subject: string;
  nullifier: string;
  verified: boolean;
};

function writeClaimsFile(claims: AttestedClaimsJson): string {
  const dir = mkdtempSync(join(tmpdir(), "wraith-claims-"));
  const path = join(dir, "claims.json");
  writeFileSync(path, JSON.stringify(claims));
  return path;
}

export async function prepareAttestedRecord(opts: {
  subject: string;
  claims: AttestedClaimsJson;
  appId?: string;
  nullifier?: string;
  skipRegister?: boolean;
}): Promise<PreparedAttested> {
  const claimsPath = writeClaimsFile(opts.claims);
  try {
    const args = buildAttestedArgs(claimsPath, opts, ["--prepare-only", "--json"]);
    const { stdout } = await runAttestedReady(args);
    return JSON.parse(stdout.trim()) as PreparedAttested;
  } finally {
    safeUnlink(claimsPath);
  }
}

export async function submitAttestedRecord(opts: {
  subject: string;
  claims: AttestedClaimsJson;
  appId?: string;
  nullifier?: string;
  skipRegister?: boolean;
}): Promise<SubmittedAttested> {
  const claimsPath = writeClaimsFile(opts.claims);
  try {
    const args = buildAttestedArgs(claimsPath, opts, ["--json"]);
    const { stdout } = await runAttestedReady(args);
    return JSON.parse(stdout.trim()) as SubmittedAttested;
  } finally {
    safeUnlink(claimsPath);
  }
}

function buildAttestedArgs(
  claimsPath: string,
  opts: {
    subject: string;
    appId?: string;
    nullifier?: string;
    skipRegister?: boolean;
  },
  extra: string[],
): string[] {
  const args = [
    join(ROOT, "scripts/attested-ready.sh"),
    claimsPath,
    "--subject",
    opts.subject,
    "--app",
    opts.appId ?? stableAppId(),
  ];
  if (opts.nullifier) {
    args.push("--nullifier", opts.nullifier.replace(/^0x/i, ""));
  }
  if (opts.skipRegister) {
    args.push("--no-register");
  }
  return [...args, ...extra];
}

async function runAttestedReady(args: string[]) {
  if (!process.env.SOROBAN_SOURCE_ACCOUNT) {
    throw new Error("SOROBAN_SOURCE_ACCOUNT is required to submit attested records");
  }
  return execFileAsync("bash", args, {
    cwd: ROOT,
    env: {
      ...process.env,
      STABLE_APP_ID: stableAppId(),
    },
    maxBuffer: 10 * 1024 * 1024,
  });
}

function safeUnlink(path: string) {
  try {
    unlinkSync(path);
  } catch {
    /* ignore */
  }
}
